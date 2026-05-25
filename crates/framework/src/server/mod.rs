use std::collections::HashMap;
use std::sync::Arc;
use crate::http::{Method, Request, Response, StatusCode};
use crate::middleware::MiddlewareChain;

pub mod server;

pub use server::{Server, is_shutdown_requested, request_shutdown};

pub type IoResult<T> = Result<T, String>;
pub type Handler = Arc<dyn Fn(Request) -> Response + Send + Sync>;

#[derive(Clone)]
struct RouteEntry {
    method: Method,
    path: String,
    handler: Handler,
    middlewares: Option<Arc<MiddlewareChain>>,
}

#[derive(Default)]
pub struct Router {
    routes: Vec<RouteEntry>,
    global_middlewares: Option<Arc<MiddlewareChain>>,
    not_found: Option<Handler>,
}

impl Router {
    pub fn new() -> Self { Router { routes: Vec::new(), global_middlewares: None, not_found: None } }

    fn add_route(mut self, method: Method, path: &str, h: Handler, mw: Option<MiddlewareChain>) -> Self {
        self.routes.push(RouteEntry { method, path: path.into(), handler: h, middlewares: mw.map(Arc::new) });
        self
    }

    pub fn get(self, path: &str, h: Handler) -> Self { self.add_route(Method::Get, path, h, None) }
    pub fn post(self, path: &str, h: Handler) -> Self { self.add_route(Method::Post, path, h, None) }
    pub fn any(self, path: &str, h: Handler) -> Self {
        let mut r = self;
        for m in &[Method::Get, Method::Post, Method::Put, Method::Delete] {
            r = r.add_route(m.clone(), path, h.clone(), None);
        }
        r
    }

    pub fn get_with(self, path: &str, h: Handler, mw: MiddlewareChain) -> Self { self.add_route(Method::Get, path, h, Some(mw)) }
    pub fn post_with(self, path: &str, h: Handler, mw: MiddlewareChain) -> Self { self.add_route(Method::Post, path, h, Some(mw)) }

    pub fn with_global_middleware(mut self, mw: MiddlewareChain) -> Self {
        self.global_middlewares = Some(Arc::new(mw)); self
    }

    pub fn static_files(self, url_prefix: &str, dir: &str) -> Self {
        let prefix = url_prefix.trim_end_matches('/').to_string();
        let dir = dir.to_string();
        self.get(&format!("{}/*path", prefix), Arc::new(move |req| {
            let file_path = req.param("path").unwrap_or("index.html");
            if file_path.contains("..") { return Response::new(StatusCode::FORBIDDEN).with_body("Forbidden"); }
            let full_path = std::path::Path::new(&dir).join(file_path);
            match std::fs::read(&full_path) {
                Ok(bytes) => {
                    let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    Response::ok().with_header("content-type", mime_type(ext)).with_body(bytes)
                }
                Err(_) => Response::new(StatusCode::NOT_FOUND).with_body("Not Found"),
            }
        }))
    }

    pub fn spa_fallback(mut self, file: &str) -> Self {
        let file = file.to_string();
        self.not_found = Some(Arc::new(move |_req| {
            match std::fs::read_to_string(&file) {
                Ok(html) => Response::html(&html),
                Err(_) => Response::new(StatusCode::NOT_FOUND).with_body("Not Found"),
            }
        }));
        self
    }

    pub fn extend_from_registry(mut self) -> Self {
        for (m, p, h) in crate::route::take_handlers() {
            self.routes.push(RouteEntry { method: m, path: p, handler: h, middlewares: None });
        }
        self
    }

    fn find_route(&self, m: &Method, p: &str) -> Option<&RouteEntry> {
        self.routes.iter().find(|r| r.method == *m && path_matches(&r.path, p))
    }

    pub fn handle(&self, mut request: Request) -> Response {
        let path = request.uri.path.clone();
        if let Some(entry) = self.find_route(&request.method, &path) {
            request.params = route_params(&entry.path, &request.uri.path);
            let handler = entry.handler.clone();
            let handler_fn = move |req: &mut Request| -> Response { handler(std::mem::take(req)) };
            if let Some(ref chain) = entry.middlewares {
                chain.apply(&mut request, &handler_fn)
            } else if let Some(ref global) = self.global_middlewares {
                global.apply(&mut request, &handler_fn)
            } else {
                handler_fn(&mut request)
            }
        } else {
            self.not_found.as_ref()
                .map_or_else(|| Response::not_found().with_body("404"), |h| h(request))
        }
    }
}

fn path_matches(p: &str, r: &str) -> bool {
    let pp: Vec<&str> = p.split('/').collect();
    let rp: Vec<&str> = r.split('/').collect();
    if let Some(wild_pos) = pp.iter().position(|&s| s == "*") && pp.len() - 1 <= rp.len() {
        return pp[..wild_pos].iter().zip(rp.iter()).all(|(a, b)| a.starts_with(':') || a == b);
    }
    pp.len() == rp.len() && pp.iter().zip(rp.iter()).all(|(a, b)| a.starts_with(':') || a == b)
}

fn route_params(pat: &str, req: &str) -> HashMap<String, String> {
    let mut p = HashMap::new();
    let pp: Vec<&str> = pat.split('/').collect();
    let rp: Vec<&str> = req.split('/').collect();
    for (a, b) in pp.iter().zip(rp.iter()) {
        if *a == "*" { p.insert("path".into(), rp[pp.iter().position(|&s| s == "*").unwrap()..].join("/")); break; }
        if a.starts_with(':') { p.insert(a[1..].into(), (*b).into()); }
    }
    p
}

fn mime_type(ext: &str) -> &'static str {
    match ext {
        "html" | "htm" => "text/html; charset=utf-8", "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8", "json" => "application/json",
        "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif",
        "svg" => "image/svg+xml", "ico" => "image/x-icon", "wasm" => "application/wasm",
        "woff2" => "font/woff2", "woff" => "font/woff", "ttf" => "font/ttf", "otf" => "font/otf",
        "pdf" => "application/pdf", "txt" => "text/plain; charset=utf-8", "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

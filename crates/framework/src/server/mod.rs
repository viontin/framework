use std::collections::HashMap;
use std::sync::Arc;
use crate::http::{Method, Request, Response, StatusCode};
use crate::middleware::MiddlewareChain;

pub mod server;

pub use server::{Server, is_shutdown_requested, request_shutdown};

pub type IoResult<T> = Result<T, String>;
pub type Handler = Arc<dyn Fn(Request) -> Response + Send + Sync>;

/// Extension type stored on Request after route matching.
/// Used by `route::current_name()` to retrieve the matched route name.
#[derive(Debug, Clone)]
pub struct RouteName(pub String);

#[derive(Clone)]
struct RouteEntry {
    method: Method,
    path: String,
    handler: Handler,
    middlewares: Option<Arc<MiddlewareChain>>,
    name: Option<String>,
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
        self.routes.push(RouteEntry { method, path: path.into(), handler: h, middlewares: mw.map(Arc::new), name: None });
        self
    }

    /// Internal route addition without middleware wrapping (used by route module).
    pub fn add_route_internal(mut self, method: Method, path: &str, h: Handler) -> Self {
        self.routes.push(RouteEntry { method, path: path.into(), handler: h, middlewares: None, name: None });
        self
    }

    /// Push a route directly (used by route module's build_router).
    /// Takes `&mut self` instead of consuming self.
    pub fn push_route(&mut self, method: Method, path: String, handler: Handler) {
        self.routes.push(RouteEntry { method, path, handler, middlewares: None, name: None });
    }

    /// Push a route with an optional name for URL generation.
    pub fn push_named_route(&mut self, method: Method, path: String, handler: Handler, name: Option<String>) {
        self.routes.push(RouteEntry { method, path, handler, middlewares: None, name });
    }

    pub fn get(self, path: &str, h: Handler) -> Self { self.add_route(Method::Get, path, h, None) }
    pub fn post(self, path: &str, h: Handler) -> Self { self.add_route(Method::Post, path, h, None) }
    pub fn put(self, path: &str, h: Handler) -> Self { self.add_route(Method::Put, path, h, None) }
    pub fn patch(self, path: &str, h: Handler) -> Self { self.add_route(Method::Patch, path, h, None) }
    pub fn delete(self, path: &str, h: Handler) -> Self { self.add_route(Method::Delete, path, h, None) }
    pub fn any(self, path: &str, h: Handler) -> Self {
        let mut r = self;
        for m in &[Method::Get, Method::Post, Method::Put, Method::Delete] {
            r = r.add_route(m.clone(), path, h.clone(), None);
        }
        r
    }

    pub fn get_with(self, path: &str, h: Handler, mw: MiddlewareChain) -> Self { self.add_route(Method::Get, path, h, Some(mw)) }
    pub fn post_with(self, path: &str, h: Handler, mw: MiddlewareChain) -> Self { self.add_route(Method::Post, path, h, Some(mw)) }

    pub fn extend(mut self, other: Router) -> Self {
        self.routes.extend(other.routes);
        self
    }

    pub fn is_empty_having_routes(&self) -> bool { self.routes.is_empty() }

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
                    Response::ok().header("content-type", mime_type(ext)).with_body(bytes)
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

    pub fn extend_from_registry(self) -> Self {
        // Routes are now built by RouteProvider::boot()
        // If routes were registered directly, try to retrieve the router
        if let Some(router) = crate::route::take_router() {
            return router;
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
            if let Some(ref name) = entry.name {
                request.set_extension(RouteName(name.clone()));
            }
            let handler = entry.handler.clone();
            let handler_fn = move |req: &mut Request| -> Response { handler(std::mem::take(req)) };

            let has_route_mw = entry.middlewares.is_some();
            let has_global_mw = self.global_middlewares.is_some();

            match (has_global_mw, has_route_mw) {
                (true, true) => {
                    let global = self.global_middlewares.as_ref().unwrap();
                    let route_mw = entry.middlewares.as_ref().unwrap();
                    let global_handler = move |req: &mut Request| -> Response {
                        route_mw.apply(req, &handler_fn)
                    };
                    global.apply(&mut request, &global_handler)
                }
                (true, false) => {
                    let global = self.global_middlewares.as_ref().unwrap();
                    global.apply(&mut request, &handler_fn)
                }
                (false, true) => {
                    let route_mw = entry.middlewares.as_ref().unwrap();
                    route_mw.apply(&mut request, &handler_fn)
                }
                (false, false) => {
                    handler_fn(&mut request)
                }
            }
        } else if self.routes.iter().any(|r| path_matches(&r.path, &path)) {
            // Path matches but method doesn't
            Response::new(StatusCode::METHOD_NOT_ALLOWED).with_body("Method Not Allowed")
        } else {
            self.not_found.as_ref()
                .map_or_else(|| Response::not_found().with_body("404"), |h| h(std::mem::take(&mut request)))
        }
    }
}

pub(crate) fn is_wildcard(s: &str) -> bool {
    s == "*" || s.starts_with('*')
}

pub(crate) fn wildcard_name(s: &str) -> &str {
    if s == "*" { "path" } else { &s[1..] }
}

pub(crate) fn path_matches(p: &str, r: &str) -> bool {
    let pp: Vec<&str> = p.split('/').collect();
    let rp: Vec<&str> = r.split('/').collect();
    if let Some(wild_pos) = pp.iter().position(|&s| is_wildcard(s)) && pp.len() - 1 <= rp.len() {
        return pp[..wild_pos].iter().zip(rp.iter()).all(|(a, b)| a.starts_with(':') || a == b);
    }
    pp.len() == rp.len() && pp.iter().zip(rp.iter()).all(|(a, b)| a.starts_with(':') || a == b)
}

pub(crate) fn route_params(pat: &str, req: &str) -> HashMap<String, String> {
    let mut p = HashMap::new();
    let pp: Vec<&str> = pat.split('/').collect();
    let rp: Vec<&str> = req.split('/').collect();
    for (a, b) in pp.iter().zip(rp.iter()) {
        if is_wildcard(a) {
            if let Some(wild_idx) = pp.iter().position(|&s| is_wildcard(s)) {
                p.insert(wildcard_name(a).into(), rp[wild_idx..].join("/"));
            }
            break;
        }
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

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use crate::http::{Method, Request, Response, StatusCode, Headers, Uri};
use crate::middleware::MiddlewareChain;

type IoResult<T> = Result<T, String>;
pub type Handler = Arc<dyn Fn(Request) -> Response + Send + Sync>;

/// A route entry with an optional middleware chain.
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

    /// Register a route with optional middleware chain.
    fn add_route(mut self, method: Method, path: &str, h: Handler, mw: Option<MiddlewareChain>) -> Self {
        self.routes.push(RouteEntry {
            method, path: path.into(), handler: h,
            middlewares: mw.map(Arc::new),
        });
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

    /// Register a route with middleware.
    pub fn get_with(self, path: &str, h: Handler, mw: MiddlewareChain) -> Self { self.add_route(Method::Get, path, h, Some(mw)) }
    pub fn post_with(self, path: &str, h: Handler, mw: MiddlewareChain) -> Self { self.add_route(Method::Post, path, h, Some(mw)) }

    /// Set global middlewares applied to every route.
    pub fn with_global_middleware(mut self, mw: MiddlewareChain) -> Self {
        self.global_middlewares = Some(Arc::new(mw));
        self
    }

    /// Serve static files from a directory at a URL prefix.
    ///
    /// ```rust
    /// Router::new().static_files("/assets", "public");
    /// // GET /assets/style.css → serves public/style.css
    /// ```
    pub fn static_files(self, url_prefix: &str, dir: &str) -> Self {
        let prefix = url_prefix.trim_end_matches('/').to_string();
        let dir = dir.to_string();
        self.get(&format!("{}/*path", prefix), Arc::new(move |req| {
            let file_path = req.param("path").unwrap_or("index.html");
            // Prevent path traversal
            if file_path.contains("..") {
                return Response::new(StatusCode::FORBIDDEN).with_body("Forbidden");
            }
            let full_path = std::path::Path::new(&dir).join(file_path);
            match std::fs::read(&full_path) {
                Ok(bytes) => {
                    let mime = mime_type(&full_path);
                    Response::ok()
                        .with_header("content-type", mime)
                        .with_body(bytes)
                }
                Err(_) => Response::new(StatusCode::NOT_FOUND).with_body("Not Found"),
            }
        }))
    }

    /// Set a fallback handler for SPA client-side routing.
    /// All unmatched GET requests will serve this file (e.g., `index.html`).
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
            self.routes.push(RouteEntry {
                method: m, path: p, handler: h,
                middlewares: None,
            });
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

            // Wrap handler to take &mut Request
            let handler_fn = move |req: &mut Request| -> Response {
                handler(std::mem::take(req))
            };

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

    // Wildcard: * matches any remaining path
    if let Some(wild_pos) = pp.iter().position(|&s| s == "*")
        && pp.len() - 1 <= rp.len() {
            return pp[..wild_pos].iter().zip(rp.iter()).all(|(a, b)| a.starts_with(':') || a == b);
        }

    pp.len() == rp.len() && pp.iter().zip(rp.iter()).all(|(a, b)| a.starts_with(':') || a == b)
}

fn route_params(pat: &str, req: &str) -> HashMap<String, String> {
    let mut p = HashMap::new();
    let pp: Vec<&str> = pat.split('/').collect();
    let rp: Vec<&str> = req.split('/').collect();

    for (a, b) in pp.iter().zip(rp.iter()) {
        if *a == "*" {
            // Wildcard — collect remaining path
            let idx = pp.iter().position(|&s| s == "*").unwrap();
            let rest = rp[idx..].join("/");
            p.insert("path".into(), rest);
            break;
        }
        if a.starts_with(':') {
            p.insert(a[1..].into(), (*b).into());
        }
    }
    p
}

use std::sync::atomic::{AtomicBool, Ordering};

/// Global shutdown flag — set to true when SIGTERM/SIGINT is received.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Check if a graceful shutdown has been requested.
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

/// Request graceful shutdown (can be called from signal handlers or tests).
pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

pub struct Server { router: Arc<Router>, }
impl Server {
    pub fn new(router: Router) -> Self { Server { router: Arc::new(router) } }

    pub fn run(&self, addr: &str) -> IoResult<()> {
        // Register signal handler for graceful shutdown
        #[cfg(feature = "shutdown")]
        {
            if let Err(e) = ctrlc::set_handler(move || {
                eprintln!("\n  [server] Shutdown requested...");
                request_shutdown();
            }) {
                eprintln!("  [server] Warning: could not set signal handler: {}", e);
            }
        }

        let listener = TcpListener::bind(addr).map_err(|e| format!("Bind failed: {}", e))?;
        // Set a timeout on accept so we can check the shutdown flag periodically
        listener.set_nonblocking(true).ok();
        println!("  Server on http://{}", addr);

        loop {
            // Check for shutdown request
            if is_shutdown_requested() {
                println!("  [server] Stopping accept loop...");
                break;
            }

            match listener.accept() {
                Ok((s, _)) => {
                    let r = self.router.clone();
                    crate::middleware::set_connection_timeout(&s, 30);
                    thread::spawn(move || {
                        if let Err(e) = handle_conn(s, &r) {
                            eprintln!("  [server] {}", e);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => eprintln!("  [server] {}", e),
            }
        }

        println!("  [server] Graceful shutdown complete.");
        Ok(())
    }
}

/// Determine MIME type from file extension.
fn mime_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

fn io_err<T>(r: std::io::Result<T>) -> IoResult<T> { r.map_err(|e| e.to_string()) }

fn handle_conn(mut stream: TcpStream, router: &Router) -> IoResult<()> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new(); io_err(reader.read_line(&mut request_line))?;
    let request_line = request_line.trim();
    if request_line.is_empty() { return Ok(()); }
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let uri = Uri::parse(parts.get(1).unwrap_or(&""))?;
    let mut headers = Headers::new();
    loop {
        let mut line = String::new(); io_err(reader.read_line(&mut line))?;
        let line = line.trim();
        if line.is_empty() { break; }
        if let Some(eq) = line.find(':') { headers.set(line[..eq].trim(), line[eq+1..].trim()); }
    }
    let mut body = Vec::new();
    if let Some(len) = headers.content_length()
        && len > 0 { let mut buf = vec![0u8; len as usize]; io_err(reader.read_exact(&mut buf))?; body = buf; }
    let request = Request::new(Method::parse(parts[0]), uri, headers, body);
    let response = router.handle(request);
    io_err(stream.write_all(&response.to_raw()))?; io_err(stream.flush())?; Ok(())
}

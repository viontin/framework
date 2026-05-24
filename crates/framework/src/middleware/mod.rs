//! Middleware — request pipeline with pluggable handlers.
//!
//! ```rust
//! use viontin_framework::middleware::{Middleware, MiddlewareChain};
//! use viontin_framework::http::{Request, Response};
//!
//! #[derive(Debug)]
//! struct Logger;
//!
//! impl Middleware for Logger {
//!     fn handle(&self, req: &mut Request, next: &dyn Fn(&mut Request) -> Response) -> Response {
//!         println!("[mw] {} {}", req.method, req.uri.path);
//!         let res = next(req);
//!         println!("[mw] -> {}", res.status);
//!         res
//!     }
//! }
//! ```

use std::fmt;
use crate::http::{Request, Response, StatusCode};

/// Next handler in the middleware chain — calls the next middleware or the final route handler.
pub type Next<'a> = &'a dyn Fn(&mut Request) -> Response;

/// Middleware trait — intercept requests before they reach the route handler.
pub trait Middleware: fmt::Debug + Send + Sync {
    fn handle(&self, req: &mut Request, next: Next) -> Response;
}

/// Wraps a closure as a Middleware.
#[derive(Debug)]
struct ClosureMiddleware<F: Fn(&mut Request, Next) -> Response> {
    f: F,
}

impl<F: Fn(&mut Request, Next) -> Response + fmt::Debug + Send + Sync> Middleware for ClosureMiddleware<F> {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        (self.f)(req, next)
    }
}

/// Ordered chain of middlewares that wraps a route handler.
#[derive(Debug, Default)]
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        MiddlewareChain { middlewares: Vec::new() }
    }

    /// Add a middleware to the chain (applied in registration order).
    pub fn add(&mut self, m: impl Middleware + 'static) {
        self.middlewares.push(Box::new(m));
    }

    /// Add a boxed middleware.
    pub fn add_dyn(&mut self, m: Box<dyn Middleware + 'static>) {
        self.middlewares.push(m);
    }

    /// Add a closure as middleware (convenience).
    pub fn add_fn<F: Fn(&mut Request, Next) -> Response + Send + Sync + 'static>(&mut self, f: F)
    where F: fmt::Debug
    {
        self.middlewares.push(Box::new(ClosureMiddleware { f }));
    }

    /// Apply the middleware chain around a route handler.
    pub fn apply(&self, req: &mut Request, handler: impl Fn(&mut Request) -> Response) -> Response {
        if self.middlewares.is_empty() {
            return handler(req);
        }
        self.run(0, req, &handler)
    }

    fn run(&self, index: usize, req: &mut Request, handler: &dyn Fn(&mut Request) -> Response) -> Response {
        if index >= self.middlewares.len() {
            return handler(req);
        }
        let m = &self.middlewares[index];
        m.handle(req, &|req: &mut Request| self.run(index + 1, req, handler))
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }
}

// ── CorsMiddleware ──

/// Middleware for Cross-Origin Resource Sharing (CORS).
///
/// Adds CORS headers to every response. Configure allowed origins,
/// methods, headers, and whether credentials are allowed.
///
/// # Example
///
/// ```rust,ignore
/// use viontin_framework::middleware::CorsMiddleware;
///
/// boot()
///     .middleware(CorsMiddleware::permissive())  // allow all origins
///     .serve(":3000");
/// ```
#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u64>,
}

impl CorsMiddleware {
    /// Allow all origins, methods, and headers (development default).
    pub fn permissive() -> Self {
        CorsMiddleware {
            allowed_origins: vec!["*".into()],
            allowed_methods: vec![
                "GET".into(), "POST".into(), "PUT".into(),
                "PATCH".into(), "DELETE".into(), "OPTIONS".into(), "HEAD".into(),
            ],
            allowed_headers: vec!["*".into()],
            expose_headers: Vec::new(),
            allow_credentials: false,
            max_age: None,
        }
    }

    /// Restrict to a specific origin (production).
    pub fn origin(origin: &str) -> Self {
        CorsMiddleware::permissive()
            .with_allowed_origins(&[origin])
    }

    pub fn with_allowed_origins(mut self, origins: &[&str]) -> Self {
        self.allowed_origins = origins.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_allowed_methods(mut self, methods: &[&str]) -> Self {
        self.allowed_methods = methods.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_allowed_headers(mut self, headers: &[&str]) -> Self {
        self.allowed_headers = headers.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }
}

impl Default for CorsMiddleware {
    fn default() -> Self { Self::permissive() }
}

impl Middleware for CorsMiddleware {
    fn handle(&self, req: &mut Request, next: &dyn Fn(&mut Request) -> Response) -> Response {
        // Handle preflight OPTIONS request
        if req.method.as_str() == "OPTIONS" {
            let mut res = Response::new(StatusCode::NO_CONTENT);
            self.apply_headers(&mut res);
            return res;
        }

        let mut res = next(req);
        self.apply_headers(&mut res);
        res
    }
}

impl CorsMiddleware {
    fn apply_headers(&self, res: &mut Response) {
        let origin = self.allowed_origins.join(", ");
        res.headers.set("Access-Control-Allow-Origin", &origin);
        res.headers.set("Access-Control-Allow-Methods", &self.allowed_methods.join(", "));
        res.headers.set("Access-Control-Allow-Headers", &self.allowed_headers.join(", "));
        if !self.expose_headers.is_empty() {
            res.headers.set("Access-Control-Expose-Headers", &self.expose_headers.join(", "));
        }
        if self.allow_credentials {
            res.headers.set("Access-Control-Allow-Credentials", "true");
        }
        if let Some(age) = self.max_age {
            res.headers.set("Access-Control-Max-Age", &age.to_string());
        }
    }
}

// ── PanicRecoveryMiddleware ──

/// Middleware that catches panics from downstream handlers and middlewares,
/// returning a 500 Internal Server Error instead of crashing the connection.
///
/// # Example
///
/// ```rust,ignore
/// use viontin_framework::middleware::PanicRecovery;
///
/// boot()
///     .middleware(PanicRecovery)
///     .serve(":3000");
/// ```
#[derive(Debug)]
pub struct PanicRecovery;

impl Middleware for PanicRecovery {
    fn handle(&self, req: &mut Request, next: &dyn Fn(&mut Request) -> Response) -> Response {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let result = catch_unwind(AssertUnwindSafe(|| next(req)));
        match result {
            Ok(res) => res,
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".into()
                };
                eprintln!("  [panic] {} {} — {}", req.method, req.uri.path, msg);
                let mut res = Response::html("Internal Server Error");
                res.status = StatusCode::SERVER_ERROR;
                res
            }
        }
    }
}

// ── HealthCheck ──

/// Built-in health check handler for `/healthz` (liveness) and `/readyz` (readiness).
pub fn healthz_handler(_req: Request) -> Response {
    let body = serde_json::json!({
        "status": "ok",
        "service": "viontin",
    }).to_string();
    Response::text(&body).with_header("content-type", "application/json")
}

pub fn readyz_handler(_req: Request) -> Response {
    let body = serde_json::json!({
        "status": "ready",
        "service": "viontin",
    }).to_string();
    Response::text(&body).with_header("content-type", "application/json")
}

// ── Static Files ──

/// Serve static files from a directory.
///
/// # Example
///
/// ```rust,ignore
/// use viontin_framework::middleware::static_files_handler;
///
/// let router = Router::new()
///     .get("/assets/:path", Arc::new(static_files_handler("public")));
/// ```
pub fn static_files_handler(root: &'static str) -> impl Fn(Request) -> Response {
    move |req: Request| {
        let path = req.param("path").unwrap_or("index.html");
        let full_path = format!("{}/{}", root, path.trim_start_matches('/'));
        match std::fs::read(&full_path) {
            Ok(body) => {
                let ext = full_path.rsplit('.').next().unwrap_or("");
                let mime = mime_for(ext);
                Response::text(&String::from_utf8_lossy(&body))
                    .with_header("content-type", mime)
            }
            Err(_) => {
                let mut res = Response::html("Not Found");
                res.status = StatusCode::NOT_FOUND;
                res
            }
        }
    }
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

// ── Request Timeout (sync) ──

/// Set a read timeout on the TCP stream for the sync server.
/// Call this per-connection before handling.
///
/// ```rust,ignore
/// use std::net::TcpStream;
/// use viontin_framework::middleware::set_connection_timeout;
///
/// set_connection_timeout(&stream, 30); // 30 second timeout
/// ```
pub fn set_connection_timeout(stream: &std::net::TcpStream, seconds: u64) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(seconds)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(seconds)));
}

// ── RequestIdMiddleware ──

/// Middleware that attaches a unique request ID to every request.
///
/// The ID is generated as a timestamp-based hex string and stored
/// in both the request headers (`X-Request-Id`) and the response
/// headers (`X-Request-Id`). This enables request tracing across
/// services and log correlation.
///
/// # Example
///
/// ```rust,ignore
/// use viontin_framework::middleware::RequestId;
///
/// boot()
///     .middleware(RequestId)
///     .serve(":3000");
/// ```
#[derive(Debug)]
pub struct RequestId;

impl Middleware for RequestId {
    fn handle(&self, req: &mut Request, next: &dyn Fn(&mut Request) -> Response) -> Response {
        let id = generate_id();
        req.headers.set("X-Request-Id", &id);
        let mut res = next(req);
        res.headers.set("X-Request-Id", &id);
        res
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    format!("{:016x}", nanos)
}

// ── RateLimitMiddleware ──

/// Middleware that applies rate limiting to requests.
///
/// Uses the `RateLimiter` facade internally. When a client exceeds
/// the allowed number of requests, a 429 Too Many Requests response
/// is returned immediately without calling the downstream handler.
///
/// # Example
///
/// ```rust,ignore
/// use viontin_framework::middleware::RateLimitMiddleware;
///
/// boot()
///     .middleware(RateLimitMiddleware::new("global", 100, 60))  // 100 req/min
///     .serve(":3000");
/// ```
#[derive(Debug)]
pub struct RateLimitMiddleware {
    key_prefix: String,
    max_attempts: u64,
    decay_seconds: u64,
}

impl RateLimitMiddleware {
    pub fn new(key_prefix: &str, max_attempts: u64, decay_seconds: u64) -> Self {
        RateLimitMiddleware {
            key_prefix: key_prefix.into(),
            max_attempts,
            decay_seconds,
        }
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle(&self, req: &mut Request, next: &dyn Fn(&mut Request) -> Response) -> Response {
        let key = format!("{}:{}", self.key_prefix, req.uri.path);

        if crate::rate::too_many_attempts(&key, self.max_attempts) {
            let retry_after = crate::rate::available_in(&key);
            let mut res = Response::html("Too Many Requests")
                .with_header("Retry-After", &retry_after.to_string());
            res.status = StatusCode(429);
            return res;
        }

        crate::rate::hit(&key, self.decay_seconds);
        next(req)
    }
}

//! Middleware — request pipeline with pluggable handlers.
//!
//! ```ignore
//! use viontin_framework::middleware::{Middleware, MiddlewareChain};
//! use viontin_framework::http::{Request, Response};
//!
//! #[derive(Debug)]
//! struct Logger;
//!
//! impl Middleware for Logger {
//!     fn name(&self) -> &'static str { "logger" }
//!     fn handle(&self, req: &mut Request, next: viontin_framework::middleware::Next) -> Response {
//!         let res = next(req);
//!         res
//!     }
//! }
//! ```

pub mod auth;
pub mod cors;
pub mod panic;
pub mod health;
pub mod static_files;
pub mod timeout;
pub mod request_id;
pub mod rate_limit;
pub mod error_page;

pub use auth::AuthMiddleware;
pub use cors::CorsMiddleware;
pub use panic::PanicRecovery;
pub use health::{healthz_handler, readyz_handler};
pub use static_files::static_files_handler;
pub use timeout::set_connection_timeout;
pub use request_id::RequestId;
pub use rate_limit::RateLimitMiddleware;
pub use error_page::ErrorPageRenderer;

// ── Global Middleware Registry ──

use std::fmt;
use std::sync::{Mutex, OnceLock};
use crate::http::{Request, Response};

static GLOBAL_MIDDLEWARE: OnceLock<Mutex<MiddlewareChain>> = OnceLock::new();

fn global() -> &'static Mutex<MiddlewareChain> {
    GLOBAL_MIDDLEWARE.get_or_init(|| Mutex::new(MiddlewareChain::new()))
}

/// Add a middleware to the global chain. All registered middleware
/// is applied to every route. Call this from a ServiceProvider's
/// `boot()` or `register()` method.
pub fn add_global(mw: impl Middleware + 'static) {
    if let Ok(mut g) = global().lock() {
        g.add(mw);
    }
}

/// Add a boxed middleware to the global chain.
pub fn add_global_dyn(mw: Box<dyn Middleware + 'static>) {
    if let Ok(mut g) = global().lock() {
        g.add_dyn(mw);
    }
}

/// Take the global middleware chain (consumes it). Used by Boot::run().
/// Returns an empty chain if no middleware was registered.
pub fn take_global() -> MiddlewareChain {
    global().lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default()
}

/// Next handler in the middleware chain — calls the next middleware or the final route handler.
pub type Next<'a> = &'a dyn Fn(&mut Request) -> Response;

/// Middleware trait — intercept requests before they reach the route handler.
pub trait Middleware: fmt::Debug + Send + Sync {
    fn handle(&self, req: &mut Request, next: Next) -> Response;
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

    pub fn add(&mut self, m: impl Middleware + 'static) {
        self.middlewares.push(Box::new(m));
    }

    pub fn add_dyn(&mut self, m: Box<dyn Middleware + 'static>) {
        self.middlewares.push(m);
    }

    pub fn add_fn<F: Fn(&mut Request, Next) -> Response + Send + Sync + 'static>(&mut self, name: &'static str, f: F) {
        struct NamedClosure<F> {
            name: &'static str,
            f: F,
        }
        impl<F: Fn(&mut Request, Next) -> Response + Send + Sync> fmt::Debug for NamedClosure<F> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Middleware({})", self.name)
            }
        }
        impl<F: Fn(&mut Request, Next) -> Response + Send + Sync + 'static> Middleware for NamedClosure<F> {
            fn handle(&self, req: &mut Request, next: Next) -> Response { (self.f)(req, next) }
        }
        self.middlewares.push(Box::new(NamedClosure { name, f }));
    }

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

    pub fn is_empty(&self) -> bool { self.middlewares.is_empty() }
    pub fn len(&self) -> usize { self.middlewares.len() }
}

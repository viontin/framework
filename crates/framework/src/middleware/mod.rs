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
//!     fn handle(&self, req: &mut Request, next: Next) -> Response {
//!         println!("[mw] {} {}", req.method, req.uri.path);
//!         let res = next(req);
//!         println!("[mw] -> {}", res.status);
//!         res
//!     }
//! }
//! ```

pub mod cors;
pub mod panic;
pub mod health;
pub mod static_files;
pub mod timeout;
pub mod request_id;
pub mod rate_limit;

pub use cors::CorsMiddleware;
pub use panic::PanicRecovery;
pub use health::{healthz_handler, readyz_handler};
pub use static_files::static_files_handler;
pub use timeout::set_connection_timeout;
pub use request_id::RequestId;
pub use rate_limit::RateLimitMiddleware;

use std::fmt;
use crate::http::{Request, Response};

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

    pub fn add(&mut self, m: impl Middleware + 'static) {
        self.middlewares.push(Box::new(m));
    }

    pub fn add_dyn(&mut self, m: Box<dyn Middleware + 'static>) {
        self.middlewares.push(m);
    }

    pub fn add_fn<F: Fn(&mut Request, Next) -> Response + Send + Sync + 'static>(&mut self, f: F)
    where F: fmt::Debug
    {
        self.middlewares.push(Box::new(ClosureMiddleware { f }));
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

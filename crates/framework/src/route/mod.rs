pub mod provider;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use crate::http::Method;
use crate::server::Handler;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RouteMethod { Get, Post, Put, Patch, Delete, Head, Options, }
impl std::fmt::Display for RouteMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self { RouteMethod::Get => "GET", RouteMethod::Post => "POST",
            RouteMethod::Put => "PUT", RouteMethod::Patch => "PATCH",
            RouteMethod::Delete => "DELETE", RouteMethod::Head => "HEAD",
            RouteMethod::Options => "OPTIONS", })
    }
}

#[derive(Debug, Clone)]
pub struct RouteDefinition {
    pub method: RouteMethod, pub path: String,
    pub handler_name: String, pub source: String,
}

static REGISTRY: OnceLock<Mutex<RouteRegistry>> = OnceLock::new();
static ROUTE_HANDLERS: OnceLock<Mutex<Vec<(Method, String, Handler)>>> = OnceLock::new();

fn registry() -> &'static Mutex<RouteRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(RouteRegistry {
        routes: HashMap::new(),
        finalized: false,
    }))
}

#[derive(Debug)]
pub struct RouteRegistry {
    routes: HashMap<(RouteMethod, String), RouteAndSource>,
    finalized: bool,
}

#[derive(Debug, Clone)]
struct RouteAndSource {
    handler_name: String,
    source: String,
}

impl RouteRegistry {
    pub fn new() -> Self { RouteRegistry { routes: HashMap::new(), finalized: false } }

    pub fn register(&mut self, method: RouteMethod, path: &str, handler_name: &str, source: &str) {
        if self.finalized {
            panic!("Routes are finalized: cannot register {} {} after boot", method, path);
        }
        let key = (method.clone(), path.to_string());
        if let Some(existing) = self.routes.get(&key) {
            panic!(
                "\n  Route conflict: {meth} {path}\n    Defined in: {src}\n    Conflict at: {src2}\n  Resolution: remove one definition or use Route::remove() first.",
                meth = method, path = path, src = existing.source, src2 = source
            );
        }
        self.routes.insert(key, RouteAndSource {
            handler_name: handler_name.to_string(),
            source: source.to_string(),
        });
    }

    pub fn remove(&mut self, method: RouteMethod, path: &str) {
        self.routes.remove(&(method, path.to_string()));
    }

    pub fn has(&self, method: &RouteMethod, path: &str) -> bool {
        self.routes.contains_key(&(method.clone(), path.to_string()))
    }

    pub fn finalize(&mut self) {
        self.finalized = true;
    }

    pub fn all(&self) -> Vec<RouteDefinition> {
        self.routes.iter().map(|((m, p), s)| RouteDefinition {
            method: m.clone(),
            path: p.clone(),
            handler_name: s.handler_name.clone(),
            source: s.source.clone(),
        }).collect()
    }
}

impl Default for RouteRegistry { fn default() -> Self { Self::new() } }

// ── Metadata registration (tracking only, no handler) ──

fn with_registry(f: impl FnOnce(&mut RouteRegistry)) {
    if let Ok(mut reg) = registry().lock() { f(&mut reg); }
}

pub fn register(method: RouteMethod, path: &str, handler_name: &str, source: &str) {
    with_registry(|r| r.register(method, path, handler_name, source));
}

pub fn get(path: &str, handler_name: &str, source: &str) {
    with_registry(|r| r.register(RouteMethod::Get, path, handler_name, source));
}

pub fn post(path: &str, handler_name: &str, source: &str) {
    with_registry(|r| r.register(RouteMethod::Post, path, handler_name, source));
}

pub fn put(path: &str, handler_name: &str, source: &str) {
    with_registry(|r| r.register(RouteMethod::Put, path, handler_name, source));
}

pub fn delete(path: &str, handler_name: &str, source: &str) {
    with_registry(|r| r.register(RouteMethod::Delete, path, handler_name, source));
}

pub fn remove(method: RouteMethod, path: &str) {
    with_registry(|r| r.remove(method, path));
}

pub fn has(method: &RouteMethod, path: &str) -> bool {
    registry().lock().map(|r| r.has(method, path)).unwrap_or(false)
}

pub fn all() -> Vec<RouteDefinition> {
    registry().lock().map(|r| r.all()).unwrap_or_default()
}

pub fn finalize() {
    with_registry(|r| r.finalize());
}

// ── Handler registration (stores actual handler for the HTTP server) ──

/// Register a route with both metadata (for `inspect --routes`) and handler.
///
/// This is a convenience that calls both `route::register()` and
/// `route::register_handler()` in one step.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use viontin_framework::route::{self, RouteMethod};
/// use viontin_framework::http::Method;
///
/// // Single call: registers metadata + handler
/// route::bind(Method::Get, "/users", Arc::new(users_handler), "users_handler", "src/routes.rs:10");
/// ```
pub fn bind(method: Method, path: &str, handler: Handler, handler_name: &str, source: &str) {
    let rm = match &method {
        Method::Get => RouteMethod::Get,
        Method::Post => RouteMethod::Post,
        Method::Put => RouteMethod::Put,
        Method::Delete => RouteMethod::Delete,
        Method::Patch => RouteMethod::Patch,
        Method::Head => RouteMethod::Head,
        Method::Options => RouteMethod::Options,
        Method::Custom(_) => return,
    };
    with_registry(|r| r.register(rm, path, handler_name, source));
    register_handler_inner(method, path, handler);
}

/// Register a route with its handler.
///
/// This stores the actual handler function. Combine with `route::register()`
/// for metadata tracking, or use `route::bind()` for both.
///
/// Panics if the route (method + path) was already registered with a handler.
pub fn register_handler(method: Method, path: &str, handler: Handler) {
    register_handler_inner(method, path, handler);
}

fn register_handler_inner(method: Method, path: &str, handler: Handler) {
    let mut h = ROUTE_HANDLERS.get_or_init(|| Mutex::new(Vec::new())).lock()
        .expect("ROUTE_HANDLERS mutex poisoned");
    if h.iter().any(|(m, p, _)| m == &method && p == path) {
        panic!("Route {} {} already registered with a handler", method, path);
    }
    h.push((method, path.to_string(), handler));
}

/// Collect all registered handlers into a Vec for Router construction.
pub fn take_handlers() -> Vec<(Method, String, Handler)> {
    ROUTE_HANDLERS.get_or_init(|| Mutex::new(Vec::new())).lock()
        .expect("ROUTE_HANDLERS mutex poisoned").drain(..).collect()
}

//! Router — entry point for the HTTP routing system.
//!
//! All routes are registered via the global `route::` facade or via proc macro
//! attributes (`#[get]`, `#[post]`, etc.). Routes registered via proc macros
//! are collected at compile time using linkme distributed slices and are
//! automatically picked up by the built-in RouteProvider.
//!
//! # Route Registration
//!
//! ## Via proc macros (compile-time, discovered automatically)
//!
//! ```ignore
//! #[get("/")]
//! fn index() -> Response { Response::html("<h1>Home</h1>") }
//!
//! #[get("/users/:id")]
//! fn show_user(id: String) -> Response { Response::ok() }
//!
//! #[post("/users")]
//! fn create_user(req: Request) -> Response { Response::created() }
//! ```
//!
//! ## Via `route::` facade (runtime, for closures and groups)
//!
//! ```ignore
//! route::get("/", Arc::new(|_| Response::html("Home")));
//!
//! route::group(AuthMiddleware::new("web"), || {
//!     route::get("/dashboard", Arc::new(dashboard));
//! });
//! ```

pub mod provider;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use crate::http::{Method, Request, Response};
use crate::middleware::{Middleware, MiddlewareChain};
use crate::server::{Handler, RouteName};
use crate::middleware::Next;

// ──────────────────────────────────────────────
//  LINKME DISTRIBUTED SLICE — Compile-time route registration
// ──────────────────────────────────────────────

/// A single route entry registered at compile time via proc macro.
/// Collected by linkme distributed slice, processed by RouteProvider::boot().
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub method: &'static str,
    pub path: &'static str,
    pub handler: fn(Request) -> Response,
}

#[linkme::distributed_slice]
pub static ROUTES: [RouteEntry];

// ──────────────────────────────────────────────
//  GLOBAL ROUTE MANAGER — Runtime route registration
// ──────────────────────────────────────────────

struct RegisteredRoute {
    method: Method,
    path: String,
    handler: Handler,
    name: Option<String>,
}

struct GroupContext {
    middleware: Option<Arc<MiddlewareChain>>,
    prefix: String,
    name_prefix: String,
}

struct RouteManager {
    routes: Vec<RegisteredRoute>,
    group_stack: Vec<GroupContext>,
    route_names: HashMap<String, String>,
}

impl RouteManager {
    fn new() -> Self {
        Self {
            routes: Vec::new(),
            group_stack: Vec::new(),
            route_names: HashMap::new(),
        }
    }

    fn current_middleware(&self) -> Option<Arc<MiddlewareChain>> {
        self.group_stack.last().and_then(|g| g.middleware.clone())
    }

    fn current_prefix(&self) -> &str {
        self.group_stack.last().map(|g| g.prefix.as_str()).unwrap_or("")
    }

    fn current_name_prefix(&self) -> &str {
        self.group_stack.last().map(|g| g.name_prefix.as_str()).unwrap_or("")
    }

    fn register(&mut self, method: Method, path: String, handler: Handler, name: Option<String>) {
        self.routes.push(RegisteredRoute { method, path, handler, name });
    }

    fn register_name(&mut self, name: &str, path: &str) {
        self.route_names.insert(name.to_string(), path.to_string());
    }

    fn drain(&mut self) -> Vec<(Method, String, Handler, Option<String>)> {
        std::mem::take(&mut self.routes)
            .into_iter()
            .map(|r| (r.method, r.path, r.handler, r.name))
            .collect()
    }

}

static MANAGER: OnceLock<Mutex<RouteManager>> = OnceLock::new();

fn manager() -> &'static Mutex<RouteManager> {
    MANAGER.get_or_init(|| Mutex::new(RouteManager::new()))
}

/// Lock manager, call f, unlock. Returns f's result.
fn with_mgr<T>(f: impl FnOnce(&mut RouteManager) -> T) -> T {
    match manager().lock() {
        Ok(mut guard) => f(&mut *guard),
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            f(&mut *guard)
        }
    }
}

// ──────────────────────────────────────────────
//  ROUTE FACADE — API for runtime registration
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
//  ROUTE BUILDER — Chainable route configuration
// ──────────────────────────────────────────────

/// Builder for a single route. Returned by `route::get()`, `route::post()`, etc.
/// Supports `.middleware()`, `.name()` before auto-registering on drop.
pub struct RouteBuilder {
    method: Method,
    path: String,
    handler: Option<Handler>,
    middlewares: Option<Arc<MiddlewareChain>>,
    route_name: Option<String>,
    name_prefix: String,
}

impl RouteBuilder {
    fn new(method: Method, path: &str, handler: Handler) -> Self {
        Self {
            method,
            path: path.to_string(),
            handler: Some(handler),
            middlewares: None,
            route_name: None,
            name_prefix: String::new(),
        }
    }

    /// Add middleware to this specific route. Can be chained multiple times.
    pub fn middleware(mut self, mw: impl Middleware + 'static) -> Self {
        let chain = self.middlewares.get_or_insert_with(|| Arc::new(MiddlewareChain::new()));
        if let Some(c) = Arc::get_mut(chain) { c.add(mw); }
        self
    }

    /// Add a closure-based middleware.
    pub fn middleware_fn(mut self, name: &'static str, f: impl Fn(&mut Request, Next) -> Response + Send + Sync + 'static) -> Self {
        let chain = self.middlewares.get_or_insert_with(|| Arc::new(MiddlewareChain::new()));
        if let Some(c) = Arc::get_mut(chain) { c.add_fn(name, f); }
        self
    }

    /// Assign a name for URL generation.
    /// If a group name prefix is active, it's automatically prepended.
    pub fn name(mut self, name: &str) -> Self {
        if self.name_prefix.is_empty() {
            self.route_name = Some(name.to_string());
        } else {
            self.route_name = Some(format!("{}{}", self.name_prefix, name));
        }
        self
    }

    /// Internal: set name prefix (called by RouteGroupRegistrar).
    #[doc(hidden)]
    pub fn _set_name_prefix(mut self, prefix: String) -> Self {
        self.name_prefix = prefix;
        self
    }

    /// Register the route immediately (also called by Drop).
    pub fn register(mut self) {
        if let Some(handler) = self.handler.take() {
            // Wrap with route-specific middleware
            let final_handler = if let Some(ref c) = self.middlewares {
                let c = c.clone();
                Arc::new(move |req: Request| {
                    let mut r = req;
                    c.apply(&mut r, |rr: &mut Request| handler(std::mem::take(rr)))
                })
            } else {
                handler
            };

            // Apply current group middleware (from route::group stack)
            let final_handler = with_mgr(|m| {
                if let Some(ref gc) = m.current_middleware() {
                    let g = gc.clone();
                    let h = final_handler;
                    Arc::new(move |req: Request| {
                        let mut r = req;
                        g.apply(&mut r, |rr: &mut Request| h(std::mem::take(rr)))
                    })
                } else {
                    final_handler
                }
            });

            let route_name = self.route_name.clone();
            with_mgr(|m| m.register(self.method.clone(), self.path.clone(), final_handler, route_name));

            // Store route name if provided (also kept in route_names map for url()/has() queries)
            if let Some(ref name) = self.route_name {
                with_mgr(|m| m.register_name(name, &self.path));
            }
        }
    }

}

impl Drop for RouteBuilder {
    fn drop(&mut self) {
        // Only register if handler was not taken (i.e., register() was not called)
        if self.handler.is_some() {
            // We can't call self.register() in Drop because it takes self
            // Instead, register with a taken handler
            if let Some(handler) = self.handler.take() {
                let final_handler = if let Some(ref c) = self.middlewares {
                    let c = c.clone();
                    let h = handler;
                    Arc::new(move |req: Request| {
                        let mut r = req;
                        c.apply(&mut r, |rr: &mut Request| h(std::mem::take(rr)))
                    })
                } else {
                    handler
                };
                let fh2 = with_mgr(|m| {
                    if let Some(ref gc) = m.current_middleware() {
                        let g = gc.clone();
                        let h = final_handler;
                        Arc::new(move |req: Request| {
                            let mut r = req;
                            g.apply(&mut r, |rr: &mut Request| h(std::mem::take(rr)))
                        })
                    } else {
                        final_handler
                    }
                });
                let route_name = self.route_name.clone();
                if let Some(ref name) = route_name {
                    with_mgr(|m| m.register_name(name, &self.path));
                }
                with_mgr(|m| m.register(self.method.clone(), self.path.clone(), fh2, route_name));
            }
        }
    }
}

// ──────────────────────────────────────────────
//  ROUTE FACADE — Now returns RouteBuilder
// ──────────────────────────────────────────────

pub fn get(path: &str, handler: Handler) -> RouteBuilder {
    RouteBuilder::new(Method::Get, path, handler)
}

pub fn post(path: &str, handler: Handler) -> RouteBuilder {
    RouteBuilder::new(Method::Post, path, handler)
}

pub fn put(path: &str, handler: Handler) -> RouteBuilder {
    RouteBuilder::new(Method::Put, path, handler)
}

pub fn patch(path: &str, handler: Handler) -> RouteBuilder {
    RouteBuilder::new(Method::Patch, path, handler)
}

pub fn delete(path: &str, handler: Handler) -> RouteBuilder {
    RouteBuilder::new(Method::Delete, path, handler)
}

pub fn any(path: &str, handler: Handler) {
    get(path, handler.clone()).register();
    post(path, handler.clone()).register();
    put(path, handler.clone()).register();
    delete(path, handler).register();
}

// ──────────────────────────────────────────────
//  ROUTE GROUP BUILDER
// ──────────────────────────────────────────────

/// RouteGroupRegistrar — passed to `.routes()` closure.
/// Wraps route registration with automatic prefix and name prefix.
pub struct RouteGroupRegistrar;

fn path_to_name_segment(path: &str) -> String {
    path.trim_start_matches('/')
        .replace("/", ".")
        .replace(":", "")
        .replace("*", "w")
        .replace(|c: char| !c.is_alphanumeric() && c != '.', "_")
}

impl RouteGroupRegistrar {
    fn build(&self, method: Method, path: &str, handler: Handler) -> RouteBuilder {
        let full_path = apply_prefix(path);
        let mut builder = RouteBuilder::new(method, &full_path, handler);
        if let Some(ns) = name_prefix() {
            let segment = path_to_name_segment(path);
            builder.route_name = Some(format!("{}.{}", ns.trim_end_matches('.'), segment));
            builder.name_prefix = ns.trim_end_matches('.').to_string();
        }
        builder
    }

    pub fn get(&self, path: &str, handler: Handler) -> RouteBuilder {
        self.build(Method::Get, path, handler)
    }

    pub fn post(&self, path: &str, handler: Handler) -> RouteBuilder {
        self.build(Method::Post, path, handler)
    }

    pub fn put(&self, path: &str, handler: Handler) -> RouteBuilder {
        self.build(Method::Put, path, handler)
    }

    pub fn patch(&self, path: &str, handler: Handler) -> RouteBuilder {
        self.build(Method::Patch, path, handler)
    }

    pub fn delete(&self, path: &str, handler: Handler) -> RouteBuilder {
        self.build(Method::Delete, path, handler)
    }
}

fn apply_prefix(path: &str) -> String {
    with_mgr(|m| {
        let prefix = m.current_prefix();
        if prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}{}", prefix, path)
        }
    })
}

fn name_prefix() -> Option<String> {
    with_mgr(|m| {
        let ns = m.current_name_prefix();
        if ns.is_empty() { None } else { Some(format!("{}.", ns)) }
    })
}

/// RouteGroup — builder for route groups with middleware, prefix, and name.
pub fn group() -> RouteGroup {
    RouteGroup::new()
}

pub struct RouteGroup {
    middleware_chain: Option<MiddlewareChain>,
    prefix: String,
    name: String,
}

impl RouteGroup {
    pub fn new() -> Self {
        Self {
            middleware_chain: None,
            prefix: String::new(),
            name: String::new(),
        }
    }

    /// Add middleware to this group. Can be chained.
    pub fn middleware(mut self, mw: impl Middleware + 'static) -> Self {
        let chain = self.middleware_chain.get_or_insert_with(MiddlewareChain::new);
        chain.add(mw);
        self
    }

    /// Set URL prefix for all routes in this group.
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.trim_end_matches('/').to_string();
        self
    }

    /// Set name prefix. Routes inside will auto-prefix their names:
    /// `name("admin")` + route `.name("users")` → `admin.users`
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Define routes inside this group.
    pub fn routes(self, f: impl FnOnce(&RouteGroupRegistrar)) {
        let registrar = RouteGroupRegistrar;

        // Push group context onto the stack
        with_mgr(|m| {
            let chain = self.middleware_chain.map(Arc::new);
            m.group_stack.push(GroupContext {
                middleware: chain,
                prefix: self.prefix.clone(),
                name_prefix: self.name.clone(),
            });
        });

        f(&registrar);

        with_mgr(|m| { m.group_stack.pop(); });
    }
}

impl Default for RouteGroup {
    fn default() -> Self { Self::new() }
}

/// Convenience: group with middleware only (no builder).
/// For full builder support (prefix, name, middleware chain), use `route::group()`.
pub fn group_mw(mw: impl Middleware + 'static, f: impl FnOnce()) {
    with_mgr(|m| {
        let mut chain = MiddlewareChain::new();
        chain.add(mw);
        m.group_stack.push(GroupContext {
            middleware: Some(Arc::new(chain)),
            prefix: String::new(),
            name_prefix: String::new(),
        });
    });
    f();
    with_mgr(|m| { m.group_stack.pop(); });
}

// ──────────────────────────────────────────────
//  INTEGRATION WITH ROUTER
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
//  GLOBAL ROUTER — Built by RouteProvider, consumed by Boot
// ──────────────────────────────────────────────

static GLOBAL_ROUTER: OnceLock<Mutex<Option<crate::server::Router>>> = OnceLock::new();

fn global_router() -> &'static Mutex<Option<crate::server::Router>> {
    GLOBAL_ROUTER.get_or_init(|| Mutex::new(None))
}

/// Build and store the Router with all registered routes.
/// Called by the built-in RouteProvider::boot().
pub fn build_router() {
    let mut router = crate::server::Router::new();

    // 1. Process linkme slice entries (compile-time registration via #[get])
    for entry in ROUTES.iter() {
        let method = match entry.method {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "PATCH" => Method::Patch,
            "DELETE" => Method::Delete,
            _ => continue,
        };
        router.push_route(method, entry.path.to_string(), Arc::new(move |req: Request| (entry.handler)(req)));
    }

    // 2. Process runtime-registered routes (route::get, route::group)
    let runtime_routes = with_mgr(|m| m.drain());
    for (method, path, handler, name) in runtime_routes {
        router.push_named_route(method, path, handler, name);
    }

    if let Ok(mut g) = global_router().lock() {
        *g = Some(router);
    }
}

/// Take the built Router, consuming it from the global state.
/// Called by Boot::run() after RouteProvider has completed.
pub fn take_router() -> Option<crate::server::Router> {
    if let Ok(mut g) = global_router().lock() {
        g.take()
    } else {
        None
    }
}

// ──────────────────────────────────────────────
//  NAMED ROUTE API — URL generation & query
// ──────────────────────────────────────────────

/// Generate a URL for a named route.
///
/// Returns `None` if no route with that name is registered.
///
/// ```ignore
/// let url = route::url("admin.users").unwrap();
/// // => "/admin/users"
/// ```
pub fn url(name: &str) -> Option<String> {
    with_mgr(|m| m.route_names.get(name).cloned())
}

/// Generate a URL for a named route, substituting parameters into `:param` segments.
///
/// ```ignore
/// let url = route::url_with("users.show", &[("id", "42")]).unwrap();
/// // => "/users/42"
/// ```
pub fn url_with(name: &str, params: &[(&str, &str)]) -> Option<String> {
    let mut path = url(name)?;
    for (key, val) in params {
        path = path.replace(&format!(":{}", key), val);
    }
    Some(path)
}

/// Check if a named route exists.
///
/// ```ignore
/// if route::has("users.index") { ... }
/// ```
pub fn has(name: &str) -> bool {
    with_mgr(|m| m.route_names.contains_key(name))
}

/// Return all registered route names and their path templates, sorted by name.
///
/// ```ignore
/// for (name, path) in route::all() {
///     println!("{} → {}", name, path);
/// }
/// ```
pub fn all() -> Vec<(String, String)> {
    with_mgr(|m| {
        let mut entries: Vec<(String, String)> = m.route_names.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    })
}

/// Get the name of the route that matched the current request.
///
/// Returns `None` if the request was not matched by any named route
/// (e.g., 404, static file, or the route was registered without a name).
///
/// ```ignore
/// if let Some(name) = route::current_name(&req) {
///     output.line(&format!("Current route: {}", name));
/// }
/// ```
pub fn current_name(req: &Request) -> Option<String> {
    req.extension::<RouteName>().map(|n| n.0)
}

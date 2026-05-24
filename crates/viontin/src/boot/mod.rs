//! Boot — application builder with fluent API.
//!
//! Sections:
//!   - context     (BootContext)
//!   - struct & constructors
//!   - entry       (entry)
//!   - terminal    (run, serve, run_with)
//!   - providers   (provider, withProviders, withoutProvider, withoutDefaultProviders)
//!   - commands    (command, withCommands, withoutCommand, withoutCommands)
//!   - gems        (gem, withGems, withoutGem, withoutGems)
//!   - middlewares (middleware, withMiddlewares, withoutMiddlewares)
//!   - routes      (routes, get, post, any, ws, ws_with_config)

use std::sync::Arc;
use viontin_framework::app::{Application, ServiceProvider};
use viontin_framework::cli::{Command, Kernel};
use viontin_framework::gem::{GemBinding, GemRegistry};
use viontin_gems::GemBuilder;
use viontin_framework::http::{Request, Response};
use viontin_framework::middleware::{Middleware, MiddlewareChain};
use viontin_framework::module::Module;
use viontin_framework::server::Router;
use viontin_framework::ws::{self, WebSocketConfig, WebSocketHandler, WsRouter, WsServer};

// ──────────────────────────────────────────────
//  BOOT CONTEXT
// ──────────────────────────────────────────────

/// Runtime context with all components initialized and ready.
///
/// Passed to the `entry()` callback after framework initialization,
/// provider registration, and router finalization. Only dispatched
/// if no CLI command was found in the process arguments.
pub struct BootContext {
    pub app: Application,
    pub ws_server: WsServer,
    pub kernel: Kernel,
}

impl BootContext {
    /// Start the HTTP server (blocking).
    pub fn serve(self, addr: &str) {
        self.ws_server.run(addr).unwrap();
    }

    /// Dispatch CLI commands manually.
    pub fn cli(self) {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            self.kernel.run(&args).exit();
        }
    }

    /// Consume the context and return the underlying `Application`.
    pub fn into_inner(self) -> Application {
        self.app
    }
}

// ──────────────────────────────────────────────
//  BOOT BUILDER
// ──────────────────────────────────────────────

pub fn boot() -> Boot {
    Boot {
        app: Application::new(),
        kernel: Kernel::new(),
        router: Router::new(),
        ws_router: ws::ws_router(),
        gems: GemRegistry::new(),
        middlewares: MiddlewareChain::new(),
        entry_fn: None,
    }
}

pub struct Boot {
    pub(crate) app: Application,
    pub(crate) kernel: Kernel,
    pub(crate) router: Router,
    pub(crate) ws_router: WsRouter,
    pub(crate) gems: GemRegistry,
    pub(crate) middlewares: MiddlewareChain,
    entry_fn: Option<Box<dyn FnOnce(BootContext) + 'static>>,
}

impl Boot {
    // ──────────────────────────────────────────────
    //  ENTRY
    // ──────────────────────────────────────────────

    /// Define the application entry point.
    ///
    /// The callback receives a [`BootContext`] with all runtime components
    /// already initialized — providers registered, gems built, routes finalized.
    /// It is only called when no CLI command was dispatched (argv has no args
    /// or the first argument does not match a registered command).
    pub fn entry<F>(mut self, f: F) -> Self
    where
        F: FnOnce(BootContext) + 'static,
    {
        self.entry_fn = Some(Box::new(f));
        self
    }

    // ──────────────────────────────────────────────
    //  TERMINAL
    // ──────────────────────────────────────────────

    /// Finalize and execute the application.
    ///
    /// 1. Run all gem build hooks (`before_build`).
    /// 2. Register and boot all service providers.
    /// 3. Dispatch a CLI command if `argv[1]` matches a registered command (takes priority).
    /// 4. Finalize the router (merge registry, attach middleware, WebSocket routes).
    /// 5. Call the `entry` callback with a ready [`BootContext`].
    pub fn run(mut self) {
        self.gems.before_build_all().ok();
        self.app.run();

        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            self.kernel.run(&args).exit();
        }

        let router = self.router.extend_from_registry();
        let router = if !self.middlewares.is_empty() {
            router.with_global_middleware(self.middlewares)
        } else {
            router
        };
        let ws_server = self.ws_router.attach(router);

        if let Some(entry) = self.entry_fn.take() {
            entry(BootContext {
                app: self.app,
                ws_server,
                kernel: self.kernel,
            });
        }
    }

    /// Start the HTTP server (shortcut for `entry(|ctx| ctx.serve(addr)).run()`).
    pub fn serve(self, addr: &str) {
        let owned = addr.to_owned();
        self.entry(move |ctx| ctx.serve(&owned)).run()
    }

    /// Define the entry point and run (shortcut for `entry(f).run()`).
    pub fn run_with<F>(self, f: F)
    where
        F: FnOnce(BootContext) + 'static,
    {
        self.entry(f).run()
    }

    // ──────────────────────────────────────────────
    //  PROVIDERS
    // ──────────────────────────────────────────────

    pub fn provider(mut self, provider: impl ServiceProvider + 'static) -> Self {
        self.app = self.app.with(provider);
        self
    }

    pub fn with_providers(mut self, providers: Vec<Box<dyn ServiceProvider + 'static>>) -> Self {
        for p in providers { self.app = self.app.with_boxed(p); }
        self
    }

    /// Remove a built-in service provider by name (e.g. "config", "log").
    pub fn without_provider(mut self, name: &str) -> Self {
        self.app = self.app.without(name);
        self
    }

    /// Remove all built-in service providers (env, config, log, queue, events).
    pub fn without_default_providers(mut self) -> Self {
        for name in &["env", "config", "log", "queue", "events"] {
            self.app = self.app.without(name);
        }
        self
    }

    // ──────────────────────────────────────────────
    //  COMMANDS
    // ──────────────────────────────────────────────

    pub fn command<C: Command + 'static>(mut self, command: C) -> Self {
        self.kernel = self.kernel.register(command);
        self
    }

    pub fn with_commands(mut self, commands: Vec<Box<dyn Command + 'static>>) -> Self {
        for cmd in commands { self.kernel = self.kernel.register_dyn(cmd); }
        self
    }

    /// Remove a registered command by its signature name.
    pub fn without_command(mut self, name: &str) -> Self {
        self.kernel = self.kernel.remove(name);
        self
    }

    /// Remove all registered commands.
    pub fn without_commands(mut self) -> Self {
        self.kernel = Kernel::new();
        self
    }

    // ──────────────────────────────────────────────
    //  GEMS
    // ──────────────────────────────────────────────

    pub fn gem(mut self, gem: impl GemBinding + GemBuilder + 'static) -> Self {
        for mw in gem.gem_middlewares() { self.middlewares.add_dyn(mw); }
        for p in gem.gem_providers() { self.app = self.app.with_boxed(p); }
        for c in gem.gem_commands() { self.kernel = self.kernel.register_dyn(c); }
        if let Some(f) = gem.gem_routes() { self.router = f(self.router); }
        self.gems.register(gem);
        self
    }

    pub fn with_gems(mut self, gems: Vec<Box<dyn GemBinding + 'static>>) -> Self {
        for g in gems {
            for mw in g.gem_middlewares() { self.middlewares.add_dyn(mw); }
            for p in g.gem_providers() { self.app = self.app.with_boxed(p); }
            for c in g.gem_commands() { self.kernel = self.kernel.register_dyn(c); }
            if let Some(f) = g.gem_routes() { self.router = f(self.router); }
            self.gems.register_dyn(g);
        }
        self
    }

    /// Remove a registered gem by name.
    pub fn without_gem(mut self, name: &str) -> Self {
        self.gems = self.gems.remove(name);
        self
    }

    /// Remove all registered gems.
    pub fn without_gems(mut self) -> Self {
        self.gems = GemRegistry::new();
        self
    }

    // ──────────────────────────────────────────────
    //  MODULES
    // ──────────────────────────────────────────────

    /// Register a self-contained module with its routes, commands, and providers.
    ///
    /// Modules enable a modular monolith architecture. Each module is
    /// self-contained and can later be extracted into a microservice.
    pub fn module(mut self, m: impl Module + 'static) -> Self {
        self.router = m.routes(self.router);
        for cmd in m.commands() { self.kernel = self.kernel.register_dyn(cmd); }
        for p in m.providers() { self.app = self.app.with_boxed(p); }
        self
    }

    // ──────────────────────────────────────────────
    //  MIDDLEWARES
    // ──────────────────────────────────────────────

    pub fn middleware(mut self, m: impl Middleware + 'static) -> Self {
        self.middlewares.add(m);
        self
    }

    pub fn with_middlewares(mut self, mws: Vec<Box<dyn Middleware + 'static>>) -> Self {
        for m in mws { self.middlewares.add_dyn(m); }
        self
    }

    /// Remove all registered middlewares.
    pub fn without_middlewares(mut self) -> Self {
        self.middlewares = MiddlewareChain::new();
        self
    }

    // ──────────────────────────────────────────────
    //  ROUTES
    // ──────────────────────────────────────────────

    pub fn routes(mut self, f: fn(Router) -> Router) -> Self {
        self.router = f(self.router);
        self
    }

    pub fn get(mut self, path: &str, handler: fn(Request) -> Response) -> Self {
        self.router = self.router.get(path, Arc::new(handler));
        self
    }

    pub fn post(mut self, path: &str, handler: fn(Request) -> Response) -> Self {
        self.router = self.router.post(path, Arc::new(handler));
        self
    }

    pub fn any(mut self, path: &str, handler: fn(Request) -> Response) -> Self {
        self.router = self.router.any(path, Arc::new(handler));
        self
    }

    pub fn ws(mut self, path: &str, handler: impl WebSocketHandler) -> Self {
        self.ws_router = self.ws_router.ws(path, handler);
        self
    }

    pub fn ws_with_config(mut self, path: &str, config: WebSocketConfig, handler: impl WebSocketHandler) -> Self {
        self.ws_router = self.ws_router.ws_with_config(path, config, handler);
        self
    }
}

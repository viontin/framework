//! Boot — application builder with fluent API.
//!
//! Sections:
//!   - struct & constructors
//!   - providers   (provider, withProviders, withoutProvider)
//!   - commands    (command, withCommands, withoutCommands)
//!   - gems        (gem, withGems, withoutGems)
//!   - middlewares (middleware, withMiddlewares, withoutMiddlewares)
//!   - routes      (get, post, any, ws, routes)
//!   - terminal    (serve, run)

use std::sync::Arc;
use viontin_framework::cli::{Command, Kernel};
use viontin_framework::http::{Request, Response};
use viontin_framework::middleware::{Middleware, MiddlewareChain};
use viontin_framework::ws::{WebSocketHandler, WebSocketConfig, self, WsRouter};
use viontin_framework::app::{Application, ServiceProvider};
use viontin_framework::server::Router;
use viontin_framework::gem::{GemBinding, GemRegistry};

pub fn boot() -> Boot {
    Boot {
        app: Application::new(),
        kernel: Kernel::new(),
        router: Router::new(),
        ws_router: ws::ws_router(),
        gems: GemRegistry::new(),
        middlewares: MiddlewareChain::new(),
    }
}

pub struct Boot {
    pub(crate) app: Application,
    pub(crate) kernel: Kernel,
    pub(crate) router: Router,
    pub(crate) ws_router: WsRouter,
    pub(crate) gems: GemRegistry,
    pub(crate) middlewares: MiddlewareChain,
}

impl Boot {
    // ──────────────────────────────────────────────
    //  PROVIDERS
    // ──────────────────────────────────────────────

    pub fn provider(mut self, provider: impl ServiceProvider + 'static) -> Self {
        self.app = self.app.with(provider);
        self
    }

    pub fn withProviders(mut self, providers: Vec<Box<dyn ServiceProvider + 'static>>) -> Self {
        for p in providers { self.app = self.app.with_boxed(p); }
        self
    }

    /// Remove a built-in service provider by name (e.g. "config", "log").
    pub fn withoutProvider(mut self, name: &str) -> Self {
        self.app = self.app.without(name);
        self
    }

    /// Remove all built-in service providers (env, config, log, queue, events).
    pub fn withoutDefaultProviders(mut self) -> Self {
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

    pub fn withCommands(mut self, commands: Vec<Box<dyn Command + 'static>>) -> Self {
        for cmd in commands { self.kernel = self.kernel.register_dyn(cmd); }
        self
    }

    /// Remove a registered command by its signature name.
    pub fn withoutCommand(mut self, name: &str) -> Self {
        self.kernel = self.kernel.remove(name);
        self
    }

    /// Remove all registered commands.
    pub fn withoutCommands(mut self) -> Self {
        self.kernel = Kernel::new();
        self
    }

    // ──────────────────────────────────────────────
    //  GEMS
    // ──────────────────────────────────────────────

    pub fn gem(mut self, gem: impl GemBinding + 'static) -> Self {
        for mw in gem.gem_middlewares() { self.middlewares.add_dyn(mw); }
        for p in gem.gem_providers() { self.app = self.app.with_boxed(p); }
        for c in gem.gem_commands() { self.kernel = self.kernel.register_dyn(c); }
        if let Some(f) = gem.gem_routes() { self.router = f(self.router); }
        self.gems.register(gem);
        self
    }

    pub fn withGems(mut self, gems: Vec<Box<dyn GemBinding + 'static>>) -> Self {
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
    pub fn withoutGem(mut self, name: &str) -> Self {
        self.gems = self.gems.remove(name);
        self
    }

    /// Remove all registered gems.
    pub fn withoutGems(mut self) -> Self {
        self.gems = GemRegistry::new();
        self
    }

    // ──────────────────────────────────────────────
    //  MIDDLEWARES
    // ──────────────────────────────────────────────

    pub fn middleware(mut self, m: impl Middleware + 'static) -> Self {
        self.middlewares.add(m);
        self
    }

    pub fn withMiddlewares(mut self, mws: Vec<Box<dyn Middleware + 'static>>) -> Self {
        for m in mws { self.middlewares.add_dyn(m); }
        self
    }

    /// Remove all registered middlewares.
    pub fn withoutMiddlewares(mut self) -> Self {
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

    // ──────────────────────────────────────────────
    //  TERMINAL
    // ──────────────────────────────────────────────

    pub fn serve(self, addr: &str) {
        self.gems.before_build_all().ok();
        let args: Vec<String> = std::env::args().collect();
        self.app.run();
        if args.len() > 1 {
            self.kernel.run(&args).exit();
        }
        let mut router = self.router.extend_from_registry();
        if !self.middlewares.is_empty() {
            router = router.with_global_middleware(self.middlewares);
        }
        let server = self.ws_router.attach(router);
        server.run(addr).unwrap();
    }

    pub fn run<F>(self, f: F)
    where
        F: FnOnce(),
    {
        if let Err(e) = self.gems.before_build_all() {
            eprintln!("  [gems] before_build error: {}", e);
        }
        let args: Vec<String> = std::env::args().collect();
        self.app.run();
        if args.len() > 1 {
            self.kernel.run(&args).exit();
        }
        f();
    }
}

use std::sync::Arc;
use crate::app::{Application, ServiceProvider};
use crate::route;
use crate::middleware::{healthz_handler, readyz_handler};

/// Built-in ServiceProvider that builds the HTTP Router with all registered routes.
///
/// During `boot()`, this provider:
/// 1. Registers default health check routes (/healthz, /readyz)
/// 2. Iterates the linkme `ROUTES` distributed slice (compile-time `#[get]` routes)
/// 3. Drains runtime-registered routes (`route::get()`, `route::group()`)
/// 4. Builds a `Router` and stores it in the global route state
/// 5. `Boot::run()` retrieves the router via `route::take_router()`
///
/// This provider is automatically registered by `Application::new()`.
/// No manual registration needed.
#[derive(Debug)]
pub struct RouteProvider;

impl ServiceProvider for RouteProvider {
    fn id(&self) -> &'static str { "route" }
    fn depends_on(&self) -> &[&'static str] { &["config"] }

    fn register(&self, _app: &mut Application) -> Result<(), String> {
        Ok(())
    }

    fn boot(&self, _app: &Application) -> Result<(), String> {
        route::get("/healthz", Arc::new(healthz_handler)).register();
        route::get("/readyz", Arc::new(readyz_handler)).register();
        route::build_router();
        Ok(())
    }
}

use crate::app::{Application, ServiceProvider};
use crate::route;

/// ServiceProvider that finalizes all routes after registration.
///
/// Gems and service providers register routes via `route::register_handler()`
/// during their `register()` phase. This provider locks the registry
/// during `boot()` to prevent further modifications.
#[derive(Debug)]
pub struct RouteProvider;

impl ServiceProvider for RouteProvider {
    fn name(&self) -> &str { "route" }

    fn register(&self, _app: &mut Application) {}

    fn boot(&self, _app: &Application) {
        route::finalize();
    }
}

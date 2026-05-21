//! ServiceProvider trait — the building block of application bootstrapping.

use std::fmt;
use super::Application;

/// A service provider registers and boots services into the application.
///
/// Each provider has two phases:
/// 1. **register** — register services into the container
/// 2. **boot** — initialize services (after all providers are registered)
pub trait ServiceProvider: fmt::Debug + Send + Sync {
    /// Unique name for this provider (used for `.with()` and `.without()`).
    fn name(&self) -> &str;

    /// Register services into the application container.
    fn register(&self, _app: &mut Application) {}

    /// Boot services after all providers have registered.
    fn boot(&self, _app: &Application) {}
}

//! ServiceProvider trait — the building block of application bootstrapping.
//!
//! Each provider has two phases:
//! 1. **register** — bind services into the container
//! 2. **boot** — initialize after all services are registered
//!
//! Providers declare dependencies via `depends_on()`. The system performs
//! a topological sort to ensure correct ordering.

use std::fmt;
use crate::app::Application;
use crate::env::Environment;

/// A service provider registers and boots services into the application.
pub trait ServiceProvider: fmt::Debug + Send + Sync {
    /// Unique identifier for this provider (used for dependency ordering).
    fn id(&self) -> &'static str;

    /// Providers that must register BEFORE this one.
    fn depends_on(&self) -> &[&'static str] { &[] }

    /// Priority within the same dependency level (lower = earlier).
    fn priority(&self) -> u8 { 100 }

    /// Only run in specific environments. None = all environments.
    fn environments(&self) -> Option<&[Environment]> { None }

    /// Phase 1: Register services into the container.
    fn register(&self, _app: &mut Application) -> Result<(), String> { Ok(()) }

    /// Phase 2: Initialize services after all providers have registered.
    fn boot(&self, _app: &Application) -> Result<(), String> { Ok(()) }

    /// Optional: Cleanup on graceful shutdown.
    fn shutdown(&self, _app: &Application) -> Result<(), String> { Ok(()) }
}

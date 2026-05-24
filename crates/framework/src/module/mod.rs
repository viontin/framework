//! Modular architecture — self-contained modules with auto-registration.
//!
//! A `Module` is a self-contained unit that bundles routes, CLI commands,
//! and service providers together. This enables a modular monolith
//! architecture that can later be split into microservices.

use crate::app::ServiceProvider;
use crate::cli::Command;
use crate::server::Router;

/// A self-contained module with its own routes, commands, and providers.
///
/// Modules are the building block of a modular monolith. Each module
/// can be independently developed, tested, and later extracted into
/// a separate microservice.
pub trait Module: Send + Sync + std::fmt::Debug {
    /// Unique module name (used for identification and service naming).
    fn name(&self) -> &str;

    /// Register HTTP routes.
    fn routes(&self, router: Router) -> Router {
        router
    }

    /// Register CLI commands.
    fn commands(&self) -> Vec<Box<dyn Command + 'static>> {
        Vec::new()
    }

    /// Register service providers.
    fn providers(&self) -> Vec<Box<dyn ServiceProvider + 'static>> {
        Vec::new()
    }
}

/// A group of modules that are registered together.
///
/// Useful for grouping related functionality (e.g., all billing-related modules).
pub struct ModuleGroup {
    name: String,
    modules: Vec<Box<dyn Module>>,
}

impl ModuleGroup {
    pub fn new(name: &str) -> Self {
        ModuleGroup {
            name: name.into(),
            modules: Vec::new(),
        }
    }

    pub fn add(mut self, module: impl Module + 'static) -> Self {
        self.modules.push(Box::new(module));
        self
    }
}

impl std::fmt::Debug for ModuleGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleGroup")
            .field("name", &self.name)
            .field("modules", &self.modules.len())
            .finish()
    }
}

impl Module for ModuleGroup {
    fn name(&self) -> &str {
        &self.name
    }

    fn routes(&self, router: Router) -> Router {
        let mut r = router;
        for m in &self.modules {
            r = m.routes(r);
        }
        r
    }

    fn commands(&self) -> Vec<Box<dyn Command + 'static>> {
        self.modules.iter().flat_map(|m| m.commands()).collect()
    }

    fn providers(&self) -> Vec<Box<dyn ServiceProvider + 'static>> {
        self.modules.iter().flat_map(|m| m.providers()).collect()
    }
}

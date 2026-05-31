//! DomainServiceProvider — entry point for all domain services.

use std::fmt;
use crate::app::{Application, ServiceProvider};
use crate::domain::{self, Domain, DomainListener};

/// Configuration for a single domain's services.
pub struct DomainConfig {
    pub domain: Domain,
    pub listeners: Vec<Box<dyn DomainListener>>,
}

impl fmt::Debug for DomainConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DomainConfig").field("domain", &self.domain.name).finish()
    }
}

impl DomainConfig {
    pub fn new(domain: Domain) -> Self {
        DomainConfig { domain, listeners: Vec::new() }
    }

    /// Register a domain event listener.
    pub fn listener(mut self, listener: impl DomainListener + 'static) -> Self {
        self.listeners.push(Box::new(listener));
        self
    }
}

/// Service provider that bootstraps domain services.
///
/// ```rust
/// boot()
///     .provider(DomainServiceProvider::new()
///         .domain(DomainConfig::new(Domain::new("billing"))
///             .listener(InvoicePaidHandler)
///         )
///     )
///     .serve(":3000");
/// ```
#[derive(Debug)]
pub struct DomainServiceProvider {
    configs: Vec<DomainConfig>,
}

impl DomainServiceProvider {
    pub fn new() -> Self {
        DomainServiceProvider { configs: Vec::new() }
    }

    pub fn domain(mut self, config: DomainConfig) -> Self {
        self.configs.push(config);
        self
    }

    pub fn with_domains(mut self, configs: Vec<DomainConfig>) -> Self {
        for c in configs { self.configs.push(c); }
        self
    }
}

impl Default for DomainServiceProvider {
    fn default() -> Self { Self::new() }
}

impl ServiceProvider for DomainServiceProvider {
    fn id(&self) -> &'static str { "domain" }
    fn depends_on(&self) -> &[&'static str] { &["config", "events"] }

    fn register(&self, _app: &mut Application) -> Result<(), String> {
        for config in &self.configs {
            domain::register(config.domain.clone());
        }
        Ok(())
    }

    fn boot(&self, _app: &Application) -> Result<(), String> {
        let domains = domain::domains();
        println!("  [domain] {} domain(s) registered", domains.len());
        for d in &domains {
            let deps = d.allows.join(", ");
            let deps_str = if deps.is_empty() { "none".to_string() } else { deps };
            println!("    └── {} (allows: {})", d.name, deps_str);
        }
        Ok(())
    }
}

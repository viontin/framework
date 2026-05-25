//! Service contracts — explicit boundaries between services.
//!
//! A `ServiceContract` defines a clear API boundary that can be
//! implemented locally (same process) or remotely (microservice).
//! This enables a modular monolith that can be split into microservices
//! without changing the contract code.
//!
//! # Usage
//!
//! ```rust,ignore
//! use viontin_framework::service::contracts::{ServiceContract, ServiceRegistry};
//!
//! // Define the contract
//! pub struct BillingService;
//!
//! impl ServiceContract for BillingService {
//!     fn name(&self) -> &str { "billing" }
//!     fn version(&self) -> &str { "1.0" }
//!
//!     fn handle(&self, command: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
//!         match command {
//!             "create-invoice" => { /* ... */ Ok(vec![]) }
//!             "process-payment" => { /* ... */ Ok(vec![]) }
//!             _ => Err("Unknown command".into()),
//!         }
//!     }
//! }
//!
//! // Register and resolve
//! let mut registry = ServiceRegistry::new();
//! registry.add(BillingService);
//!
//! // Resolve by name (works locally; remote adapter would look the same)
//! let service = registry.get("billing").unwrap();
//! let result = service.handle("create-invoice", b"{}")?;
//! ```

use std::collections::HashMap;

/// A service contract defines an explicit API boundary.
///
/// The same trait is used whether the service runs in-process (modular monolith)
/// or as a remote microservice. This makes extraction seamless — change the
/// implementation, not the call site.
pub trait ServiceContract: Send + Sync + std::fmt::Debug {
    /// Unique service name (used for service discovery).
    fn name(&self) -> &str;

    /// Semver version of this contract.
    fn version(&self) -> &str {
        "0.1.0"
    }

    /// Handle a command with a serialized payload.
    ///
    /// For in-process services, the payload can be any serialization format
    /// (JSON, MessagePack, etc.). For remote microservices, this is called
    /// over HTTP/gRPC with the same serialization.
    fn handle(&self, command: &str, payload: &[u8]) -> Result<Vec<u8>, String>;

    /// List all commands this service supports.
    fn commands(&self) -> Vec<&str> {
        Vec::new()
    }
}

/// Registry of service contracts for the application.
///
/// Services registered here can be resolved by name. In a modular monolith,
/// all services are in-process. When extracting to microservices, replace
/// the local implementation with a remote adapter — the call site stays the same.
#[derive(Debug, Default)]
pub struct ServiceRegistry {
    services: HashMap<String, Box<dyn ServiceContract>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: HashMap::new(),
        }
    }

    /// Register a service contract.
    pub fn add(&mut self, service: impl ServiceContract + 'static) {
        let name = service.name().to_string();
        self.services.insert(name, Box::new(service));
    }

    /// Resolve a service by name.
    pub fn get(&self, name: &str) -> Option<&dyn ServiceContract> {
        self.services.get(name).map(|b| b.as_ref())
    }

    /// Check if a service is registered.
    pub fn has(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }

    /// List all registered service names.
    pub fn names(&self) -> Vec<&str> {
        self.services.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a service from the registry.
    pub fn remove(&mut self, name: &str) {
        self.services.remove(name);
    }
}

/// Adapter for calling a service remotely (microservice mode).
///
/// This wraps an HTTP/gRPC client behind the same `ServiceContract` trait,
/// so the caller never knows if the service is local or remote.
///
/// ```rust,ignore
/// use viontin_framework::service::contracts::{RemoteServiceAdapter, ServiceRegistry};
///
/// let adapter = RemoteServiceAdapter::new("billing", "http://billing-service:8080");
/// registry.add(adapter);
/// ```
#[derive(Debug)]
pub struct RemoteServiceAdapter {
    name: String,
    base_url: String,
    version: String,
}

impl RemoteServiceAdapter {
    pub fn new(name: &str, base_url: &str) -> Self {
        RemoteServiceAdapter {
            name: name.into(),
            base_url: base_url.into(),
            version: "0.1.0".into(),
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.into();
        self
    }
}

impl ServiceContract for RemoteServiceAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn handle(&self, command: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
        // In production, this would make an HTTP request:
        // POST {base_url}/{command}
        // Body: payload
        // Response: Vec<u8>
        Err(format!(
            "Remote service adapter for '{}' — integrate with reqwest or your HTTP client. \
             Called: {}/{} with {} bytes",
            self.name, self.base_url, command, payload.len()
        ))
    }

    fn commands(&self) -> Vec<&str> {
        Vec::new()
    }
}

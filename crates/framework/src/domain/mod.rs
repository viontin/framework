mod rule;
pub mod provider;

pub use rule::{DomainBoundary, DomainViolation};
pub use provider::{DomainServiceProvider, DomainConfig};

use std::fmt;
use std::sync::{Mutex, OnceLock};
use crate::error::Result as FrameworkResult;
use crate::events::Event;

// ── Domain ──

#[derive(Debug, Clone)]
pub struct Domain {
    pub name: &'static str,
    pub allows: &'static [&'static str],
    pub provides: &'static [&'static str],
}

impl Domain {
    pub const fn new(name: &'static str) -> Self {
        Domain { name, allows: &[], provides: &[] }
    }

    pub const fn allows(mut self, deps: &'static [&'static str]) -> Self {
        self.allows = deps;
        self
    }

    pub const fn provides(mut self, api: &'static [&'static str]) -> Self {
        self.provides = api;
        self
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for Domain {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl Eq for Domain {}

// ── Domain Event (extends basic Event) ──

/// A domain event — it is also a basic `Event`, so it works with the standard
/// `EventDispatcher` and any `Listener`.
pub trait DomainEvent: Event {
    fn domain(&self) -> &str;
    fn event_name(&self) -> &str;
}

/// Generic domain event for ad-hoc usage.
#[derive(Debug, Clone)]
pub struct GenericDomainEvent {
    pub domain: String,
    pub name: String,
    pub payload: Option<String>,
}

impl Event for GenericDomainEvent {}
impl DomainEvent for GenericDomainEvent {
    fn domain(&self) -> &str { &self.domain }
    fn event_name(&self) -> &str { &self.name }
}

/// A domain-aware listener. It receives domain events via the standard EventDispatcher.
/// Register by adding as a wildcard listener.
pub trait DomainListener: fmt::Debug + Send + Sync {
    fn domain(&self) -> &str;
    fn handle_domain(&self, event: &dyn DomainEvent);
}



// ── Aggregate Root ──

/// An aggregate root — a domain entity that guarantees consistency boundaries.
pub trait AggregateRoot: fmt::Debug + Send + Sync {
    fn domain(&self) -> &str;
    fn id(&self) -> &str;
    fn events(&self) -> Vec<Box<dyn DomainEvent>>;
    fn clear_events(&mut self);
}

// ── Repository ──

/// A repository — data access abstraction scoped to a domain.
pub trait Repository<T: AggregateRoot>: fmt::Debug + Send + Sync {
    fn domain(&self) -> &str;
    fn save(&self, aggregate: &T) -> FrameworkResult<()>;
    fn find_by_id(&self, id: &str) -> std::result::Result<Option<T>, String>;
    fn delete(&self, aggregate: &T) -> FrameworkResult<()>;
}

// ── Registry ──

static REGISTRY: OnceLock<Mutex<Vec<Domain>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<Domain>> {
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn register(domain: Domain) {
    let mut reg = registry().lock().unwrap();
    if !reg.iter().any(|d| d.name == domain.name) {
        reg.push(domain);
    }
}

pub fn domains() -> Vec<Domain> {
    registry().lock().unwrap().clone()
}

pub fn find(name: &str) -> Option<Domain> {
    registry().lock().unwrap().iter().find(|d| d.name == name).cloned()
}

pub fn is_allowed(from: &str, to: &str) -> bool {
    if from == to { return true; }
    let reg = registry().lock().unwrap();
    match reg.iter().find(|d| d.name == from) {
        Some(domain) => domain.allows.contains(&to),
        None => true,
    }
}

pub fn check_all() -> Vec<DomainViolation> {
    rule::check_all_boundaries(&domains())
}

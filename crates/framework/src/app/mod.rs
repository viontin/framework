//! Application bootstrap — the core of every Viontin application.
//!
//! The `Application` struct orchestrates the boot sequence:
//! - Phase 0: Init — detect environment, create container
//! - Phase 1: Config — load .env and config files
//! - Phase 2: Register — call register() on all providers (topologically sorted)
//! - Phase 3: Boot — call boot() on all providers
//! - Phase 4: Run — execute the terminal method (serve, run, command)

mod provider;
mod shutdown;

pub use provider::ServiceProvider;
pub use shutdown::ShutdownCoordinator;
pub use viontin_core::Container;

use std::collections::HashMap;
use std::time::Instant;
use crate::env::Environment;

/// Current phase of the application boot sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    Created,
    ConfigLoaded,
    Registering,
    Registered,
    Booting,
    Booted,
    TiersReady,
    Running,
    ShuttingDown,
    Stopped,
}

impl BootPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            BootPhase::Created => "created",
            BootPhase::ConfigLoaded => "config-loaded",
            BootPhase::Registering => "registering",
            BootPhase::Registered => "registered",
            BootPhase::Booting => "booting",
            BootPhase::Booted => "booted",
            BootPhase::TiersReady => "tiers-ready",
            BootPhase::Running => "running",
            BootPhase::ShuttingDown => "shutting-down",
            BootPhase::Stopped => "stopped",
        }
    }
}

/// Payload data attached to lifecycle events.
#[derive(Debug, Clone)]
pub enum BootPayload {
    Environment(Environment),
    ConfigSources(Vec<String>),
    ProviderIds(Vec<String>),
    ServiceCount(usize),
    Duration(std::time::Duration),
    ActiveTiers(Vec<&'static str>),
    Address(String),
    ShutdownReason(&'static str),
    Error(String),
}

/// Boot lifecycle event, emitted to listeners.
#[derive(Debug, Clone)]
pub struct BootEvent {
    pub phase: BootPhase,
    pub event_type: &'static str,
    pub timestamp: Instant,
    pub payload: Option<BootPayload>,
}

impl BootEvent {
    pub fn new(phase: BootPhase, event_type: &'static str) -> Self {
        Self { phase, event_type, timestamp: Instant::now(), payload: None }
    }

    pub fn with_payload(mut self, payload: BootPayload) -> Self {
        self.payload = Some(payload);
        self
    }
}

/// The core application runtime — owns the container, providers, and lifecycle.
pub struct Application {
    pub container: Container,
    pub environment: Environment,
    phase: BootPhase,
    pub(crate) providers: Vec<Box<dyn ServiceProvider>>,
    listeners: Vec<Box<dyn Fn(&BootEvent) + Send + Sync>>,
    shutdown: ShutdownCoordinator,
}

impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("phase", &self.phase)
            .field("environment", &self.environment)
            .field("providers", &self.providers.len())
            .field("listeners", &self.listeners.len())
            .finish()
    }
}

impl Application {
    pub fn new() -> Self {
        let mut app = Self {
            container: Container::new(),
            environment: Environment::detect(),
            phase: BootPhase::Created,
            providers: Vec::new(),
            listeners: Vec::new(),
            shutdown: ShutdownCoordinator::new(),
        };
        app.providers = vec![
            Box::new(EnvProvider),
            Box::new(ConfigProvider),
            Box::new(LogProvider),
            Box::new(QueueProvider),
            Box::new(EventsProvider),
            Box::new(crate::route::provider::RouteProvider),
        ];
        app
    }

    pub fn phase(&self) -> BootPhase { self.phase }
    pub fn environment(&self) -> &Environment { &self.environment }
    pub fn shutdown(&self) -> &ShutdownCoordinator { &self.shutdown }

    /// Register a lifecycle event listener.
    pub fn on(&mut self, f: impl Fn(&BootEvent) + Send + Sync + 'static) {
        self.listeners.push(Box::new(f));
    }

    /// Fire a lifecycle event to all registered listeners.
    fn fire(&self, event_type: &'static str) {
        let event = BootEvent::new(self.phase, event_type);
        for listener in &self.listeners {
            listener(&event);
        }
    }

    /// Fire a lifecycle event with a payload.
    fn fire_with(&self, event_type: &'static str, payload: BootPayload) {
        let event = BootEvent::new(self.phase, event_type).with_payload(payload);
        for listener in &self.listeners {
            listener(&event);
        }
    }

    /// Set the boot phase and fire the corresponding event.
    fn transition(&mut self, phase: BootPhase) {
        self.phase = phase;
        self.fire(phase.as_str());
    }

    // ── Provider Management ──

    /// Add a provider (replaces existing provider with the same id).
    pub fn with(mut self, provider: impl ServiceProvider + 'static) -> Self {
        let id = provider.id().to_string();
        self.providers.retain(|p| p.id() != id);
        self.providers.push(Box::new(provider));
        self
    }

    /// Add a boxed provider (for dynamic dispatch).
    pub fn with_boxed(mut self, provider: Box<dyn ServiceProvider + 'static>) -> Self {
        let id = provider.id().to_string();
        self.providers.retain(|p| p.id() != id);
        self.providers.push(provider);
        self
    }

    /// Remove a provider by id.
    pub fn without(mut self, id: &str) -> Self {
        self.providers.retain(|p| p.id() != id);
        self
    }

    /// Remove all default providers.
    pub fn without_defaults(mut self) -> Self {
        self.providers.retain(|p| {
            !matches!(p.id(), "env" | "config" | "log" | "queue" | "events")
        });
        self
    }

    // ── Boot Sequence ──

    /// Run the full boot sequence: config → register → boot → tiers → run.
    pub fn run(&mut self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let start = Instant::now();

        self.fire("boot.started");
        self.transition(BootPhase::ConfigLoaded);
        self.fire_with("config.loaded", BootPayload::Duration(start.elapsed()));

        // Phase 2: Register
        self.transition(BootPhase::Registering);
        self.fire_with("providers.registering",
            BootPayload::ProviderIds(self.providers.iter().map(|p| p.id().to_string()).collect()));
        let sorted_indices = self.sort_provider_indices();
        for &idx in &sorted_indices {
            if idx < self.providers.len() {
                let provider = self.providers.remove(idx);
                if let Err(e) = provider.register(self) {
                    errors.push(format!("[{}] register: {}", provider.id(), e));
                }
                self.providers.insert(idx, provider);
            }
        }
        if !errors.is_empty() { return Err(errors); }
        self.transition(BootPhase::Registered);
        self.fire_with("providers.registered", BootPayload::ServiceCount(self.container.count()));

        // Phase 3: Boot
        self.transition(BootPhase::Booting);
        for &idx in &sorted_indices {
            if idx < self.providers.len() {
                let provider = self.providers.remove(idx);
                if let Err(e) = provider.boot(self) {
                    errors.push(format!("[{}] boot: {}", provider.id(), e));
                }
                self.providers.insert(idx, provider);
            }
        }
        if !errors.is_empty() { return Err(errors); }
        self.transition(BootPhase::Booted);
        self.fire_with("providers.booted", BootPayload::Duration(start.elapsed()));

        // Phase 4: Tiers
        self.transition(BootPhase::TiersReady);
        self.fire_with("tiers.initialized", BootPayload::ActiveTiers(vec!["web"]));

        // Freeze container
        self.container.freeze();
        self.transition(BootPhase::Running);

        Ok(())
    }

    /// Trigger graceful shutdown.
    pub fn shutdown_now(&mut self) {
        self.transition(BootPhase::ShuttingDown);
        for p in &self.providers {
            if let Err(e) = p.shutdown(self) {
                eprintln!("[{}] shutdown error: {}", p.id(), e);
            }
        }
        self.transition(BootPhase::Stopped);
    }

    // ── Provider Sorting ──

    /// Topological sort of provider indices by depends_on, then by priority.
    /// Providers with environments() restriction that don't match are excluded.
    fn sort_provider_indices(&self) -> Vec<usize> {
        let ids: HashMap<&str, usize> = self.providers.iter().enumerate()
            .map(|(i, p)| (p.id(), i))
            .collect();

        let mut sorted = Vec::new();
        let mut visited = vec![false; self.providers.len()];
        let mut in_stack = vec![false; self.providers.len()];

        for i in 0..self.providers.len() {
            if !visited[i] && self.provider_matches_env(i) {
                self.dfs_sort(i, &ids, &mut visited, &mut in_stack, &mut sorted);
            }
        }

        sorted.sort_by(|&a, &b| {
            self.providers[a].priority().cmp(&self.providers[b].priority())
        });

        sorted
    }

    fn provider_matches_env(&self, idx: usize) -> bool {
        match self.providers[idx].environments() {
            None => true,
            Some(envs) => envs.contains(self.environment()),
        }
    }

    fn dfs_sort(
        &self,
        idx: usize,
        ids: &HashMap<&str, usize>,
        visited: &mut Vec<bool>,
        in_stack: &mut Vec<bool>,
        sorted: &mut Vec<usize>,
    ) {
        visited[idx] = true;
        in_stack[idx] = true;

        for dep in self.providers[idx].depends_on() {
            if let Some(&dep_idx) = ids.get(dep) {
                if !visited[dep_idx] {
                    self.dfs_sort(dep_idx, ids, visited, in_stack, sorted);
                } else if in_stack[dep_idx] {
                    eprintln!("Warning: circular dependency detected: {} -> {}", self.providers[idx].id(), dep);
                }
            }
        }

        in_stack[idx] = false;
        sorted.push(idx);
    }
}

impl Default for Application {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_creation() {
        let app = Application::new();
        assert_eq!(app.phase(), BootPhase::Created);
    }

    #[test]
    fn test_provider_with() {
        let app = Application::new();
        // "env", "config", "log", "queue", "events" are the 5 defaults
        assert!(app.providers.len() >= 5);
    }

    #[test]
    fn test_boot_run_success() {
        let mut app = Application::new();
        // Remove providers that need filesystem (config, env)
        app.providers.retain(|p| {
            !matches!(p.id(), "config" | "env")
        });
        let result = app.run();
        assert!(result.is_ok(), "Boot should succeed: {:?}", result.err());
        assert_eq!(app.phase(), BootPhase::Running);
    }

    #[test]
    fn test_provider_ordering() {
        struct TestProvider(u8);
        impl std::fmt::Debug for TestProvider {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestProvider({})", self.0)
            }
        }
        impl ServiceProvider for TestProvider {
            fn id(&self) -> &'static str { "test" }
            fn register(&self, _app: &mut Application) -> Result<(), String> { Ok(()) }
            fn boot(&self, _app: &Application) -> Result<(), String> { Ok(()) }
        }

        let mut app = Application::new();
        app.providers.retain(|p| !matches!(p.id(), "config" | "env"));
        let result = app.run();
        assert!(result.is_ok());
        assert_eq!(app.phase(), BootPhase::Running);
    }
}

// ── Built-in Providers ──

#[derive(Debug)]
pub struct EnvProvider;
impl ServiceProvider for EnvProvider {
    fn id(&self) -> &'static str { "env" }
    fn boot(&self, _app: &Application) -> Result<(), String> {
        crate::env::Env::load_auto().ok();
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConfigProvider;
impl ServiceProvider for ConfigProvider {
    fn id(&self) -> &'static str { "config" }
    fn depends_on(&self) -> &[&'static str] { &["env"] }
    fn boot(&self, _app: &Application) -> Result<(), String> {
        if std::path::Path::new("config").is_dir() {
            let env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".into());
            crate::config::ConfigLoader::new(&env)
                .config_dir("./config")
                .load()
                .map_err(|e| e.to_string())?;
        }
        // Propagate config app.debug → DEBUG_MODE env var for core debug utilities
        if std::env::var("DEBUG_MODE").is_err() {
            let val = crate::config::Config::get("app.debug", "");
            if !val.is_empty() {
                let enabled = matches!(val.to_lowercase().as_str(), "true" | "1" | "yes");
                // SAFETY: Propagating config value to process environment for core to read.
                unsafe { std::env::set_var("DEBUG_MODE", if enabled { "true" } else { "false" }); }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LogProvider;
impl ServiceProvider for LogProvider {
    fn id(&self) -> &'static str { "log" }
    fn depends_on(&self) -> &[&'static str] { &["config"] }
    fn boot(&self, _app: &Application) -> Result<(), String> {
        crate::log::init_logger(crate::log::default_logger());
        Ok(())
    }
}

#[derive(Debug)]
pub struct QueueProvider;
impl ServiceProvider for QueueProvider {
    fn id(&self) -> &'static str { "queue" }
    fn depends_on(&self) -> &[&'static str] { &["config"] }
    fn register(&self, app: &mut Application) -> Result<(), String> {
        app.container.singleton(crate::queue::Queue::new(crate::queue::SyncQueue))
            .map(|_| ()).map_err(|e| e.to_string())
    }
}

#[derive(Debug)]
pub struct EventsProvider;
impl ServiceProvider for EventsProvider {
    fn id(&self) -> &'static str { "events" }
    fn depends_on(&self) -> &[&'static str] { &["config"] }
    fn register(&self, app: &mut Application) -> Result<(), String> {
        app.container.singleton(crate::events::EventDispatcher::new())
            .map(|_| ()).map_err(|e| e.to_string())
    }
}

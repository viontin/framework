mod container;
mod provider;

pub use container::Container;
pub use provider::ServiceProvider;

use std::path::Path;

use crate::config::ConfigLoader;
use crate::env::load_env_auto;
use crate::log::{default_logger, init_logger};
use crate::queue::{SyncQueue, Queue as QueueFacade};
use crate::events::EventDispatcher;

#[derive(Debug)]
pub struct Application {
    pub container: Container,
    providers: Vec<Box<dyn ServiceProvider>>,
}

impl Application {
    pub fn new() -> Self {
        let mut app = Application { container: Container::new(), providers: Vec::new() };
        app.providers = vec![
            Box::new(EnvProvider),
            Box::new(ConfigProvider),
            Box::new(LogProvider),
            Box::new(QueueProvider),
            Box::new(EventsProvider),
        ];
        app
    }

    pub fn with(mut self, provider: impl ServiceProvider + 'static) -> Self {
        let name = provider.name().to_string();
        self.providers.retain(|p| p.name() != name);
        self.providers.push(Box::new(provider));
        self
    }

    pub fn with_boxed(mut self, provider: Box<dyn ServiceProvider + 'static>) -> Self {
        let name = provider.name().to_string();
        self.providers.retain(|p| p.name() != name);
        self.providers.push(provider);
        self
    }

    pub fn without(mut self, name: &str) -> Self {
        self.providers.retain(|p| p.name() != name);
        self
    }

    pub fn run(mut self) {
        let providers: Vec<Box<dyn ServiceProvider>> = std::mem::take(&mut self.providers);
        for p in &providers { p.register(&mut self); }
        for p in &providers { p.boot(&self); }
    }
}

impl Default for Application { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct EnvProvider;
impl ServiceProvider for EnvProvider {
    fn name(&self) -> &str { "env" }
    fn boot(&self, _: &Application) { if Path::new(".env").exists() { load_env_auto().ok(); } }
}

#[derive(Debug)]
pub struct ConfigProvider;
impl ServiceProvider for ConfigProvider {
    fn name(&self) -> &str { "config" }
    fn boot(&self, _: &Application) {
        if Path::new("config").is_dir() {
            let env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".into());
            let mut l = ConfigLoader::new(&env).config_dir("./config");
            l.load().ok();
        }
    }
}

#[derive(Debug)]
pub struct LogProvider;
impl ServiceProvider for LogProvider {
    fn name(&self) -> &str { "log" }
    fn boot(&self, _: &Application) { init_logger(default_logger()); }
}

#[derive(Debug)]
pub struct QueueProvider;
impl ServiceProvider for QueueProvider {
    fn name(&self) -> &str { "queue" }
    fn register(&self, app: &mut Application) { app.container.singleton(QueueFacade::new(SyncQueue)); }
}

#[derive(Debug)]
pub struct EventsProvider;
impl ServiceProvider for EventsProvider {
    fn name(&self) -> &str { "events" }
    fn register(&self, app: &mut Application) { app.container.singleton(EventDispatcher::new()); }
}

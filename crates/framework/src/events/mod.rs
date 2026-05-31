//! Event system — pub/sub event dispatching.
//!
//! Wraps viontin_core::EventDispatcher and adds framework-specific
//! convenience types (Subscriber, GenericEvent).

pub use viontin_core::{Event, EventDispatcher, Listener};

use std::sync::Mutex;
use std::sync::OnceLock;

/// A subscriber can register multiple listeners at once.
pub trait Subscriber: Send + Sync {
    fn subscribe(&self, dispatcher: &mut EventDispatcher);
}

/// Generic event for ad-hoc usage without defining custom types.
#[derive(Debug, Clone)]
pub struct GenericEvent {
    pub name: String,
    pub payload: Option<String>,
}

impl Event for GenericEvent {
    fn event_name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }
}

// ── Global Dispatcher ──

static GLOBAL: OnceLock<Mutex<EventDispatcher>> = OnceLock::new();

fn global() -> &'static Mutex<EventDispatcher> {
    GLOBAL.get_or_init(|| Mutex::new(EventDispatcher::new()))
}

pub fn init(dispatcher: EventDispatcher) {
    if let Ok(mut g) = global().lock() { *g = dispatcher; }
}

pub fn dispatch<E: Event>(event: &E) {
    if let Ok(g) = global().lock() { g.dispatch(event); }
}

pub fn dispatch_all(events: &[Box<dyn std::any::Any + Send + Sync>]) {
    if let Ok(g) = global().lock() {
        for event in events {
            g.dispatch_any(event.as_ref());
        }
    }
}

pub fn subscribe(subscriber: impl Subscriber) {
    if let Ok(mut g) = global().lock() {
        subscriber.subscribe(&mut g);
    }
}

pub fn listen_wildcard(f: impl Fn(&str, &dyn std::any::Any) + Send + Sync + 'static) {
    if let Ok(mut g) = global().lock() {
        g.listen_wildcard(f);
    }
}

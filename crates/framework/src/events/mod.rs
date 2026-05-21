use std::collections::HashMap;
use std::fmt;

pub trait Event: fmt::Debug + Send + Sync + 'static {}

pub trait Listener: fmt::Debug + Send + Sync {
    fn listens(&self) -> Vec<&'static str>;
    fn handle(&self, event: &dyn Event);
}

pub trait Subscriber: fmt::Debug + Send + Sync {
    fn subscribe(&self, dispatcher: &mut dyn Subscribable);
}

pub trait Subscribable: fmt::Debug + Send + Sync {
    fn listen(&mut self, event: &'static str, listener: Box<dyn Listener>);
}

#[derive(Debug, Clone)]
pub struct GenericEvent {
    pub name: String,
    pub payload: Option<String>,
}

impl Event for GenericEvent {}

#[derive(Debug, Default)]
pub struct EventDispatcher {
    listeners: HashMap<&'static str, Vec<Box<dyn Listener>>>,
}

impl EventDispatcher {
    pub fn new() -> Self { EventDispatcher { listeners: HashMap::new() } }

    pub fn dispatch(&self, event: &dyn Event) {
        let type_name = std::any::type_name_of_val(event);
        if let Some(listeners) = self.listeners.get(type_name) {
            for listener in listeners {
                listener.handle(event);
            }
        }
        if let Some(wildcards) = self.listeners.get("*") {
            for listener in wildcards {
                listener.handle(event);
            }
        }
    }

    pub fn add_subscriber(&mut self, subscriber: impl Subscriber + 'static) {
        subscriber.subscribe(self);
    }
}

impl Subscribable for EventDispatcher {
    fn listen(&mut self, event: &'static str, listener: Box<dyn Listener>) {
        self.listeners.entry(event).or_default().push(listener);
    }
}

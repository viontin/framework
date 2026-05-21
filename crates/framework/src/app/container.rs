use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
pub struct Container {
    singletons: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Container {
    pub fn new() -> Self { Container { singletons: HashMap::new() } }

    pub fn singleton<T: Any + Send + Sync>(&mut self, value: T) {
        self.singletons.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn make<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.singletons.get(&TypeId::of::<T>()).and_then(|b| b.downcast_ref::<T>())
    }
}

impl fmt::Debug for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Container({} singletons)", self.singletons.len())
    }
}

use std::fmt;

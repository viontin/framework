pub mod binding;
pub use binding::GemBinding;

pub mod validator;

use std::collections::HashMap;
use std::fmt;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct GemMeta {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub kind: GemKind,
    pub homepage: &'static str,
}

impl GemMeta {
    pub const fn new(name: &'static str, version: &'static str, description: &'static str, kind: GemKind) -> Self {
        GemMeta { name, version, description, kind, homepage: "" }
    }
    pub fn homepage(mut self, url: &'static str) -> Self { self.homepage = url; self }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemKind {
    Integration, Platform, Database, Auth, DevTool, Theme,
    Custom(&'static str),
}

impl GemKind {
    pub fn as_str(&self) -> &'static str {
        match self { GemKind::Integration => "integration", GemKind::Platform => "platform",
            GemKind::Database => "database", GemKind::Auth => "auth",
            GemKind::DevTool => "devtool", GemKind::Theme => "theme",
            GemKind::Custom(s) => s, }
    }
}

impl fmt::Display for GemKind { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) } }

pub trait GemFacade: fmt::Debug + Send + Sync {
    fn meta(&self) -> &GemMeta;
    fn before_build(&self) -> Result<()> { Ok(()) }
    fn after_build(&self) -> Result<()> { Ok(()) }
}

pub struct SimpleGem {
    pub meta: GemMeta,
    before_build_fn: Option<Box<dyn Fn() -> Result<()> + Send + Sync>>,
    after_build_fn: Option<Box<dyn Fn() -> Result<()> + Send + Sync>>,
}

impl SimpleGem {
    pub fn new(meta: GemMeta) -> Self { SimpleGem { meta, before_build_fn: None, after_build_fn: None } }
    pub fn on_before_build(mut self, f: Box<dyn Fn() -> Result<()> + Send + Sync>) -> Self { self.before_build_fn = Some(f); self }
    pub fn on_after_build(mut self, f: Box<dyn Fn() -> Result<()> + Send + Sync>) -> Self { self.after_build_fn = Some(f); self }
}

impl fmt::Debug for SimpleGem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimpleGem").field("meta", &self.meta)
            .field("has_before_build", &self.before_build_fn.is_some())
            .field("has_after_build", &self.after_build_fn.is_some()).finish()
    }
}

impl GemFacade for SimpleGem {
    fn meta(&self) -> &GemMeta { &self.meta }
    fn before_build(&self) -> Result<()> { self.before_build_fn.as_ref().map_or(Ok(()), |f| f()) }
    fn after_build(&self) -> Result<()> { self.after_build_fn.as_ref().map_or(Ok(()), |f| f()) }
}

#[derive(Debug)]
pub struct DynamicGem {
    pub meta: GemMeta,
    pub wasm_path: String,
}

impl DynamicGem {
    pub fn new(meta: GemMeta, wasm_path: impl Into<String>) -> Self { DynamicGem { meta, wasm_path: wasm_path.into() } }
}

impl GemFacade for DynamicGem {
    fn meta(&self) -> &GemMeta { &self.meta }
}

#[derive(Debug, Default)]
pub struct GemRegistry {
    installed: HashMap<String, Box<dyn GemFacade>>,
}

impl GemRegistry {
    pub fn new() -> Self { GemRegistry { installed: HashMap::new() } }
    pub fn register(&mut self, gem: impl GemFacade + 'static) {
        let name = gem.meta().name.to_string();
        self.installed.insert(name, Box::new(gem));
    }

    /// Remove a gem by name.
    pub fn remove(mut self, name: &str) -> Self {
        self.installed.remove(name);
        self
    }

    pub fn register_dyn(&mut self, gem: Box<dyn GemFacade + 'static>) {
        let name = gem.meta().name.to_string();
        self.installed.insert(name, gem);
    }
    pub fn register_simple(&mut self, gem: SimpleGem) {
        let name = gem.meta.name.to_string();
        self.installed.insert(name, Box::new(gem));
    }
    pub fn register_dynamic(&mut self, gem: DynamicGem) {
        let name = gem.meta.name.to_string();
        self.installed.insert(name, Box::new(gem));
    }
    pub fn get(&self, name: &str) -> Option<&dyn GemFacade> { self.installed.get(name).map(|b| b.as_ref()) }
    pub fn all(&self) -> Vec<&dyn GemFacade> {
        let mut list: Vec<&dyn GemFacade> = self.installed.values().map(|b| b.as_ref()).collect();
        list.sort_by_key(|g| g.meta().name); list
    }
    pub fn by_kind(&self, kind: GemKind) -> Vec<&dyn GemFacade> {
        self.installed.values().filter(|g| g.meta().kind == kind).map(|b| b.as_ref()).collect()
    }
    pub fn before_build_all(&self) -> Result<()> {
        for gem in self.installed.values() { gem.before_build()?; }
        Ok(())
    }
    pub fn after_build_all(&self) -> Result<()> {
        for gem in self.installed.values() { gem.after_build()?; }
        Ok(())
    }
}

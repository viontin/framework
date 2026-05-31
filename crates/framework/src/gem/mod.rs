pub mod binding;
pub use binding::GemBinding;

pub mod validator;

use std::collections::HashMap;
use std::fmt;
use crate::CoreResult;

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
    pub const fn as_str(&self) -> &'static str {
        match self {
            GemKind::Integration => "Integration",
            GemKind::Platform => "Platform",
            GemKind::Database => "Database",
            GemKind::Auth => "Auth",
            GemKind::DevTool => "DevTool",
            GemKind::Theme => "Theme",
            GemKind::Custom(s) => s,
        }
    }
}

impl fmt::Display for GemKind { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) } }

pub trait GemFacade: fmt::Debug + Send + Sync {
    fn meta(&self) -> &GemMeta;
    fn before_build(&self) -> CoreResult<()> { Ok(()) }
    fn after_build(&self) -> CoreResult<()> { Ok(()) }
}

pub struct SimpleGem {
    pub meta: GemMeta,
    before_build_fn: Option<Box<dyn Fn() -> CoreResult<()> + Send + Sync>>,
    after_build_fn: Option<Box<dyn Fn() -> CoreResult<()> + Send + Sync>>,
}

impl SimpleGem {
    pub fn new(meta: GemMeta) -> Self { SimpleGem { meta, before_build_fn: None, after_build_fn: None } }
    pub fn on_before_build(mut self, f: Box<dyn Fn() -> CoreResult<()> + Send + Sync>) -> Self { self.before_build_fn = Some(f); self }
    pub fn on_after_build(mut self, f: Box<dyn Fn() -> CoreResult<()> + Send + Sync>) -> Self { self.after_build_fn = Some(f); self }
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
    fn before_build(&self) -> CoreResult<()> { self.before_build_fn.as_ref().map_or(Ok(()), |f| f()) }
    fn after_build(&self) -> CoreResult<()> { self.after_build_fn.as_ref().map_or(Ok(()), |f| f()) }
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
    pub fn get(&self, name: &str) -> Option<&Box<dyn GemFacade>> {
        self.installed.get(name)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Box<dyn GemFacade>)> {
        self.installed.iter()
    }
    pub fn before_build_all(&self) -> CoreResult<()> {
        for (_, gem) in &self.installed {
            gem.before_build()?;
        }
        Ok(())
    }
    pub fn after_build_all(&self) -> CoreResult<()> {
        for (_, gem) in &self.installed {
            gem.after_build()?;
        }
        Ok(())
    }
    pub fn remove(&mut self, name: &str) {
        self.installed.remove(name);
    }
    pub fn is_empty(&self) -> bool {
        self.installed.is_empty()
    }
    pub fn len(&self) -> usize {
        self.installed.len()
    }
}

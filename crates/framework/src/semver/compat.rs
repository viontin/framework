use crate::semver::Version;
use crate::semver::VersionReq;

pub struct Meta {
    pub name: &'static str,
    pub version: Version,
}

impl Meta {
    pub fn current() -> Self {
        Meta {
            name: env!("CARGO_PKG_NAME"),
            version: Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION must be valid semver"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Compatibility {
    pub framework: VersionReq,
    pub min_app: Option<VersionReq>,
    pub min_plugin: Option<VersionReq>,
}

impl Compatibility {
    pub fn new(framework: VersionReq) -> Self { Compatibility { framework, min_app: None, min_plugin: None } }
    pub fn with_min_app(mut self, req: VersionReq) -> Self { self.min_app = Some(req); self }
    pub fn with_min_plugin(mut self, req: VersionReq) -> Self { self.min_plugin = Some(req); self }
    pub fn check_framework(&self, fw: &Version) -> bool { self.framework.matches(fw) }
    pub fn check_plugin(&self, plugin_version: &Version) -> bool {
        self.min_plugin.as_ref().is_none_or(|req| req.matches(plugin_version))
    }
    pub fn check_app(&self, app_version: &Version) -> bool {
        self.min_app.as_ref().is_none_or(|req| req.matches(app_version))
    }
    pub fn current_framework() -> Self {
        let fw = Meta::current();
        Compatibility::new(VersionReq::Compatible(fw.version))
    }
    pub fn any() -> Self { Compatibility::new(VersionReq::Any) }
}

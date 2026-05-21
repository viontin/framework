use crate::semver::Version;
use std::fmt;

#[derive(Debug, Clone)]
pub enum VersionReq {
    Exact(Version), Compatible(Version), PatchCompatible(Version),
    GreaterEqual(Version), Greater(Version), Less(Version), LessEqual(Version),
    Any, MajorWildcard(u64), MinorWildcard(u64, u64),
    And(Box<VersionReq>, Box<VersionReq>), Or(Box<VersionReq>, Box<VersionReq>),
}

impl VersionReq {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.contains("||") {
            let parts: Vec<&str> = s.split("||").collect();
            let mut req = VersionReq::parse(parts[0].trim())?;
            for part in &parts[1..] {
                req = VersionReq::Or(Box::new(req), Box::new(VersionReq::parse(part.trim())?));
            }
            return Ok(req);
        }
        let words: Vec<&str> = s.split_whitespace().collect();
        if words.len() > 1 {
            let mut req = VersionReq::parse(words[0])?;
            for w in &words[1..] { req = VersionReq::And(Box::new(req), Box::new(VersionReq::parse(w)?)); }
            return Ok(req);
        }
        let s = words[0];
        if s == "*" || s == "x" || s == "X" { return Ok(VersionReq::Any); }
        if let Some(r) = s.strip_prefix("^") { return Ok(VersionReq::Compatible(Version::parse(r)?)); }
        if let Some(r) = s.strip_prefix("~") { return Ok(VersionReq::PatchCompatible(Version::parse(r)?)); }
        if let Some(r) = s.strip_prefix(">=") { return Ok(VersionReq::GreaterEqual(Version::parse(r)?)); }
        if let Some(r) = s.strip_prefix(">") { return Ok(VersionReq::Greater(Version::parse(r)?)); }
        if let Some(r) = s.strip_prefix("<=") { return Ok(VersionReq::LessEqual(Version::parse(r)?)); }
        if let Some(r) = s.strip_prefix("<") { return Ok(VersionReq::Less(Version::parse(r)?)); }
        if s.ends_with(".x") || s.ends_with(".X") || s.ends_with(".*") {
            let base = s.trim_end_matches(".x").trim_end_matches(".X").trim_end_matches(".*");
            let parts: Vec<&str> = base.split('.').collect();
            if parts.len() == 1 { return Ok(VersionReq::MajorWildcard(parts[0].parse::<u64>().map_err(|_| format!("Invalid major: {}", parts[0]))?)); }
            if parts.len() == 2 { return Ok(VersionReq::MinorWildcard(parts[0].parse::<u64>().map_err(|_| format!("Invalid major: {}", parts[0]))?, parts[1].parse::<u64>().map_err(|_| format!("Invalid minor: {}", parts[1]))?)); }
        }
        let s = if let Some(r) = s.strip_prefix("=") { r } else { s };
        Ok(VersionReq::Exact(Version::parse(s)?))
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionReq::Exact(v) => version == v,
            VersionReq::Compatible(v) => {
                let compat = if v.major > 0 { version.major == v.major }
                    else if v.minor > 0 { version.major == 0 && version.minor == v.minor }
                    else { version.major == 0 && version.minor == 0 && version.patch == v.patch };
                version >= v && compat
            }
            VersionReq::PatchCompatible(v) => version >= v && version.major == v.major && version.minor == v.minor,
            VersionReq::GreaterEqual(v) => version >= v,
            VersionReq::Greater(v) => version > v,
            VersionReq::Less(v) => version < v,
            VersionReq::LessEqual(v) => version <= v,
            VersionReq::Any => true,
            VersionReq::MajorWildcard(m) => version.major == *m,
            VersionReq::MinorWildcard(m, n) => version.major == *m && version.minor == *n,
            VersionReq::And(a, b) => a.matches(version) && b.matches(version),
            VersionReq::Or(a, b) => a.matches(version) || b.matches(version),
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionReq::Exact(v) => write!(f, "{}", v),
            VersionReq::Compatible(v) => write!(f, "^{}", v),
            VersionReq::PatchCompatible(v) => write!(f, "~{}", v),
            VersionReq::GreaterEqual(v) => write!(f, ">={}", v),
            VersionReq::Greater(v) => write!(f, ">{}", v),
            VersionReq::Less(v) => write!(f, "<{}", v),
            VersionReq::LessEqual(v) => write!(f, "<={}", v),
            VersionReq::Any => write!(f, "*"),
            VersionReq::MajorWildcard(m) => write!(f, "{}.x", m),
            VersionReq::MinorWildcard(m, n) => write!(f, "{}.{}.x", m, n),
            VersionReq::And(a, b) => write!(f, "{} {}", a, b),
            VersionReq::Or(a, b) => write!(f, "{} || {}", a, b),
        }
    }
}

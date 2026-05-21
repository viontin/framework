use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<String>,
    pub build: Vec<String>,
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Version { major, minor, patch, pre: Vec::new(), build: Vec::new() }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        let (rest, build_str) = match s.find('+') {
            Some(pos) => (&s[..pos], Some(&s[pos + 1..])), None => (s, None),
        };
        let (ver_part, pre_str) = match rest.find('-') {
            Some(pos) => (&rest[..pos], Some(&rest[pos + 1..])), None => (rest, None),
        };
        let parts: Vec<&str> = ver_part.split('.').collect();
        if parts.len() != 3 { return Err(format!("Invalid semver: {}", s)); }
        let major = parts[0].parse::<u64>().map_err(|_| format!("Invalid major: {}", parts[0]))?;
        let minor = parts[1].parse::<u64>().map_err(|_| format!("Invalid minor: {}", parts[1]))?;
        let patch = parts[2].parse::<u64>().map_err(|_| format!("Invalid patch: {}", parts[2]))?;
        let pre = pre_str.map(|p| p.split('.').map(|s| s.to_string()).collect()).unwrap_or_default();
        let build = build_str.map(|b| b.split('.').map(|s| s.to_string()).collect()).unwrap_or_default();
        Ok(Version { major, minor, patch, pre, build })
    }
    pub fn is_prerelease(&self) -> bool { !self.pre.is_empty() }
    pub fn cmp_core(&self, other: &Version) -> Ordering {
        match self.major.cmp(&other.major) { Ordering::Equal => {} ord => return ord }
        match self.minor.cmp(&other.minor) { Ordering::Equal => {} ord => return ord }
        self.patch.cmp(&other.patch)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() { write!(f, "-{}", self.pre.join("."))?; }
        if !self.build.is_empty() { write!(f, "+{}", self.build.join("."))?; }
        Ok(())
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.cmp_core(other) { Ordering::Equal => {} ord => return ord }
        match (self.is_prerelease(), other.is_prerelease()) {
            (true, false) => Ordering::Less, (false, true) => Ordering::Greater, _ => {
                for (a, b) in self.pre.iter().zip(other.pre.iter()) {
                    match (a.parse::<u64>(), b.parse::<u64>()) {
                        (Ok(an), Ok(bn)) => match an.cmp(&bn) { Ordering::Equal => continue, ord => return ord },
                        _ => match a.cmp(b) { Ordering::Equal => continue, ord => return ord },
                    }
                }
                self.pre.len().cmp(&other.pre.len())
            }
        }
    }
}
impl PartialOrd for Version { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }

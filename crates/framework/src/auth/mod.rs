use std::collections::HashMap;
use std::fmt;
use crate::support::hash::SimpleHasher;
use crate::support::Hasher;

pub mod jwt;

pub use jwt::{JwtGuard, TokenClaims};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult { Success, Failure(String) }

impl AuthResult {
    pub fn ok() -> Self { AuthResult::Success }
    pub fn fail(reason: impl Into<String>) -> Self { AuthResult::Failure(reason.into()) }
    pub fn is_success(&self) -> bool { matches!(self, AuthResult::Success) }
}

pub trait AuthGuard: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, credentials: &HashMap<String, String>) -> AuthResult;
    fn user_id(&self) -> Option<String>;
    fn set_user(&mut self, id: String);
    fn logout(&mut self);
}

pub trait AuthUser {
    fn auth_id(&self) -> String;
    fn auth_password(&self) -> &str;
}

pub struct BasicGuard {
    name: String, current_user: Option<String>,
    provider: Box<dyn Fn(&str, &str) -> bool + Send + Sync>,
    hasher: Box<dyn Hasher>,
}

impl fmt::Debug for BasicGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicGuard")
            .field("name", &self.name)
            .field("current_user", &self.current_user)
            .field("hasher", &"..")
            .finish()
    }
}

impl BasicGuard {
    pub fn new(name: impl Into<String>, provider: impl Fn(&str, &str) -> bool + Send + Sync + 'static) -> Self {
        BasicGuard { name: name.into(), current_user: None, provider: Box::new(provider), hasher: Box::new(SimpleHasher) }
    }
    pub fn with_hasher(mut self, hasher: impl Hasher + 'static) -> Self { self.hasher = Box::new(hasher); self }
}

impl AuthGuard for BasicGuard {
    fn name(&self) -> &str { &self.name }
    fn validate(&self, credentials: &HashMap<String, String>) -> AuthResult {
        let username = credentials.get("username").or_else(|| credentials.get("email"));
        let password = credentials.get("password");
        match (username, password) {
            (Some(u), Some(p)) if (self.provider)(u, p) => AuthResult::Success,
            _ => AuthResult::fail("Invalid credentials"),
        }
    }
    fn user_id(&self) -> Option<String> { self.current_user.clone() }
    fn set_user(&mut self, id: String) { self.current_user = Some(id); }
    fn logout(&mut self) { self.current_user = None; }
}

#[derive(Debug)]
pub struct SessionGuard { guard: Box<dyn AuthGuard>, }

impl SessionGuard {
    pub fn new(guard: impl AuthGuard + 'static) -> Self { SessionGuard { guard: Box::new(guard) } }
}

impl AuthGuard for SessionGuard {
    fn name(&self) -> &str { self.guard.name() }
    fn validate(&self, c: &HashMap<String, String>) -> AuthResult { self.guard.validate(c) }
    fn user_id(&self) -> Option<String> { self.guard.user_id() }
    fn set_user(&mut self, id: String) { self.guard.set_user(id); }
    fn logout(&mut self) { self.guard.logout(); }
}

#[derive(Debug)]
pub struct TokenGuard {
    name: String,
    current_user: Option<String>,
    valid_tokens: Vec<String>,
}

impl TokenGuard {
    pub fn new(name: impl Into<String>, tokens: Vec<String>) -> Self {
        TokenGuard { name: name.into(), current_user: None, valid_tokens: tokens }
    }
}

impl AuthGuard for TokenGuard {
    fn name(&self) -> &str { &self.name }
    fn validate(&self, credentials: &HashMap<String, String>) -> AuthResult {
        match credentials.get("token") {
            Some(token) if self.valid_tokens.contains(token) => AuthResult::Success,
            _ => AuthResult::fail("Invalid token"),
        }
    }
    fn user_id(&self) -> Option<String> { self.current_user.clone() }
    fn set_user(&mut self, id: String) { self.current_user = Some(id); }
    fn logout(&mut self) { self.current_user = None; }
}

#[derive(Debug)]
pub struct Auth { guards: HashMap<String, Box<dyn AuthGuard>>, default: String, }

impl Auth {
    pub fn new() -> Self { Auth { guards: HashMap::new(), default: "web".into() } }
    pub fn register(&mut self, name: impl Into<String>, guard: impl AuthGuard + 'static) { self.guards.insert(name.into(), Box::new(guard)); }
    pub fn guard(&self, name: &str) -> Option<&dyn AuthGuard> { self.guards.get(name).map(|g| g.as_ref()) }
    pub fn attempt(&self, credentials: HashMap<String, String>) -> AuthResult {
        self.guards.get(&self.default).map_or_else(|| AuthResult::fail("No guard"), |g| g.validate(&credentials))
    }
    pub fn user(&self) -> Option<String> { self.guards.get(&self.default).and_then(|g| g.user_id()) }
    pub fn is_authenticated(&self) -> bool { self.user().is_some() }
}
impl Default for Auth { fn default() -> Self { Self::new() } }

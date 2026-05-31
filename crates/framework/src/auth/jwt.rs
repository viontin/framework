//! JWT Guard — validate Bearer tokens from Authorization headers.
//!
//! The guard accepts a validation callback that checks the token string
//! and returns user claims on success. This keeps JWT flexible — users
//! can plug in any token format (JWT, PASETO, opaque) and any crypto library.

use std::collections::HashMap;
use std::fmt;
use crate::auth::{AuthGuard, AuthResult};
use crate::http::Request;

/// Claims extracted from a validated token.
#[derive(Debug, Clone)]
pub struct TokenClaims {
    pub subject: String,
    pub expires_at: Option<u64>,
    pub data: HashMap<String, String>,
}

impl TokenClaims {
    pub fn new(subject: &str) -> Self {
        TokenClaims {
            subject: subject.to_string(),
            expires_at: None,
            data: HashMap::new(),
        }
    }

    pub fn with_expiry(mut self, ts: u64) -> Self {
        self.expires_at = Some(ts);
        self
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return now > exp;
        }
        false
    }
}

/// JWT Auth Guard — validates Bearer tokens against a user-provided callback.
///
/// The callback receives the raw token string and must return either
/// `Ok(TokenClaims)` on success or `Err(String)` with a failure reason.
pub struct JwtGuard {
    name: String,
    current_user: Option<String>,
    current_claims: Option<TokenClaims>,
    validator: Box<dyn Fn(&str) -> Result<TokenClaims, String> + Send + Sync>,
}

impl fmt::Debug for JwtGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtGuard")
            .field("name", &self.name)
            .field("current_user", &self.current_user)
            .finish()
    }
}

impl JwtGuard {
    pub fn new(
        name: impl Into<String>,
        validator: impl Fn(&str) -> Result<TokenClaims, String> + Send + Sync + 'static,
    ) -> Self {
        JwtGuard {
            name: name.into(),
            current_user: None,
            current_claims: None,
            validator: Box::new(validator),
        }
    }

    /// Extract the Bearer token from an HTTP request.
    pub fn extract_token(req: &Request) -> Option<String> {
        req.header("authorization")
            .and_then(|h| {
                let lower = h.to_lowercase();
                if lower.starts_with("bearer ") {
                    Some(h[7..].trim().to_string())
                } else {
                    None
                }
            })
    }

    /// Validate a token using the configured validator.
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, String> {
        (self.validator)(token)
    }

    /// Get the current validated claims.
    pub fn claims(&self) -> Option<&TokenClaims> {
        self.current_claims.as_ref()
    }

    /// Attempt authentication from a request. Updates internal state on success.
    pub fn authenticate(&mut self, req: &Request) -> AuthResult {
        match Self::extract_token(req) {
            Some(token) => match self.validate_token(&token) {
                Ok(claims) => {
                    if claims.is_expired() {
                        return AuthResult::fail("Token expired");
                    }
                    self.current_user = Some(claims.subject.clone());
                    self.current_claims = Some(claims);
                    AuthResult::Success
                }
                Err(e) => AuthResult::fail(e),
            },
            None => AuthResult::fail("Missing Bearer token"),
        }
    }
}

impl AuthGuard for JwtGuard {
    fn name(&self) -> &str { &self.name }
    fn validate(&self, credentials: &HashMap<String, String>) -> AuthResult {
        match credentials.get("token") {
            Some(token) => match self.validate_token(token) {
                Ok(_) => AuthResult::Success,
                Err(e) => AuthResult::fail(e),
            },
            None => AuthResult::fail("No token provided"),
        }
    }
    fn user_id(&self) -> Option<String> { self.current_user.clone() }
    fn set_user(&mut self, id: String) { self.current_user = Some(id); }
    fn logout(&mut self) {
        self.current_user = None;
        self.current_claims = None;
    }
}

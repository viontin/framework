//! CSRF manager — generates, stores, and validates tokens.
//!
//! Integration:
//! - `CsrfMiddleware` can be used in the HTTP pipeline
//! - Tokens are stored in the session under `_csrf_token`
//! - Auto-generated for GET requests, validated for state-changing methods

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CsrfConfig {
    pub token_length: usize,
    pub header_name: String,
    pub form_key: String,
    pub protected_methods: Vec<&'static str>,
    pub excluded_paths: Vec<String>,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        CsrfConfig {
            token_length: 32,
            header_name: "X-CSRF-Token".into(),
            form_key: "_csrf_token".into(),
            protected_methods: vec!["POST", "PUT", "PATCH", "DELETE"],
            excluded_paths: vec![],
        }
    }
}
use crate::session::Session;

/// Manages CSRF token lifecycle — generation, validation, and storage.
#[derive(Debug)]
#[derive(Default)]
pub struct CsrfManager {
    config: CsrfConfig,
}

impl CsrfManager {
    pub fn new(config: CsrfConfig) -> Self { CsrfManager { config } }

    /// Generate a new CSRF token.
    pub fn generate(&self) -> String {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let mut state = (seed as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut bytes = Vec::with_capacity(self.config.token_length);
        for _ in 0..self.config.token_length {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            bytes.push((state >> 33) as u8);
        }
        hex::encode(&bytes)
    }

    /// Get the current token from session, or generate one.
    pub fn get_or_create(&self, session: &mut Session) -> String {
        match session.get("_csrf_token") {
            Some(token) => token,
            None => {
                let token = self.generate();
                session.set("_csrf_token", &token);
                token
            }
        }
    }

    /// Validate a token against what's stored in session.
    pub fn validate(&self, session: &Session, token: &str) -> bool {
        session.peek("_csrf_token").is_some_and(|stored| {
            // Constant-time comparison to prevent timing attacks
            if stored.len() != token.len() { return false; }
            let mut result = 0u8;
            for (a, b) in stored.bytes().zip(token.bytes()) {
                result |= a ^ b;
            }
            result == 0
        })
    }

    /// Check if a request method requires CSRF protection.
    pub fn needs_protection(&self, method: &str, path: &str) -> bool {
        if self.config.excluded_paths.iter().any(|e| path.starts_with(e)) {
            return false;
        }
        self.config.protected_methods.contains(&method)
    }

    pub fn config(&self) -> &CsrfConfig { &self.config }
}


use crate::http::{Request, Response, StatusCode, Cookie};
use crate::middleware::{Middleware, Next};

const CSRF_COOKIE: &str = "XSRF-TOKEN";

/// CSRF Middleware — validates tokens on state-changing requests.
///
/// Uses the double-submit cookie pattern (no server-side session needed):
/// - On GET: generates a token, sets it as a non-http-only cookie, injects into request
/// - On POST/PUT/PATCH/DELETE: validates the cookie value against:
///   1. `X-CSRF-Token` HTTP header, or
///   2. `_csrf_token` form field in request body, or
///   3. `X-XSRF-Token` header (for frameworks like Axios)
///
/// If validation fails, returns 403 Forbidden.
#[derive(Debug)]
pub struct CsrfMiddleware {
    manager: CsrfManager,
}

impl CsrfMiddleware {
    pub fn new(config: CsrfConfig) -> Self {
        CsrfMiddleware { manager: CsrfManager::new(config) }
    }

    pub fn new_default() -> Self {
        CsrfMiddleware { manager: CsrfManager::new(CsrfConfig::default()) }
    }
}

impl Middleware for CsrfMiddleware {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        let method = req.method.as_str();
        let path = &req.uri.path;

        if !self.manager.needs_protection(method, path) {
            let token = self.manager.generate();
            req.set_extension::<String>(token.clone());
            let mut resp = next(req);
            let mut cookie = Cookie::new(CSRF_COOKIE, token);
            cookie.path = Some("/".to_string());
            cookie.http_only = false;
            resp = resp.cookie(cookie);
            return resp;
        }

        let stored = match req.cookie(CSRF_COOKIE).or_else(|| req.header("x-xsrf-token").map(|s| s.to_string())) {
            Some(t) => t,
            None => return Response::new(StatusCode::FORBIDDEN).with_body("CSRF token cookie missing"),
        };

        let provided = req.header("x-csrf-token")
            .map(|s| s.to_string())
            .or_else(|| {
                req.body_str()
                    .split('&')
                    .find(|p| p.starts_with("_csrf_token="))
                    .map(|p| p.split('=').nth(1).unwrap_or("").to_string())
            });

        match provided {
            Some(token) if self.manager.constant_time_eq(&stored, &token) => next(req),
            Some(_) => Response::new(StatusCode::FORBIDDEN).with_body("Invalid CSRF token"),
            None => Response::new(StatusCode::FORBIDDEN).with_body("CSRF token missing from request"),
        }
    }
}

impl CsrfManager {
    fn constant_time_eq(&self, a: &str, b: &str) -> bool {
        if a.len() != b.len() { return false; }
        let mut result = 0u8;
        for (x, y) in a.bytes().zip(b.bytes()) {
            result |= x ^ y;
        }
        result == 0
    }
}

/// Simple hex encoding without external deps.
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        const CHARS: &[u8] = b"0123456789abcdef";
        let mut result = Vec::with_capacity(bytes.len() * 2);
        for &b in bytes {
            result.push(CHARS[(b >> 4) as usize]);
            result.push(CHARS[(b & 0x0f) as usize]);
        }
        String::from_utf8(result).unwrap_or_else(|_| String::new())
    }
}

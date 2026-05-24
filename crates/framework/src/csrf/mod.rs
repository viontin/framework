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
        session.peek("_csrf_token").map_or(false, |stored| {
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
        self.config.protected_methods.iter().any(|m| *m == method)
    }

    pub fn config(&self) -> &CsrfConfig { &self.config }
}

impl Default for CsrfManager {
    fn default() -> Self {
        CsrfManager { config: CsrfConfig::default() }
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

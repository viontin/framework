use std::fmt;
use std::sync::Arc;
use crate::http::{Request, Response, StatusCode};
use super::{Middleware, Next};

type AuthFn = Arc<dyn Fn(&Request) -> Result<String, String> + Send + Sync>;

#[derive(Clone)]
pub struct AuthMiddleware {
    name: &'static str,
    checker: AuthFn,
}

impl fmt::Debug for AuthMiddleware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AuthMiddleware({})", self.name)
    }
}

impl AuthMiddleware {
    pub fn new(name: &'static str, checker: AuthFn) -> Self {
        AuthMiddleware { name, checker }
    }

    /// Authenticate via Bearer token.
    pub fn bearer(name: &'static str, valid_tokens: Vec<String>) -> Self {
        AuthMiddleware {
            name,
            checker: Arc::new(move |req| {
                let auth = req.header("authorization").ok_or("Missing Authorization header")?;
                let token = auth.strip_prefix("Bearer ").ok_or("Not a Bearer token")?;
                if valid_tokens.contains(&token.to_string()) {
                    Ok("authenticated".into())
                } else {
                    Err("Invalid token".into())
                }
            }),
        }
    }

    /// Authenticate via Basic auth with a credential validator.
    pub fn basic<F>(name: &'static str, validator: F) -> Self
    where
        F: Fn(&str, &str) -> bool + Send + Sync + 'static,
    {
        AuthMiddleware {
            name,
            checker: Arc::new(move |req| {
                let auth = req.header("authorization").ok_or("Missing Authorization header")?;
                let encoded = auth.strip_prefix("Basic ").ok_or("Not Basic auth")?;
                let decoded = simple_base64_decode(encoded).map_err(|_| "Invalid base64")?;
                let (user, pass) = decoded.split_once(':').ok_or("Invalid format")?;
                if validator(user, pass) {
                    Ok(user.to_string())
                } else {
                    Err("Invalid credentials".into())
                }
            }),
        }
    }
}

impl Middleware for AuthMiddleware {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        match (self.checker)(req) {
            Ok(_user) => next(req),
            Err(msg) => Response::new(StatusCode::UNAUTHORIZED)
                .header("www-authenticate", "Bearer")
                .with_body(msg),
        }
    }
}

fn simple_base64_decode(input: &str) -> Result<String, String> {
    let input = input.trim().bytes().filter(|&b| b != b'=' && b != b'\n' && b != b'\r').collect::<Vec<_>>();
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let idx = |b: u8| -> Option<u8> { charset.iter().position(|&c| c == b).map(|i| i as u8) };
    let mut out = Vec::new();
    for chunk in input.chunks(4) {
        if chunk.len() < 2 { break; }
        let a = idx(chunk[0]).unwrap_or(0);
        let b = idx(chunk[1]).unwrap_or(0);
        let c = chunk.get(2).map(|&b| idx(b).unwrap_or(0)).unwrap_or(0);
        let d = chunk.get(3).map(|&b| idx(b).unwrap_or(0)).unwrap_or(0);
        out.push((a << 2) | (b >> 4));
        if chunk.len() > 2 { out.push((b << 4) | (c >> 2)); }
        if chunk.len() > 3 { out.push((c << 6) | d); }
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

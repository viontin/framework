//! HTTP types — request, response, and protocol primitives.
//!
//! These types are defined here (not in kernel) because HTTP is
//! a Tier 1 (web framework) concern. Other tiers do not need them.

pub mod form_request;

#[cfg(feature = "http-client")]
pub mod client;

/// HTTP handler type alias.
pub type Handler = std::sync::Arc<dyn Fn(&Request) -> Response + Send + Sync>;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;

// ── StatusCode ──

/// HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub const OK: Self = StatusCode(200);
    pub const CREATED: Self = StatusCode(201);
    pub const NO_CONTENT: Self = StatusCode(204);
    pub const MOVED: Self = StatusCode(301);
    pub const FOUND: Self = StatusCode(302);
    pub const BAD_REQUEST: Self = StatusCode(400);
    pub const UNAUTHORIZED: Self = StatusCode(401);
    pub const FORBIDDEN: Self = StatusCode(403);
    pub const NOT_FOUND: Self = StatusCode(404);
    pub const METHOD_NOT_ALLOWED: Self = StatusCode(405);
    pub const CONFLICT: Self = StatusCode(409);
    pub const UNPROCESSABLE: Self = StatusCode(422);
    pub const TOO_MANY_REQUESTS: Self = StatusCode(429);
    pub const SERVER_ERROR: Self = StatusCode(500);
    pub const BAD_GATEWAY: Self = StatusCode(502);
    pub const SERVICE_UNAVAILABLE: Self = StatusCode(503);

    pub fn new(c: u16) -> Self { StatusCode(c) }
    pub fn as_u16(&self) -> u16 { self.0 }
    pub fn is_success(&self) -> bool { self.0 >= 200 && self.0 < 300 }
    pub fn is_client_error(&self) -> bool { self.0 >= 400 && self.0 < 500 }
    pub fn is_server_error(&self) -> bool { self.0 >= 500 }
    pub fn as_str(&self) -> &'static str {
        match self.0 {
            200 => "OK", 201 => "Created", 204 => "No Content",
            301 => "Moved Permanently", 302 => "Found",
            400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden",
            404 => "Not Found", 405 => "Method Not Allowed", 409 => "Conflict",
            422 => "Unprocessable Entity", 429 => "Too Many Requests",
            500 => "Internal Server Error", 502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.as_str())
    }
}

// ── Method ──

/// HTTP method.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Custom(String),
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Method::Get => "GET", Method::Post => "POST", Method::Put => "PUT",
            Method::Patch => "PATCH", Method::Delete => "DELETE",
            Method::Head => "HEAD", Method::Options => "OPTIONS",
            Method::Custom(s) => s,
        }
    }
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => Method::Get, "POST" => Method::Post, "PUT" => Method::Put,
            "PATCH" => Method::Patch, "DELETE" => Method::Delete,
            "HEAD" => Method::Head, "OPTIONS" => Method::Options,
            _ => Method::Custom(s.into()),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Headers ──

/// HTTP headers (case-insensitive keys).
#[derive(Debug, Clone)]
pub struct Headers {
    data: HashMap<String, String>,
}

impl Headers {
    pub fn new() -> Self { Headers { data: HashMap::new() } }
    pub fn set(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.data.insert(k.into().to_lowercase(), v.into());
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.data.get(&k.to_lowercase()).map(|s| s.as_str())
    }
    pub fn has(&self, k: &str) -> bool { self.data.contains_key(&k.to_lowercase()) }
    pub fn all(&self) -> &HashMap<String, String> { &self.data }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.data.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
    pub fn content_length(&self) -> Option<u64> {
        self.get("content-length").and_then(|s| s.parse().ok())
    }
    pub fn content_type(&self) -> Option<&str> { self.get("content-type") }
}

impl Default for Headers { fn default() -> Self { Self::new() } }

// ── Uri ──

#[derive(Debug, Clone, Default)]
pub struct Uri {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: HashMap<String, String>,
    pub fragment: Option<String>,
}

fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

impl Uri {
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        let (rest, scheme) = if let Some(pos) = s.find("://") {
            (&s[pos + 3..], &s[..pos])
        } else {
            (s, "http")
        };
        let (authority, path_query) = match rest.find('/') {
            Some(pos) => (&rest[..pos], &rest[pos..]),
            None => (rest, "/"),
        };
        let (host, port) = if let Some(pos) = authority.find(':') {
            (&authority[..pos], authority[pos + 1..].parse::<u16>().unwrap_or(80))
        } else {
            (authority, if scheme == "https" { 443 } else { 80 })
        };
        let (path_str, fragment) = if let Some(pos) = path_query.find('#') {
            (&path_query[..pos], Some(path_query[pos + 1..].to_string()))
        } else {
            (path_query, None)
        };
        let (path, query) = if let Some(pos) = path_str.find('?') {
            let qs = &path_str[pos + 1..];
            let mut m = HashMap::new();
            for pair in qs.split('&') {
                if let Some(eq) = pair.find('=') {
                    m.insert(url_decode(&pair[..eq]), url_decode(&pair[eq + 1..]));
                } else if !pair.is_empty() {
                    m.insert(url_decode(pair), String::new());
                }
            }
            (&path_str[..pos], m)
        } else {
            (path_str, HashMap::new())
        };
        Uri {
            scheme: scheme.into(),
            host: host.into(),
            port,
            path: path.into(),
            query,
            fragment,
        }
    }
}

// ── Cookie ──

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub expires: Option<u64>,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Cookie {
            name: name.into(), value: value.into(),
            expires: None, path: Some("/".into()),
            domain: None, secure: false, http_only: true,
            same_site: Some("Lax".into()),
        }
    }
    pub fn to_header_string(&self) -> String {
        let mut s = format!("{}={}", self.name, self.value);
        if let Some(expires) = self.expires {
            s.push_str(&format!("; Max-Age={}", expires));
        }
        if let Some(path) = &self.path { s.push_str(&format!("; Path={}", path)); }
        if let Some(domain) = &self.domain { s.push_str(&format!("; Domain={}", domain)); }
        if self.secure { s.push_str("; Secure"); }
        if self.http_only { s.push_str("; HttpOnly"); }
        if let Some(same_site) = &self.same_site { s.push_str(&format!("; SameSite={}", same_site)); }
        s
    }
}

// ── Request ──

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub headers: Headers,
    pub body: Vec<u8>,
    pub params: HashMap<String, String>,
    pub extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Clone for Request {
    fn clone(&self) -> Self {
        Request {
            method: self.method.clone(),
            uri: self.uri.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            params: self.params.clone(),
            extensions: HashMap::new(),
        }
    }
}

impl Default for Request {
    fn default() -> Self {
        Request {
            method: Method::default(),
            uri: Uri::default(),
            headers: Headers::new(),
            body: Vec::new(),
            params: HashMap::new(),
            extensions: HashMap::new(),
        }
    }
}

fn parse_cookie_header(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in s.split(';') {
        let pair = pair.trim();
        if let Some(eq) = pair.find('=') {
            map.insert(pair[..eq].trim().to_string(), pair[eq + 1..].trim().to_string());
        } else if !pair.is_empty() {
            map.insert(pair.to_string(), String::new());
        }
    }
    map
}

impl Request {
    pub fn new(method: Method, uri: Uri, headers: Headers, body: Vec<u8>) -> Self {
        Request { method, uri, headers, body, params: HashMap::new(), extensions: HashMap::new() }
    }
    pub fn extension<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.extensions.get(&TypeId::of::<T>())
            .and_then(|e| e.downcast_ref::<T>())
            .cloned()
    }
    pub fn set_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.extensions.insert(TypeId::of::<T>(), Box::new(val));
        self
    }
    pub fn body_str(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap_or("")
    }
    pub fn query(&self, key: &str) -> Option<&str> {
        self.uri.query.get(key).map(|s| s.as_str())
    }
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key)
    }
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }
    pub fn param_names(&self) -> Vec<&str> {
        self.params.keys().map(|s| s.as_str()).collect()
    }
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_slice(&self.body).map_err(|e| format!("JSON parse error: {}", e))
    }
    pub fn cookie(&self, key: &str) -> Option<String> {
        self.cookies().remove(key)
    }
    pub fn cookies(&self) -> HashMap<String, String> {
        self.headers.get("cookie").map(parse_cookie_header).unwrap_or_default()
    }
}

// ── Response ──

#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Response { status, headers: Headers::new(), body: Vec::new() }
    }
    pub fn ok() -> Self { Response::new(StatusCode::OK) }
    pub fn created() -> Self { Response::new(StatusCode::CREATED) }
    pub fn no_content() -> Self { Response::new(StatusCode::NO_CONTENT) }
    pub fn not_found() -> Self { Response::new(StatusCode::NOT_FOUND) }
    pub fn html(body: &str) -> Self {
        let mut r = Response::ok();
        r.headers.set("content-type", "text/html; charset=utf-8");
        r.body = body.as_bytes().to_vec();
        r
    }
    pub fn text(body: &str) -> Self {
        let mut r = Response::ok();
        r.headers.set("content-type", "text/plain");
        r.body = body.as_bytes().to_vec();
        r
    }
    pub fn json<T: serde::Serialize>(value: &T) -> Self {
        match serde_json::to_string(value) {
            Ok(json) => {
                let mut r = Response::ok();
                r.headers.set("content-type", "application/json");
                r.body = json.into_bytes();
                r
            },
            Err(e) => {
                let mut r = Response::new(StatusCode::SERVER_ERROR);
                r.headers.set("content-type", "application/json");
                r.body = format!("{{\"error\":\"{}\"}}", e).into_bytes();
                r
            }
        }
    }
    pub fn redirect(url: &str) -> Self {
        let mut r = Response::new(StatusCode::FOUND);
        r.headers.set("location", url);
        r
    }
    pub fn redirect_permanent(url: &str) -> Self {
        let mut r = Response::new(StatusCode::MOVED);
        r.headers.set("location", url);
        r
    }
    pub fn file(path: impl AsRef<std::path::Path>) -> Self {
        let path = path.as_ref();
        match std::fs::read(path) {
            Ok(bytes) => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let mime = match ext {
                    "html" | "htm" => "text/html; charset=utf-8",
                    "css" => "text/css; charset=utf-8",
                    "js" | "mjs" => "application/javascript; charset=utf-8",
                    "json" => "application/json",
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "svg" => "image/svg+xml",
                    "ico" => "image/x-icon",
                    "wasm" => "application/wasm",
                    "woff2" => "font/woff2",
                    "woff" => "font/woff",
                    "ttf" => "font/ttf",
                    "otf" => "font/otf",
                    "pdf" => "application/pdf",
                    "txt" => "text/plain; charset=utf-8",
                    "xml" => "application/xml",
                    _ => "application/octet-stream",
                };
                let mut r = Response::ok();
                r.headers.set("content-type", mime);
                r.body = bytes;
                r
            }
            Err(_) => Response::not_found().with_body("File not found"),
        }
    }
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.set(key, value);
        self
    }
    pub fn cookie(mut self, cookie: Cookie) -> Self {
        self.headers.set("set-cookie", cookie.to_header_string());
        self
    }
    pub fn body_str(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap_or("")
    }
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    pub fn to_raw(&self) -> Vec<u8> {
        let mut raw = format!("HTTP/1.1 {}\r\n", self.status);
        for (k, v) in self.headers.iter() {
            raw.push_str(&format!("{}: {}\r\n", k, v));
        }
        raw.push_str(&format!("content-length: {}\r\n", self.body.len()));
        raw.push_str("\r\n");
        let mut bytes = raw.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}



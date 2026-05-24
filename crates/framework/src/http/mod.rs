pub mod form_request;

use std::collections::HashMap;
use std::fmt;

fn parse_cookie_header(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in s.split(';') {
        let pair = pair.trim();
        if let Some(eq) = pair.find('=') {
            let k = pair[..eq].trim().to_string();
            let v = pair[eq + 1..].trim().to_string();
            map.insert(k, v);
        } else if !pair.is_empty() {
            map.insert(pair.to_string(), String::new());
        }
    }
    map
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(pub u16);
impl StatusCode {
    pub const OK: Self = StatusCode(200); pub const CREATED: Self = StatusCode(201);
    pub const NO_CONTENT: Self = StatusCode(204); pub const BAD_REQUEST: Self = StatusCode(400);
    pub const UNAUTHORIZED: Self = StatusCode(401); pub const FORBIDDEN: Self = StatusCode(403);
    pub const NOT_FOUND: Self = StatusCode(404); pub const MOVED: Self = StatusCode(301);
    pub const FOUND: Self = StatusCode(302); pub const SERVER_ERROR: Self = StatusCode(500);
    pub fn new(c: u16) -> Self { StatusCode(c) }
    pub fn as_u16(&self) -> u16 { self.0 }
    pub fn is_success(&self) -> bool { self.0 >= 200 && self.0 < 300 }
    pub fn is_client_error(&self) -> bool { self.0 >= 400 && self.0 < 500 }
    pub fn is_server_error(&self) -> bool { self.0 >= 500 }
    pub fn as_str(&self) -> &'static str {
        match self.0 { 200 => "OK", 201 => "Created", 204 => "No Content", 301 => "Moved Permanently",
            302 => "Found", 400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden",
            404 => "Not Found", 500 => "Internal Server Error", _ => "Unknown", }
    }
}
impl fmt::Display for StatusCode { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{} {}", self.0, self.as_str()) } }

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Method { #[default] Get, Post, Put, Patch, Delete, Head, Options, Custom(String) }
impl Method {
    pub fn as_str(&self) -> &str { match self { Method::Get => "GET", Method::Post => "POST",
        Method::Put => "PUT", Method::Patch => "PATCH", Method::Delete => "DELETE",
        Method::Head => "HEAD", Method::Options => "OPTIONS", Method::Custom(s) => s, } }
    pub fn parse(s: &str) -> Self { match s.to_uppercase().as_str() { "GET" => Method::Get, "POST" => Method::Post,
        "PUT" => Method::Put, "PATCH" => Method::Patch, "DELETE" => Method::Delete,
        "HEAD" => Method::Head, "OPTIONS" => Method::Options, _ => Method::Custom(s.into()), } }
}
impl fmt::Display for Method { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) } }

#[derive(Debug, Clone)]
pub struct Headers { h: HashMap<String, String> }
impl Headers {
    pub fn new() -> Self { Headers { h: HashMap::new() } }
    pub fn set(&mut self, k: impl Into<String>, v: impl Into<String>) { self.h.insert(k.into().to_lowercase(), v.into()); }
    pub fn get(&self, k: &str) -> Option<&str> { self.h.get(&k.to_lowercase()).map(|s| s.as_str()) }
    pub fn has(&self, k: &str) -> bool { self.h.contains_key(&k.to_lowercase()) }
    pub fn all(&self) -> &HashMap<String, String> { &self.h }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> { self.h.iter().map(|(k, v)| (k.as_str(), v.as_str())) }
    pub fn content_length(&self) -> Option<u64> { self.get("content-length").and_then(|s| s.parse().ok()) }
    pub fn content_type(&self) -> Option<&str> { self.get("content-type") }
}
impl Default for Headers { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Default)]
pub struct Uri { pub scheme: String, pub host: String, pub port: u16, pub path: String,
    pub query: HashMap<String, String>, pub fragment: Option<String>, }
impl Uri {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let (rest, scheme) = if let Some(pos) = s.find("://") { (&s[pos+3..], &s[..pos]) } else { (s, "http") };
        let (authority, path_query) = match rest.find('/') { Some(pos) => (&rest[..pos], &rest[pos..]), None => (rest, "/") };
        let (host, port) = if let Some(pos) = authority.find(':') { (&authority[..pos], authority[pos+1..].parse::<u16>().unwrap_or(80)) } else { (authority, if scheme == "https" { 443 } else { 80 }) };
        let (path_str, fragment) = if let Some(pos) = path_query.find('#') { (&path_query[..pos], Some(path_query[pos+1..].to_string())) } else { (path_query, None) };
        let (path, query) = if let Some(pos) = path_str.find('?') {
            let qs = &path_str[pos+1..]; let mut m = HashMap::new();
            for pair in qs.split('&') { if let Some(eq) = pair.find('=') { m.insert(url_decode(&pair[..eq]), url_decode(&pair[eq+1..])); } else if !pair.is_empty() { m.insert(url_decode(pair), String::new()); } }
            (&path_str[..pos], m)
        } else { (path_str, HashMap::new()) };
        Ok(Uri { scheme: scheme.into(), host: host.into(), port, path: path.into(), query, fragment })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub method: Method, pub uri: Uri, pub headers: Headers, pub body: Vec<u8>,
    pub params: HashMap<String, String>,
}
impl Request {
    pub fn new(method: Method, uri: Uri, headers: Headers, body: Vec<u8>) -> Self {
        Request { method, uri, headers, body, params: HashMap::new() }
    }
    pub fn body_str(&self) -> &str { std::str::from_utf8(&self.body).unwrap_or("") }
    pub fn query(&self, key: &str) -> Option<&str> { self.uri.query.get(key).map(|s| s.as_str()) }
    pub fn header(&self, key: &str) -> Option<&str> { self.headers.get(key) }
    pub fn param(&self, key: &str) -> Option<&str> { self.params.get(key).map(|s| s.as_str()) }
    pub fn cookie(&self, key: &str) -> Option<String> { self.cookies().remove(key) }
    pub fn cookies(&self) -> HashMap<String, String> {
        self.headers.get("cookie").map(parse_cookie_header).unwrap_or_default()
    }
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_str(self.body_str()).map_err(|e| format!("JSON error: {}", e))
    }
}

#[derive(Debug, Clone)]
pub struct Response { pub status: StatusCode, pub headers: Headers, pub body: Vec<u8>, }
impl Response {
    pub fn new(status: StatusCode) -> Self { Response { status, headers: Headers::new(), body: Vec::new() } }
    pub fn ok() -> Self { Response::new(StatusCode::OK) }
    pub fn not_found() -> Self { Response::new(StatusCode::NOT_FOUND) }
    pub fn html(body: &str) -> Self { let mut r = Response::ok(); r.headers.set("content-type", "text/html; charset=utf-8"); r.body = body.as_bytes().to_vec(); r }
    pub fn json<T: serde::Serialize>(value: &T) -> Result<Self, String> { let j = serde_json::to_string(value).map_err(|e| e.to_string())?; let mut r = Response::ok(); r.headers.set("content-type", "application/json"); r.body = j.into_bytes(); Ok(r) }
    pub fn text(body: &str) -> Self { let mut r = Response::ok(); r.headers.set("content-type", "text/plain"); r.body = body.as_bytes().to_vec(); r }
    pub fn body_str(&self) -> &str { std::str::from_utf8(&self.body).unwrap_or("") }
    pub fn to_raw(&self) -> Vec<u8> {
        let mut raw = format!("HTTP/1.1 {}\r\n", self.status);
        for (k, v) in self.headers.iter() { raw.push_str(&format!("{}: {}\r\n", k, v)); }
        raw.push_str(&format!("content-length: {}\r\n", self.body.len())); raw.push_str("\r\n");
        let mut bytes = raw.into_bytes(); bytes.extend_from_slice(&self.body); bytes
    }
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self { self.body = body.into(); self }
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self { self.headers.set(key, value); self }
    pub fn cookie(mut self, cookie: Cookie) -> Self { self.headers.set("set-cookie", cookie.to_header_string()); self }
    pub fn remove_cookie(mut self, name: &str) -> Self {
        self.headers.set("set-cookie", &format!("{}=; Max-Age=0; Path=/; HttpOnly", name));
        self
    }
}

#[derive(Debug, Clone)]
pub struct Cookie { pub name: String, pub value: String, pub expires: Option<u64>,
    pub path: Option<String>, pub domain: Option<String>, pub secure: bool, pub http_only: bool, pub same_site: Option<String>, }
impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self { Cookie {
        name: name.into(), value: value.into(), expires: None, path: Some("/".into()),
        domain: None, secure: false, http_only: true, same_site: Some("Lax".into()), }
    }
    pub fn forever(name: impl Into<String>, value: impl Into<String>) -> Self {
        let mut c = Cookie::new(name, value);
        c.expires = Some(5 * 365 * 86400);
        c
    }
    pub fn forgotten(name: impl Into<String>) -> Self {
        Cookie { name: name.into(), value: String::new(), expires: Some(0), path: Some("/".into()),
            domain: None, secure: false, http_only: true, same_site: None }
    }
    pub fn without_http_only(mut self) -> Self { self.http_only = false; self }
    pub fn with_secure(mut self) -> Self { self.secure = true; self }
    pub fn with_path(mut self, path: &str) -> Self { self.path = Some(path.into()); self }
    pub fn with_domain(mut self, domain: &str) -> Self { self.domain = Some(domain.into()); self }
    pub fn with_same_site(mut self, samesite: &str) -> Self { self.same_site = Some(samesite.into()); self }
    pub fn expires_in(mut self, seconds: u64) -> Self { self.expires = Some(seconds); self }

    pub fn to_header_string(&self) -> String {
        let mut s = format!("{}={}", self.name, self.value);
        if let Some(secs) = self.expires {
            let expiry = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() + secs
                );
            let datetime = http_date(expiry);
            s.push_str(&format!("; Expires={}", datetime));
        }
        if let Some(ref path) = self.path { s.push_str(&format!("; Path={}", path)); }
        if let Some(ref domain) = self.domain { s.push_str(&format!("; Domain={}", domain)); }
        if self.secure { s.push_str("; Secure"); }
        if self.http_only { s.push_str("; HttpOnly"); }
        if let Some(ref ss) = self.same_site { s.push_str(&format!("; SameSite={}", ss)); }
        s
    }
}

fn http_date(time: std::time::SystemTime) -> String {
    const DAYS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: &[&str] = &["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let secs = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let d = secs / 86400;
    let t = secs % 86400;
    let h = t / 3600;
    let m = (t % 3600) / 60;
    let s = t % 60;
    let y_adj = d as i64 - 719468;
    let era = if y_adj >= 0 { y_adj } else { y_adj - 146096 } / 146097;
    let doe = y_adj - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + 400 * era;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp as usize + 3 } else { mp as usize - 9 };
    let year = if month <= 2 { y as usize + 1 } else { y as usize };
    let weekday = ((d + 3) % 7) as usize;
    format!("{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT", DAYS[weekday], day, MONTHS[month - 1], year, h, m, s)
}

fn url_decode(s: &str) -> String {
    let mut r = String::with_capacity(s.len()); let mut c = s.chars();
    while let Some(ch) = c.next() {
        match ch { '+' => r.push(' '), '%' => {
            let hi = c.next().and_then(|c| c.to_digit(16)); let lo = c.next().and_then(|c| c.to_digit(16));
            match (hi, lo) { (Some(h), Some(l)) => r.push(((h << 4 | l) as u8) as char), _ => r.push('%') }
        } _ => r.push(ch) }
    }
    r
}

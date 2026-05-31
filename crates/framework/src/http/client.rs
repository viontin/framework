//! Minimal HTTP client for service-to-service communication.
//!
//! Uses `ureq` behind the scenes. Enable with the `http-client` feature flag.
//!
//! # Example
//!
//! ```rust,ignore
//! use viontin_framework::http_client::HttpClient;
//!
//! let client = HttpClient::new();
//! let resp = client.get("https://api.example.com/users").call()?;
//! println!("{}", resp.body());
//! ```

use std::collections::HashMap;

/// A minimal HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn body_str(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap_or_default()
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

/// A minimal HTTP client for outbound requests.
///
/// Wraps the `ureq` crate. Intended for:
/// - Service-to-service calls in microservices
/// - Webhook callbacks
/// - External API integration
/// - `RemoteServiceAdapter` implementation
#[derive(Debug, Clone)]
pub struct HttpClient {
    base_url: String,
    timeout_secs: u64,
    default_headers: HashMap<String, String>,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            base_url: String::new(),
            timeout_secs: 30,
            default_headers: HashMap::new(),
        }
    }

    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.into()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            method: "GET".into(),
            url: self.url(path),
            headers: self.default_headers.clone(),
            body: None,
            timeout: self.timeout_secs,
        }
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            method: "POST".into(),
            url: self.url(path),
            headers: self.default_headers.clone(),
            body: None,
            timeout: self.timeout_secs,
        }
    }

    pub fn put(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            method: "PUT".into(),
            url: self.url(path),
            headers: self.default_headers.clone(),
            body: None,
            timeout: self.timeout_secs,
        }
    }

    pub fn delete(&self, path: &str) -> RequestBuilder {
        RequestBuilder {
            method: "DELETE".into(),
            url: self.url(path),
            headers: self.default_headers.clone(),
            body: None,
            timeout: self.timeout_secs,
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self { Self::new() }
}

/// A request in progress.
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    timeout: u64,
}

impl RequestBuilder {
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn json(mut self, body: &str) -> Self {
        self.headers.insert("content-type".into(), "application/json".into());
        self.body = Some(body.as_bytes().to_vec());
        self
    }

    pub fn body(mut self, data: &[u8]) -> Self {
        self.body = Some(data.to_vec());
        self
    }

    /// Execute the request using `ureq`.
    #[cfg(feature = "http-client")]
    pub fn call(&self) -> Result<HttpResponse, String> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(self.timeout))
            .timeout_read(std::time::Duration::from_secs(self.timeout))
            .timeout_write(std::time::Duration::from_secs(self.timeout))
            .build();

        let mut req = match self.method.as_str() {
            "GET" => agent.get(&self.url),
            "POST" => agent.post(&self.url),
            "PUT" => agent.put(&self.url),
            "DELETE" => agent.delete(&self.url),
            m => return Err(format!("Unsupported method: {}", m)),
        };

        for (k, v) in &self.headers {
            req = req.set(k, v);
        }

        let result = if let Some(body) = &self.body {
            req.send_bytes(body)
        } else {
            req.call()
        };

        match result {
            Ok(resp) => {
                let status = resp.status();
                let mut headers = HashMap::new();
                for name in &resp.headers_names() {
                    if let Some(val) = resp.header(name) {
                        headers.insert(name.to_lowercase(), val.to_string());
                    }
                }
                use std::io::Read;
                let mut body = Vec::new();
                resp.into_reader().read_to_end(&mut body)
                    .map_err(|e| format!("Read response: {}", e))?;
                Ok(HttpResponse { status, headers, body })
            }
            Err(ureq::Error::Status(code, resp)) => {
                use std::io::Read;
                let mut body = Vec::new();
                resp.into_reader().read_to_end(&mut body).ok();
                Ok(HttpResponse { status: code, headers: HashMap::new(), body })
            }
            Err(e) => Err(format!("HTTP request failed: {}", e)),
        }
    }

    /// Execute the request without the `http-client` feature.
    #[cfg(not(feature = "http-client"))]
    pub fn call(&self) -> Result<HttpResponse, String> {
        Err("HTTP client requires the `http-client` feature: add `http-client` to viontin-framework features".into())
    }
}

impl std::fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP/1.1 {} ({} bytes)", self.status, self.body.len())
    }
}

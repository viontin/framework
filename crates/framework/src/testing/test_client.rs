//! HTTP testing utilities — test routes without a TCP server.
//!
//! `TestClient` creates mock HTTP requests and processes them through
//! a `Router` in-process. No network I/O, no thread spawning.
//!
//! # Example
//!
//! ```rust
//! use viontin_framework::testing::TestClient;
//! use viontin_framework::http::{Request, Response, StatusCode, Method};
//! use std::sync::Arc;
//!
//! let router = viontin_framework::Router::new()
//!     .get("/hello", Arc::new(|_| Response::html("Hello!")));
//!
//! let client = TestClient::new(router);
//!
//! let res = client.get("/hello");
//! assert_eq!(res.status.0, 200);
//! assert_eq!(res.body_str(), "Hello!");
//!
//! let res = client.get("/not-found");
//! assert_eq!(res.status.0, 404);
//! ```

use crate::http::{Method, Request, Response, Uri, Headers};
use crate::server::Router;
use std::collections::HashMap;

/// A test client that sends requests through a Router in-process.
///
/// No TCP server is started — requests are processed synchronously
/// in the current thread. This makes tests fast and deterministic.
pub struct TestClient {
    router: Router,
}

impl TestClient {
    pub fn new(router: Router) -> Self {
        TestClient { router }
    }

    pub fn get(&self, path: &str) -> Response {
        self.request(Method::Get, path, "", "")
    }

    pub fn post(&self, path: &str, body: &str, content_type: &str) -> Response {
        self.request(Method::Post, path, body, content_type)
    }

    pub fn put(&self, path: &str, body: &str, content_type: &str) -> Response {
        self.request(Method::Put, path, body, content_type)
    }

    pub fn delete(&self, path: &str) -> Response {
        self.request(Method::Delete, path, "", "")
    }

    pub fn request(&self, method: Method, path: &str, body: &str, content_type: &str) -> Response {
        let uri = Uri {
            scheme: "http".into(),
            host: "test".into(),
            port: 80,
            path: path.into(),
            query: HashMap::new(),
            fragment: None,
        };

        let mut headers = Headers::new();
        headers.set("Host", "test");
        if !content_type.is_empty() {
            headers.set("Content-Type", content_type);
        }

        let req = Request::new(method, uri, headers, body.as_bytes().to_vec());
        self.router.handle(req)
    }
}

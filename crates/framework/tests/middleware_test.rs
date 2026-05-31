use std::sync::Arc;
use viontin_framework::http::{Request, Response, StatusCode, Method, Headers, Uri};
use viontin_framework::middleware::{Middleware, MiddlewareChain, Next};
use viontin_framework::server::Router;
use viontin_framework::testing::TestClient;
use std::collections::HashMap;

#[test]
fn test_middleware_chain_execution_order() {
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let order1 = order.clone();
    let mw1 = TestMiddleware::new("mw1", move |req, next| {
        order1.lock().unwrap().push("mw1-before");
        let resp = next(req);
        order1.lock().unwrap().push("mw1-after");
        resp
    });

    let order2 = order.clone();
    let mw2 = TestMiddleware::new("mw2", move |req, next| {
        order2.lock().unwrap().push("mw2-before");
        let resp = next(req);
        order2.lock().unwrap().push("mw2-after");
        resp
    });

    let mut chain = MiddlewareChain::new();
    chain.add(mw1);
    chain.add(mw2);

    let mut req = make_request("/hello");
    let result = chain.apply(&mut req, |_req: &mut Request| {
        order.lock().unwrap().push("handler");
        Response::html("Hello")
    });

    assert_eq!(result.status, StatusCode::OK);

    let log = order.lock().unwrap();
    assert_eq!(*log, vec!["mw1-before", "mw2-before", "handler", "mw2-after", "mw1-after"]);
}

#[test]
fn test_middleware_short_circuit() {
    let mut chain = MiddlewareChain::new();
    chain.add_fn("auth", |req, _next| {
        if req.header("Authorization").is_none() {
            return Response::new(StatusCode::UNAUTHORIZED).with_body("Unauthorized");
        }
        _next(req)
    });

    let router = Router::new()
        .with_global_middleware(chain)
        .get("/admin", Arc::new(|_| Response::html("Admin Panel")));

    let client = TestClient::new(router);

    // Without auth header — should be blocked
    let resp = client.get("/admin");
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn test_global_middleware_on_router() {
    let mut chain = MiddlewareChain::new();
    chain.add_fn("x-request-id", |req, next| {
        req.headers.set("x-request-id", "test-123");
        next(req)
    });

    let router = Router::new()
        .with_global_middleware(chain)
        .get("/hello", Arc::new(|req| {
            let id = req.header("x-request-id").unwrap_or("none");
            Response::text(&format!("Request ID: {}", id))
        }));

    let client = TestClient::new(router);
    let resp = client.get("/hello");

    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body_str().contains("Request ID: test-123"));
}

#[test]
fn test_middleware_chain_empty() {
    let chain = MiddlewareChain::new();
    let mut req = make_request("/hello");
    let result = chain.apply(&mut req, |_| {
        Response::html("Direct handler")
    });
    assert_eq!(result.status, StatusCode::OK);
    assert!(result.body_str().contains("Direct handler"));
}

#[test]
fn test_middleware_add_fn_debug() {
    let mut chain = MiddlewareChain::new();
    chain.add_fn("test", |req, next| next(req));
    assert_eq!(chain.len(), 1);
    assert!(!chain.is_empty());
}

// Helper: simpler middleware factory using add_fn-like pattern
struct TestMiddleware {
    name: &'static str,
    f: Box<dyn Fn(&mut Request, Next) -> Response + Send + Sync>,
}

impl std::fmt::Debug for TestMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestMiddleware").field("name", &self.name).finish()
    }
}

impl TestMiddleware {
    fn new(name: &'static str, f: impl Fn(&mut Request, Next) -> Response + Send + Sync + 'static) -> Self {
        TestMiddleware { name, f: Box::new(f) }
    }
}

impl Middleware for TestMiddleware {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        (self.f)(req, next)
    }
}

fn make_request(path: &str) -> Request {
    let uri = Uri {
        scheme: "http".into(),
        host: "test".into(),
        port: 80,
        path: path.into(),
        query: HashMap::new(),
        fragment: None,
    };
    Request::new(Method::Get, uri, Headers::new(), Vec::new())
}

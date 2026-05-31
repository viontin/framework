//! Integration tests for the boot sequence and HTTP routing.

#[cfg(test)]
mod integration {
    use std::sync::Arc;
use viontin_framework::route;
    use viontin_framework::http::{Response, StatusCode};
    use viontin_framework::server::Router;
    use viontin_framework::testing::TestClient;

    #[test]
    fn test_router_get_handler() {
        let router = Router::new()
            .get("/hello", Arc::new(|_| Response::html("Hello World!")));

        let client = TestClient::new(router);
        let resp = client.get("/hello");

        assert_eq!(resp.status, StatusCode::OK);
        assert!(resp.body_str().contains("Hello World!"));
    }

    #[test]
    fn test_router_404() {
        let router = Router::new()
            .get("/hello", Arc::new(|_| Response::html("Hello")));

        let client = TestClient::new(router);
    let resp = client.get("/not-found");
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
}

#[test]
fn test_route_facade_global_registration() {
    route::get("/facade", Arc::new(|_| Response::html("from facade")));

    // Simulate what RouteProvider does
    viontin_framework::route::build_router();
    let router = viontin_framework::route::take_router().unwrap();
    let client = viontin_framework::testing::TestClient::new(router);

    let resp = client.get("/facade");
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.body_str().contains("from facade"));
}

#[test]
fn test_router_post() {
        let router = Router::new()
            .post("/submit", Arc::new(|req| {
                Response::json(&serde_json::json!({
                    "received": req.body_str(),
                    "method": "POST",
                }))
            }));

        let client = TestClient::new(router);
        let resp = client.post("/submit", "test data", "text/plain");

        assert_eq!(resp.status, StatusCode::OK);
        let body = resp.body_str();
        assert!(body.contains("test data"));
    }

    #[test]
    fn test_router_param_capture() {
        let router = Router::new()
            .get("/users/:id", Arc::new(|req| {
                let id = req.param("id").unwrap_or("unknown");
                Response::text(&format!("User: {}", id))
            }));

        let client = TestClient::new(router);
        let resp = client.get("/users/42");

        assert_eq!(resp.status, StatusCode::OK);
        assert!(resp.body_str().contains("User: 42"));
    }

    #[test]
    fn test_router_multi_params() {
        let router = Router::new()
            .get("/posts/:year/:slug", Arc::new(|req| {
                let year = req.param("year").unwrap_or("");
                let slug = req.param("slug").unwrap_or("");
                Response::text(&format!("Post {}/{}", year, slug))
            }));

        let client = TestClient::new(router);
        let resp = client.get("/posts/2024/hello-world");

        assert_eq!(resp.status, StatusCode::OK);
        assert!(resp.body_str().contains("2024/hello-world"));
    }

    #[test]
    fn test_router_static_files() {
        use std::fs;
        let dir = "/tmp/viontin_test_static";
        fs::create_dir_all(dir).unwrap();
        fs::write(format!("{}/test.txt", dir), "static content").unwrap();

        let router = Router::new().static_files("/static", dir);
        let client = TestClient::new(router);

        let resp = client.get("/static/test.txt");
        assert_eq!(resp.status, StatusCode::OK);
        assert!(resp.body_str().contains("static content"));

        let resp = client.get("/static/nonexistent.txt");
        assert_eq!(resp.status, StatusCode::NOT_FOUND);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_router_method_not_allowed() {
        let router = Router::new()
            .get("/only-get", Arc::new(|_| Response::html("OK")));

        let client = TestClient::new(router);
        let resp = client.post("/only-get", "", "");

        assert_eq!(resp.status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_router_response_json() {
        let router = Router::new()
            .get("/data", Arc::new(|_| {
                Response::json(&serde_json::json!({"name": "Alice", "age": 30}))
            }));

        let client = TestClient::new(router);
        let resp = client.get("/data");

        assert_eq!(resp.status, StatusCode::OK);
        let body = resp.body_str();
        assert!(body.contains("Alice"));
        assert!(body.contains("30"));
    }

    // ── Named route URL generation ──

    #[test]
    fn test_named_route_url() {
        route::get("/users", Arc::new(|_| Response::ok())).name("users.index").register();
        route::get("/users/:id", Arc::new(|_| Response::ok())).name("users.show").register();
        route::get("/posts/:year/:slug", Arc::new(|_| Response::ok())).name("posts.show").register();
        viontin_framework::route::build_router();

        assert_eq!(route::url("users.index"), Some("/users".to_string()));
        assert_eq!(route::url("users.show"), Some("/users/:id".to_string()));
        assert_eq!(route::url("nonexistent"), None);
    }

    #[test]
    fn test_named_route_url_with_params() {
        route::get("/users/:id", Arc::new(|_| Response::ok())).name("users.show").register();
        route::get("/posts/:year/:slug", Arc::new(|_| Response::ok())).name("posts.show").register();
        viontin_framework::route::build_router();

        assert_eq!(
            route::url_with("users.show", &[("id", "42")]),
            Some("/users/42".to_string())
        );
        assert_eq!(
            route::url_with("posts.show", &[("year", "2024"), ("slug", "hello-world")]),
            Some("/posts/2024/hello-world".to_string())
        );
    }

    #[test]
    fn test_named_route_has() {
        route::get("/", Arc::new(|_| Response::ok())).name("home").register();
        viontin_framework::route::build_router();

        assert!(route::has("home"));
        assert!(!route::has("nope"));
    }

    #[test]
    fn test_named_route_all() {
        route::get("/a", Arc::new(|_| Response::ok())).name("alpha").register();
        route::get("/b", Arc::new(|_| Response::ok())).name("beta").register();
        viontin_framework::route::build_router();

        let all = route::all();
        assert!(all.contains(&("alpha".to_string(), "/a".to_string())));
        assert!(all.contains(&("beta".to_string(), "/b".to_string())));
    }

    #[test]
    fn test_named_route_current_name() {
        use std::sync::Mutex;
        let captured = Arc::new(Mutex::new(None::<String>));
        let captured2 = captured.clone();
        route::get("/dashboard", Arc::new(move |req| {
            let name: Option<viontin_framework::server::RouteName> = req.extension();
            *captured2.lock().unwrap() = name.map(|n| n.0);
            Response::ok()
        })).name("dashboard").register();
        viontin_framework::route::build_router();
        let router = viontin_framework::route::take_router().unwrap();

        let req = viontin_framework::http::Request::new(
            viontin_framework::http::Method::Get,
            viontin_framework::http::Uri::parse("/dashboard"),
            viontin_framework::http::Headers::new(),
            Vec::new(),
        );
        let _resp = router.handle(req);
        assert_eq!(captured.lock().unwrap().as_deref(), Some("dashboard"));
    }

    #[test]
    fn test_named_route_group_auto_naming() {
        route::group()
            .prefix("/admin")
            .name("admin")
            .routes(|r| {
                r.get("/users", Arc::new(|_| Response::ok()));
                r.get("/users/:id", Arc::new(|_| Response::ok()));
            });
        viontin_framework::route::build_router();

        assert_eq!(route::url("admin.users"), Some("/admin/users".to_string()));
        assert_eq!(route::url("admin.users.id"), Some("/admin/users/:id".to_string()));
        assert_eq!(
            route::url_with("admin.users.id", &[("id", "7")]),
            Some("/admin/users/7".to_string())
        );
    }
}

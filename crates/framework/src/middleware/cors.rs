use crate::http::{Request, Response, StatusCode};
use crate::middleware::{Middleware, Next};

#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u64>,
}

impl CorsMiddleware {
    pub fn permissive() -> Self {
        CorsMiddleware {
            allowed_origins: vec!["*".into()],
            allowed_methods: vec!["GET".into(), "POST".into(), "PUT".into(),
                "PATCH".into(), "DELETE".into(), "OPTIONS".into(), "HEAD".into()],
            allowed_headers: vec!["*".into()],
            expose_headers: Vec::new(),
            allow_credentials: false,
            max_age: None,
        }
    }

    pub fn origin(origin: &str) -> Self {
        CorsMiddleware::permissive().with_allowed_origins(&[origin])
    }

    pub fn with_allowed_origins(mut self, origins: &[&str]) -> Self {
        self.allowed_origins = origins.iter().map(|s| s.to_string()).collect(); self
    }

    pub fn with_allowed_methods(mut self, methods: &[&str]) -> Self {
        self.allowed_methods = methods.iter().map(|s| s.to_string()).collect(); self
    }

    pub fn with_allowed_headers(mut self, headers: &[&str]) -> Self {
        self.allowed_headers = headers.iter().map(|s| s.to_string()).collect(); self
    }

    pub fn with_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow; self
    }

    fn apply_headers(&self, res: &mut Response) {
        res.headers.set("Access-Control-Allow-Origin", self.allowed_origins.join(", "));
        res.headers.set("Access-Control-Allow-Methods", self.allowed_methods.join(", "));
        res.headers.set("Access-Control-Allow-Headers", self.allowed_headers.join(", "));
        if !self.expose_headers.is_empty() {
            res.headers.set("Access-Control-Expose-Headers", self.expose_headers.join(", "));
        }
        if self.allow_credentials {
            res.headers.set("Access-Control-Allow-Credentials", "true");
        }
        if let Some(age) = self.max_age {
            res.headers.set("Access-Control-Max-Age", age.to_string());
        }
    }
}

impl Default for CorsMiddleware { fn default() -> Self { Self::permissive() } }

impl Middleware for CorsMiddleware {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        if req.method.as_str() == "OPTIONS" {
            let mut res = Response::new(StatusCode(204));
            self.apply_headers(&mut res);
            return res;
        }
        let mut res = next(req);
        self.apply_headers(&mut res);
        res
    }
}

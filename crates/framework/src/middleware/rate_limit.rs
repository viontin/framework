use crate::http::{Request, Response, StatusCode};
use crate::middleware::{Middleware, Next};

#[derive(Debug)]
pub struct RateLimitMiddleware {
    key_prefix: String,
    max_attempts: u64,
    decay_seconds: u64,
}

impl RateLimitMiddleware {
    pub fn new(key_prefix: &str, max_attempts: u64, decay_seconds: u64) -> Self {
        RateLimitMiddleware { key_prefix: key_prefix.into(), max_attempts, decay_seconds }
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        let key = format!("{}:{}", self.key_prefix, req.uri.path);

        if crate::rate_limit::too_many_attempts(&key, self.max_attempts) {
            let retry_after = crate::rate_limit::available_in(&key);
            let mut res = Response::html("Too Many Requests")
                .with_header("Retry-After", retry_after.to_string());
            res.status = StatusCode(429);
            return res;
        }

        crate::rate_limit::hit(&key, self.decay_seconds);
        next(req)
    }
}

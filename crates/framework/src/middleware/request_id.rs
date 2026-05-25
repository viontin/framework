use std::time::{SystemTime, UNIX_EPOCH};
use crate::http::{Request, Response};
use crate::middleware::{Middleware, Next};

#[derive(Debug)]
pub struct RequestId;

impl Middleware for RequestId {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        let id = generate_id();
        req.headers.set("X-Request-Id", &id);
        let mut res = next(req);
        res.headers.set("X-Request-Id", &id);
        res
    }
}

fn generate_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    format!("{:016x}", nanos)
}

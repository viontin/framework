use crate::http::{Request, Response, StatusCode, Headers};
use crate::middleware::{Middleware, Next};

#[derive(Debug)]
pub struct PanicRecovery;

impl Middleware for PanicRecovery {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        match catch_unwind(AssertUnwindSafe(|| next(req))) {
            Ok(res) => res,
            Err(panic) => {
                let msg = panic.downcast_ref::<&str>()
                    .copied().unwrap_or_else(|| panic.downcast_ref::<String>()
                    .map(|s| s.as_str()).unwrap_or("Unknown panic"));
                eprintln!("  [panic] {} {} — {}", req.method, req.uri.path, msg);
                let mut h = Headers::new();
                h.set("content-type", "text/html");
                Response { status: StatusCode::SERVER_ERROR, headers: h, body: b"Internal Server Error"[..].into() }
            }
        }
    }
}

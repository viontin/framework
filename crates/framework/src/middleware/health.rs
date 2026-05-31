use crate::http::{Request, Response};

pub fn healthz_handler(_req: Request) -> Response {
    Response::text("{\"status\":\"ok\",\"service\":\"viontin\"}")
        .header("content-type", "application/json")
}

pub fn readyz_handler(_req: Request) -> Response {
    Response::text("{\"status\":\"ready\",\"service\":\"viontin\"}")
        .header("content-type", "application/json")
}

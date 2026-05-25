use crate::http::{Request, Response, StatusCode, Headers};

pub fn static_files_handler(root: &'static str) -> impl Fn(Request) -> Response {
    move |req: Request| {
        let path = req.param("path").unwrap_or("index.html");
        let full_path = format!("{}/{}", root, path.trim_start_matches('/'));
        match std::fs::read(&full_path) {
            Ok(body) => {
                let ext = full_path.rsplit('.').next().unwrap_or("");
                let mut h = Headers::new();
                h.set("content-type", mime_for(ext));
                Response { status: StatusCode::OK, headers: h, body }
            }
            Err(_) => {
                let mut res = Response::html("Not Found");
                res.status = StatusCode::NOT_FOUND;
                res
            }
        }
    }
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

//! Error page rendering — catches 4xx/5xx responses and renders styled HTML.
//!
//! Returns a styled error page for common HTTP errors. Falls back
//! to plain text if the request is not from a browser (checks Accept header).

use crate::http::{Request, Response, StatusCode};
use crate::middleware::{Middleware, Next};

const ERROR_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    <title>{code} {title}</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: system-ui, -apple-system, sans-serif; background: #0d1117; color: #c9d1d9;
               display: flex; justify-content: center; align-items: center; min-height: 100vh; }
        .container { text-align: center; padding: 2rem; }
        .code { font-size: 6rem; font-weight: 900; color: #30363d; line-height: 1; }
        .title { font-size: 1.5rem; color: #8b949e; margin: 1rem 0; }
        .message { color: #6e7681; max-width: 400px; margin: 0 auto; }
        .footer { margin-top: 2rem; font-size: 0.75rem; color: #30363d; }
    </style>
</head>
<body>
    <div class="container">
        <div class="code">{code}</div>
        <div class="title">{title}</div>
        <div class="message">{message}</div>
        <div class="footer">Viontin</div>
    </div>
</body>
</html>"#;

#[derive(Debug)]
pub struct ErrorPageRenderer {
    pub show_details: bool,
}

impl ErrorPageRenderer {
    pub fn new() -> Self {
        ErrorPageRenderer { show_details: false }
    }

    pub fn with_details(mut self) -> Self {
        self.show_details = true;
        self
    }

    fn render(&self, status: StatusCode, msg: &str) -> String {
        ERROR_PAGE
            .replace("{code}", &status.0.to_string())
            .replace("{title}", status.as_str())
            .replace("{code}", &status.0.to_string())
            .replace("{title}", status.as_str())
            .replace("{message}", if self.show_details { msg } else { status_brief(status) })
    }
}

impl Default for ErrorPageRenderer {
    fn default() -> Self { Self::new() }
}

impl Middleware for ErrorPageRenderer {
    fn handle(&self, req: &mut Request, next: Next) -> Response {
        let resp = next(req);
        if resp.status.is_success() {
            return resp;
        }

        let accepts_html = req.header("accept")
            .map(|a| a.contains("text/html"))
            .unwrap_or(false);

        if !accepts_html {
            return resp;
        }

        let msg = std::str::from_utf8(&resp.body).unwrap_or("").to_string();
        let html = self.render(resp.status, &msg);
        Response::html(&html).status(resp.status)
    }
}

fn status_brief(status: StatusCode) -> &'static str {
    match status.0 {
        400 => "The request could not be understood.",
        401 => "You need to authenticate to access this page.",
        403 => "You don't have permission to view this page.",
        404 => "The page you're looking for doesn't exist.",
        405 => "This method is not allowed for this URL.",
        429 => "Too many requests. Please slow down.",
        500 => "Something went wrong on our end. Please try again.",
        502 => "The upstream server returned an invalid response.",
        503 => "The service is temporarily unavailable.",
        _ => "An error occurred.",
    }
}

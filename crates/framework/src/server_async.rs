//! Async HTTP server — requires the `async` feature (tokio).
//!
//! ```rust
//! use viontin_framework::server::Router;
//! use viontin_framework::server_async::AsyncServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::new().get("/", Arc::new(|_| Response::html("Hello")));
//!     let server = AsyncServer::new(router);
//!     server.run("127.0.0.1:3000").await.unwrap();
//! }
//! ```

#![cfg(feature = "async")]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use crate::http::{Method, Request, Response, Headers, Uri};
use crate::server::Router;

type IoResult<T> = Result<T, String>;

pub struct AsyncServer {
    router: Arc<Router>,
}

impl AsyncServer {
    pub fn new(router: Router) -> Self {
        AsyncServer { router: Arc::new(router) }
    }

    pub async fn run(&self, addr: &str) -> IoResult<()> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| format!("Bind failed: {}", e))?;
        println!("  Async server on http://{}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let router = self.router.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(stream, &router).await {
                            eprintln!("  [async-server] {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("  [async-server] {}", e),
            }
        }
    }
}

async fn handle_conn(stream: TcpStream, router: &Router) -> IoResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).await
        .map_err(|e| e.to_string())?;

    let request_line = request_line.trim();
    if request_line.is_empty() { return Ok(()); }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let uri = Uri::parse(parts.get(1).unwrap_or(&""))?;

    let mut headers = Headers::new();
    let mut raw_headers = Vec::new();
    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line).await
            .map_err(|e| e.to_string())?;
        let trimmed = line.trim();
        if trimmed.is_empty() { break; }
        raw_headers.push(trimmed.to_string());
        if let Some(eq) = trimmed.find(':') {
            headers.set(trimmed[..eq].trim(), trimmed[eq+1..].trim());
        }
    }

    let mut body = Vec::new();
    if let Some(len) = headers.content_length() {
        if len > 0 {
            let mut buf = vec![0u8; len as usize];
            buf_reader.read_exact(&mut buf).await
                .map_err(|e| e.to_string())?;
            body = buf;
        }
    }

    let request = Request::new(Method::parse(parts[0]), uri, headers, body);
    let response = router.handle(request);
    let raw = response.to_raw();
    writer.write_all(&raw).await
        .map_err(|e| e.to_string())?;
    writer.flush().await
        .map_err(|e| e.to_string())?;

    Ok(())
}

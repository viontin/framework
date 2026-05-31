use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use crate::http::{Method, Request, Headers, Uri};
use crate::server::{Router, IoResult};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn is_shutdown_requested() -> bool { SHUTDOWN.load(Ordering::Relaxed) }
pub fn request_shutdown() { SHUTDOWN.store(true, Ordering::Relaxed); }

pub struct Server { router: Arc<Router>, }

impl Server {
    pub fn new(router: Router) -> Self { Server { router: Arc::new(router) } }

    pub fn run(&self, addr: &str) -> IoResult<()> {
        #[cfg(feature = "shutdown")]
        if let Err(e) = ctrlc::set_handler(move || { eprintln!("\n  [server] Shutdown requested..."); request_shutdown(); }) {
            eprintln!("  [server] Warning: could not set signal handler: {}", e);
        }

        let listener = TcpListener::bind(addr).map_err(|e| format!("Bind failed: {}", e))?;
        listener.set_nonblocking(true).ok();
        println!("  Server on http://{}", addr);

        loop {
            if is_shutdown_requested() { println!("  [server] Stopping accept loop..."); break; }

            match listener.accept() {
                Ok((s, _)) => {
                    let r = self.router.clone();
                    crate::middleware::set_connection_timeout(&s, 30);
                    thread::spawn(move || {
                        if let Err(e) = handle_conn(s, &r) { eprintln!("  [server] {}", e); }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => eprintln!("  [server] {}", e),
            }
        }
        println!("  [server] Graceful shutdown complete.");
        Ok(())
    }
}

fn handle_conn(mut stream: std::net::TcpStream, router: &Router) -> IoResult<()> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new(); io_err(reader.read_line(&mut request_line))?;
    let request_line = request_line.trim();
    if request_line.is_empty() { return Ok(()); }
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let uri = Uri::parse(parts.get(1).unwrap_or(&""));
    let mut headers = Headers::new();
    loop {
        let mut line = String::new(); io_err(reader.read_line(&mut line))?;
        let line = line.trim();
        if line.is_empty() { break; }
        if let Some(eq) = line.find(':') { headers.set(line[..eq].trim(), line[eq+1..].trim()); }
    }
    let mut body = Vec::new();
    if let Some(len) = headers.content_length() && len > 0 {
        let mut buf = vec![0u8; len as usize]; io_err(reader.read_exact(&mut buf))?; body = buf;
    }
    let request = Request::new(Method::parse(parts[0]), uri, headers, body);
    let response = router.handle(request);
    io_err(stream.write_all(&response.to_raw()))?;
    io_err(stream.flush())?;
    Ok(())
}

fn io_err<T>(r: std::io::Result<T>) -> IoResult<T> { r.map_err(|e| e.to_string()) }

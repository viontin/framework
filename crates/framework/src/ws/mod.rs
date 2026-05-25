use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::http::{Uri, Headers, Request, Method};
use crate::server::Router;

pub mod frame;
pub mod sha1;
pub mod base64;

use frame::*;
use sha1::sha1;

pub type IoResult<T> = Result<T, String>;
type HandlerMap = Arc<Mutex<HashMap<String, (WebSocketConfig, Arc<dyn WebSocketHandler>)>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode { Text, Binary, Close, Ping, Pong, Continue, }
impl Opcode {
    pub fn from_u8(b: u8) -> Option<Self> { match b & 0x0f {
        0 => Some(Opcode::Continue), 1 => Some(Opcode::Text), 2 => Some(Opcode::Binary),
        8 => Some(Opcode::Close), 9 => Some(Opcode::Ping), 10 => Some(Opcode::Pong), _ => None, } }
    pub fn as_u8(&self) -> u8 { match self {
        Opcode::Continue => 0, Opcode::Text => 1, Opcode::Binary => 2,
        Opcode::Close => 8, Opcode::Ping => 9, Opcode::Pong => 10, } }
}

#[derive(Debug, Clone)]
pub enum Message { Text(String), Binary(Vec<u8>), Ping(Vec<u8>), Pong(Vec<u8>), Close(Option<u16>, Option<String>), }

#[derive(Debug, Clone)]
pub struct WebSocketConfig { pub max_message_size: usize, pub max_frame_size: usize, }
impl Default for WebSocketConfig {
    fn default() -> Self { WebSocketConfig { max_message_size: 1024 * 1024, max_frame_size: 65536 } }
}

pub trait WebSocketHandler: Send + Sync + 'static {
    fn on_open(&self, _key: &str) {}
    fn on_message(&self, _key: &str, _msg: Message) -> Vec<Message> { Vec::new() }
    fn on_close(&self, _key: &str, _code: Option<u16>, _reason: Option<&str>) {}
    fn on_error(&self, _key: &str, _err: &str) {}
}

pub fn ws_router() -> WsRouter {
    WsRouter { routes: Arc::new(Mutex::new(HashMap::new())) }
}

pub struct WsRouter { routes: HandlerMap, }

impl WsRouter {
    pub fn ws(self, path: &str, handler: impl WebSocketHandler) -> Self {
        if let Ok(mut r) = self.routes.lock() { r.insert(path.into(), (WebSocketConfig::default(), Arc::new(handler))); }
        self
    }
    pub fn ws_with_config(self, path: &str, config: WebSocketConfig, handler: impl WebSocketHandler) -> Self {
        if let Ok(mut r) = self.routes.lock() { r.insert(path.into(), (config, Arc::new(handler))); }
        self
    }
    pub fn attach(self, router: Router) -> WsServer {
        WsServer { router: Arc::new(router), routes: self.routes }
    }
}

pub struct WsServer { router: Arc<Router>, routes: HandlerMap, }

impl WsServer {
    pub fn run(&self, addr: &str) -> IoResult<()> {
        let listener = std::net::TcpListener::bind(addr).map_err(|e| format!("Bind failed: {}", e))?;
        println!("  Server on http://{} (WebSocket ready)", addr);
        let routes = self.routes.clone();
        let router = self.router.clone();
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let routes = routes.clone();
                    let router = router.clone();
                    crate::middleware::set_connection_timeout(&s, 30);
                    thread::spawn(move || {
                        if let Err(e) = handle_conn(s, &router, &routes) {
                            eprintln!("  [ws] {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("  [ws] {}", e),
            }
        }
        Ok(())
    }
}

pub fn handle_conn(stream: TcpStream, router: &Router, routes: &HandlerMap) -> IoResult<()> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| e.to_string())?;
    let request_line = request_line.trim();
    if request_line.is_empty() { return Ok(()); }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"");
    let path = parts.get(1).unwrap_or(&"");

    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        let line = line.trim().to_string();
        if line.is_empty() { break; }
        if let Some(eq) = line.find(':') {
            headers.insert(line[..eq].trim().to_lowercase(), line[eq+1..].trim().to_string());
        }
    }

    let upgrade = headers.get("upgrade").map(|s| s.as_str()).unwrap_or("");
    if *method != "GET" || upgrade.to_lowercase() != "websocket" {
        let uri = Uri::parse(path).unwrap_or_default();
        let mut h = Headers::new();
        for (k, v) in &headers { h.set(k, v); }
        let mut request = Request::new(Method::parse(method), uri, h, Vec::new());
        request.params = HashMap::new();
        let response = router.handle(request);
        let mut out = &stream;
        out.write_all(&response.to_raw()).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let key = headers.get("sec-websocket-key").ok_or_else(|| "Missing Sec-WebSocket-Key".to_string())?;
    let routes = routes.lock().unwrap();
    let (_config, handler) = routes.iter().find(|(p, _)| path.starts_with(p.as_str()))
        .map(|(_, v)| v).ok_or_else(|| format!("No handler for {}", path))?;
    let handler = handler.clone();
    drop(routes);

    let accept_key = ws_accept_key(key);
    let upgrade = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key);
    let mut out = &stream;
    out.write_all(upgrade.as_bytes()).map_err(|e| e.to_string())?;

    let conn_id = format!("ws_{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos());
    handler.on_open(&conn_id);
    ws_loop(stream, &handler, conn_id);
    Ok(())
}

fn ws_loop(mut stream: TcpStream, handler: &Arc<dyn WebSocketHandler>, conn_id: String) {
    let mut buf = Vec::new();
    let mut frame_buf = [0u8; 2];
    loop {
        match read_frame(&mut stream, &mut buf, &mut frame_buf) {
            Ok(Some(Message::Close(code, reason))) => {
                let close = encode_frame(Opcode::Close, &encode_close_payload(code, reason.as_deref()));
                let _ = stream.write_all(&close);
                handler.on_close(&conn_id, code, reason.as_deref());
                break;
            }
            Ok(Some(Message::Ping(payload))) => {
                let pong = encode_frame(Opcode::Pong, &payload);
                let _ = stream.write_all(&pong);
            }
            Ok(Some(msg)) => {
                for reply in handler.on_message(&conn_id, msg) {
                    let (opcode, data) = message_to_frame(&reply);
                    if let Some(frame) = data {
                        let _ = stream.write_all(&encode_frame(opcode, &frame));
                    }
                }
            }
            Ok(None) => break,
            Err(e) => { handler.on_error(&conn_id, &e); break; }
        }
    }
}

fn ws_accept_key(key: &str) -> String {
    const MAGIC: &str = "258EAFA5-E914-47DA-95CA-5AB5E16B09EC";
    let hash = sha1(format!("{}{}", key.trim(), MAGIC).as_bytes());
    base64::encode(&hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_known() {
        let cases = [
            ("test", "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"),
            ("abc", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        ];
        for (input, expected) in &cases {
            let hash = sha1(input.as_bytes());
            let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
            assert_eq!(&hex, expected);
        }
    }

    #[test]
    fn test_ws_accept_key() {
        let accept = ws_accept_key("dGhlIHNhbXBsZSBub25jZQ==");
        assert_eq!(accept, "e5nGk/O6nlESgXkZ9jvCMqJeV4Y=");
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64::encode(b"hello"), "aGVsbG8=");
    }

    #[test]
    fn test_encode_decode_text_frame() {
        let frame = encode_frame(Opcode::Text, b"hello");
        assert_eq!(frame[0] & 0x0f, Opcode::Text.as_u8());
        assert_eq!(&frame[2..], b"hello");
    }

    #[test]
    fn test_encode_close_frame() {
        let (op, data) = message_to_frame(&Message::Close(Some(1000), Some("normal".into())));
        let frame = encode_frame(op, &data.unwrap());
        assert_eq!(frame[0] & 0x0f, Opcode::Close.as_u8());
    }

    #[test]
    fn test_close_payload_roundtrip() {
        let payload = encode_close_payload(Some(1001), Some("going away"));
        let (code, reason) = decode_close_payload(&payload);
        assert_eq!(code, Some(1001));
        assert_eq!(reason.as_deref(), Some("going away"));
    }

    #[test]
    fn test_large_frame_length() {
        let payload = vec![0x42u8; 65536];
        let frame = encode_frame(Opcode::Binary, &payload);
        assert_eq!(frame[1], 127);
        let len = u64::from_be_bytes(frame[2..10].try_into().unwrap()) as usize;
        assert_eq!(len, 65536);
        assert_eq!(&frame[10..], &payload);
    }

    #[test]
    fn test_message_to_frame_all_variants() {
        let cases = vec![
            Message::Text("hi".into()),
            Message::Binary(vec![1, 2, 3]),
            Message::Ping(vec![0]),
            Message::Pong(vec![0]),
            Message::Close(None, None),
        ];
        for msg in &cases {
            let (op, data) = message_to_frame(msg);
            let frame = encode_frame(op, &data.unwrap());
            assert!(frame[0] & 0x80 != 0, "FIN bit must be set");
            assert_eq!(frame[0] & 0x0f, op.as_u8());
        }
    }
}

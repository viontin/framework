use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::http::{Uri, Headers, Request, Method};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode { Text, Binary, Close, Ping, Pong, Continue, }
impl Opcode {
    pub fn from_u8(b: u8) -> Option<Self> { match b & 0x0f { 0 => Some(Opcode::Continue), 1 => Some(Opcode::Text),
        2 => Some(Opcode::Binary), 8 => Some(Opcode::Close), 9 => Some(Opcode::Ping), 10 => Some(Opcode::Pong), _ => None, } }
    pub fn as_u8(&self) -> u8 { match self { Opcode::Continue => 0, Opcode::Text => 1, Opcode::Binary => 2,
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
use crate::server::Router;

type IoResult<T> = Result<T, String>;
type HandlerMap = Arc<Mutex<HashMap<String, (WebSocketConfig, Arc<dyn WebSocketHandler>)>>>;

pub fn ws_router() -> WsRouter {
    WsRouter { routes: Arc::new(Mutex::new(HashMap::new())) }
}

pub struct WsRouter {
    routes: HandlerMap,
}

impl WsRouter {
    pub fn ws(self, path: &str, handler: impl WebSocketHandler) -> Self {
        self.routes.lock().unwrap().insert(path.into(), (WebSocketConfig::default(), Arc::new(handler)));
        self
    }

    pub fn ws_with_config(self, path: &str, config: WebSocketConfig, handler: impl WebSocketHandler) -> Self {
        self.routes.lock().unwrap().insert(path.into(), (config, Arc::new(handler)));
        self
    }

    pub fn attach(self, router: Router) -> WsServer {
        WsServer { router: Arc::new(router), routes: self.routes }
    }
}

pub struct WsServer {
    router: Arc<Router>,
    routes: HandlerMap,
}

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

fn handle_conn(stream: TcpStream, router: &Router, routes: &HandlerMap) -> IoResult<()> {
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
        let uri = Uri::parse(path).unwrap_or(Uri { scheme: "http".into(), host: "localhost".into(), port: 80, path: (*path).into(), query: std::collections::HashMap::new(), fragment: None });
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
    let (config, handler) = routes.iter().find(|(p, _)| path.starts_with(p.as_str()))
        .map(|(_, v)| v)
        .ok_or_else(|| format!("No WebSocket handler for {}", path))?;
    let config = config.clone();
    let handler = handler.clone();
    drop(routes);

    let accept_key = ws_accept_key(key);
    let upgrade_response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\
         \r\n",
        accept_key
    );
    let mut out = &stream;
    out.write_all(upgrade_response.as_bytes()).map_err(|e| e.to_string())?;

    let conn_id = format!("ws_{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos());
    handler.on_open(&conn_id);
    ws_loop(stream, &handler, &config, &conn_id);
    Ok(())
}

fn ws_loop(mut stream: TcpStream, handler: &Arc<dyn WebSocketHandler>, config: &WebSocketConfig, conn_id: &str) {
    let mut buf = Vec::new();
    let mut frame_buf = [0u8; 2];
    loop {
        match read_frame(&mut stream, &mut buf, &mut frame_buf, config) {
            Ok(Some(Message::Close(code, reason))) => {
                let close_frame = encode_frame(Opcode::Close, &encode_close_payload(code, reason.as_deref()));
                let _ = stream.write_all(&close_frame);
                handler.on_close(conn_id, code, reason.as_deref());
                break;
            }
            Ok(Some(Message::Ping(payload))) => {
                let pong = encode_frame(Opcode::Pong, &payload);
                let _ = stream.write_all(&pong);
            }
            Ok(Some(msg)) => {
                let replies = handler.on_message(conn_id, msg);
                for reply in replies {
                    let (opcode, data) = message_to_frame(&reply);
                    if let Some(frame) = data {
                        let raw = encode_frame(opcode, &frame);
                        let _ = stream.write_all(&raw);
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                handler.on_error(conn_id, &e);
                break;
            }
        }
    }
}

fn read_frame(stream: &mut TcpStream, _buf: &mut Vec<u8>, frame_buf: &mut [u8; 2], config: &WebSocketConfig) -> IoResult<Option<Message>> {
    read_exact(stream, frame_buf)?;
    let _fin = (frame_buf[0] & 0x80) != 0;
    let opcode = Opcode::from_u8(frame_buf[0]).ok_or_else(|| format!("Unknown opcode: {}", frame_buf[0]))?;
    let masked = (frame_buf[1] & 0x80) != 0;
    let mut payload_len = (frame_buf[1] & 0x7f) as u64;

    if payload_len == 126 {
        let mut ext = [0u8; 2];
        read_exact(stream, &mut ext)?;
        payload_len = u16::from_be_bytes(ext) as u64;
    } else if payload_len == 127 {
        let mut ext = [0u8; 8];
        read_exact(stream, &mut ext)?;
        payload_len = u64::from_be_bytes(ext);
    }

    if payload_len as usize > config.max_frame_size {
        return Err(format!("Frame too large: {} > {}", payload_len, config.max_frame_size));
    }

    let mut mask_key = [0u8; 4];
    if masked {
        read_exact(stream, &mut mask_key)?;
    }

    let mut payload = vec![0u8; payload_len as usize];
    if payload_len > 0 {
        read_exact(stream, &mut payload)?;
    }

    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask_key[i % 4];
        }
    }

    match opcode {
        Opcode::Close => {
            let (code, reason) = decode_close_payload(&payload);
            Ok(Some(Message::Close(code, reason)))
        }
        Opcode::Ping => Ok(Some(Message::Ping(payload))),
        Opcode::Pong => Ok(Some(Message::Pong(payload))),
        Opcode::Text => {
            let s = String::from_utf8(payload).map_err(|e| format!("Invalid UTF-8: {}", e))?;
            Ok(Some(Message::Text(s)))
        }
        Opcode::Binary => Ok(Some(Message::Binary(payload))),
        Opcode::Continue => Ok(None),
    }
}

fn encode_frame(opcode: Opcode, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(0x80 | opcode.as_u8());

    let len = payload.len();
    if len < 126 {
        frame.push(len as u8);
    } else if len <= 0xFFFF {
        frame.push(126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);
    frame
}

fn message_to_frame(msg: &Message) -> (Opcode, Option<Vec<u8>>) {
    match msg {
        Message::Text(s) => (Opcode::Text, Some(s.as_bytes().to_vec())),
        Message::Binary(d) => (Opcode::Binary, Some(d.clone())),
        Message::Ping(d) => (Opcode::Ping, Some(d.clone())),
        Message::Pong(d) => (Opcode::Pong, Some(d.clone())),
        Message::Close(code, reason) => (Opcode::Close, Some(encode_close_payload(*code, reason.as_deref()))),
    }
}

fn encode_close_payload(code: Option<u16>, reason: Option<&str>) -> Vec<u8> {
    let mut payload = Vec::new();
    let c = code.unwrap_or(1000);
    payload.extend_from_slice(&c.to_be_bytes());
    if let Some(r) = reason {
        payload.extend_from_slice(r.as_bytes());
    }
    payload
}

fn decode_close_payload(payload: &[u8]) -> (Option<u16>, Option<String>) {
    if payload.len() >= 2 {
        let code = u16::from_be_bytes([payload[0], payload[1]]);
        let reason = if payload.len() > 2 {
            Some(String::from_utf8_lossy(&payload[2..]).to_string())
        } else {
            None
        };
        (Some(code), reason)
    } else {
        (None, None)
    }
}

fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> IoResult<()> {
    let mut off = 0;
    while off < buf.len() {
        match stream.read(&mut buf[off..]) {
            Ok(0) => return Err("Connection closed".into()),
            Ok(n) => off += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn ws_accept_key(key: &str) -> String {
    const MAGIC: &str = "258EAFA5-E914-47DA-95CA-5AB5E16B09EC";
    let combined = format!("{}{}", key.trim(), MAGIC);
    let hash = sha1(combined.as_bytes());
    base64_encode(&hash)
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0 = 0x67452301u32;
    let mut h1 = 0xEFCDAB89u32;
    let mut h2 = 0x98BADCFEu32;
    let mut h3 = 0x10325476u32;
    let mut h4 = 0xC3D2E1F0u32;

    let ml = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while ((msg.len() * 8) % 512) != 448 {
        msg.push(0);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for (i, c) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);

        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | (!b & d), 0x5A827999u32)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ED9EBA1u32)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32)
            } else {
                (b ^ c ^ d, 0xCA62C1D6u32)
            };

            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);

            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_text_frame() {
        let msg = Message::Text("hello".into());
        let (op, data) = message_to_frame(&msg);
        let frame = encode_frame(op, &data.unwrap());
        let expected_op = Opcode::Text;
        assert_eq!(op, expected_op);
        assert!(frame.len() >= 2);
        assert_eq!(frame[0] & 0x0f, Opcode::Text.as_u8());
        assert_eq!(&frame[2..], b"hello");
    }

    #[test]
    fn test_encode_decode_binary_frame() {
        let payload = vec![0x01, 0x02, 0x03, 0xFF];
        let frame = encode_frame(Opcode::Binary, &payload);
        assert_eq!(frame[0] & 0x0f, Opcode::Binary.as_u8());
        assert_eq!(&frame[2..], &payload);
    }

    #[test]
    fn test_encode_close_frame() {
        let msg = Message::Close(Some(1000), Some("normal".into()));
        let (op, data) = message_to_frame(&msg);
        let frame = encode_frame(op, &data.unwrap());
        assert_eq!(op, Opcode::Close);
        assert_eq!(frame[0] & 0x0f, Opcode::Close.as_u8());
        assert!(frame.len() >= 4);
    }

    #[test]
    fn test_close_payload_roundtrip() {
        let payload = encode_close_payload(Some(1001), Some("going away"));
        let (code, reason) = decode_close_payload(&payload);
        assert_eq!(code, Some(1001));
        assert_eq!(reason.as_deref(), Some("going away"));
    }

    #[test]
    fn test_sha1_known() {
        let cases = [("test", "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"), ("abc", "a9993e364706816aba3e25717850c26c9cd0d89d")];
        for (input, expected) in &cases {
            let hash = sha1(input.as_bytes());
            let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
            assert_eq!(&hash_hex, expected, "SHA1 of {}", input);
        }
    }

    #[test]
    fn test_ws_accept_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = ws_accept_key(key);
        let expected = "e5nGk/O6nlESgXkZ9jvCMqJeV4Y=";
        assert_eq!(accept, expected);
    }

    #[test]
    fn test_base64_encode() {
        let result = base64_encode(b"hello");
        assert_eq!(result, "aGVsbG8=");
    }

    #[test]
    fn test_large_frame_length() {
        let payload = vec![0x42u8; 65536];
        let frame = encode_frame(Opcode::Binary, &payload);
        assert_eq!(frame[0] & 0x0f, Opcode::Binary.as_u8());
        assert_eq!(frame[1], 127);
        let len = u64::from_be_bytes([frame[2], frame[3], frame[4], frame[5], frame[6], frame[7], frame[8], frame[9]]) as usize;
        assert_eq!(len, 65536);
        assert_eq!(&frame[10..], &payload);
    }

    #[test]
    fn test_message_to_frame_all_variants() {
        let cases: Vec<Message> = vec![
            Message::Text("hi".into()),
            Message::Binary(vec![1, 2, 3]),
            Message::Ping(vec![0]),
            Message::Pong(vec![0]),
            Message::Close(None, None),
        ];
        for msg in &cases {
            let (op, data) = message_to_frame(msg);
            let frame = encode_frame(op, &data.unwrap());
            assert_eq!(frame[0] & 0x80, 0x80, "FIN bit must be set");
            assert_eq!(frame[0] & 0x0f, op.as_u8());
        }
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); } else { result.push('='); }
        if chunk.len() > 2 { result.push(CHARS[(triple & 0x3F) as usize] as char); } else { result.push('='); }
    }
    result
}

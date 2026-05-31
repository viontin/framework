//! Redis cache driver — communicates via TCP RESP2 protocol.
//!
//! No external dependencies — uses `std::net::TcpStream`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use crate::cache::CacheDriver;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 6379;

pub struct RedisCache {
    host: String,
    port: u16,
    prefix: String,
    timeout: Duration,
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl RedisCache {
    pub fn new(host: &str, port: u16) -> Self {
        RedisCache { host: host.to_string(), port, prefix: String::new(), timeout: Duration::from_secs(3) }
    }

    pub fn localhost() -> Self {
        Self::new(DEFAULT_HOST, DEFAULT_PORT)
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = format!("{}:", prefix);
        self
    }

    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    fn key(&self, k: &str) -> String {
        format!("{}{}", self.prefix, k)
    }

    fn connect(&self) -> Result<TcpStream, String> {
        let addr: std::net::SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?;
        let stream = TcpStream::connect_timeout(&addr, self.timeout)
            .map_err(|e: std::io::Error| e.to_string())?;
        stream.set_read_timeout(Some(self.timeout)).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(self.timeout)).map_err(|e| e.to_string())?;
        Ok(stream)
    }

    fn cmd(&self, args: &[&[u8]]) -> Result<Vec<u8>, String> {
        let mut stream = self.connect()?;
        let mut req = Vec::new();
        write!(&mut req, "*{}\r\n", args.len()).map_err(|e: std::io::Error| e.to_string())?;
        for arg in args {
            write!(&mut req, "${}\r\n", arg.len()).map_err(|e: std::io::Error| e.to_string())?;
            req.extend_from_slice(arg);
            req.extend_from_slice(b"\r\n");
        }
        stream.write_all(&req).map_err(|e: std::io::Error| e.to_string())?;
        stream.flush().map_err(|e: std::io::Error| e.to_string())?;

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e: std::io::Error| e.to_string())?;
        let line = line.trim().to_string();
        if line.is_empty() { return Ok(Vec::new()); }

        match line.as_bytes()[0] {
            b'+' => Ok(line[1..].as_bytes().to_vec()),
            b'-' => Err(format!("Redis: {}", &line[1..])),
            b':' => Ok(line[1..].as_bytes().to_vec()),
            b'$' => {
                let len: i64 = line[1..].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                if len < 0 { return Ok(Vec::new()); }
                let mut buf = vec![0u8; len as usize + 2];
                reader.read_exact(&mut buf).map_err(|e: std::io::Error| e.to_string())?;
                Ok(buf[..len as usize].to_vec())
            }
            b'*' => {
                let count: usize = line[1..].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                let mut result = Vec::new();
                for _ in 0..count {
                    let mut l = String::new();
                    reader.read_line(&mut l).map_err(|e: std::io::Error| e.to_string())?;
                    let l = l.trim();
                    if l.as_bytes()[0] == b'$' {
                        let len: i64 = l[1..].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                        if len >= 0 {
                            let mut buf = vec![0u8; len as usize + 2];
                            reader.read_exact(&mut buf).map_err(|e: std::io::Error| e.to_string())?;
                            result.extend_from_slice(&buf[..len as usize]);
                        }
                    }
                }
                Ok(result)
            }
            _ => Err("Unexpected Redis response".into()),
        }
    }
}

impl CacheDriver for RedisCache {
    fn name(&self) -> &str { "redis" }
    fn get(&self, key: &str) -> Option<String> {
        match self.cmd(&[b"GET", self.key(key).as_bytes()]) {
            Ok(v) if !v.is_empty() => String::from_utf8(v).ok(),
            _ => None,
        }
    }
    fn set(&self, key: &str, value: &str, ttl: Option<u64>) {
        let k = self.key(key);
        if let Some(secs) = ttl {
            let _ = self.cmd(&[b"SETEX", k.as_bytes(), &secs.to_string().into_bytes(), value.as_bytes()]);
        } else {
            let _ = self.cmd(&[b"SET", k.as_bytes(), value.as_bytes()]);
        }
    }
    fn has(&self, key: &str) -> bool {
        match self.cmd(&[b"EXISTS", self.key(key).as_bytes()]) {
            Ok(v) => v == b"1",
            _ => false,
        }
    }
    fn delete(&self, key: &str) {
        let _ = self.cmd(&[b"DEL", self.key(key).as_bytes()]);
    }
    fn clear(&self) {
        let _ = self.cmd(&[b"FLUSHDB"]);
    }
    fn increment(&self, key: &str, amount: i64) -> i64 {
        match self.cmd(&[b"INCRBY", self.key(key).as_bytes(), &amount.to_string().into_bytes()]) {
            Ok(v) => String::from_utf8(v).ok().and_then(|s| s.parse().ok()).unwrap_or(0),
            _ => 0,
        }
    }
}

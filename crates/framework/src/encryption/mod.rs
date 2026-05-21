//! Encryption implementations — SimpleEncrypter (XOR-based, dev only).
//!
//! For production, install an AES encryption gem.

use std::time::{SystemTime, UNIX_EPOCH};
use crate::support::Encrypter;

/// Simple XOR-based encrypter — **not cryptographically secure.**
///
/// For development/testing only. Production should use AES via a gem.
#[derive(Debug)]
pub struct SimpleEncrypter {
    key: String,
}

impl SimpleEncrypter {
    pub fn new(key: impl Into<String>) -> Self {
        SimpleEncrypter { key: key.into() }
    }
}

impl Encrypter for SimpleEncrypter {
    fn name(&self) -> &str { "simple" }

    fn encrypt(&self, plaintext: &str) -> String {
        let key = self.key.as_bytes();
        let bytes: Vec<u8> = plaintext.bytes()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        let salt = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().subsec_nanos() as u8;
        // Prepend salt byte
        let mut result = vec![salt];
        result.extend(bytes.iter().map(|b| b.wrapping_add(salt)));
        hex::encode(&result)
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        let decoded = hex::decode(ciphertext).map_err(|e| format!("Invalid hex: {}", e))?;
        if decoded.is_empty() { return Err("Empty ciphertext".into()); }
        let salt = decoded[0];
        let bytes: Vec<u8> = decoded[1..].iter()
            .map(|b| b.wrapping_sub(salt))
            .collect();
        let key = self.key.as_bytes();
        let plain: Vec<u8> = bytes.iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        String::from_utf8(plain).map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    fn keygen() -> String {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_nanos();
        format!("key_{:x}_{:x}", nanos, fast_rand())
    }
}

fn fast_rand() -> u64 {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    let mut state = seed as u64;
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

// Simple hex encoding without external crate
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        let chars: Vec<char> = "0123456789abcdef".chars().collect();
        let mut result = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            result.push(chars[(b >> 4) as usize]);
            result.push(chars[(b & 0x0f) as usize]);
        }
        result
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let s = s.trim();
        if s.len() % 2 != 0 { return Err("Odd hex length".into()); }
        let mut bytes = Vec::with_capacity(s.len() / 2);
        for chunk in s.as_bytes().chunks(2) {
            let hi = from_hex(chunk[0])?;
            let lo = from_hex(chunk[1])?;
            bytes.push((hi << 4) | lo);
        }
        Ok(bytes)
    }

    fn from_hex(c: u8) -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(format!("Invalid hex char: {}", c as char)),
        }
    }
}

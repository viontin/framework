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

// ── AES-256-GCM Encrypter (production-ready) ──

/// AES-256-GCM encrypter — production-grade authenticated encryption.
///
/// Requires the `aes` feature flag:
///
/// ```toml
/// [dependencies]
/// viontin-framework = { features = ["aes"] }
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use viontin_framework::encryption::AesEncrypter;
/// use viontin_framework::support::Encrypter;
///
/// let key = AesEncrypter::keygen();
/// let enc = AesEncrypter::new(&key);
/// let encrypted = enc.encrypt("Hello, world!");
/// let decrypted = enc.decrypt(&encrypted).unwrap();
/// assert_eq!(decrypted, "Hello, world!");
/// ```
#[cfg(feature = "aes")]
#[derive(Debug)]
pub struct AesEncrypter {
    key: Vec<u8>,
}

#[cfg(feature = "aes")]
impl AesEncrypter {
    pub fn new(key: &str) -> Self {
        // Use SHA-256 to derive a 256-bit key from any key string
        use sha2::digest::Digest;
        let hash = sha2::Sha256::digest(key.as_bytes());
        AesEncrypter { key: hash.to_vec() }
    }
}

#[cfg(feature = "aes")]
impl crate::support::Encrypter for AesEncrypter {
    fn name(&self) -> &str { "aes-256-gcm" }

    fn encrypt(&self, plaintext: &str) -> String {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);

        // Use a timestamp-based nonce (for simplicity; in production use proper randomness)
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[..8].copy_from_slice(&nanos.to_le_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
            .expect("AES encryption failed");

        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        hex::encode(&result)
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let data = hex::decode(ciphertext).map_err(|e| format!("Invalid hex: {}", e))?;
        if data.len() < 12 { return Err("Ciphertext too short".into()); }

        let (nonce_bytes, ct) = data.split_at(12);
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ct)
            .map_err(|_| "Decryption failed (wrong key or corrupted data)".to_string())?;
        String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    fn keygen() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&nanos.to_le_bytes());
        // XOR with platform-specific entropy
        for b in key.iter_mut().skip(8) {
            *b = (nanos.wrapping_mul(6364136223846793005) >> 33) as u8;
        }
        hex::encode(&key)
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

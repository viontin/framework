//! Encryption implementations — SimpleEncrypter (dev), AesEncrypter (production).

pub mod xor;
#[cfg(feature = "aes")]
pub mod aes;

pub use xor::SimpleEncrypter;
#[cfg(feature = "aes")]
pub use aes::AesEncrypter;

pub(crate) mod hex {
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
        if !s.len().is_multiple_of(2) { return Err("Odd hex length".into()); }
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

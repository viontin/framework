use std::time::{SystemTime, UNIX_EPOCH};
use crate::support::Encrypter;

#[derive(Debug)]
pub struct SimpleEncrypter { key: String }

impl SimpleEncrypter {
    pub fn new(key: impl Into<String>) -> Self { SimpleEncrypter { key: key.into() } }
}

impl Encrypter for SimpleEncrypter {
    fn name(&self) -> &str { "simple" }

    fn encrypt(&self, plaintext: &str) -> String {
        let key = self.key.as_bytes();
        let bytes: Vec<u8> = plaintext.bytes().enumerate()
            .map(|(i, b)| b ^ key[i % key.len()]).collect();
        let salt = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().subsec_nanos() as u8;
        let mut result = vec![salt];
        result.extend(bytes.iter().map(|b| b.wrapping_add(salt)));
        super::hex::encode(&result)
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        let decoded = super::hex::decode(ciphertext)?;
        if decoded.is_empty() { return Err("Empty ciphertext".into()); }
        let salt = decoded[0];
        let bytes: Vec<u8> = decoded[1..].iter().map(|b| b.wrapping_sub(salt)).collect();
        let key = self.key.as_bytes();
        let plain: Vec<u8> = bytes.iter().enumerate()
            .map(|(i, b)| b ^ key[i % key.len()]).collect();
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
    state ^= state >> 12; state ^= state << 25; state ^= state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

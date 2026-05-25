use crate::support::Encrypter;

#[derive(Debug)]
pub struct AesEncrypter { key: Vec<u8> }

impl AesEncrypter {
    pub fn new(key: &str) -> Self {
        use sha2::digest::Digest;
        let hash = sha2::Sha256::digest(key.as_bytes());
        AesEncrypter { key: hash.to_vec() }
    }
}

impl Encrypter for AesEncrypter {
    fn name(&self) -> &str { "aes-256-gcm" }

    fn encrypt(&self, plaintext: &str) -> String {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[..8].copy_from_slice(&nanos.to_le_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
            .expect("AES encryption failed");

        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        super::hex::encode(&result)
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let data = super::hex::decode(ciphertext)?;
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
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&nanos.to_le_bytes());
        for b in key.iter_mut().skip(8) {
            *b = (nanos.wrapping_mul(6364136223846793005) >> 33) as u8;
        }
        super::hex::encode(&key)
    }
}

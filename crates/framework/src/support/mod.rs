pub mod path;

use std::fmt;

pub trait Hasher: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn hash(&self, value: &str) -> String;
    fn verify(&self, value: &str, hash: &str) -> bool;
    fn needs_rehash(&self, _hash: &str) -> bool { false }
}

pub trait Encrypter: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn encrypt(&self, plaintext: &str) -> String;
    fn decrypt(&self, ciphertext: &str) -> Result<String, String>;
    fn keygen() -> String;
}

pub mod hash;
pub mod str;
pub mod url;

pub use hash::{SimpleHasher, hex_digest, quick_hash, random_token};
pub use str::{truncate, kebab_case, snake_case, slug, camel_case, pascal_case, random, pluralize};
pub use url::{url_decode, url_encode, parse_query, build_query, is_valid_url};

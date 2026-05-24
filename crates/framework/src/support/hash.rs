use std::time::{SystemTime, UNIX_EPOCH};
use crate::support::Hasher;

#[derive(Debug)]
pub struct SimpleHasher;

impl Hasher for SimpleHasher {
    fn name(&self) -> &str { "simple" }
    fn hash(&self, value: &str) -> String {
        let salt = format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos().wrapping_mul(2654435761));
        format!("{}:{}", salt, simple_hash(&format!("{}{}", salt, value)))
    }
    fn verify(&self, value: &str, stored: &str) -> bool {
        stored.find(':').is_some_and(|eq| { let (salt, expected) = stored.split_at(eq); simple_hash(&format!("{}{}", salt, value)) == expected[1..] })
    }
}

fn simple_hash(input: &str) -> String {
    use std::hash::{Hash, Hasher as _};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut h);
    format!("{:x}", h.finish())
}

pub fn hex_digest(input: &str) -> String { simple_hash(input) }
pub fn quick_hash<T: std::hash::Hash>(value: &T) -> u64 {
    use std::hash::Hasher as _;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut h); h.finish()
}
pub fn random_token(len: usize) -> String {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let mut state = seed as u64;
    (0..len).map(|_| { state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (b'a' + ((state >> 33) as u8 % 26)) as char }).collect()
}

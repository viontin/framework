use std::time::{SystemTime, UNIX_EPOCH};
use crate::cache::CacheDriver;
use crate::rate_limit::RateLimiterDriver;

#[derive(Debug)]
pub struct TokenBucketLimiter {
    cache: Box<dyn CacheDriver>,
    prefix: String,
}

impl TokenBucketLimiter {
    pub fn new(cache: impl CacheDriver + 'static) -> Self {
        TokenBucketLimiter { cache: Box::new(cache), prefix: "rate:".into() }
    }

    fn key_for(&self, key: &str) -> String { format!("{}{}", self.prefix, key) }
    fn now(&self) -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
}

impl RateLimiterDriver for TokenBucketLimiter {
    fn name(&self) -> &str { "token_bucket" }

    fn attempt(&self, key: &str, max_attempts: u64, decay_seconds: u64) -> bool {
        let cache_key = self.key_for(key);
        let reset_key = format!("{}:reset", cache_key);
        let hits = self.cache.get(&cache_key).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);

        if hits >= max_attempts && let Some(reset) = self.cache.get(&reset_key).and_then(|v| v.parse::<u64>().ok()) {
            if self.now() >= reset { self.cache.delete(&cache_key); self.cache.delete(&reset_key); }
            else { return false; }
        }

        let new_hits = hits + 1;
        self.cache.set(&cache_key, &new_hits.to_string(), Some(decay_seconds));
        if new_hits == 1 { self.cache.set(&reset_key, &self.now().wrapping_add(decay_seconds).to_string(), Some(decay_seconds)); }
        true
    }

    fn too_many_attempts(&self, key: &str, max_attempts: u64) -> bool { self.hits(key) >= max_attempts }
    fn remaining(&self, key: &str, max_attempts: u64) -> u64 { max_attempts.saturating_sub(self.hits(key)) }

    fn available_in(&self, key: &str) -> u64 {
        let reset = self.cache.get(&format!("{}:reset", self.key_for(key)))
            .and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        reset.saturating_sub(self.now())
    }

    fn hits(&self, key: &str) -> u64 {
        self.cache.get(&self.key_for(key)).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0)
    }

    fn clear(&self, key: &str) {
        let k = self.key_for(key); self.cache.delete(&k); self.cache.delete(&format!("{}:reset", k));
    }
}

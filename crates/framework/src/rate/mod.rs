//! Rate limiting — Laravel-style facade with token bucket backend.
//!
//! ```rust
//! use viontin::RateLimiter;
//!
//! // Attempt with callback
//! let result = RateLimiter::attempt("login:42", 5, 60, || {
//!     Ok::<_, ()>("logged in")
//! });
//!
//! // Or check manually
//! if RateLimiter::too_many_attempts("login:42", 5) {
//!     let secs = RateLimiter::available_in("login:42");
//!     eprintln!("Retry in {}s", secs);
//! }
//! ```

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::cache::{CacheDriver, MemoryCache};

// ── Driver Trait ──

pub trait RateLimiterDriver: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn attempt(&self, key: &str, max_attempts: u64, decay_seconds: u64) -> bool;
    fn too_many_attempts(&self, key: &str, max_attempts: u64) -> bool;
    fn remaining(&self, key: &str, max_attempts: u64) -> u64;
    fn available_in(&self, key: &str) -> u64;
    fn hits(&self, key: &str) -> u64;
    fn clear(&self, key: &str);
}

// ── TokenBucket Implementation ──

#[derive(Debug)]
pub struct TokenBucketLimiter {
    cache: Box<dyn CacheDriver>,
    prefix: String,
}

impl TokenBucketLimiter {
    pub fn new(cache: impl CacheDriver + 'static) -> Self {
        TokenBucketLimiter {
            cache: Box::new(cache),
            prefix: "rate:".into(),
        }
    }

    fn key_for(&self, key: &str) -> String { format!("{}{}", self.prefix, key) }
    fn now(&self) -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
}

impl RateLimiterDriver for TokenBucketLimiter {
    fn name(&self) -> &str { "token_bucket" }

    fn attempt(&self, key: &str, max_attempts: u64, decay_seconds: u64) -> bool {
        let cache_key = self.key_for(key);
        let reset_key = format!("{}:reset", cache_key);

        let hits = self.cache.get(&cache_key)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        if hits >= max_attempts {
            if let Some(reset) = self.cache.get(&reset_key)
                .and_then(|v| v.parse::<u64>().ok()) {
                if self.now() >= reset {
                    self.cache.delete(&cache_key);
                    self.cache.delete(&reset_key);
                } else {
                    return false;
                }
            }
        }

        let new_hits = hits + 1;
        self.cache.set(&cache_key, &new_hits.to_string(), Some(decay_seconds));
        if new_hits == 1 {
            let reset_time = self.now() + decay_seconds;
            self.cache.set(&reset_key, &reset_time.to_string(), Some(decay_seconds));
        }
        true
    }

    fn too_many_attempts(&self, key: &str, max_attempts: u64) -> bool {
        !self.attempt(key, max_attempts, 1)
    }

    fn remaining(&self, key: &str, max_attempts: u64) -> u64 {
        let cache_key = self.key_for(key);
        let hits = self.cache.get(&cache_key)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        max_attempts.saturating_sub(hits)
    }

    fn available_in(&self, key: &str) -> u64 {
        let reset_key = format!("{}:reset", self.key_for(key));
        let reset = self.cache.get(&reset_key)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let now = self.now();
        if reset > now { reset - now } else { 0 }
    }

    fn hits(&self, key: &str) -> u64 {
        let cache_key = self.key_for(key);
        self.cache.get(&cache_key)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
    }

    fn clear(&self, key: &str) {
        let cache_key = self.key_for(key);
        self.cache.delete(&cache_key);
        self.cache.delete(&format!("{}:reset", cache_key));
    }
}

// ── Facade ──

pub struct RateLimiter {
    driver: Box<dyn RateLimiterDriver>,
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter").field("driver", &self.driver.name()).finish()
    }
}

impl RateLimiter {
    pub fn with_driver(d: impl RateLimiterDriver + 'static) -> Self {
        RateLimiter { driver: Box::new(d) }
    }

    pub fn memory() -> Self {
        RateLimiter::with_driver(TokenBucketLimiter::new(MemoryCache::new()))
    }

    pub fn file(path: impl Into<std::path::PathBuf>) -> Self {
        RateLimiter::with_driver(TokenBucketLimiter::new(crate::cache::FileCache::new(path)))
    }

    pub fn attempt<F, T>(&self, key: &str, max_attempts: u64, decay_seconds: u64, f: F) -> Result<T, ()>
    where
        F: FnOnce() -> T,
    {
        if self.driver.attempt(key, max_attempts, decay_seconds) {
            Ok(f())
        } else {
            Err(())
        }
    }

    pub fn too_many_attempts(&self, key: &str, max_attempts: u64) -> bool {
        self.driver.too_many_attempts(key, max_attempts)
    }

    pub fn remaining(&self, key: &str, max_attempts: u64) -> u64 {
        self.driver.remaining(key, max_attempts)
    }

    pub fn hits(&self, key: &str) -> u64 {
        self.driver.hits(key)
    }

    pub fn available_in(&self, key: &str) -> u64 {
        self.driver.available_in(key)
    }

    pub fn clear(&self, key: &str) {
        self.driver.clear(key);
    }

    pub fn hit(&self, key: &str, decay_seconds: u64) {
        self.driver.attempt(key, u64::MAX, decay_seconds);
    }
}

// ── Global Singleton ──

static GLOBAL: OnceLock<RateLimiter> = OnceLock::new();

pub fn init(limiter: RateLimiter) {
    let _ = GLOBAL.set(limiter);
}

fn global() -> &'static RateLimiter {
    GLOBAL.get_or_init(|| RateLimiter::memory())
}

pub fn attempt<F, T>(key: &str, max_attempts: u64, decay_seconds: u64, f: F) -> Result<T, ()>
where
    F: FnOnce() -> T,
{
    global().attempt(key, max_attempts, decay_seconds, f)
}

pub fn too_many_attempts(key: &str, max_attempts: u64) -> bool {
    global().too_many_attempts(key, max_attempts)
}

pub fn remaining(key: &str, max_attempts: u64) -> u64 {
    global().remaining(key, max_attempts)
}

pub fn hits(key: &str) -> u64 {
    global().hits(key)
}

pub fn available_in(key: &str) -> u64 {
    global().available_in(key)
}

pub fn clear(key: &str) {
    global().clear(key);
}

pub fn hit(key: &str, decay_seconds: u64) {
    global().hit(key, decay_seconds);
}

//! Rate limiting — Laravel-style facade with token bucket backend.

pub mod token_bucket;

pub use token_bucket::TokenBucketLimiter;

use std::sync::OnceLock;
use crate::cache::MemoryCache;

pub trait RateLimiterDriver: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn attempt(&self, key: &str, max_attempts: u64, decay_seconds: u64) -> bool;
    fn too_many_attempts(&self, key: &str, max_attempts: u64) -> bool;
    fn remaining(&self, key: &str, max_attempts: u64) -> u64;
    fn available_in(&self, key: &str) -> u64;
    fn hits(&self, key: &str) -> u64;
    fn clear(&self, key: &str);
}

pub struct RateLimiter { driver: Box<dyn RateLimiterDriver> }

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
    where F: FnOnce() -> T {
        if self.driver.attempt(key, max_attempts, decay_seconds) { Ok(f()) } else { Err(()) }
    }

    pub fn too_many_attempts(&self, key: &str, max_attempts: u64) -> bool { self.driver.too_many_attempts(key, max_attempts) }
    pub fn remaining(&self, key: &str, max_attempts: u64) -> u64 { self.driver.remaining(key, max_attempts) }
    pub fn hits(&self, key: &str) -> u64 { self.driver.hits(key) }
    pub fn available_in(&self, key: &str) -> u64 { self.driver.available_in(key) }
    pub fn clear(&self, key: &str) { self.driver.clear(key); }
    /// Apply a hit (unlimited attempts).
    pub fn hit(&self, key: &str, decay_seconds: u64) { self.driver.attempt(key, u64::MAX, decay_seconds); }
}

// ── Global Singleton ──

static GLOBAL: OnceLock<RateLimiter> = OnceLock::new();

impl RateLimiter {
    /// Initialize the global singleton.
    pub fn init(limiter: RateLimiter) { let _ = GLOBAL.set(limiter); }

    /// Get the global singleton (initializes with memory backend if not set).
    fn global() -> &'static RateLimiter { GLOBAL.get_or_init(RateLimiter::memory) }

    /// Global attempt with rate limit check.
    pub fn global_attempt<F, T>(key: &str, max_attempts: u64, decay_seconds: u64, f: F) -> Result<T, ()>
    where F: FnOnce() -> T { Self::global().attempt(key, max_attempts, decay_seconds, f) }

    /// Check if global rate limit is exceeded.
    pub fn global_is_limited(key: &str, max_attempts: u64) -> bool { Self::global().too_many_attempts(key, max_attempts) }

    /// Get remaining attempts from global limiter.
    pub fn global_remaining(key: &str, max_attempts: u64) -> u64 { Self::global().remaining(key, max_attempts) }

    /// Get hit count from global limiter.
    pub fn global_hits(key: &str) -> u64 { Self::global().hits(key) }

    /// Get seconds until available from global limiter.
    pub fn global_available_in(key: &str) -> u64 { Self::global().available_in(key) }

    /// Clear rate limit data from global limiter.
    pub fn global_clear(key: &str) { Self::global().clear(key); }

    /// Record a hit on the global limiter.
    pub fn global_hit(key: &str, decay_seconds: u64) { Self::global().hit(key, decay_seconds); }
}

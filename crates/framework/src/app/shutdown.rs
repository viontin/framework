//! Graceful shutdown coordination.
//!
//! Manages signal handling, in-flight request tracking, and drain timeout.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct ShutdownCoordinator {
    shutting_down: Arc<AtomicBool>,
    in_flight: Arc<AtomicUsize>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            shutting_down: Arc::new(AtomicBool::new(false)),
            in_flight: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::SeqCst)
    }

    pub fn initiate_shutdown(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
    }

    pub fn acquire(&self) -> RequestGuard {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        RequestGuard {
            in_flight: self.in_flight.clone(),
        }
    }

    pub fn in_flight_count(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

    pub fn wait_for_drain(&self, timeout: std::time::Duration) {
        let start = std::time::Instant::now();
        while self.in_flight.load(Ordering::SeqCst) > 0 {
            if start.elapsed() > timeout {
                eprintln!("Shutdown drain timeout, forcing shutdown");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// Register SIGTERM/SIGINT signal handlers. On receiving a shutdown signal,
    /// calls `initiate_shutdown()` to begin graceful drain.
    #[cfg(feature = "shutdown")]
    pub fn listen_for_signal(&self) {
        let flag = self.shutting_down.clone();
        ctrlc::set_handler(move || {
            eprintln!("\nShutdown signal received, draining in-flight requests...");
            flag.store(true, Ordering::SeqCst);
        }).ok();
    }

    /// No-op when shutdown feature is not enabled.
    #[cfg(not(feature = "shutdown"))]
    pub fn listen_for_signal(&self) {}
}

impl Default for ShutdownCoordinator {
    fn default() -> Self { Self::new() }
}

/// Guard that decrements the in-flight counter when dropped.
#[derive(Debug)]
pub struct RequestGuard {
    in_flight: Arc<AtomicUsize>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

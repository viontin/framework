//! Debug utilities — re-exported from viontin-core with framework-specific extensions.

// Re-export all core debug utilities
pub use viontin_core::{dump, dd, dump_many, dd_many, Profiler, benchmark, memory_usage, MemoryStats, is_debug_mode, debug_only, when_local};

use crate::env::Environment;

/// Check if running in local/development environment.
pub fn is_local() -> bool {
    match Environment::detect() {
        Environment::Local | Environment::Development => true,
        _ => false,
    }
}

/// Execute only in local environment.
pub fn local_only<F: FnOnce()>(f: F) {
    if is_local() {
        f();
    }
}

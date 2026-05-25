//! HTTP types — re-exported from viontin-core with extensions.
//!
//! Core types (Request, Response, StatusCode, Method, Headers, Uri, Cookie)
//! are defined in viontin-core. This module adds framework-specific helpers.

pub mod form_request;

#[cfg(feature = "http-client")]
pub mod client;

// Re-export core types
pub use viontin_core::{Request, Response, StatusCode, Method, Headers, Uri, Cookie};

/// HTTP handler type alias.
pub type Handler = std::sync::Arc<dyn Fn(&Request) -> Response + Send + Sync>;

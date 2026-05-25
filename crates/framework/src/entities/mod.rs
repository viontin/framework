//! Entity — re-exported from viontin-core.
//!
//! An `Entity` is a pure data container. It knows nothing about databases.
//! Persistence is handled separately by `Repository` or `Model`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use viontin_framework::entity::Entity;
//!
//! #[derive(Debug, Clone)]
//! pub struct User { pub id: i64, pub name: String }
//!
//! impl Entity for User {
//!     fn id(&self) -> String { self.id.to_string() }
//!     fn table_name() -> &'static str { "users" }
//! }
//! ```

// Re-export Entity trait from viontin-core
pub use viontin_core::Entity;

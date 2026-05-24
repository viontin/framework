//! Entity — domain business object with identity and validation.
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

/// Base trait for domain entities.
///
/// Entities represent business objects with identity and lifecycle.
/// They are data containers — not active-record. All persistence
/// logic lives in the corresponding `Repository` or `Model`.
pub trait Entity: Clone + std::fmt::Debug + Send + Sync + 'static {
    /// Unique identifier for this entity.
    fn id(&self) -> String;

    /// Database table name.
    fn table_name() -> &'static str;

    /// Primary key column name (defaults to "id").
    fn primary_key() -> &'static str { "id" }

    /// Validate the entity before saving.
    /// Return Err with error messages to prevent saving.
    fn validate(&self) -> Result<(), Vec<String>> {
        Ok(())
    }
}

//! Viontin ORM integration (optional).
//!
//! This module is only available when the `orm` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! viontin-framework = { features = ["orm"] }
//! ```
//!
//! It re-exports the full `viontin-orm` API for convenience.
//!
//! # No Vendor Lock-In
//!
//! You can use `viontin-orm` directly as a standalone crate without
//! the framework. This module is purely for convenience when you want
//! both framework and ORM in one place.

#![cfg(feature = "orm")]

pub use viontin_orm::{
    Value, Row, DbConfig,
    Connection, ConnectionPool,
    DatabaseType, DriverCapabilities,
    NoSqlConnection,
    DriverRegistry, DriverInfo,
    QueryBuilder, Page, SimplePage,
    Schema, Blueprint,
    Migration, Migrator,
};

pub use viontin_orm::create_table;
pub use viontin_orm::update_table;

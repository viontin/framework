//! Model — active-record style convenience trait.
//!
//! `Model` combines data (like `Entity`) with persistence (like `Repository`)
//! into a single trait. This is the recommended default for most applications.
//!
//! Requires `features = ["orm"]` on `viontin-framework`.

use crate::db::{Connection, Row, Value};
use crate::entity::Entity;

/// Active-record style Model — default convenience trait.
///
/// Combines data + persistence in one trait.
/// Default CRUD requires the `orm` feature (uses `viontin_orm::QueryBuilder`).
///
/// For clean architecture with separated concerns, use `Entity` + `Repository`.
pub trait Model: Entity {
    fn connection(&self) -> &dyn Connection;
    fn from_row(row: &Row) -> Result<Self, String>;
    fn to_values(&self) -> Vec<(&str, Value)>;

    fn before_save(&mut self) -> Result<(), String> { Ok(()) }
    fn after_save(&self) -> Result<(), String> { Ok(()) }
    fn before_delete(&self) -> Result<(), String> { Ok(()) }
    fn after_delete(&self) -> Result<(), String> { Ok(()) }

    /// Save (insert or update) — uses self.connection().
    #[cfg(feature = "orm")]
    fn save(&mut self) -> Result<Self, String> {
        self.before_save()?;
        let values = self.to_values();
        let conn = self.connection();
        use viontin_orm::QueryBuilder;
        let id = QueryBuilder::table(conn, Self::table_name()).insert(values)?;
        self.after_save()?;
        Self::find(conn, id)?.ok_or_else(|| "Failed to re-fetch after save".into())
    }

    /// Delete — uses self.connection().
    #[cfg(feature = "orm")]
    fn delete(&self) -> Result<u64, String> {
        self.before_delete()?;
        use viontin_orm::QueryBuilder;
        let r = QueryBuilder::table(self.connection(), Self::table_name())
            .where_eq(Self::primary_key(), self.id())
            .delete()?;
        self.after_delete()?;
        Ok(r)
    }

    /// Get all.
    #[cfg(feature = "orm")]
    fn all(conn: &dyn Connection) -> Result<Vec<Self>, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name())
            .get()?.into_iter().map(|r| Self::from_row(&r)).collect()
    }

    /// Find by primary key.
    #[cfg(feature = "orm")]
    fn find(conn: &dyn Connection, id: i64) -> Result<Option<Self>, String> {
        use viontin_orm::QueryBuilder;
        let rows = QueryBuilder::table(conn, Self::table_name())
            .where_eq(Self::primary_key(), id).get()?;
        rows.into_iter().next().map(|r| Self::from_row(&r)).transpose()
    }

    /// Create from column data.
    #[cfg(feature = "orm")]
    fn create(conn: &dyn Connection, data: Vec<(&str, Value)>) -> Result<i64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name()).insert(data)
    }

    /// Update by primary key.
    #[cfg(feature = "orm")]
    fn update(conn: &dyn Connection, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name())
            .where_eq(Self::primary_key(), id).update(data)
    }

    /// Delete by primary key.
    #[cfg(feature = "orm")]
    fn delete_by_id(conn: &dyn Connection, id: i64) -> Result<u64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name())
            .where_eq(Self::primary_key(), id).delete()
    }
}



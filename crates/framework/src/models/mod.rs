//! Model — active-record style convenience trait.
//!
//! `Model` combines data (like `Entity`) with persistence (like `Repository`)
//! into a single trait. This is the recommended default for most applications.
//!
//! Requires `features = ["orm"]` on `viontin-framework`.

use crate::db::{Connection, Row, Value};
use crate::entities::Entity;

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
    fn all(conn: &dyn Connection) -> Result<Vec<Self>, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name())
            .get()?.into_iter().map(|r| Self::from_row(&r)).collect()
    }

    /// Find by primary key.
    fn find(conn: &dyn Connection, id: i64) -> Result<Option<Self>, String> {
        use viontin_orm::QueryBuilder;
        let rows = QueryBuilder::table(conn, Self::table_name())
            .where_eq(Self::primary_key(), id).get()?;
        rows.into_iter().next().map(|r| Self::from_row(&r)).transpose()
    }

    /// Create from column data.
    fn create(conn: &dyn Connection, data: Vec<(&str, Value)>) -> Result<i64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name()).insert(data)
    }

    /// Update by primary key.
    fn update(conn: &dyn Connection, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name())
            .where_eq(Self::primary_key(), id).update(data)
    }

    /// Delete by primary key.
    fn delete_by_id(conn: &dyn Connection, id: i64) -> Result<u64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(conn, Self::table_name())
            .where_eq(Self::primary_key(), id).delete()
    }

    // ── Relationships ──

    /// Get all child records linked by `foreign_key = self.id()`.
    fn has_many<Child: Model>(&self, foreign_key: &str) -> Result<Vec<Child>, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), Child::table_name())
            .where_eq(foreign_key, self.id())
            .get()?.into_iter().map(|r| Child::from_row(&r)).collect()
    }

    /// Get the parent record linked by `foreign_key = parent.id()`.
    fn belongs_to<Parent: Model>(&self, foreign_key: &str) -> Result<Option<Parent>, String> {
        let fk_value = self.foreign_key_value(foreign_key)?;
        use viontin_orm::QueryBuilder;
        let rows = QueryBuilder::table(self.connection(), Parent::table_name())
            .where_eq(Parent::primary_key(), fk_value)
            .get()?;
        rows.into_iter().next().map(|r| Parent::from_row(&r)).transpose()
    }

    /// Get the value of a foreign key column from this model.
    /// Override if your column names differ from the default pattern.
    fn foreign_key_value(&self, foreign_key: &str) -> Result<i64, String> {
        let values = self.to_values();
        values.iter()
            .find(|(k, _)| *k == foreign_key)
            .and_then(|(_, v)| match v {
                Value::Int(i) => Some(*i),
                Value::Text(t) => t.parse().ok(),
                _ => None,
            })
            .ok_or_else(|| format!("Foreign key '{}' not found on {}", foreign_key, Self::table_name()))
    }
}



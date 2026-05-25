//! Repository — data access with DI and lifecycle hooks.
//!
//! Default CRUD methods (`all`, `find`, `create`, `save`, `update`, `delete`)
//! require the `orm` feature. Hooks and type definitions are always available.

pub mod default;
pub use default::QueryScoped;

use crate::db::{Connection, Row, Value};
use crate::entities::Entity;

pub trait Repository<M: Entity>: std::fmt::Debug + Send + Sync {
    fn connection(&self) -> &dyn Connection;
    fn from_row(&self, row: &Row) -> Result<M, String>;
    fn to_values(&self, entity: &M) -> Vec<(&str, Value)>;
    fn table(&self) -> &str { "" }
    fn primary_key(&self) -> &str { "id" }

    fn tbl(&self) -> String {
        let t = self.table();
        if t.is_empty() { M::table_name().to_string() } else { t.to_string() }
    }

    // ── Hooks ──

    fn before_save(&self, _entity: &mut M) -> Result<(), String> { Ok(()) }
    fn after_save(&self, _entity: &M) -> Result<(), String> { Ok(()) }
    fn before_delete(&self, _entity: &M) -> Result<(), String> { Ok(()) }
    fn after_delete(&self, _entity: &M) -> Result<(), String> { Ok(()) }

    // ── Default CRUD (requires `orm`) ──

    fn all(&self) -> Result<Vec<M>, String> { self._all() }
    fn find(&self, id: i64) -> Result<Option<M>, String> { self._find(id) }
    fn create(&self, data: Vec<(&str, Value)>) -> Result<i64, String> { self._create(data) }
    fn save(&self, entity: &mut M) -> Result<M, String> { self._save(entity) }
    fn update(&self, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> { self._update(id, data) }
    fn delete(&self, entity: &M) -> Result<u64, String> { self._delete(entity) }
    fn query(&self) -> QueryScoped<'_, M, Self> where Self: Sized { self._query() }

    // ── Internal implementations ──

    fn _all(&self) -> Result<Vec<M>, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl()).get()?
            .into_iter().map(|r| self.from_row(&r)).collect()
    }

    fn _find(&self, id: i64) -> Result<Option<M>, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl())
            .where_eq(self.primary_key(), id).get()?
            .into_iter().next().map(|r| self.from_row(&r)).transpose()
    }

    fn _create(&self, data: Vec<(&str, Value)>) -> Result<i64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl()).insert(data)
    }

    fn _save(&self, entity: &mut M) -> Result<M, String> {
        let mut e = entity.clone(); self.before_save(&mut e)?;
        let values = self.to_values(&e);
        use viontin_orm::QueryBuilder;
        let id = QueryBuilder::table(self.connection(), &self.tbl()).insert(values)?;
        self.after_save(&e)?;
        self._find(id)?.ok_or_else(|| "Failed to re-fetch after save".into())
    }

    fn _update(&self, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl())
            .where_eq(self.primary_key(), id).update(data)
    }

    fn _delete(&self, entity: &M) -> Result<u64, String> {
        self.before_delete(entity)?;
        use viontin_orm::QueryBuilder;
        let affected = QueryBuilder::table(self.connection(), &self.tbl())
            .where_eq(self.primary_key(), entity.id()).delete()?;
        self.after_delete(entity)?; Ok(affected)
    }

    fn _query(&self) -> QueryScoped<'_, M, Self> where Self: Sized { QueryScoped::new(self) }
}

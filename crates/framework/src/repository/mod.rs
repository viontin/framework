use crate::db::{Connection, Row, Value};
use crate::entity::Entity;

/// Repository trait — data access with DI and lifecycle hooks.
///
/// Receives a `Connection` via `connection()` method (dependency injection).
/// Provides hooks: `before_save`, `after_save`, `before_delete`, `after_delete`.
///
/// Default CRUD methods (`all`, `find`, `create`, `save`, `update`, `delete`)
/// require the `orm` feature (they use `viontin_orm::QueryBuilder`).
/// Hooks and type definitions are always available.
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

    // ── Default CRUD (requires `orm` feature to use viontin_orm::QueryBuilder) ──

    /// Get all entities.
    #[cfg(feature = "orm")]
    fn all(&self) -> Result<Vec<M>, String> { self._all() }

    /// Find by primary key.
    #[cfg(feature = "orm")]
    fn find(&self, id: i64) -> Result<Option<M>, String> { self._find(id) }

    /// Create from column data.
    #[cfg(feature = "orm")]
    fn create(&self, data: Vec<(&str, Value)>) -> Result<i64, String> { self._create(data) }

    /// Save (insert or update) an entity.
    #[cfg(feature = "orm")]
    fn save(&self, entity: &mut M) -> Result<M, String> { self._save(entity) }

    /// Update by primary key.
    #[cfg(feature = "orm")]
    fn update(&self, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> { self._update(id, data) }

    /// Delete an entity.
    #[cfg(feature = "orm")]
    fn delete(&self, entity: &M) -> Result<u64, String> { self._delete(entity) }

    /// Begin a scoped query chain.
    #[cfg(feature = "orm")]
    fn query(&self) -> QueryScoped<'_, M, Self> where Self: Sized { self._query() }

    // ── Internal (always compiled, called by #[cfg] wrappers) ──

    #[cfg(feature = "orm")]
    fn _all(&self) -> Result<Vec<M>, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl())
            .get()?
            .into_iter()
            .map(|r| self.from_row(&r))
            .collect()
    }

    #[cfg(feature = "orm")]
    fn _find(&self, id: i64) -> Result<Option<M>, String> {
        use viontin_orm::QueryBuilder;
        let rows = QueryBuilder::table(self.connection(), &self.tbl())
            .where_eq(self.primary_key(), id)
            .get()?;
        rows.into_iter().next().map(|r| self.from_row(&r)).transpose()
    }

    #[cfg(feature = "orm")]
    fn _create(&self, data: Vec<(&str, Value)>) -> Result<i64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl()).insert(data)
    }

    #[cfg(feature = "orm")]
    fn _save(&self, entity: &mut M) -> Result<M, String> {
        let mut e = entity.clone();
        self.before_save(&mut e)?;
        let values = self.to_values(&e);
        use viontin_orm::QueryBuilder;
        let id = QueryBuilder::table(self.connection(), &self.tbl()).insert(values)?;
        self.after_save(&e)?;
        self._find(id)?.ok_or_else(|| "Failed to re-fetch after save".into())
    }

    #[cfg(feature = "orm")]
    fn _update(&self, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> {
        use viontin_orm::QueryBuilder;
        QueryBuilder::table(self.connection(), &self.tbl())
            .where_eq(self.primary_key(), id)
            .update(data)
    }

    #[cfg(feature = "orm")]
    fn _delete(&self, entity: &M) -> Result<u64, String> {
        self.before_delete(entity)?;
        use viontin_orm::QueryBuilder;
        let affected = QueryBuilder::table(self.connection(), &self.tbl())
            .where_eq(self.primary_key(), entity.id())
            .delete()?;
        self.after_delete(entity)?;
        Ok(affected)
    }

    #[cfg(feature = "orm")]
    fn _query(&self) -> QueryScoped<'_, M, Self> where Self: Sized {
        QueryScoped::new(self)
    }
}

/// Scoped query builder for chaining additional conditions.
///
/// Builds a query using `viontin_orm::QueryBuilder` internally and maps
/// results through the repository's `from_row()`.
///
/// # Example
///
/// ```rust,ignore
/// let admins = repo.query()
///     .where_eq("role", "admin")
///     .where_eq("active", true)
///     .order_by("name", "asc")
///     .all()?;
/// ```
#[cfg(feature = "orm")]
pub struct QueryScoped<'a, M: Entity, R: Repository<M> + 'a> {
    repo: &'a R,
    conn: &'a dyn Connection,
    qb: viontin_orm::QueryBuilder<'a>,
    _marker: ::std::marker::PhantomData<M>,
}

#[cfg(feature = "orm")]
impl<'a, M: Entity, R: Repository<M>> QueryScoped<'a, M, R> {
    pub fn where_eq(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.qb = self.qb.where_eq(col, val);
        self
    }

    pub fn where_gt(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.qb = self.qb.where_gt(col, val);
        self
    }

    pub fn where_null(mut self, col: &str) -> Self {
        self.qb = self.qb.where_null(col);
        self
    }

    pub fn order_by(mut self, col: &str, dir: &str) -> Self {
        self.qb = self.qb.order_by(col, dir);
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.qb = self.qb.limit(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.qb = self.qb.offset(n);
        self
    }

    pub fn all(&self) -> Result<Vec<M>, String> {
        self.qb.get()?.into_iter().map(|r| self.repo.from_row(&r)).collect()
    }

    pub fn first(&self) -> Result<Option<M>, String> {
        let rows = self.qb.clone().limit(1).get()?;
        rows.into_iter().next().map(|r| self.repo.from_row(&r)).transpose()
    }

    pub fn count(&self) -> Result<u64, String> {
        self.qb.count()
    }
}

#[cfg(feature = "orm")]
impl<'a, M: Entity, R: Repository<M>> QueryScoped<'a, M, R> {
    fn new(repo: &'a R) -> Self {
        use viontin_orm::QueryBuilder;
        let conn = repo.connection();
        let tbl = repo.tbl();
        QueryScoped {
            repo,
            conn,
            qb: QueryBuilder::table(conn, &tbl),
            _marker: ::std::marker::PhantomData,
        }
    }
}

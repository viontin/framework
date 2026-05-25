use crate::db::{Connection, Value};
use crate::entities::Entity;
use crate::repositories::Repository;

pub struct QueryScoped<'a, M: Entity, R: Repository<M> + 'a> {
    repo: &'a R,
    conn: &'a dyn Connection,
    qb: viontin_orm::QueryBuilder<'a>,
    _marker: std::marker::PhantomData<M>,
}

impl<'a, M: Entity, R: Repository<M>> QueryScoped<'a, M, R> {
    pub fn new(repo: &'a R) -> Self {
        let conn = repo.connection();
        QueryScoped {
            repo,
            conn,
            qb: viontin_orm::QueryBuilder::table(conn, &repo.tbl()),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn where_eq(mut self, col: &str, val: impl Into<Value>) -> Self { self.qb = self.qb.where_eq(col, val); self }
    pub fn where_gt(mut self, col: &str, val: impl Into<Value>) -> Self { self.qb = self.qb.where_gt(col, val); self }
    pub fn where_null(mut self, col: &str) -> Self { self.qb = self.qb.where_null(col); self }
    pub fn order_by(mut self, col: &str, dir: &str) -> Self { self.qb = self.qb.order_by(col, dir); self }
    pub fn limit(mut self, n: u64) -> Self { self.qb = self.qb.limit(n); self }
    pub fn offset(mut self, n: u64) -> Self { self.qb = self.qb.offset(n); self }

    pub fn all(&self) -> Result<Vec<M>, String> {
        self.qb.get()?.into_iter().map(|r| self.repo.from_row(&r)).collect()
    }

    pub fn first(&self) -> Result<Option<M>, String> {
        self.qb.clone().limit(1).get()?.into_iter().next()
            .map(|r| self.repo.from_row(&r)).transpose()
    }

    pub fn count(&self) -> Result<u64, String> { self.qb.count() }
}

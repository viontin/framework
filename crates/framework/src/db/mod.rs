//! Framework database layer.
//!
//! Database types come from viontin-orm.

pub mod query_log;

// Re-export from viontin-orm
pub use viontin_orm::{Value, Row, DbConfig, Connection, ConnectionPool, Transaction, with_transaction};

mod compat {
    /// Lightweight raw SQL query builder — only available when `orm` is enabled.
    /// For the full-featured builder, use `viontin_orm::QueryBuilder` directly.
    pub struct QueryBuilder<'a> {
        conn: &'a dyn viontin_orm::Connection,
        table: String,
        cols: Vec<String>,
        wheres: Vec<WhereClause>,
        order_bys: Vec<(String, String)>,
        limit: Option<u64>,
        offset: Option<u64>,
    }

    #[derive(Clone)]
    struct WhereClause { col: String, op: String, val: viontin_orm::Value, bool: String }

    impl<'a> QueryBuilder<'a> {
        pub fn new(conn: &'a dyn viontin_orm::Connection, table: impl Into<String>) -> Self {
            QueryBuilder {
                conn, table: table.into(), cols: vec!["*".into()],
                wheres: Vec::new(), order_bys: Vec::new(), limit: None, offset: None,
            }
        }
        pub fn where_eq(mut self, col: impl Into<String>, val: impl Into<viontin_orm::Value>) -> Self {
            self.wheres.push(WhereClause { col: col.into(), op: "=".into(), val: val.into(), bool: "and".into() });
            self
        }
        pub fn order_by(mut self, col: impl Into<String>, dir: impl Into<String>) -> Self {
            self.order_bys.push((col.into(), dir.into()));
            self
        }
        pub fn limit(mut self, n: u64) -> Self { self.limit = Some(n); self }
        pub fn offset(mut self, n: u64) -> Self { self.offset = Some(n); self }
        pub fn get(&self) -> Result<Vec<viontin_orm::Row>, String> {
            let (s, p) = self.sql();
            self.conn.query(&s, &p)
        }
        pub fn count(&self) -> Result<u64, String> {
            let rows = self.conn.query(&format!("SELECT COUNT(*) as count FROM {} {}", self.table,
                if self.wheres.is_empty() { String::new() } else {
                    format!("WHERE {}", self.wheres.iter().map(|w| format!("{} {} ?", w.col, w.op)).collect::<Vec<_>>().join(" AND "))
                }), &self.wheres.iter().map(|w| w.val.clone()).collect::<Vec<_>>())?;
            rows.first().and_then(|r| r.int("count")).map(|c| c as u64).ok_or_else(|| "Count failed".into())
        }
        pub fn insert(&self, data: Vec<(&str, viontin_orm::Value)>) -> Result<i64, String> {
            let cols: Vec<String> = data.iter().map(|(c, _)| c.to_string()).collect();
            let params: Vec<viontin_orm::Value> = data.into_iter().map(|(_, v)| v).collect();
            let placeholders: Vec<String> = params.iter().map(|_| "?".to_string()).collect();
            self.conn.execute(&format!("INSERT INTO {} ({}) VALUES ({})", self.table, cols.join(", "), placeholders.join(", ")), &params)?;
            self.conn.last_insert_id()
        }
        fn sql(&self) -> (String, Vec<viontin_orm::Value>) {
            let mut sql = format!("SELECT {} FROM {}", self.cols.join(", "), self.table);
            let mut params = Vec::new();
            if !self.wheres.is_empty() {
                sql.push_str(" WHERE ");
                for (i, w) in self.wheres.iter().enumerate() {
                    if i > 0 { sql.push(' '); sql.push_str(&w.bool); sql.push(' '); }
                    sql.push_str(&format!("{} {} ?", w.col, w.op));
                    params.push(w.val.clone());
                }
            }
            if !self.order_bys.is_empty() {
                sql.push_str(" ORDER BY ");
                sql.push_str(&self.order_bys.iter().map(|(c, d)| format!("{} {}", c, d)).collect::<Vec<_>>().join(", "));
            }
            if let Some(n) = self.limit { sql.push_str(&format!(" LIMIT {}", n)); }
            if let Some(n) = self.offset { sql.push_str(&format!(" OFFSET {}", n)); }
            (sql, params)
        }
    }
}

pub use compat::QueryBuilder;

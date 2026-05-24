use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Null, Int(i64), Float(f64), Text(String), Bool(bool), Blob(Vec<u8>),
}

impl From<i64> for Value { fn from(v: i64) -> Self { Value::Int(v) } }
impl From<f64> for Value { fn from(v: f64) -> Self { Value::Float(v) } }
impl From<String> for Value { fn from(v: String) -> Self { Value::Text(v) } }
impl From<&str> for Value { fn from(v: &str) -> Self { Value::Text(v.to_string()) } }
impl From<bool> for Value { fn from(v: bool) -> Self { Value::Bool(v) } }

#[derive(Debug, Clone)]
pub struct Row { columns: HashMap<String, Value> }

impl Row {
    pub fn new(columns: HashMap<String, Value>) -> Self { Row { columns } }
    pub fn get(&self, col: &str) -> Option<&Value> { self.columns.get(col) }
    pub fn int(&self, col: &str) -> Option<i64> {
        self.columns.get(col).and_then(|v| if let Value::Int(i) = v { Some(*i) } else if let Value::Text(t) = v { t.parse().ok() } else { None })
    }
    pub fn string(&self, col: &str) -> Option<String> {
        self.columns.get(col).and_then(|v| match v { Value::Text(t) => Some(t.clone()), Value::Int(i) => Some(i.to_string()), _ => None })
    }
    pub fn is_empty(&self) -> bool { self.columns.is_empty() }
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub driver: String, pub host: String, pub port: u16, pub database: String,
    pub username: String, pub password: String, pub charset: String,
    pub prefix: String, pub pool_min: u32, pub pool_max: u32,
}

impl Default for DbConfig {
    fn default() -> Self {
        DbConfig {
            driver: String::new(), host: "127.0.0.1".into(), port: 0, database: String::new(),
            username: String::new(), password: String::new(), charset: "utf8".into(),
            prefix: String::new(), pool_min: 2, pool_max: 10,
        }
    }
}

pub trait Connection: std::fmt::Debug + Send + Sync {
    fn driver_name(&self) -> &str;
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>, String>;
    fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, String>;
    fn last_insert_id(&self) -> Result<i64, String>;
    fn begin(&self) -> Result<(), String>;
    fn commit(&self) -> Result<(), String>;
    fn rollback(&self) -> Result<(), String>;
    fn is_connected(&self) -> bool;
}

pub trait ConnectionPool: std::fmt::Debug + Send + Sync {
    fn config(&self) -> &DbConfig;
    fn connection(&self) -> Result<Box<dyn Connection>, String>;
}

pub struct QueryBuilder<'a> {
    conn: &'a dyn Connection,
    table: String, cols: Vec<String>, wheres: Vec<WhereClause>,
    order_bys: Vec<(String, String)>, limit: Option<u64>, offset: Option<u64>,
}

#[derive(Clone)]
struct WhereClause { col: String, op: String, val: Value, bool: String }

impl<'a> QueryBuilder<'a> {
    pub fn new(conn: &'a dyn Connection, table: impl Into<String>) -> Self {
        QueryBuilder {
            conn, table: table.into(), cols: vec!["*".into()],
            wheres: Vec::new(), order_bys: Vec::new(), limit: None, offset: None,
        }
    }

    pub fn where_eq(mut self, col: impl Into<String>, val: impl Into<Value>) -> Self {
        self.wheres.push(WhereClause { col: col.into(), op: "=".into(), val: val.into(), bool: "and".into() });
        self
    }

    pub fn order_by(mut self, col: impl Into<String>, dir: impl Into<String>) -> Self {
        self.order_bys.push((col.into(), dir.into()));
        self
    }

    pub fn limit(mut self, n: u64) -> Self { self.limit = Some(n); self }
    pub fn offset(mut self, n: u64) -> Self { self.offset = Some(n); self }

    fn sql(&self) -> (String, Vec<Value>) {
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

    pub fn get(&self) -> Result<Vec<Row>, String> { let (s, p) = self.sql(); self.conn.query(&s, &p) }

    pub fn count(&self) -> Result<u64, String> {
        let rows = self.conn.query(&format!("SELECT COUNT(*) as count FROM {} {}", self.table,
            if self.wheres.is_empty() { String::new() } else {
                format!("WHERE {}", self.wheres.iter().map(|w| format!("{} {} ?", w.col, w.op)).collect::<Vec<_>>().join(" AND "))
            }), &self.wheres.iter().map(|w| w.val.clone()).collect::<Vec<_>>())?;
        rows.first().and_then(|r| r.int("count")).map(|c| c as u64).ok_or_else(|| "Count failed".into())
    }

    pub fn insert(&self, data: Vec<(&str, Value)>) -> Result<i64, String> {
        let cols: Vec<String> = data.iter().map(|(c, _)| c.to_string()).collect();
        let params: Vec<Value> = data.into_iter().map(|(_, v)| v).collect();
        let placeholders: Vec<String> = params.iter().map(|_| "?".to_string()).collect();
        self.conn.execute(&format!("INSERT INTO {} ({}) VALUES ({})", self.table, cols.join(", "), placeholders.join(", ")), &params)?;
        self.conn.last_insert_id()
    }
}

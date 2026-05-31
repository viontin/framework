//! Database-backed queue — persists jobs to SQLite via ORM.
//!
//! Jobs survive restarts. Status tracking: pending → running → done/failed.
//! Inspect and retry via direct SQL queries on the `_jobs` table.

use std::fmt;
use std::sync::Mutex;
use crate::db::{Connection, Value};
use crate::queue::{Job, Driver};

struct StoredJob {
    pub id: i64,
    pub name: String,
    pub payload: String,
    pub created_at: i64,
    pub available_at: i64,
}

pub struct DatabaseQueue {
    conn: Mutex<Box<dyn Connection>>,
    table: String,
}

impl fmt::Debug for DatabaseQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseQueue").field("table", &self.table).finish()
    }
}

impl DatabaseQueue {
    pub fn new(conn: Box<dyn Connection>) -> Self {
        let queue = DatabaseQueue { conn: Mutex::new(conn), table: "_jobs".into() };
        queue.ensure_table();
        queue
    }

    pub fn with_table(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self.ensure_table();
        self
    }

    fn ensure_table(&self) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (\
                     id INTEGER PRIMARY KEY AUTOINCREMENT, \
                     name TEXT NOT NULL, \
                     payload TEXT, \
                     status TEXT NOT NULL DEFAULT 'pending', \
                     attempts INTEGER NOT NULL DEFAULT 0, \
                     created_at INTEGER NOT NULL, \
                     available_at INTEGER NOT NULL, \
                     error TEXT)",
                    self.table
                ),
                &[],
            );
        }
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// Mark a pending job as running.
    pub fn mark_running(&self, job_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "UPDATE {} SET status = 'running', attempts = attempts + 1 WHERE id = ?",
            self.table
        );
        conn.execute(&sql, &[Value::Int(job_id)])?;
        Ok(())
    }

    /// Mark a job as successfully completed.
    pub fn mark_done(&self, job_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!("UPDATE {} SET status = 'done' WHERE id = ?", self.table);
        conn.execute(&sql, &[Value::Int(job_id)])?;
        Ok(())
    }

    /// Mark a job as failed with error message.
    pub fn mark_failed(&self, job_id: i64, error: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "UPDATE {} SET status = 'failed', error = ? WHERE id = ?",
            self.table
        );
        conn.execute(&sql, &[Value::Text(error.to_string()), Value::Int(job_id)])?;
        Ok(())
    }

    /// Get the next pending job ready for processing.
    pub fn next_pending(&self) -> Option<StoredJob> {
        let conn = self.conn.lock().ok()?;
        let now = Self::now();
        let sql = format!(
            "SELECT id, name, payload, created_at, available_at FROM {} \
             WHERE status = 'pending' AND available_at <= ? \
             ORDER BY available_at ASC LIMIT 1",
            self.table
        );
        let rows = conn.query(&sql, &[Value::Int(now)]).ok()?;
        rows.into_iter().next().map(|r| StoredJob {
            id: r.int("id").unwrap_or(0),
            name: r.string("name").unwrap_or_default(),
            payload: r.string("payload").unwrap_or_default(),
            created_at: r.int("created_at").unwrap_or(0),
            available_at: r.int("available_at").unwrap_or(0),
        })
    }

    /// Get all failed jobs for inspection.
    pub fn failed_jobs(&self) -> Result<Vec<StoredJob>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "SELECT id, name, payload, created_at, available_at FROM {} \
             WHERE status = 'failed' ORDER BY id DESC LIMIT 100",
            self.table
        );
        conn.query(&sql, &[]).map(|rows| {
            rows.into_iter()
                .map(|r| StoredJob {
                    id: r.int("id").unwrap_or(0),
                    name: r.string("name").unwrap_or_default(),
                    payload: r.string("payload").unwrap_or_default(),
                    created_at: r.int("created_at").unwrap_or(0),
                    available_at: r.int("available_at").unwrap_or(0),
                })
                .collect()
        })
    }

    /// Retry a previously failed job.
    pub fn retry(&self, job_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "UPDATE {} SET status = 'pending', error = NULL, available_at = ? WHERE id = ?",
            self.table
        );
        conn.execute(&sql, &[Value::Int(Self::now()), Value::Int(job_id)])?;
        Ok(())
    }

    /// Clean up completed jobs older than N seconds.
    pub fn cleanup(&self, older_than_secs: i64) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let cutoff = Self::now() - older_than_secs;
        let sql = format!(
            "DELETE FROM {} WHERE (status = 'done' OR status = 'failed') AND created_at < ?",
            self.table
        );
        conn.execute(&sql, &[Value::Int(cutoff)])
    }
}

impl StoredJob {
    pub fn id(&self) -> i64 { self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn payload(&self) -> &str { &self.payload }
}

impl Driver for DatabaseQueue {
    fn name(&self) -> &str { "database" }
    fn push(&self, job: Box<dyn Job>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = Self::now();
        let sql = format!(
            "INSERT INTO {} (name, payload, status, created_at, available_at) VALUES (?, ?, 'pending', ?, ?)",
            self.table
        );
        conn.execute(
            &sql,
            &[
                Value::Text(job.name().to_string()),
                Value::Text(String::new()),
                Value::Int(now),
                Value::Int(now),
            ],
        )?;
        Ok(())
    }
    fn schedule(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let available = Self::now() + delay_secs as i64;
        let sql = format!(
            "INSERT INTO {} (name, payload, status, created_at, available_at) VALUES (?, ?, 'pending', ?, ?)",
            self.table
        );
        conn.execute(
            &sql,
            &[
                Value::Text(job.name().to_string()),
                Value::Text(String::new()),
                Value::Int(Self::now()),
                Value::Int(available),
            ],
        )?;
        Ok(())
    }
    fn pop(&self) -> Option<Box<dyn Job>> {
        None
    }
}

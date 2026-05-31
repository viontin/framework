//! Database-backed queue — persists jobs to SQLite via ORM.
//!
//! Jobs survive restarts. Status tracking: pending → running → done/failed.
//! Inspect, retry, and flush via CLI commands (`queue:*`).

use std::fmt;
use std::sync::Mutex;
use crate::db::{Connection, Value};
use crate::queue::{self, Job, Driver};

pub struct StoredJob {
    pub id: i64,
    pub name: String,
    pub payload: String,
    pub created_at: i64,
    pub available_at: i64,
    pub attempts: i64,
    pub error: Option<String>,
}

pub struct DatabaseQueue {
    conn: Mutex<Box<dyn Connection>>,
    table: String,
    max_tries: u8,
    retry_delay_secs: u64,
}

impl fmt::Debug for DatabaseQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseQueue").field("table", &self.table).finish()
    }
}

impl DatabaseQueue {
    pub fn new(conn: Box<dyn Connection>) -> Self {
        let queue = DatabaseQueue { conn: Mutex::new(conn), table: "_jobs".into(), max_tries: 1, retry_delay_secs: 5 };
        queue.ensure_table();
        queue
    }

    pub fn with_table(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self.ensure_table();
        self
    }

    pub fn with_max_tries(mut self, n: u8) -> Self {
        self.max_tries = n;
        self
    }

    pub fn with_retry_delay(mut self, secs: u64) -> Self {
        self.retry_delay_secs = secs;
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

    pub fn connection(&self) -> Result<std::sync::MutexGuard<'_, Box<dyn Connection>>, String> {
        self.conn.lock().map_err(|e| e.to_string())
    }

    pub fn table_name(&self) -> &str {
        &self.table
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

    /// Reset a job back to pending for retry.
    pub fn mark_pending(&self, job_id: i64, delay_secs: u64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let available = Self::now() + delay_secs as i64;
        let sql = format!(
            "UPDATE {} SET status = 'pending', error = NULL, available_at = ? WHERE id = ?",
            self.table
        );
        conn.execute(&sql, &[Value::Int(available), Value::Int(job_id)]).map(|_| ())
    }

    /// Get the next pending job ready for processing.
    pub fn next_pending(&self) -> Option<StoredJob> {
        let conn = self.conn.lock().ok()?;
        let now = Self::now();
        let sql = format!(
            "SELECT id, name, payload, created_at, available_at, attempts, error FROM {} \
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
            attempts: r.int("attempts").unwrap_or(0),
            error: r.string("error"),
        })
    }

    /// Get all failed jobs for inspection.
    pub fn failed_jobs(&self) -> Result<Vec<StoredJob>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "SELECT id, name, payload, created_at, available_at, attempts, error FROM {} \
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
                    attempts: r.int("attempts").unwrap_or(0),
                    error: r.string("error"),
                })
                .collect()
        })
    }

    /// Retry a previously failed job.
    pub fn retry(&self, job_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "UPDATE {} SET status = 'pending', error = NULL, attempts = 0, available_at = ? WHERE id = ?",
            self.table
        );
        conn.execute(&sql, &[Value::Int(Self::now()), Value::Int(job_id)])?;
        Ok(())
    }

    /// Retry all failed jobs.
    pub fn retry_all(&self) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = Self::now();
        let sql = format!(
            "UPDATE {} SET status = 'pending', error = NULL, attempts = 0, available_at = ? WHERE status = 'failed'",
            self.table
        );
        conn.execute(&sql, &[Value::Int(now)])
    }

    /// Flush (delete) all jobs with a given status.
    pub fn flush(&self, status: &str) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!("DELETE FROM {} WHERE status = ?", self.table);
        conn.execute(&sql, &[Value::Text(status.to_string())])
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

    pub fn count_by_status(&self) -> Result<Vec<(String, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!(
            "SELECT status, COUNT(*) as cnt FROM {} GROUP BY status ORDER BY status",
            self.table
        );
        conn.query(&sql, &[]).map(|rows| {
            rows.into_iter()
                .filter_map(|r| {
                    let status = r.string("status")?;
                    let cnt = r.int("cnt").unwrap_or(0);
                    Some((status, cnt))
                })
                .collect()
        })
    }
}

impl Driver for DatabaseQueue {
    fn name(&self) -> &str { "database" }
    fn push(&self, job: Box<dyn Job>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = Self::now();
        let payload = serde_json::to_string(&serde_json::json!({
            "name": job.name(),
        }))
        .unwrap_or_default();
        let sql = format!(
            "INSERT INTO {} (name, payload, status, created_at, available_at) VALUES (?, ?, 'pending', ?, ?)",
            self.table
        );
        conn.execute(
            &sql,
            &[
                Value::Text(job.name().to_string()),
                Value::Text(payload),
                Value::Int(now),
                Value::Int(now),
            ],
        )?;
        Ok(())
    }
    fn schedule(&self, delay_secs: u64, job: Box<dyn Job>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let available = Self::now() + delay_secs as i64;
        let payload = serde_json::to_string(&serde_json::json!({
            "name": job.name(),
        }))
        .unwrap_or_default();
        let sql = format!(
            "INSERT INTO {} (name, payload, status, created_at, available_at) VALUES (?, ?, 'pending', ?, ?)",
            self.table
        );
        conn.execute(
            &sql,
            &[
                Value::Text(job.name().to_string()),
                Value::Text(payload),
                Value::Int(Self::now()),
                Value::Int(available),
            ],
        )?;
        Ok(())
    }
    fn pop(&self) -> Option<Box<dyn Job>> {
        let stored = self.next_pending()?;
        let id = stored.id;
        let attempts_before = stored.attempts;
        self.mark_running(id).ok()?;
        let job = queue::make_job(&stored.name, &stored.payload)?;
        Some(Box::new(WrappedDbJob {
            inner: job,
            queue_id: id,
            attempts: attempts_before + 1,
            max_tries: self.max_tries,
            retry_delay_secs: self.retry_delay_secs,
            db: self as *const DatabaseQueue,
        }))
    }
}

struct WrappedDbJob {
    inner: Box<dyn Job>,
    queue_id: i64,
    attempts: i64,
    max_tries: u8,
    retry_delay_secs: u64,
    db: *const DatabaseQueue,
}

unsafe impl Send for WrappedDbJob {}
unsafe impl Sync for WrappedDbJob {}

impl fmt::Debug for WrappedDbJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WrappedDbJob").field("id", &self.queue_id).finish()
    }
}

impl Job for WrappedDbJob {
    fn handle(self: Box<Self>) -> Result<(), String> {
        let result = self.inner.handle();
        if let Some(db) = unsafe { self.db.as_ref() } {
            match &result {
                Ok(_) => { let _ = db.mark_done(self.queue_id); }
                Err(e) => {
                    if (self.attempts as u8) < self.max_tries {
                        let _ = db.mark_pending(self.queue_id, self.retry_delay_secs);
                    } else {
                        let _ = db.mark_failed(self.queue_id, e);
                    }
                }
            }
        }
        result
    }
    fn name(&self) -> &str { self.inner.name() }
}

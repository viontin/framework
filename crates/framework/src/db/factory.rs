//! Model factories and database seeders.
//!
//! Factories generate test/model data. Seeders populate the database.

use std::fmt;
use crate::db::{Connection, Value};

/// Generate model instances for testing and seeding.
pub trait Factory<T>: fmt::Debug + Send + Sync {
    fn model_name(&self) -> &str;
    fn definition(&self) -> Vec<(&str, Value)>;

    /// Create a single instance without persisting.
    fn make(&self) -> Vec<(&str, Value)> {
        self.definition()
    }

    /// Create multiple instances without persisting.
    fn make_many(&self, count: usize) -> Vec<Vec<(&str, Value)>> {
        (0..count).map(|_| self.make()).collect()
    }

    /// Create and persist a single instance to the database.
    fn create(&self, conn: &dyn Connection) -> Result<i64, String> {
        use viontin_orm::QueryBuilder;
        let data = self.make();
        QueryBuilder::table(conn, self.model_name()).insert(data)
    }

    /// Create and persist multiple instances.
    fn create_many(&self, conn: &dyn Connection, count: usize) -> Result<Vec<i64>, String> {
        let mut ids = Vec::new();
        for _ in 0..count {
            ids.push(self.create(conn)?);
        }
        Ok(ids)
    }

    /// Override to provide stateful data (e.g., unique emails).
    fn make_with_state(&self, _state: &mut u64) -> Vec<(&str, Value)> {
        self.make()
    }
}

/// Seed the database with initial or test data.
pub trait Seeder: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, conn: &dyn Connection) -> Result<(), String>;
}

/// Run a list of seeders in order.
pub fn run_seeders(conn: &dyn Connection, seeders: &[Box<dyn Seeder>]) -> Result<(), String> {
    for seeder in seeders {
        seeder.run(conn)?;
    }
    Ok(())
}

/// Factory helper — generate unique values using a counter.
pub fn unique<F: Fn(u64) -> String>(counter: &mut u64, f: F) -> Value {
    *counter += 1;
    Value::Text(f(*counter))
}

/// Factory helper — pick a random value from a slice.
pub fn random_from<'a, T: Clone>(values: &'a [T], seed: &mut u64) -> T {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let idx = (*seed as usize) % values.len();
    values[idx].clone()
}

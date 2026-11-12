//! For the performance tests: which SQL ran, how long it took, and how SQLite plans it.

use std::sync::Mutex;
use std::time::Duration;

use crate::{Result, Store};

static STATEMENTS: Mutex<Vec<(String, Duration)>> = Mutex::new(Vec::new());

fn record(sql: &str, took: Duration) {
    if let Ok(mut statements) = STATEMENTS.lock() {
        statements.push((sql.to_string(), took));
    }
}

impl Store {
    /// Starts recording every statement this store runs.
    pub fn trace(&mut self) {
        use rusqlite::trace::{TraceEvent, TraceEventCodes};
        let _ = self.connection.trace_v2(
            TraceEventCodes::SQLITE_TRACE_PROFILE,
            Some(|event: TraceEvent<'_>| {
                if let TraceEvent::Profile(statement, took) = event {
                    record(&statement.sql(), took);
                }
            }),
        );
    }

    /// The statements recorded since the last call, in order.
    pub fn take_trace() -> Vec<(String, Duration)> {
        STATEMENTS.lock().map(|mut statements| std::mem::take(&mut *statements)).unwrap_or_default()
    }

    /// SQLite's plan for a statement, one line per step.
    pub fn query_plan(&self, sql: &str) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(3))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

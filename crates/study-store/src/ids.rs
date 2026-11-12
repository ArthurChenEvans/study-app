//! IDs that are unique on every computer, and stamps that say when a record last changed
//! (R109 to R111). Both are made by SQL functions the database's triggers call, so every
//! insert, edit and delete is covered without each save having to remember.
//!
//! - `new_uid()` is a UUIDv7: 48 bits of Unix milliseconds then 74 random bits, so IDs
//!   sort roughly by when they were made.
//! - `stamp_now()` is a hybrid logical clock stamp, `0001730733120000-0000-1a2b3c4d`:
//!   milliseconds, a counter for several changes in one millisecond, and this computer's
//!   ID. Stamps only ever go up on one computer, even if its clock is set back, and two
//!   computers never make the same one.
//! - `device_id()` is this computer's ID, kept in `this_device`.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;
use rusqlite::functions::FunctionFlags;
use rusqlite::{Connection, OptionalExtension};

use crate::Result;

fn wall_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}

/// A UUIDv7 for the millisecond `ms`.
pub fn uid_at(ms: u64) -> String {
    let mut random = rand::rng();
    let rand_a: u16 = random.random::<u16>() & 0x0fff;
    let rand_b: u64 = random.random::<u64>() & 0x3fff_ffff_ffff_ffff;
    let time = ms & 0xffff_ffff_ffff;
    format!(
        "{:08x}-{:04x}-7{:03x}-{:04x}-{:012x}",
        time >> 16,
        time & 0xffff,
        rand_a,
        0x8000 | (rand_b >> 48) as u16,
        rand_b & 0xffff_ffff_ffff
    )
}

pub fn new_uid() -> String {
    uid_at(wall_ms())
}

/// The hybrid logical clock for one computer.
#[derive(Debug)]
pub struct Clock {
    device: String,
    last_ms: u64,
    counter: u16,
}

impl Clock {
    pub fn new(device: impl Into<String>) -> Self {
        Self { device: device.into(), last_ms: 0, counter: 0 }
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    /// A stamp for now, given the wall clock in milliseconds.
    pub fn stamp_at(&mut self, wall: u64) -> String {
        if wall > self.last_ms {
            self.last_ms = wall;
            self.counter = 0;
        } else if self.counter == u16::MAX {
            self.last_ms += 1;
            self.counter = 0;
        } else {
            self.counter += 1;
        }
        format!("{:016}-{:04x}-{}", self.last_ms, self.counter, self.device)
    }

    pub fn stamp(&mut self) -> String {
        self.stamp_at(wall_ms())
    }

    /// Never stamp below one already made, such as the newest in a database just opened
    /// or a backup just restored.
    pub fn catch_up(&mut self, seen: &str) {
        let mut parts = seen.splitn(3, '-');
        let (Some(Ok(ms)), Some(Ok(counter))) = (parts.next().map(str::parse::<u64>), parts.next().map(|hex| u16::from_str_radix(hex, 16))) else { return };
        if (ms, counter) > (self.last_ms, self.counter) {
            self.last_ms = ms;
            self.counter = counter;
        }
    }
}

pub type SharedClock = Arc<Mutex<Clock>>;

/// Tables whose records carry a `uid` and a `changed` stamp.
pub const STAMPED: &[&str] = &[
    "modules", "topics", "questions", "sources", "attempts", "images", "recall_checklists", "recall_checks", "notes", "cards",
    "card_reviews", "projects", "project_sessions", "issues", "drills", "drill_runs", "struggles", "dates",
];

/// This computer's ID if the database already has one, otherwise a new one.
fn device_of(connection: &Connection) -> Result<String> {
    let has_table: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'this_device'", [], |row| row.get(0))?;
    if has_table == 1
        && let Some(id) = connection.query_row("SELECT device FROM this_device", [], |row| row.get::<_, String>(0)).optional()?
    {
        return Ok(id);
    }
    Ok(format!("{:08x}", rand::rng().random::<u32>()))
}

/// Registers `new_uid()`, `stamp_now()` and `device_id()` on the connection. Call before
/// migrating, since migration 8 uses them to fill in records that are already there.
pub fn register(connection: &Connection) -> Result<SharedClock> {
    let clock: SharedClock = Arc::new(Mutex::new(Clock::new(device_of(connection)?)));
    connection.create_scalar_function("new_uid", 0, FunctionFlags::SQLITE_UTF8, |_| Ok(new_uid()))?;
    let stamps = Arc::clone(&clock);
    connection.create_scalar_function("stamp_now", 0, FunctionFlags::SQLITE_UTF8, move |_| Ok(stamps.lock().map(|mut clock| clock.stamp()).unwrap_or_default()))?;
    let device = clock.lock().map(|clock| clock.device().to_string()).unwrap_or_default();
    connection.create_scalar_function("device_id", 0, FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC, move |_| Ok(device.clone()))?;
    Ok(clock)
}

/// Moves the clock past every stamp already in the database.
pub fn catch_up(connection: &Connection, clock: &SharedClock) -> Result<()> {
    let has_stamps: i64 = connection.query_row("SELECT COUNT(*) FROM pragma_table_info('topics') WHERE name = 'changed'", [], |row| row.get(0))?;
    if has_stamps == 0 {
        return Ok(());
    }
    let tables: Vec<String> = STAMPED.iter().map(|table| format!("SELECT MAX(changed) AS stamp FROM {table}")).chain(["SELECT MAX(at) FROM deleted".into(), "SELECT MAX(changed) FROM settings".into()]).collect();
    let newest: Option<String> = connection.query_row(&format!("SELECT MAX(stamp) FROM ({})", tables.join(" UNION ALL ")), [], |row| row.get(0))?;
    if let (Some(newest), Ok(mut clock)) = (newest, clock.lock()) {
        clock.catch_up(&newest);
    }
    Ok(())
}

/// A record's ID and when it last changed (R109, R110).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Stamp {
    pub uid: String,
    pub changed: String,
}

/// A record that was deleted, so other computers can delete it too (R111).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Deletion {
    pub uid: String,
    pub kind: String,
    pub at: String,
}

impl crate::Store {
    /// The ID and stamp of record `id` in `table`, one of [`STAMPED`].
    pub fn stamp_of(&self, table: &str, id: i64) -> Result<Stamp> {
        if !STAMPED.contains(&table) {
            return Err(crate::StoreError::Invalid("that kind of record has no ID"));
        }
        self.connection
            .query_row(&format!("SELECT uid, changed FROM {table} WHERE id = ?1"), [id], |row| Ok(Stamp { uid: row.get(0)?, changed: row.get(1)? }))
            .optional()?
            .ok_or(crate::StoreError::NotFound("record"))
    }

    /// Every deletion marker, oldest first.
    pub fn deletions(&self) -> Result<Vec<Deletion>> {
        let mut statement = self.connection.prepare("SELECT uid, kind, at FROM deleted ORDER BY at")?;
        let rows = statement.query_map([], |row| Ok(Deletion { uid: row.get(0)?, kind: row.get(1)?, at: row.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The database's tables, for checks that every one is covered.
    pub fn table_names(&self) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn columns_of(&self, table: &str) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare("SELECT name FROM pragma_table_info(?1)")?;
        let rows = statement.query_map([table], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// When a setting last changed, if it's set (R110).
    pub fn setting_changed(&self, key: &str) -> Result<Option<String>> {
        Ok(self.connection.query_row("SELECT changed FROM settings WHERE key = ?1", [key], |row| row.get(0)).optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_v7(uid: &str) -> bool {
        let parts: Vec<&str> = uid.split('-').collect();
        parts.iter().map(|part| part.len()).collect::<Vec<_>>() == [8, 4, 4, 4, 12]
            && uid.chars().all(|letter| letter == '-' || letter.is_ascii_hexdigit())
            && parts[2].starts_with('7')
            && matches!(parts[3].chars().next(), Some('8' | '9' | 'a' | 'b'))
    }

    // R109
    #[test]
    fn uids_are_version_7_unique_and_sort_by_time() {
        let earlier = uid_at(1_730_733_120_000);
        let later = uid_at(1_730_733_120_001);
        assert!(is_v7(&earlier) && is_v7(&later), "{earlier} {later}");
        assert!(earlier < later);
        assert_eq!(&earlier[..13], "0192f7bb-9a00", "the time is in the first 48 bits");
        let many: std::collections::HashSet<String> = (0..10_000).map(|_| new_uid()).collect();
        assert_eq!(many.len(), 10_000);
    }

    // R110
    #[test]
    fn stamps_only_go_up_even_when_the_clock_goes_back() {
        let mut clock = Clock::new("1a2b3c4d");
        let first = clock.stamp_at(5_000);
        let same_ms = clock.stamp_at(5_000);
        let clock_set_back = clock.stamp_at(1_000);
        let later = clock.stamp_at(9_000);
        assert_eq!(first, "0000000000005000-0000-1a2b3c4d");
        assert!(first < same_ms && same_ms < clock_set_back && clock_set_back < later, "{first} {same_ms} {clock_set_back} {later}");
        let mut previous = later.clone();
        for _ in 0..70_000 {
            let next = clock.stamp_at(9_000);
            assert!(next > previous, "{next} after {previous}");
            previous = next;
        }
    }

    #[test]
    fn a_clock_catches_up_with_stamps_it_has_seen() {
        let mut clock = Clock::new("aaaa0000");
        clock.catch_up("0000000000009000-0003-bbbb1111");
        assert_eq!(clock.stamp_at(1_000), "0000000000009000-0004-aaaa0000");
        clock.catch_up("not a stamp");
        clock.catch_up("0000000000002000-0000-bbbb1111");
        assert_eq!(clock.stamp_at(1_000), "0000000000009000-0005-aaaa0000", "an older stamp changes nothing");
    }
}

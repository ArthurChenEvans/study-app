//! Copying the database out and putting a copy back (R101, R105).

use std::path::Path;

use rusqlite::{Connection, MAIN_DB, OpenFlags};
use serde::Serialize;

use crate::{Result, Store, StoreError, schema};

/// How much is in a database, to say what a restore would undo (R105).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub topics: i64,
    pub questions: i64,
    pub notes: i64,
    pub card_reviews: i64,
    pub quizzes: i64,
    pub recall_checks: i64,
    pub drill_runs: i64,
    pub struggles: i64,
}

impl Store {
    /// A full, compacted copy at `path`, safe while the app is in use (R101). `path`
    /// must not exist yet.
    pub fn backup_into(&self, path: &Path) -> Result<()> {
        let target = path.to_str().ok_or(StoreError::Invalid("the backup folder's name isn't valid text"))?;
        self.connection.execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }

    /// Replaces this database with the one at `path`, then upgrades it if it's from an
    /// older version (R105).
    pub fn restore_from(&mut self, path: &Path) -> Result<()> {
        let found = schema_version_of(path)?;
        if found > schema::LATEST {
            return Err(StoreError::NewerSchema { found, known: schema::LATEST });
        }
        self.connection.restore(MAIN_DB, path, None::<fn(rusqlite::backup::Progress)>)?;
        self.connection.pragma_update(None, "foreign_keys", false)?;
        schema::migrate(&self.connection)?;
        self.connection.pragma_update(None, "foreign_keys", true)?;
        Ok(())
    }

    pub fn counts(&self) -> Result<Counts> {
        counts_on(&self.connection)
    }

    /// True when the database has anything worth backing up (R102).
    pub fn has_data(&self) -> Result<bool> {
        let counts = self.counts()?;
        Ok(counts.topics + counts.notes + counts.quizzes + counts.struggles > 0)
    }
}

fn read_only(path: &Path) -> Result<Connection> {
    if !path.is_file() {
        return Err(StoreError::NotFound("backup file"));
    }
    Ok(Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?)
}

/// A database at an older schema version, for testing upgrades and the backup before one.
pub fn open_at_version(path: &Path, version: i64) -> Result<Connection> {
    let connection = Connection::open(path)?;
    schema::migrate_to(&connection, version)?;
    Ok(connection)
}

/// The schema version of a database file, without changing it.
pub fn schema_version_of(path: &Path) -> Result<i64> {
    let connection = read_only(path)?;
    // Reading the version alone doesn't touch the file's contents, so check it's really SQLite.
    connection.query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))?;
    Ok(connection.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// What's in a database file, without changing it. Tables an older version didn't have
/// count as empty.
pub fn counts_of(path: &Path) -> Result<Counts> {
    counts_on(&read_only(path)?)
}

fn counts_on(connection: &Connection) -> Result<Counts> {
    let count = |table: &str| -> Result<i64> {
        let exists: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [table], |row| row.get(0))?;
        if exists == 0 {
            return Ok(0);
        }
        Ok(connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))?)
    };
    Ok(Counts {
        topics: count("topics")?,
        questions: count("questions")?,
        notes: count("notes")?,
        card_reviews: count("card_reviews")?,
        quizzes: count("attempts")?,
        recall_checks: count("recall_checks")?,
        drill_runs: count("drill_runs")?,
        struggles: count("struggles")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("study-store-backup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // backup.feature: Restoring keeps a way back (R101, R105)
    #[test]
    fn a_backup_copies_everything_and_restores_over_later_changes() {
        let dir = scratch("roundtrip");
        let mut store = Store::in_memory().unwrap();
        store.create_topic("Caches", None).unwrap();
        assert!(store.has_data().unwrap());
        let file = dir.join("copy.sqlite");
        store.backup_into(&file).unwrap();
        assert_eq!(counts_of(&file).unwrap().topics, 1);
        assert_eq!(schema_version_of(&file).unwrap(), schema::LATEST);

        store.create_topic("Pipelining", None).unwrap();
        assert_eq!(store.counts().unwrap().topics, 2);
        store.restore_from(&file).unwrap();
        assert_eq!(store.topics().unwrap().iter().map(|topic| topic.name.as_str()).collect::<Vec<_>>(), ["Caches"]);
        // The store still works after a restore
        store.create_topic("Branch prediction", None).unwrap();
        assert_eq!(store.counts().unwrap().topics, 2);
    }

    #[test]
    fn an_empty_database_has_no_data_and_backing_up_onto_an_existing_file_fails() {
        let dir = scratch("empty");
        let store = Store::in_memory().unwrap();
        assert!(!store.has_data().unwrap());
        let file = dir.join("copy.sqlite");
        store.backup_into(&file).unwrap();
        assert!(store.backup_into(&file).is_err(), "never overwrites a backup");
    }

    // backup.feature: Restoring a backup from an older version (R105)
    #[test]
    fn restoring_an_older_backup_upgrades_it() {
        let dir = scratch("older");
        let file = dir.join("old.sqlite");
        {
            let connection = Connection::open(&file).unwrap();
            schema::migrate_to(&connection, 4).unwrap();
            connection.execute("INSERT INTO topics (name) VALUES ('From long ago')", []).unwrap();
        }
        assert_eq!(schema_version_of(&file).unwrap(), 4);
        let before = counts_of(&file).unwrap();
        assert_eq!((before.topics, before.drill_runs), (1, 0), "tables it didn't have yet count as empty");
        let mut store = Store::in_memory().unwrap();
        store.restore_from(&file).unwrap();
        assert_eq!(store.schema_version().unwrap(), schema::LATEST);
        assert_eq!(store.topics().unwrap()[0].name, "From long ago");
        assert_eq!(schema_version_of(&file).unwrap(), 4, "the backup itself is left as it was");
    }

    #[test]
    fn a_file_that_isnt_a_database_is_refused_and_nothing_changes() {
        let dir = scratch("junk");
        let file = dir.join("junk.sqlite");
        std::fs::write(&file, b"not a database at all, just some text that is long enough").unwrap();
        let mut store = Store::in_memory().unwrap();
        store.create_topic("Keep me", None).unwrap();
        assert!(store.restore_from(&file).is_err());
        assert!(store.restore_from(&dir.join("missing.sqlite")).is_err());
        assert_eq!(store.counts().unwrap().topics, 1);
    }
}

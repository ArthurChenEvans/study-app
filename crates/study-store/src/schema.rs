//! Schema migrations, tracked with SQLite's `user_version`.

use rusqlite::Connection;

use crate::{Result, StoreError};

/// Each entry moves the schema up one version. Never edit a shipped entry, add a new one.
const MIGRATIONS: &[&str] = &[
    // 1: first schema
    "
    CREATE TABLE modules (
        id   INTEGER PRIMARY KEY,
        name TEXT NOT NULL
    );
    CREATE TABLE topics (
        id        INTEGER PRIMARY KEY,
        name      TEXT NOT NULL,
        module_id INTEGER REFERENCES modules(id)
    );
    CREATE TABLE topic_tags (
        topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
        tag      TEXT NOT NULL,
        PRIMARY KEY (topic_id, tag)
    );
    CREATE TABLE questions (
        id          INTEGER PRIMARY KEY,
        topic_id    INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
        kind        TEXT NOT NULL CHECK (kind IN ('single', 'multiple')),
        text        TEXT NOT NULL,
        explanation TEXT NOT NULL,
        edited_at   TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE options (
        question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
        position    INTEGER NOT NULL,
        text        TEXT NOT NULL,
        correct     INTEGER NOT NULL CHECK (correct IN (0, 1)),
        PRIMARY KEY (question_id, position)
    );
    CREATE TABLE question_tags (
        question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
        tag         TEXT NOT NULL,
        PRIMARY KEY (question_id, tag)
    );
    CREATE TABLE sources (
        id          INTEGER PRIMARY KEY,
        authors     TEXT NOT NULL,
        year        TEXT,
        title       TEXT NOT NULL,
        edition     TEXT,
        description TEXT,
        publisher   TEXT,
        url         TEXT
    );
    CREATE TABLE citations (
        question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
        position    INTEGER NOT NULL,
        source_id   INTEGER NOT NULL REFERENCES sources(id),
        location    TEXT NOT NULL,
        PRIMARY KEY (question_id, position)
    );
    -- History is self-contained snapshots, no foreign keys, so deletes never touch it (R28, R30).
    CREATE TABLE attempts (
        id       INTEGER PRIMARY KEY,
        day      INTEGER NOT NULL,
        status   TEXT NOT NULL,
        retry_of INTEGER,
        attempt  TEXT NOT NULL,
        reasons  TEXT NOT NULL
    );
    CREATE INDEX questions_by_topic ON questions(topic_id);
    CREATE INDEX citations_by_source ON citations(source_id);
    ",
];

pub const LATEST: i64 = MIGRATIONS.len() as i64;

pub fn migrate(connection: &Connection) -> Result<()> {
    let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if found > LATEST {
        return Err(StoreError::NewerSchema {
            found,
            known: LATEST,
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", index as i64 + 1)?;
        transaction.commit()?;
    }
    Ok(())
}

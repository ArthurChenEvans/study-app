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
    // 2: images pasted or picked into questions, options and explanations (R7).
    // Markdown points at them as image:<id>, so they live in the same file as the questions.
    "
    CREATE TABLE images (
        id     INTEGER PRIMARY KEY,
        format TEXT NOT NULL,
        bytes  BLOB NOT NULL
    );
    ",
    // 3: free recall checklists (R52 to R60). The subject's name is kept, so a checklist and
    // its history survive the module or topic being deleted. Checks are snapshots, like attempts.
    "
    CREATE TABLE recall_checklists (
        id           INTEGER PRIMARY KEY,
        module_id    INTEGER REFERENCES modules(id) ON DELETE SET NULL,
        topic_id     INTEGER REFERENCES topics(id) ON DELETE SET NULL,
        subject_name TEXT NOT NULL,
        subject_kind TEXT NOT NULL CHECK (subject_kind IN ('Module', 'Topic')),
        created      INTEGER NOT NULL,
        step         INTEGER,
        rep          INTEGER,
        due          INTEGER NOT NULL
    );
    CREATE TABLE recall_points (
        checklist_id INTEGER NOT NULL REFERENCES recall_checklists(id) ON DELETE CASCADE,
        position     INTEGER NOT NULL,
        text         TEXT NOT NULL,
        question_id  INTEGER REFERENCES questions(id) ON DELETE SET NULL,
        PRIMARY KEY (checklist_id, position)
    );
    CREATE TABLE recall_checks (
        id           INTEGER PRIMARY KEY,
        checklist_id INTEGER NOT NULL REFERENCES recall_checklists(id) ON DELETE CASCADE,
        day          INTEGER NOT NULL,
        dump         TEXT,
        points       TEXT NOT NULL,
        score        INTEGER NOT NULL,
        passed       INTEGER NOT NULL,
        from_label   TEXT,
        to_label     TEXT NOT NULL
    );
    CREATE INDEX recall_checks_by_checklist ON recall_checks(checklist_id, day);
    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    ",
    // 4: flashcards (R62 to R77). A note makes cards, each with its own FSRS state.
    // Reviews are a log for the optimiser and the Cards screen.
    "
    CREATE TABLE notes (
        id       INTEGER PRIMARY KEY,
        topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
        note     TEXT NOT NULL,
        source   TEXT,
        created  INTEGER NOT NULL
    );
    CREATE TABLE cards (
        id          INTEGER PRIMARY KEY,
        note_id     INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
        key         TEXT NOT NULL,
        front       TEXT NOT NULL,
        back        TEXT NOT NULL,
        stability   REAL,
        difficulty  REAL,
        due         INTEGER NOT NULL,
        last_review INTEGER,
        reps        INTEGER NOT NULL DEFAULT 0,
        lapses      INTEGER NOT NULL DEFAULT 0,
        archived    INTEGER NOT NULL DEFAULT 0,
        UNIQUE (note_id, key)
    );
    CREATE INDEX cards_by_due ON cards(archived, due);
    CREATE TABLE card_reviews (
        id      INTEGER PRIMARY KEY,
        card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
        day     INTEGER NOT NULL,
        rating  INTEGER NOT NULL,
        elapsed INTEGER NOT NULL,
        was_new INTEGER NOT NULL,
        seconds INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX card_reviews_by_day ON card_reviews(day);
    CREATE TABLE card_suggestions_done (
        key    TEXT PRIMARY KEY,
        status TEXT NOT NULL CHECK (status IN ('made', 'dismissed'))
    );
    ",
    // 5: recall points added from flashcards (R54)
    "ALTER TABLE recall_points ADD COLUMN note_id INTEGER REFERENCES notes(id) ON DELETE SET NULL;",
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

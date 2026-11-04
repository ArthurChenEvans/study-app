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
    // 6: short answer questions (R78, R79). SQLite can't change a CHECK, so the table is
    // rebuilt. Migrations run with foreign keys off, so the options, tags, citations and
    // recall points that point at questions are kept.
    "
    CREATE TABLE questions_new (
        id          INTEGER PRIMARY KEY,
        topic_id    INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
        kind        TEXT NOT NULL CHECK (kind IN ('single', 'multiple', 'short')),
        text        TEXT NOT NULL,
        explanation TEXT NOT NULL,
        short       TEXT,
        edited_at   TEXT NOT NULL DEFAULT (datetime('now'))
    );
    INSERT INTO questions_new (id, topic_id, kind, text, explanation, edited_at)
        SELECT id, topic_id, kind, text, explanation, edited_at FROM questions;
    DROP TABLE questions;
    ALTER TABLE questions_new RENAME TO questions;
    CREATE INDEX questions_by_topic ON questions(topic_id);
    ",
    // 7: quiz schedules per topic, projects with sessions and struggles, shared issues and
    // drills (R87 to R96). Quiz moves point at attempts without a foreign key, like history.
    "
    CREATE TABLE topic_schedule (
        topic_id INTEGER PRIMARY KEY REFERENCES topics(id) ON DELETE CASCADE,
        step     INTEGER NOT NULL,
        rep      INTEGER NOT NULL,
        due      INTEGER NOT NULL
    );
    CREATE TABLE quiz_moves (
        attempt_id INTEGER NOT NULL,
        topic_id   INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
        day        INTEGER NOT NULL,
        from_step  INTEGER,
        from_rep   INTEGER,
        from_due   INTEGER,
        score      INTEGER NOT NULL,
        passed     INTEGER NOT NULL,
        to_step    INTEGER NOT NULL,
        to_rep     INTEGER NOT NULL,
        next_due   INTEGER NOT NULL,
        PRIMARY KEY (attempt_id, topic_id)
    );
    CREATE TABLE projects (
        id      INTEGER PRIMARY KEY,
        name    TEXT NOT NULL,
        kind    TEXT NOT NULL DEFAULT '',
        created INTEGER NOT NULL
    );
    CREATE TABLE project_sessions (
        id         INTEGER PRIMARY KEY,
        project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        day        INTEGER NOT NULL,
        started    INTEGER NOT NULL,
        ended      INTEGER
    );
    CREATE TABLE issues (
        id   INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        key  TEXT NOT NULL UNIQUE
    );
    CREATE TABLE drills (
        id       INTEGER PRIMARY KEY,
        issue_id INTEGER REFERENCES issues(id) ON DELETE SET NULL,
        topic_id INTEGER REFERENCES topics(id) ON DELETE SET NULL,
        task     TEXT NOT NULL,
        hints    TEXT NOT NULL DEFAULT '',
        created  INTEGER NOT NULL,
        due      INTEGER
    );
    CREATE TABLE drill_runs (
        id       INTEGER PRIMARY KEY,
        drill_id INTEGER NOT NULL REFERENCES drills(id) ON DELETE CASCADE,
        day      INTEGER NOT NULL,
        rating   TEXT NOT NULL CHECK (rating IN ('Struggled', 'Shaky', 'Solid')),
        seconds  INTEGER NOT NULL,
        note     TEXT NOT NULL DEFAULT ''
    );
    CREATE TABLE struggles (
        id         INTEGER PRIMARY KEY,
        project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        session_id INTEGER REFERENCES project_sessions(id) ON DELETE SET NULL,
        day        INTEGER NOT NULL,
        minute     INTEGER NOT NULL,
        what       TEXT NOT NULL,
        why        TEXT NOT NULL,
        kind       TEXT NOT NULL CHECK (kind IN ('Recall', 'Apply', 'Unsure')),
        topic_id   INTEGER REFERENCES topics(id) ON DELETE SET NULL,
        issue_id   INTEGER REFERENCES issues(id) ON DELETE SET NULL,
        outcome    TEXT
    );
    CREATE INDEX struggles_by_project ON struggles(project_id, day);
    CREATE INDEX drill_runs_by_day ON drill_runs(day);
    ",    // 8: IDs that are the same on every computer, "last changed" stamps and deletion
    // markers, the groundwork for export, import and sync (R109 to R111). Triggers fill
    // them in, calling new_uid(), stamp_now() and device_id() from ids.rs, so every save
    // is covered. A table rebuilt in a later migration must get its triggers back.
    "
    CREATE TABLE this_device (device TEXT NOT NULL);
    INSERT INTO this_device (device) VALUES (device_id());
    CREATE TABLE deleted (
        uid  TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        at   TEXT NOT NULL
    );

    ALTER TABLE modules ADD COLUMN uid TEXT;
    ALTER TABLE modules ADD COLUMN changed TEXT;
    UPDATE modules SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX modules_by_uid ON modules(uid);
    CREATE TRIGGER modules_added AFTER INSERT ON modules WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE modules SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER modules_edited AFTER UPDATE ON modules WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE modules SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER modules_deleted AFTER DELETE ON modules WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'modules', stamp_now());
    END;

    ALTER TABLE topics ADD COLUMN uid TEXT;
    ALTER TABLE topics ADD COLUMN changed TEXT;
    UPDATE topics SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX topics_by_uid ON topics(uid);
    CREATE TRIGGER topics_added AFTER INSERT ON topics WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE topics SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER topics_edited AFTER UPDATE ON topics WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE topics SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER topics_deleted AFTER DELETE ON topics WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'topics', stamp_now());
    END;

    ALTER TABLE questions ADD COLUMN uid TEXT;
    ALTER TABLE questions ADD COLUMN changed TEXT;
    UPDATE questions SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX questions_by_uid ON questions(uid);
    CREATE TRIGGER questions_added AFTER INSERT ON questions WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE questions SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER questions_edited AFTER UPDATE ON questions WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER questions_deleted AFTER DELETE ON questions WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'questions', stamp_now());
    END;

    ALTER TABLE sources ADD COLUMN uid TEXT;
    ALTER TABLE sources ADD COLUMN changed TEXT;
    UPDATE sources SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX sources_by_uid ON sources(uid);
    CREATE TRIGGER sources_added AFTER INSERT ON sources WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE sources SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER sources_edited AFTER UPDATE ON sources WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE sources SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER sources_deleted AFTER DELETE ON sources WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'sources', stamp_now());
    END;

    ALTER TABLE attempts ADD COLUMN uid TEXT;
    ALTER TABLE attempts ADD COLUMN changed TEXT;
    UPDATE attempts SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX attempts_by_uid ON attempts(uid);
    CREATE TRIGGER attempts_added AFTER INSERT ON attempts WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE attempts SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER attempts_edited AFTER UPDATE ON attempts WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE attempts SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER attempts_deleted AFTER DELETE ON attempts WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'attempts', stamp_now());
    END;

    ALTER TABLE images ADD COLUMN uid TEXT;
    ALTER TABLE images ADD COLUMN changed TEXT;
    UPDATE images SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX images_by_uid ON images(uid);
    CREATE TRIGGER images_added AFTER INSERT ON images WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE images SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER images_edited AFTER UPDATE ON images WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE images SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER images_deleted AFTER DELETE ON images WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'images', stamp_now());
    END;

    ALTER TABLE recall_checklists ADD COLUMN uid TEXT;
    ALTER TABLE recall_checklists ADD COLUMN changed TEXT;
    UPDATE recall_checklists SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX recall_checklists_by_uid ON recall_checklists(uid);
    CREATE TRIGGER recall_checklists_added AFTER INSERT ON recall_checklists WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE recall_checklists SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER recall_checklists_edited AFTER UPDATE OF module_id, topic_id, subject_name, subject_kind ON recall_checklists WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE recall_checklists SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER recall_checklists_deleted AFTER DELETE ON recall_checklists WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'recall_checklists', stamp_now());
    END;

    ALTER TABLE recall_checks ADD COLUMN uid TEXT;
    ALTER TABLE recall_checks ADD COLUMN changed TEXT;
    UPDATE recall_checks SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX recall_checks_by_uid ON recall_checks(uid);
    CREATE TRIGGER recall_checks_added AFTER INSERT ON recall_checks WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE recall_checks SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER recall_checks_edited AFTER UPDATE ON recall_checks WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE recall_checks SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER recall_checks_deleted AFTER DELETE ON recall_checks WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'recall_checks', stamp_now());
    END;

    ALTER TABLE notes ADD COLUMN uid TEXT;
    ALTER TABLE notes ADD COLUMN changed TEXT;
    UPDATE notes SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX notes_by_uid ON notes(uid);
    CREATE TRIGGER notes_added AFTER INSERT ON notes WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE notes SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER notes_edited AFTER UPDATE ON notes WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE notes SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER notes_deleted AFTER DELETE ON notes WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'notes', stamp_now());
    END;

    ALTER TABLE cards ADD COLUMN uid TEXT;
    ALTER TABLE cards ADD COLUMN changed TEXT;
    UPDATE cards SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX cards_by_uid ON cards(uid);
    CREATE TRIGGER cards_added AFTER INSERT ON cards WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE cards SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER cards_edited AFTER UPDATE OF note_id, key, front, back, archived ON cards WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE cards SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER cards_deleted AFTER DELETE ON cards WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'cards', stamp_now());
    END;

    ALTER TABLE card_reviews ADD COLUMN uid TEXT;
    ALTER TABLE card_reviews ADD COLUMN changed TEXT;
    UPDATE card_reviews SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX card_reviews_by_uid ON card_reviews(uid);
    CREATE TRIGGER card_reviews_added AFTER INSERT ON card_reviews WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE card_reviews SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER card_reviews_edited AFTER UPDATE ON card_reviews WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE card_reviews SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER card_reviews_deleted AFTER DELETE ON card_reviews WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'card_reviews', stamp_now());
    END;

    ALTER TABLE projects ADD COLUMN uid TEXT;
    ALTER TABLE projects ADD COLUMN changed TEXT;
    UPDATE projects SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX projects_by_uid ON projects(uid);
    CREATE TRIGGER projects_added AFTER INSERT ON projects WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE projects SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER projects_edited AFTER UPDATE ON projects WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE projects SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER projects_deleted AFTER DELETE ON projects WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'projects', stamp_now());
    END;

    ALTER TABLE project_sessions ADD COLUMN uid TEXT;
    ALTER TABLE project_sessions ADD COLUMN changed TEXT;
    UPDATE project_sessions SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX project_sessions_by_uid ON project_sessions(uid);
    CREATE TRIGGER project_sessions_added AFTER INSERT ON project_sessions WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE project_sessions SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER project_sessions_edited AFTER UPDATE ON project_sessions WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE project_sessions SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER project_sessions_deleted AFTER DELETE ON project_sessions WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'project_sessions', stamp_now());
    END;

    ALTER TABLE issues ADD COLUMN uid TEXT;
    ALTER TABLE issues ADD COLUMN changed TEXT;
    UPDATE issues SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX issues_by_uid ON issues(uid);
    CREATE TRIGGER issues_added AFTER INSERT ON issues WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE issues SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER issues_edited AFTER UPDATE ON issues WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE issues SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER issues_deleted AFTER DELETE ON issues WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'issues', stamp_now());
    END;

    ALTER TABLE drills ADD COLUMN uid TEXT;
    ALTER TABLE drills ADD COLUMN changed TEXT;
    UPDATE drills SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX drills_by_uid ON drills(uid);
    CREATE TRIGGER drills_added AFTER INSERT ON drills WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE drills SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER drills_edited AFTER UPDATE ON drills WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE drills SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER drills_deleted AFTER DELETE ON drills WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'drills', stamp_now());
    END;

    ALTER TABLE drill_runs ADD COLUMN uid TEXT;
    ALTER TABLE drill_runs ADD COLUMN changed TEXT;
    UPDATE drill_runs SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX drill_runs_by_uid ON drill_runs(uid);
    CREATE TRIGGER drill_runs_added AFTER INSERT ON drill_runs WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE drill_runs SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER drill_runs_edited AFTER UPDATE ON drill_runs WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE drill_runs SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER drill_runs_deleted AFTER DELETE ON drill_runs WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'drill_runs', stamp_now());
    END;

    ALTER TABLE struggles ADD COLUMN uid TEXT;
    ALTER TABLE struggles ADD COLUMN changed TEXT;
    UPDATE struggles SET uid = new_uid(), changed = stamp_now();
    CREATE UNIQUE INDEX struggles_by_uid ON struggles(uid);
    CREATE TRIGGER struggles_added AFTER INSERT ON struggles WHEN NEW.uid IS NULL OR NEW.changed IS NULL BEGIN
        UPDATE struggles SET uid = coalesce(NEW.uid, new_uid()), changed = coalesce(NEW.changed, stamp_now()) WHERE id = NEW.id;
    END;
    CREATE TRIGGER struggles_edited AFTER UPDATE ON struggles WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE struggles SET changed = stamp_now() WHERE id = NEW.id;
    END;
    CREATE TRIGGER struggles_deleted AFTER DELETE ON struggles WHEN OLD.uid IS NOT NULL BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES (OLD.uid, 'struggles', stamp_now());
    END;

    -- Changing a question's options, tags or sources, a topic's tags or a checklist's points changes the parent.
    CREATE TRIGGER topic_tags_insert_touches AFTER INSERT ON topic_tags BEGIN
        UPDATE topics SET changed = stamp_now() WHERE id = NEW.topic_id;
    END;
    CREATE TRIGGER topic_tags_update_touches AFTER UPDATE ON topic_tags BEGIN
        UPDATE topics SET changed = stamp_now() WHERE id = NEW.topic_id;
    END;
    CREATE TRIGGER topic_tags_delete_touches AFTER DELETE ON topic_tags BEGIN
        UPDATE topics SET changed = stamp_now() WHERE id = OLD.topic_id;
    END;
    CREATE TRIGGER options_insert_touches AFTER INSERT ON options BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.question_id;
    END;
    CREATE TRIGGER options_update_touches AFTER UPDATE ON options BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.question_id;
    END;
    CREATE TRIGGER options_delete_touches AFTER DELETE ON options BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = OLD.question_id;
    END;
    CREATE TRIGGER question_tags_insert_touches AFTER INSERT ON question_tags BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.question_id;
    END;
    CREATE TRIGGER question_tags_update_touches AFTER UPDATE ON question_tags BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.question_id;
    END;
    CREATE TRIGGER question_tags_delete_touches AFTER DELETE ON question_tags BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = OLD.question_id;
    END;
    CREATE TRIGGER citations_insert_touches AFTER INSERT ON citations BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.question_id;
    END;
    CREATE TRIGGER citations_update_touches AFTER UPDATE ON citations BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = NEW.question_id;
    END;
    CREATE TRIGGER citations_delete_touches AFTER DELETE ON citations BEGIN
        UPDATE questions SET changed = stamp_now() WHERE id = OLD.question_id;
    END;
    CREATE TRIGGER recall_points_insert_touches AFTER INSERT ON recall_points BEGIN
        UPDATE recall_checklists SET changed = stamp_now() WHERE id = NEW.checklist_id;
    END;
    CREATE TRIGGER recall_points_update_touches AFTER UPDATE ON recall_points BEGIN
        UPDATE recall_checklists SET changed = stamp_now() WHERE id = NEW.checklist_id;
    END;
    CREATE TRIGGER recall_points_delete_touches AFTER DELETE ON recall_points BEGIN
        UPDATE recall_checklists SET changed = stamp_now() WHERE id = OLD.checklist_id;
    END;

    ALTER TABLE settings ADD COLUMN changed TEXT;
    UPDATE settings SET changed = stamp_now();
    CREATE TRIGGER settings_added AFTER INSERT ON settings WHEN NEW.changed IS NULL BEGIN
        UPDATE settings SET changed = stamp_now() WHERE key = NEW.key;
    END;
    CREATE TRIGGER settings_edited AFTER UPDATE ON settings WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE settings SET changed = stamp_now() WHERE key = NEW.key;
    END;
    CREATE TRIGGER settings_deleted AFTER DELETE ON settings BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES ('settings:' || OLD.key, 'settings', stamp_now());
    END;

    ALTER TABLE card_suggestions_done ADD COLUMN changed TEXT;
    UPDATE card_suggestions_done SET changed = stamp_now();
    CREATE TRIGGER card_suggestions_done_added AFTER INSERT ON card_suggestions_done WHEN NEW.changed IS NULL BEGIN
        UPDATE card_suggestions_done SET changed = stamp_now() WHERE key = NEW.key;
    END;
    CREATE TRIGGER card_suggestions_done_edited AFTER UPDATE ON card_suggestions_done WHEN NEW.changed IS OLD.changed BEGIN
        UPDATE card_suggestions_done SET changed = stamp_now() WHERE key = NEW.key;
    END;
    CREATE TRIGGER card_suggestions_done_deleted AFTER DELETE ON card_suggestions_done BEGIN
        INSERT OR REPLACE INTO deleted (uid, kind, at) VALUES ('card_suggestions_done:' || OLD.key, 'card_suggestions_done', stamp_now());
    END;
    ",
];

pub const LATEST: i64 = MIGRATIONS.len() as i64;

/// Runs every migration not yet applied. Call with foreign keys off, see migration 6.
pub fn migrate(connection: &Connection) -> Result<()> {
    migrate_to(connection, LATEST)
}

pub(crate) fn migrate_to(connection: &Connection, target: i64) -> Result<()> {
    let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if found > LATEST {
        return Err(StoreError::NewerSchema {
            found,
            known: LATEST,
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().take(target as usize).skip(found as usize) {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", index as i64 + 1)?;
        transaction.commit()?;
    }
    Ok(())
}

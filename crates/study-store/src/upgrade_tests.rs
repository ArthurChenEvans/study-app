//! Upgrading from every older schema version, with rows in every table (R102, R105).
//! Rows are made generically from each version's own table definitions, so a new
//! migration is covered without writing fixtures for it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{Store, ids, schema};

pub(crate) fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("study-store-upgrade-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tables(connection: &Connection) -> Vec<String> {
    let mut statement = connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY rowid").unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

pub(crate) fn counts(connection: &Connection) -> BTreeMap<String, i64> {
    tables(connection).into_iter().map(|table| { let count = connection.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| row.get(0)).unwrap(); (table, count) }).collect()
}

/// Puts up to `rows` rows in every table, parents first. Returns the tables it couldn't fill.
pub(crate) fn fill(connection: &Connection, rows: i64) -> Vec<String> {
    let mut skipped = vec![];
    let mut filled: BTreeMap<String, i64> = BTreeMap::new();
    let mut pending = tables(connection);
    // Parents before children: keep going round until nothing more can be filled.
    for _ in 0..6 {
        let mut later = vec![];
        for table in pending {
            let links: Vec<(String, String)> = connection
                .prepare(&format!("SELECT \"from\", \"table\" FROM pragma_foreign_key_list('{table}')"))
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            if links.iter().any(|(_, parent)| parent != &table && !filled.contains_key(parent)) {
                later.push(table);
                continue;
            }
            let columns: Vec<(String, String, bool)> = connection
                .prepare(&format!("SELECT name, type, pk FROM pragma_table_info('{table}')"))
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get::<_, String>(1)?.to_uppercase(), row.get::<_, i64>(2)? > 0)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            let single_integer_key = columns.iter().filter(|(_, _, key)| *key).count() == 1;
            // A column limited to a few words, like kind IN ('single', 'multiple'), gets one of them.
            let definition: String = connection.query_row("SELECT sql FROM sqlite_master WHERE name = ?1", [&table], |row| row.get(0)).unwrap();
            let allowed = |name: &str, n: i64| -> Option<String> {
                let rest = definition.split(&format!("CHECK ({name} IN (")).nth(1)?;
                let words: Vec<&str> = rest.split(')').next()?.split(',').map(str::trim).collect();
                Some(words[n as usize % words.len()].to_string())
            };
            let mut made = 0;
            for n in 1..=rows {
                let mut names = vec![];
                let mut values = vec![];
                for (name, kind, key) in &columns {
                    if *key && single_integer_key && kind.contains("INT") {
                        continue; // let SQLite number it
                    }
                    let parent = links.iter().find(|(from, _)| from == name).map(|(_, parent)| parent);
                    let value = match parent {
                        _ if allowed(name, n).is_some() => allowed(name, n).unwrap(),
                        Some(parent) if parent == &table => "NULL".to_string(),
                        Some(parent) => format!("{}", (n - 1) % filled[parent].max(1) + 1),
                        None if name == "uid" || name == "changed" => "NULL".to_string(),
                        None if kind.contains("INT") || kind.contains("REAL") => format!("{}", if name.contains("day") || name == "due" { 20_000 + n } else { n % 2 }),
                        None if kind.contains("BLOB") => "x'89504e47'".to_string(),
                        // JSON columns hold whatever the app wrote. An object reads as valid JSON either way.
                        None if ["attempt", "reasons", "points", "value", "short", "settings", "fields", "covers"].contains(&name.as_str()) => "'{}'".to_string(),
                        None => format!("'{name} {n}'"),
                    };
                    names.push(format!("\"{name}\""));
                    values.push(value);
                }
                let sql = if names.is_empty() { format!("INSERT INTO \"{table}\" DEFAULT VALUES") } else { format!("INSERT INTO \"{table}\" ({}) VALUES ({})", names.join(", "), values.join(", ")) };
                if connection.execute(&sql, []).is_ok() {
                    made += 1;
                }
            }
            if made == 0 {
                skipped.push(table.clone());
            }
            filled.insert(table, made);
        }
        pending = later;
    }
    skipped.extend(pending);
    skipped
}

/// A database file at `version` with rows in every table that could be filled.
pub(crate) fn old_database(path: &Path, version: i64, rows: i64) -> (BTreeMap<String, i64>, Vec<String>) {
    let connection = Connection::open(path).unwrap();
    ids::register(&connection).unwrap();
    schema::migrate_to(&connection, version).unwrap();
    connection.pragma_update(None, "foreign_keys", true).unwrap();
    let skipped = fill(&connection, rows);
    (counts(&connection), skipped)
}

pub(crate) fn sound(path: &Path) -> Result<(), String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    if integrity != "ok" {
        return Err(format!("integrity_check: {integrity}"));
    }
    let broken: i64 = connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    if broken > 0 {
        return Err(format!("{broken} broken links between tables"));
    }
    Ok(())
}

#[test]
fn every_older_version_upgrades_with_its_rows_intact() {
    let dir = scratch("every");
    let mut report = vec![];
    for version in 1..schema::LATEST {
        let path = dir.join(format!("v{version}.sqlite"));
        let (before, skipped) = old_database(&path, version, 5);
        assert!(before.values().sum::<i64>() > 0, "version {version} got no rows at all");
        match Store::open(&path) {
            Ok(store) => {
                assert_eq!(store.schema_version().unwrap(), schema::LATEST);
                drop(store);
                if let Err(problem) = sound(&path) {
                    report.push(format!("v{version}: {problem}"));
                }
                let after = counts(&Connection::open(&path).unwrap());
                for (table, count) in &before {
                    match after.get(table) {
                        Some(now) if now < count => report.push(format!("v{version}: {table} went from {count} rows to {now}")),
                        _ => {}
                    }
                }
            }
            Err(error) => report.push(format!("v{version}: didn't open after upgrade: {error}")),
        }
        if !skipped.is_empty() && std::env::var("UPGRADE_VERBOSE").is_ok() {
            println!("v{version}: no rows made for {skipped:?}");
        }
    }
    assert!(report.is_empty(), "{report:#?}");
}

/// Writes a bigger database at every older version into `OLD_DATABASES`, for
/// scripts/kill-tests.py to kill the app while it upgrades them.
#[test]
#[ignore]
fn write_old_databases_for_the_kill_test() {
    let Ok(dir) = std::env::var("OLD_DATABASES") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    for version in 1..schema::LATEST {
        let path = Path::new(&dir).join(format!("v{version}.sqlite"));
        let _ = std::fs::remove_file(&path);
        old_database(&path, version, std::env::var("OLD_ROWS").ok().and_then(|rows| rows.parse().ok()).unwrap_or(1500));
    }
}

// Store::health is what the read-back check after an import, sync or restore rests on.
#[test]
fn health_is_empty_for_a_sound_database_and_names_rows_that_point_nowhere() {
    let dir = scratch("health");
    let path = dir.join("study.sqlite");
    let mut store = Store::open(&path).unwrap();
    let topic = store.create_topic("Caches", None).unwrap();
    assert_eq!(store.health().unwrap(), Vec::<String>::new());
    // A row whose topic doesn't exist, as a damaged file could leave behind.
    store.connection.pragma_update(None, "foreign_keys", false).unwrap();
    store.connection.execute("INSERT INTO topic_tags (topic_id, tag) VALUES (?1, 'orphan')", [topic.0 + 999]).unwrap();
    store.connection.pragma_update(None, "foreign_keys", true).unwrap();
    let problems = store.health().unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("topic_tags") && problems[0].contains("topics"), "{problems:?}");
}

#[test]
fn a_trial_keeps_or_undoes_everything_written_inside_it() {
    let mut store = Store::in_memory().unwrap();
    store.begin_trial().unwrap();
    store.create_topic("Undone", None).unwrap();
    store.end_trial(false).unwrap();
    assert_eq!(store.topics().unwrap().len(), 0, "undone");
    store.begin_trial().unwrap();
    store.create_topic("Kept", None).unwrap();
    store.end_trial(true).unwrap();
    assert_eq!(store.topics().unwrap().len(), 1, "kept");
    // And the store works as normal afterwards, outside any trial.
    store.create_topic("After", None).unwrap();
    assert_eq!(store.topics().unwrap().len(), 2);
}

#[test]
fn a_running_quiz_is_saved_replaced_and_forgotten() {
    use study_core::assembly::QuizSettings;
    use study_core::attempt::Attempt;
    let mut store = Store::in_memory().unwrap();
    assert_eq!(store.running_quiz().unwrap(), None);
    let topic = store.create_topic("Caches", None).unwrap();
    let attempt = Attempt::start(QuizSettings::for_topics(vec![topic]), vec![], |_| unreachable!("no questions"));
    store.set_running_quiz(Some(&attempt)).unwrap();
    let saved = store.running_quiz().unwrap().expect("saved");
    assert_eq!(serde_json::from_str::<Attempt>(&saved).unwrap(), attempt);
    store.set_running_quiz(Some(&attempt)).unwrap();
    assert!(store.running_quiz().unwrap().is_some(), "saving again replaces it");
    store.set_running_quiz(None).unwrap();
    assert_eq!(store.running_quiz().unwrap(), None);
    // Finishing saves it to history and forgets the running copy together.
    store.set_running_quiz(Some(&attempt)).unwrap();
    let id = store.save_finished_quiz(&attempt, 20_000).unwrap();
    assert_eq!(store.running_quiz().unwrap(), None);
    assert_eq!(store.attempt(id).unwrap().0, attempt);
}

//! Upgrading to nested modules (R217): nested-modules.feature, "Existing flat modules upgrade
//! without changes" and "Existing topics with no module go to Default".
//!
//! Test IDs: specs/tests/nested-modules-and-search.md, T-MIG-01 to T-MIG-03. The new schema
//! gives `modules` a `parent_id` (NULL for a root) and an `is_default` flag.

use rusqlite::Connection;

use crate::upgrade_tests::scratch;
use crate::{Store, ids, schema};

/// The last schema before nesting. Every database at or below it upgrades through these tests.
const PRE_NESTING: i64 = 10;

/// A database at the last flat schema: two modules, a topic in one, and a topic in none.
fn flat_database(name: &str) -> std::path::PathBuf {
    assert!(schema::LATEST >= PRE_NESTING);
    let path = scratch(name).join("study.db");
    let connection = Connection::open(&path).unwrap();
    ids::register(&connection).unwrap();
    schema::migrate_to(&connection, PRE_NESTING).unwrap();
    connection
        .execute_batch(
            "INSERT INTO modules (id, name) VALUES (1, 'Computer Architecture'), (2, 'Networks');
             INSERT INTO topics (id, name, module_id) VALUES (10, 'CPU', 1), (11, 'Loose', NULL);",
        )
        .unwrap();
    path
}

fn modules(connection: &Connection) -> Vec<(i64, String, Option<i64>, bool)> {
    let mut statement = connection.prepare("SELECT id, name, parent_id, is_default FROM modules ORDER BY id").unwrap();
    statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap().map(Result::unwrap).collect()
}

fn topic_module(connection: &Connection, topic: i64) -> Option<i64> {
    connection.query_row("SELECT module_id FROM topics WHERE id = ?1", [topic], |row| row.get(0)).unwrap()
}

/// T-MIG-01. Old modules become roots, their topics unchanged.
#[test]
fn t_mig_01_flat_modules_become_roots() {
    let path = flat_database("mig-roots");
    drop(Store::open(&path).unwrap());
    let connection = Connection::open(&path).unwrap();
    let found = modules(&connection);
    assert!(found.contains(&(1, "Computer Architecture".into(), None, false)));
    assert!(found.contains(&(2, "Networks".into(), None, false)));
    assert_eq!(topic_module(&connection, 10), Some(1));
}

/// T-MIG-02. A topic with no module goes to Default, made by the upgrade.
#[test]
fn t_mig_02_topics_without_a_module_go_to_default() {
    let path = flat_database("mig-default");
    drop(Store::open(&path).unwrap());
    let connection = Connection::open(&path).unwrap();
    let defaults: Vec<_> = modules(&connection).into_iter().filter(|module| module.3).collect();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0].1, "Default");
    assert_eq!(defaults[0].2, None, "Default is a root");
    assert_eq!(topic_module(&connection, 11), Some(defaults[0].0));
}

/// T-MIG-02. With every topic already in a module, no Default is made.
#[test]
fn t_mig_02_no_default_when_nothing_needs_it() {
    let path = flat_database("mig-no-default");
    {
        let connection = Connection::open(&path).unwrap();
        ids::register(&connection).unwrap();
        connection.execute("DELETE FROM topics WHERE id = 11", []).unwrap();
    }
    drop(Store::open(&path).unwrap());
    let connection = Connection::open(&path).unwrap();
    assert!(modules(&connection).iter().all(|module| !module.3));
}

/// T-MIG-03. Opening an upgraded database again changes nothing.
#[test]
fn t_mig_03_upgrading_twice_is_harmless() {
    let path = flat_database("mig-twice");
    drop(Store::open(&path).unwrap());
    let first = modules(&Connection::open(&path).unwrap());
    drop(Store::open(&path).unwrap());
    let second = modules(&Connection::open(&path).unwrap());
    assert_eq!(first, second);
}

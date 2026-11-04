//! ids.feature through the command layer (R109 to R111): every record gets an ID that's
//! the same on every computer, editing it moves its "last changed" stamp, and deleting it
//! leaves a marker. Driven through the same App functions the screens use.

use super::*;
use crate::cards::{NoteInput, ReviewStep};
use study_core::cards::{Note, NoteKind, Rating};
use study_store::ids::{STAMPED, Stamp};

const TODAY: i64 = 20_761;

fn app() -> App {
    App::with_seed(Store::in_memory().unwrap(), 7)
}

fn mcq(topic: TopicId, text: &str, wrong: &str) -> QuestionInput {
    QuestionInput {
        topic,
        text: text.into(),
        multiple: false,
        options: ["Right", wrong, "Also wrong"].iter().enumerate().map(|(i, t)| OptionInput { text: (*t).into(), correct: i == 0 }).collect(),
        explanation: String::new(),
        tags: vec![],
        citations: vec![],
        short: None,
    }
}

fn note(topic: TopicId, front: &str) -> NoteInput {
    NoteInput { note: Note { topic, kind: NoteKind::Basic, front: front.into(), back: "Back".into(), cues: vec![], text: String::new(), extra: String::new() }, suggestion: None }
}

fn stamp(app: &App, table: &str, id: i64) -> Stamp {
    app.store.stamp_of(table, id).unwrap()
}

fn is_uid(uid: &str) -> bool {
    uid.len() == 36 && uid.as_bytes()[14] == b'7' && uid.split('-').count() == 5
}

// ids.feature: Every record gets an ID (R109)
#[test]
fn new_records_get_their_own_id_and_a_stamp_from_this_computer() {
    let mut app = app();
    let module = app.create_module("Networks").unwrap();
    let topic = app.create_topic("OSI model", Some(module)).unwrap();
    let first = app.save_question(mcq(topic, "Which layer routes?", "Session")).unwrap();
    let second = app.save_question(mcq(topic, "Which layer frames?", "Session")).unwrap();
    let project = app.create_project("Study app", "Coding", TODAY).unwrap();
    let device = app.store.device_id();
    assert_eq!(device.len(), 8);
    let stamps = [stamp(&app, "modules", module.0), stamp(&app, "topics", topic.0), stamp(&app, "questions", first.0), stamp(&app, "questions", second.0), stamp(&app, "projects", project.0)];
    for Stamp { uid, changed } in &stamps {
        assert!(is_uid(uid), "{uid}");
        assert!(changed.ends_with(&device), "{changed}");
    }
    let uids: std::collections::HashSet<&String> = stamps.iter().map(|stamp| &stamp.uid).collect();
    assert_eq!(uids.len(), stamps.len());
    assert!(stamps[2].changed < stamps[3].changed, "made later, stamped later");
}

// ids.feature: Editing moves the stamp, including a question's options and tags (R110)
#[test]
fn editing_a_record_or_its_parts_moves_its_stamp() {
    let mut app = app();
    let module = app.create_module("Networks").unwrap();
    let topic = app.create_topic("OSI model", None).unwrap();
    let question = app.save_question(mcq(topic, "Which layer routes?", "Session")).unwrap();

    let before = stamp(&app, "topics", topic.0);
    app.move_topic(topic, Some(module)).unwrap();
    let moved = stamp(&app, "topics", topic.0);
    assert!(moved.changed > before.changed);
    assert_eq!(moved.uid, before.uid, "an edit keeps the ID");
    app.set_topic_tags(topic, vec!["week 1".into()]).unwrap();
    assert!(stamp(&app, "topics", topic.0).changed > moved.changed, "tags are part of the topic");

    let before = stamp(&app, "questions", question.0);
    app.update_question(question, mcq(topic, "Which layer routes?", "Transport")).unwrap();
    let after = stamp(&app, "questions", question.0);
    assert!(after.changed > before.changed);
    assert_eq!(after.uid, before.uid);
}

// ids.feature: Reviewing a card isn't an edit, archiving it is (R110)
#[test]
fn reviewing_a_card_leaves_its_stamp_and_archiving_moves_it() {
    let mut app = app();
    let topic = app.create_topic("OSI model", None).unwrap();
    app.save_note(None, note(topic, "Which layer routes packets?"), TODAY).unwrap();
    let ReviewStep::Card(card) = app.start_review(None, TODAY).unwrap() else { panic!("a card to review") };
    let before = stamp(&app, "cards", card.card.0);
    app.grade_card(Rating::Good, 5, TODAY).unwrap();
    assert_eq!(stamp(&app, "cards", card.card.0), before, "its schedule comes from the review history");
    let reviews = app.store.reviews_since(TODAY).unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(app.store.cards().unwrap()[0].id, card.card);
    app.archive_card(card.card, true).unwrap();
    assert!(stamp(&app, "cards", card.card.0).changed > before.changed);
}

// ids.feature: Deleting leaves a marker, for everything deleted along with it (R111)
#[test]
fn deleting_a_topic_leaves_markers_for_it_and_everything_in_it() {
    let mut app = app();
    let topic = app.create_topic("OSI model", None).unwrap();
    let kept = app.create_topic("TCP/IP", None).unwrap();
    let question = app.save_question(mcq(topic, "Which layer routes?", "Session")).unwrap();
    let note_id = app.save_note(None, note(topic, "Which layer routes packets?"), TODAY).unwrap();
    let card = app.store.cards_of(note_id).unwrap()[0].id;
    let uids = [stamp(&app, "topics", topic.0).uid, stamp(&app, "questions", question.0).uid, stamp(&app, "notes", note_id.0).uid, stamp(&app, "cards", card.0).uid];
    let kept_uid = stamp(&app, "topics", kept.0).uid;

    app.delete_topic(topic).unwrap();
    let markers = app.store.deletions().unwrap();
    let kinds: Vec<(&str, &str)> = markers.iter().map(|marker| (marker.uid.as_str(), marker.kind.as_str())).collect();
    for (uid, kind) in uids.iter().zip(["topics", "questions", "notes", "cards"]) {
        assert!(kinds.contains(&(uid.as_str(), kind)), "{kind} {uid} in {kinds:?}");
    }
    assert!(!markers.iter().any(|marker| marker.uid == kept_uid));
    assert!(markers.iter().all(|marker| marker.at.ends_with(&app.store.device_id())));
}

// ids.feature: Settings are stamped too (R110, R111)
#[test]
fn a_setting_is_stamped_when_set_and_leaves_a_marker_when_reset() {
    let mut app = app();
    let schedule = app.quiz_schedule().unwrap();
    assert_eq!(app.store.setting_changed("quiz_schedule").unwrap(), None);
    app.set_quiz_schedule(schedule.clone()).unwrap();
    let first = app.store.setting_changed("quiz_schedule").unwrap().expect("stamped");
    app.set_quiz_schedule(schedule).unwrap();
    assert!(app.store.setting_changed("quiz_schedule").unwrap().unwrap() > first);
    app.reset_quiz_schedule().unwrap();
    assert!(app.store.deletions().unwrap().iter().any(|marker| marker.uid == "settings:quiz_schedule" && marker.kind == "settings"));
}

// ids.feature: Records from before the update get IDs too (R109)
#[test]
fn opening_an_older_database_gives_every_record_an_id() {
    let dir = std::env::temp_dir().join(format!("study-ids-upgrade-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("study.sqlite");
    {
        let old = study_store::backup::open_at_version(&path, 7).unwrap();
        old.execute_batch(
            "INSERT INTO modules (name) VALUES ('Networks');
             INSERT INTO topics (name, module_id) VALUES ('OSI model', 1), ('TCP/IP', 1);
             INSERT INTO projects (name, kind, created) VALUES ('Study app', 'Coding', 20000);
             INSERT INTO settings (key, value) VALUES ('quiz_schedule', '{}');",
        )
        .unwrap();
    }
    let mut app = App::new(Store::open(&path).unwrap());
    let topics = app.topics().unwrap();
    let uids: Vec<String> = topics.iter().map(|topic| stamp(&app, "topics", topic.id.0).uid).collect();
    assert!(uids.iter().all(|uid| is_uid(uid)) && uids[0] != uids[1], "{uids:?}");
    assert!(is_uid(&stamp(&app, "modules", 1).uid) && is_uid(&stamp(&app, "projects", 1).uid));
    assert!(app.store.setting_changed("quiz_schedule").unwrap().is_some());
    let device = app.store.device_id();
    // New records after the upgrade are stamped later than the backfill, and the same
    // computer keeps its ID when the database is opened again.
    let newer = app.create_topic("Subnetting", None).unwrap();
    assert!(stamp(&app, "topics", newer.0).changed > stamp(&app, "topics", topics[0].id.0).changed);
    drop(app);
    assert_eq!(Store::open(&path).unwrap().device_id(), device);
}

// ids.feature: Restoring an older backup gives it IDs, stamped after what's here (R109, R110)
#[test]
fn restoring_a_backup_from_before_ids_upgrades_it_and_stamps_keep_going_up() {
    let dir = std::env::temp_dir().join(format!("study-ids-restore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("old.sqlite");
    study_store::backup::open_at_version(&old, 7).unwrap().execute("INSERT INTO topics (name) VALUES ('From long ago')", []).unwrap();

    let mut store = Store::in_memory().unwrap();
    let topic = store.create_topic("Here now", None).unwrap();
    let newest = store.stamp_of("topics", topic.0).unwrap().changed;
    store.restore_from(&old).unwrap();
    let restored = store.topics().unwrap();
    let stamp = store.stamp_of("topics", restored[0].id.0).unwrap();
    assert!(is_uid(&stamp.uid));
    assert!(stamp.changed > newest, "{} after {newest}", stamp.changed);
}

// Guards future tables: anything with its own numeric ID needs a uid and stamps.
#[test]
fn every_table_with_its_own_id_is_stamped() {
    let app = app();
    let tables: Vec<String> = app
        .store
        .table_names()
        .unwrap()
        .into_iter()
        .filter(|table| app.store.columns_of(table).unwrap().iter().any(|column| column == "id"))
        .collect();
    for table in &tables {
        assert!(STAMPED.contains(&table.as_str()), "{table} has an id but no uid, add it to ids::STAMPED and a migration");
        let columns = app.store.columns_of(table).unwrap();
        assert!(columns.contains(&"uid".to_string()) && columns.contains(&"changed".to_string()), "{table}: {columns:?}");
    }
    assert_eq!(tables.len(), STAMPED.len());
}

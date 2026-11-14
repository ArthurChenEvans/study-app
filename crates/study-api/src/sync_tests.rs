//! sync.feature through the command layer (R119 to R128). Each `App` is one computer with
//! its own database and device ID, and they share a real folder on disk, the way two
//! computers share a Syncthing folder.

use std::path::{Path, PathBuf};

use super::*;
use crate::backup::Reason;
use crate::cards::NoteInput;
use crate::practice::Clock;
use crate::sync::SyncSetup;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::Value;
use study_core::cards::{CardSettings, Note, NoteKind, Rating, Schedule};
use study_store::cards::NoteId;
use study_store::portable::Scope;

const TODAY: i64 = 20_761;

fn at(day: i64, minute: i64) -> Clock {
    Clock { day, minute: minute as u32, unix: day * 86_400 + minute * 60 }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("study-api-sync-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One computer: its own database, backups and sync settings, and the shared folder.
fn computer(test: &str, name: &str, seed: u64) -> App {
    let mut app = App::with_seed(Store::in_memory().unwrap(), seed);
    let home = scratch(&format!("{test}-{name}"));
    app.set_backups(home.join("backups"));
    app.set_sync_home(&home);
    app
}

fn folder(test: &str) -> PathBuf {
    scratch(&format!("{test}-folder"))
}

fn join(app: &mut App, folder: &Path, name: &str, clock: Clock) {
    app.sync_setup(SyncSetup { folder: folder.display().to_string(), name: name.into() }, clock).unwrap();
}

fn sync(app: &mut App, clock: Clock) -> crate::sync::SyncResult {
    app.sync_now(false, clock).unwrap()
}

/// Everything in a database in a form that's the same on every computer (IDs, not row
/// numbers), with each list in ID order, so two databases can be compared exactly.
fn everything(app: &App) -> Value {
    let snapshot = app.store.snapshot(&Scope { topics: None, history: true }).unwrap();
    let images: Vec<(String, Vec<u8>)> = snapshot.images.iter().map(|image| (image.uid.clone(), image.bytes.clone())).collect();
    let mut value = serde_json::to_value(&snapshot).unwrap();
    fn sort(value: &mut Value) {
        if let Value::Object(fields) = value {
            for (_, field) in fields.iter_mut() {
                if let Value::Array(items) = field {
                    items.iter_mut().for_each(sort);
                    items.sort_by_key(|item| item.get("uid").or(item.get("key")).map(Value::to_string).unwrap_or_default());
                }
            }
        }
    }
    sort(&mut value);
    value["image_bytes"] = serde_json::to_value(images).unwrap();
    value
}

fn assert_same(left: &App, right: &App) {
    assert_same_values(&everything(left), &everything(right), "");
}

fn differences(left: &Value, right: &Value, path: String, out: &mut Vec<String>) {
    match (left, right) {
        (Value::Object(one), Value::Object(other)) => {
            for key in one.keys().chain(other.keys().filter(|key| !one.contains_key(*key))) {
                differences(one.get(key).unwrap_or(&Value::Null), other.get(key).unwrap_or(&Value::Null), format!("{path}/{key}"), out);
            }
        }
        (Value::Array(one), Value::Array(other)) if one.len() == other.len() => {
            for (index, (a, b)) in one.iter().zip(other).enumerate() {
                differences(a, b, format!("{path}/{index}"), out);
            }
        }
        _ if left != right => out.push(format!("{path}: {left} vs {right}")),
        _ => {}
    }
}

/// Points at the first record that differs, rather than printing both databases.
fn assert_same_values(left: &Value, right: &Value, context: &str) {
    if left == right {
        return;
    }
    for (kind, items) in left.as_object().unwrap() {
        let (Some(mine), Some(theirs)) = (items.as_array(), right[kind].as_array()) else {
            assert_eq!(items, &right[kind], "{context}{kind} differ");
            continue;
        };
        let ids = |items: &[Value]| items.iter().map(|item| item.get("uid").or(item.get("key")).map(Value::to_string).unwrap_or_default()).collect::<std::collections::BTreeSet<_>>();
        let (left_ids, right_ids) = (ids(mine), ids(theirs));
        if left_ids != right_ids {
            let only_left: Vec<_> = left_ids.difference(&right_ids).collect();
            let only_right: Vec<_> = right_ids.difference(&left_ids).collect();
            panic!("{context}{kind}: only on the first {only_left:?}, only on the second {only_right:?}");
        }
        for (index, (one, other)) in mine.iter().zip(theirs).enumerate() {
            if one != other {
                let mut paths = vec![];
                differences(one, other, String::new(), &mut paths);
                panic!("{context}{kind}[{index}] differs at {}", paths.join("; "));
            }
        }
        assert_eq!(mine.len(), theirs.len(), "{context}{kind}: different numbers of records");
    }
}

fn small_profile(app: &mut App) -> (TopicId, NoteId) {
    let module = app.create_module("Computer Architecture").unwrap();
    let topic = app.create_topic("Caches", Some(module)).unwrap();
    let note = app.save_note(None, basic(topic, "L1 latency?", "About 3 cycles"), TODAY).unwrap();
    (topic, note)
}

fn basic(topic: TopicId, front: &str, back: &str) -> NoteInput {
    NoteInput { note: Note { topic, kind: NoteKind::Basic, front: front.into(), back: back.into(), cues: vec![], text: String::new(), extra: String::new(), tags: vec![] }, suggestion: None }
}

fn question(topic: TopicId, text: &str) -> QuestionInput {
    QuestionInput { topic, text: text.into(), multiple: false, options: ["A", "B", "C"].iter().enumerate().map(|(i, text)| OptionInput { text: (*text).into(), correct: i == 0 }).collect(), explanation: String::new(), tags: vec![], citations: vec![], short: None }
}

fn only_topic(app: &App) -> TopicId {
    app.topics().unwrap()[0].id
}

fn only_note(app: &App) -> NoteId {
    let topic = only_topic(app);
    app.topic_cards(topic, TODAY).unwrap().cards[0].note
}

/// Reviews the first card due, if any.
fn review_one(app: &mut App, rating: Rating, day: i64) -> bool {
    let step = app.start_review(None, day).unwrap();
    let reviewed = matches!(step, crate::cards::ReviewStep::Card(_)) && app.grade_card(rating, 5, day).is_ok();
    app.leave_review();
    reviewed
}

// sync.feature: Setting up the second computer (R119, R120, R122)
#[test]
fn a_second_computer_gets_everything_with_the_same_history_and_due_dates() {
    let shared = folder("setup");
    let mut desktop = computer("setup", "desktop", 1);
    crate::seed::seed_profile(&mut desktop, TODAY).unwrap();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("setup", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 610));

    assert_same(&desktop, &laptop);
    assert_eq!(laptop.recall_due_count(TODAY).unwrap(), desktop.recall_due_count(TODAY).unwrap());
    assert_eq!(laptop.cards_due_count(TODAY).unwrap(), desktop.cards_due_count(TODAY).unwrap());
    for app in [&desktop, &laptop] {
        assert!(app.backups().unwrap().iter().any(|backup| backup.reason == Reason::BeforeSync), "a backup before sync");
        assert!(app.sync_to_check().unwrap().is_empty());
    }
    let status = laptop.sync_status(at(TODAY, 611)).unwrap();
    assert!(status.on && status.paused.is_none() && status.problem.is_none(), "{status:?}");
    assert_eq!(status.name, "Laptop");
    let names: Vec<&str> = status.devices.iter().map(|device| device.name.as_str()).collect();
    assert_eq!(names, ["Laptop", "Desktop"], "this one first");
    assert_eq!(status.waiting, 0);
}

// sync.feature: Studying on both (R122)
#[test]
fn reviews_quizzes_and_checks_from_both_computers_all_end_up_on_both() {
    let shared = folder("both");
    let mut desktop = computer("both", "desktop", 1);
    crate::seed::seed_profile(&mut desktop, TODAY).unwrap();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("both", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    assert!(review_one(&mut desktop, Rating::Good, TODAY));
    assert!(review_one(&mut laptop, Rating::Hard, TODAY + 1));
    let topic = only_topic(&laptop);
    let checklist = laptop.create_checklist(study_store::recall::Subject::Topic(topic), TODAY).unwrap();
    laptop.save_points(checklist, ["Write-back", "Write-through"].map(|text| crate::recall::PointInput { text: text.into(), question: None, note: None }).to_vec()).unwrap();
    laptop.finish_check(checklist, None, vec![0], TODAY + 1).unwrap();
    let project = desktop.create_project("Cache simulator", "Coding", TODAY).unwrap();
    desktop.start_session(project, at(TODAY, 620)).unwrap();
    desktop.end_session(project, at(TODAY, 650)).unwrap();

    sync(&mut desktop, at(TODAY, 700));
    sync(&mut laptop, at(TODAY, 701));
    sync(&mut desktop, at(TODAY, 702));
    assert_same(&desktop, &laptop);
    assert!(desktop.sync_to_check().unwrap().is_empty());
    assert!(laptop.sync_to_check().unwrap().is_empty());
}

// sync.feature: Editing different parts of a card (R123)
#[test]
fn edits_to_different_fields_are_both_kept() {
    let shared = folder("fields");
    let mut desktop = computer("fields", "desktop", 1);
    small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("fields", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    let mine = only_note(&desktop);
    desktop.save_note(Some(mine), basic(only_topic(&desktop), "L1 hit latency?", "About 3 cycles"), TODAY).unwrap();
    let theirs = only_note(&laptop);
    laptop.save_note(Some(theirs), basic(only_topic(&laptop), "L1 latency?", "About 4 cycles"), TODAY).unwrap();

    sync(&mut desktop, at(TODAY, 700));
    sync(&mut laptop, at(TODAY, 701));
    sync(&mut desktop, at(TODAY, 702));
    assert_same(&desktop, &laptop);
    let (note, _) = desktop.store.note(only_note(&desktop)).unwrap();
    assert_eq!((note.front.as_str(), note.back.as_str()), ("L1 hit latency?", "About 4 cycles"));
    assert_eq!(desktop.topic_cards(only_topic(&desktop), TODAY).unwrap().cards[0].front, "L1 hit latency?");
    assert!(desktop.sync_to_check().unwrap().is_empty());
}

// sync.feature: Editing the same part on both (R123)
#[test]
fn the_same_field_edited_on_both_uses_the_newer_and_lists_it_to_check_on_both() {
    let shared = folder("same");
    let mut desktop = computer("same", "desktop", 1);
    small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("same", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    let topic = only_topic(&desktop);
    desktop.save_note(Some(only_note(&desktop)), basic(topic, "L1 latency?", "About 3 cycles at L1, 10 at L2"), TODAY).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let topic = only_topic(&laptop);
    laptop.save_note(Some(only_note(&laptop)), basic(topic, "L1 latency?", "About 4 cycles at L1, 12 at L2"), TODAY).unwrap();

    sync(&mut desktop, at(TODAY, 700));
    sync(&mut laptop, at(TODAY, 701));
    sync(&mut desktop, at(TODAY, 702));
    assert_same(&desktop, &laptop);
    let back = |app: &App| app.store.note(only_note(app)).unwrap().0.back;
    assert_eq!(back(&desktop), "About 4 cycles at L1, 12 at L2");

    for app in [&desktop, &laptop] {
        let to_check = app.sync_to_check().unwrap();
        assert_eq!(to_check.len(), 1, "one thing, shown once even though the card's back changed too: {to_check:?}");
        let thing = &to_check[0];
        assert_eq!((thing.what.as_str(), thing.label.as_str(), thing.field.as_str()), ("Card", "L1 latency?", "note.back"));
        assert_eq!(thing.context.as_deref(), Some("Caches · Computer Architecture"));
        assert_eq!((thing.kept_by.as_str(), thing.kept.as_deref()), ("Laptop", Some("About 4 cycles at L1, 12 at L2")));
        assert_eq!((thing.other_by.as_str(), thing.other.as_deref()), ("Desktop", Some("About 3 cycles at L1, 10 at L2")));
    }

    // Putting Desktop's back, on Laptop, goes to both and settles it on both.
    let n = laptop.sync_to_check().unwrap()[0].n;
    assert!(laptop.sync_settle(n, true, at(TODAY, 710)).unwrap().is_empty());
    sync(&mut laptop, at(TODAY, 711));
    sync(&mut desktop, at(TODAY, 712));
    assert_eq!(back(&desktop), "About 3 cycles at L1, 10 at L2");
    assert_eq!(back(&laptop), "About 3 cycles at L1, 10 at L2");
    assert!(desktop.sync_to_check().unwrap().is_empty(), "settled on Desktop too");
    assert_same(&desktop, &laptop);
}

// sync.feature: Deleting (R124)
#[test]
fn deleting_a_topic_deletes_it_and_what_belongs_to_it_everywhere() {
    let shared = folder("delete");
    let mut desktop = computer("delete", "desktop", 1);
    let (topic, _) = small_profile(&mut desktop);
    desktop.save_question(question(topic, "Which cache is fastest?")).unwrap();
    assert!(review_one(&mut desktop, Rating::Good, TODAY));
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("delete", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));
    assert_eq!(laptop.topics().unwrap().len(), 1);

    desktop.delete_topic(topic).unwrap();
    sync(&mut desktop, at(TODAY, 700));
    sync(&mut laptop, at(TODAY, 701));
    assert!(laptop.topics().unwrap().is_empty());
    assert_eq!(laptop.cards_due_count(TODAY + 30).unwrap(), 0);
    assert_same(&desktop, &laptop);
    assert!(laptop.sync_to_check().unwrap().is_empty());
}

// dates.feature: Dates sync (R137), and a deleted topic drops out of a date on both
#[test]
fn dates_sync_and_deleted_topics_drop_out_everywhere() {
    let shared = folder("dates");
    let mut desktop = computer("dates", "desktop", 1);
    let (caches, _) = small_profile(&mut desktop);
    let cpu = desktop.create_topic("CPU", None).unwrap();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("dates", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    let mock = crate::dates::DateSave { name: "Architecture mock".into(), day: TODAY + 20, kind: "Mock exam".into(), module: None, topics: vec![caches, cpu], tighten: true };
    desktop.add_date(mock).unwrap();
    sync(&mut desktop, at(TODAY, 700));
    sync(&mut laptop, at(TODAY, 701));
    let dates = laptop.dates(TODAY).unwrap();
    assert_eq!(dates.iter().map(|d| (d.name.as_str(), d.covers_label.as_str(), d.tighten)).collect::<Vec<_>>(), vec![("Architecture mock", "Caches and CPU", true)]);
    assert_same(&desktop, &laptop);

    let cpu_here = laptop.topics().unwrap().into_iter().find(|t| t.name == "CPU").unwrap().id;
    laptop.delete_topic(cpu_here).unwrap();
    sync(&mut laptop, at(TODAY, 800));
    sync(&mut desktop, at(TODAY, 801));
    assert_eq!(desktop.dates(TODAY).unwrap()[0].covers_label, "Caches");
    assert_same(&desktop, &laptop);
    assert!(desktop.sync_to_check().unwrap().is_empty(), "{:?}", desktop.sync_to_check().unwrap());
    let date = desktop.dates(TODAY).unwrap()[0].id;
    desktop.delete_date(date).unwrap();
    sync(&mut desktop, at(TODAY, 900));
    sync(&mut laptop, at(TODAY, 901));
    assert!(laptop.dates(TODAY).unwrap().is_empty());
    assert_same(&desktop, &laptop);
}

// sync.feature: Deleting something the other computer changed (R124)
#[test]
fn a_delete_loses_to_something_added_on_the_other_computer_and_both_keep_it() {
    let shared = folder("keep");
    let mut desktop = computer("keep", "desktop", 1);
    let (topic, _) = small_profile(&mut desktop);
    desktop.save_question(question(topic, "Which cache is fastest?")).unwrap();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("keep", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    desktop.delete_topic(topic).unwrap();
    laptop.save_question(question(only_topic(&laptop), "What does a TLB cache?")).unwrap();
    sync(&mut desktop, at(TODAY, 700));
    sync(&mut laptop, at(TODAY, 701));
    sync(&mut desktop, at(TODAY, 702));

    assert_same(&desktop, &laptop);
    for app in [&desktop, &laptop] {
        let topics = app.topics().unwrap();
        assert_eq!(topics.len(), 1, "the topic is kept");
        let questions: Vec<String> = app.questions_in(topics[0].id).unwrap().into_iter().map(|(_, text)| text).collect();
        assert!(questions.contains(&"What does a TLB cache?".to_string()), "{questions:?}");
        let to_check = app.sync_to_check().unwrap();
        assert!(to_check.iter().any(|thing| thing.deleted && thing.what == "Topic" && thing.label == "Caches"), "{to_check:?}");
    }
}

// sync.feature: Reviewing the same card on both (R125)
#[test]
fn a_card_reviewed_on_both_gets_its_schedule_worked_out_from_both_reviews() {
    let shared = folder("replay");
    let mut desktop = computer("replay", "desktop", 1);
    small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("replay", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    assert!(review_one(&mut desktop, Rating::Good, TODAY));
    assert!(review_one(&mut laptop, Rating::Again, TODAY + 1));
    sync(&mut desktop, at(TODAY + 1, 700));
    sync(&mut laptop, at(TODAY + 1, 701));
    sync(&mut desktop, at(TODAY + 1, 702));
    sync(&mut laptop, at(TODAY + 1, 703));

    assert_same(&desktop, &laptop);
    let settings = CardSettings::default();
    let first = settings.grade(&Schedule::new(TODAY), Rating::Good, TODAY).unwrap();
    let expected = settings.grade(&first, Rating::Again, TODAY + 1).unwrap();
    let snapshot = desktop.store.snapshot(&Scope { topics: None, history: true }).unwrap();
    assert_eq!(snapshot.reviews.len(), 2);
    let schedule = snapshot.notes[0].cards[0].schedule.clone().unwrap();
    assert_eq!((schedule.due, schedule.reps, schedule.last_review), (expected.due, 2, Some(TODAY + 1)));
    assert!((schedule.stability.unwrap() - f64::from(expected.memory.unwrap().stability)).abs() < 1e-4);
    assert!(desktop.sync_to_check().unwrap().is_empty(), "schedules are never listed to check");
}

// sync.feature: Syncing again changes nothing
#[test]
fn once_both_have_synced_syncing_again_writes_nothing() {
    let shared = folder("quiet");
    let mut desktop = computer("quiet", "desktop", 1);
    crate::seed::seed_profile(&mut desktop, TODAY).unwrap();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("quiet", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));
    sync(&mut desktop, at(TODAY, 602));
    for minute in 603..608 {
        let result = sync(&mut laptop, at(TODAY, minute));
        assert_eq!((result.sent, result.received), (0, 0), "laptop at {minute}");
        let result = sync(&mut desktop, at(TODAY, minute));
        assert_eq!((result.sent, result.received), (0, 0), "desktop at {minute}");
    }
}

// sync.feature: Three computers in any order (R122 to R126)
#[test]
fn three_computers_doing_random_things_and_syncing_in_any_order_end_up_the_same() {
    for seed in 0..20 {
        let test = format!("random{seed}");
        let shared = folder(&test);
        let mut random = StdRng::seed_from_u64(seed);
        let mut computers: Vec<App> = (0..3).map(|n| computer(&test, &format!("c{n}"), 10 + n)).collect();
        small_profile(&mut computers[0]);
        for (n, app) in computers.iter_mut().enumerate() {
            join(app, &shared, &format!("Computer {n}"), at(TODAY, 600 + n as i64));
        }
        for step in 0..40 {
            let n = random.random_range(0..3);
            let app = &mut computers[n];
            let day = TODAY + step / 8;
            let topics = app.topics().unwrap();
            match random.random_range(0..10) {
                0 => {
                    app.create_topic(&format!("Topic {seed}-{step}"), None).unwrap();
                }
                1 | 2 if !topics.is_empty() => {
                    let topic = topics[random.random_range(0..topics.len())].id;
                    app.save_question(question(topic, &format!("Question {step}"))).unwrap();
                }
                3 if !topics.is_empty() => {
                    let topic = topics[random.random_range(0..topics.len())].id;
                    if let Some((id, _)) = app.questions_in(topic).unwrap().first() {
                        let mut input = app.question(*id).unwrap();
                        input.text = format!("{} (edited on {n} at {step})", input.text);
                        app.update_question(*id, input).unwrap();
                    }
                }
                4 if !topics.is_empty() => {
                    let topic = topics[random.random_range(0..topics.len())].id;
                    if let Some((id, _)) = app.questions_in(topic).unwrap().last() {
                        app.delete_question(*id).unwrap();
                    }
                }
                5 if topics.len() > 1 && random.random_bool(0.4) => {
                    app.delete_topic(topics[random.random_range(0..topics.len())].id).unwrap();
                }
                6 if !topics.is_empty() => {
                    let topic = topics[random.random_range(0..topics.len())].id;
                    app.save_note(None, basic(topic, &format!("Front {step}"), "Back"), day).unwrap();
                }
                7 => {
                    review_one(app, if random.random_bool(0.5) { Rating::Good } else { Rating::Again }, day);
                }
                _ => {
                    sync(app, at(day, 700 + step));
                }
            }
        }
        // Then everyone syncs until there's nothing left to send.
        for round in 0..4 {
            for (n, app) in computers.iter_mut().enumerate() {
                sync(app, at(TODAY + 6, 800 + round * 3 + n as i64));
            }
        }
        for app in &computers {
            assert_eq!(app.sync_status(at(TODAY + 6, 900)).unwrap().waiting, 0, "seed {seed}");
        }
        let (first, rest) = computers.split_first().unwrap();
        for other in rest {
            assert_same_values(&everything(first), &everything(other), &format!("seed {seed}: "));
        }
        let result = &mut computers[1];
        let quiet = sync(result, at(TODAY + 6, 950));
        assert_eq!((quiet.sent, quiet.received), (0, 0), "seed {seed} still sending");
        // Each computer keeps backups, so tidy up as it goes.
        for n in 0..3 {
            let _ = std::fs::remove_dir_all(scratch(&format!("{test}-c{n}")));
        }
        let _ = std::fs::remove_dir_all(&shared);
    }
}

// sync.feature: Something arrives before what it belongs to (R126)
#[test]
fn a_question_whose_topic_is_still_on_its_way_waits_for_it() {
    let shared = folder("waits");
    let mut browser = computer("waits", "browser", 1);
    let mut laptop = computer("waits", "laptop", 2);
    let mut desktop = computer("waits", "desktop", 3);
    join(&mut browser, &shared, "Browser", at(TODAY, 600));
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));
    join(&mut desktop, &shared, "Desktop", at(TODAY, 602));

    browser.create_topic("Virtual memory", None).unwrap();
    sync(&mut browser, at(TODAY, 610));
    sync(&mut laptop, at(TODAY, 611));
    laptop.save_question(question(only_topic(&laptop), "What does a TLB cache?")).unwrap();
    sync(&mut laptop, at(TODAY, 612));

    // Desktop sees Laptop's changes before the browser version's have arrived.
    let browser_files = shared.join("changes").join(browser.store.device_id());
    let away = shared.join("away");
    std::fs::rename(&browser_files, &away).unwrap();
    let result = sync(&mut desktop, at(TODAY, 613));
    assert!(desktop.topics().unwrap().is_empty());
    assert_eq!(result.status.problem, None, "waiting isn't a problem");

    std::fs::rename(&away, &browser_files).unwrap();
    sync(&mut desktop, at(TODAY, 614));
    let topic = only_topic(&desktop);
    assert_eq!(desktop.questions_in(topic).unwrap().len(), 1, "the question appears once its topic arrives");
    assert_same(&desktop, &laptop);
}

// sync.feature: Something arrives before what it belongs to (R126)
#[test]
fn an_edit_that_arrives_before_the_question_it_edits_waits_for_it() {
    let shared = folder("early-edit");
    let mut browser = computer("early-edit", "browser", 1);
    let mut laptop = computer("early-edit", "laptop", 2);
    let mut desktop = computer("early-edit", "desktop", 3);
    let topic = browser.create_topic("Virtual memory", None).unwrap();
    join(&mut browser, &shared, "Browser", at(TODAY, 600));
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));
    join(&mut desktop, &shared, "Desktop", at(TODAY, 602));

    let id = browser.save_question(question(topic, "What does a TLB cache?")).unwrap();
    let _ = id;
    sync(&mut browser, at(TODAY, 610));
    sync(&mut laptop, at(TODAY, 611));
    let (id, _) = laptop.questions_in(only_topic(&laptop)).unwrap()[0].clone();
    let mut input = laptop.question(id).unwrap();
    input.text = "What does a TLB cache, exactly?".into();
    laptop.update_question(id, input).unwrap();
    sync(&mut laptop, at(TODAY, 612));

    // Desktop has the topic, and gets Laptop's edit before the browser's new question.
    let browser_files = shared.join("changes").join(browser.store.device_id());
    let away = shared.join("away");
    std::fs::rename(&browser_files, &away).unwrap();
    let result = sync(&mut desktop, at(TODAY, 613));
    assert!(desktop.questions_in(only_topic(&desktop)).unwrap().is_empty(), "an edit alone isn't a question");
    assert_eq!(result.status.held, 1);

    std::fs::rename(&away, &browser_files).unwrap();
    sync(&mut desktop, at(TODAY, 614));
    let questions = desktop.questions_in(only_topic(&desktop)).unwrap();
    assert_eq!(questions.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>(), ["What does a TLB cache, exactly?"]);
    assert_same(&desktop, &laptop);
}

// sync.feature: The folder goes missing (R127)
#[test]
fn changes_made_while_the_folder_is_missing_are_counted_and_go_out_when_it_is_back() {
    let shared = folder("missing");
    let mut desktop = computer("missing", "desktop", 1);
    let (topic, _) = small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("missing", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    let unplugged = shared.with_extension("unplugged");
    std::fs::rename(&shared, &unplugged).unwrap();
    desktop.save_question(question(topic, "Which cache is fastest?")).unwrap();
    desktop.save_question(question(topic, "Which cache is biggest?")).unwrap();
    let result = sync(&mut desktop, at(TODAY, 610));
    let problem = result.status.problem.clone().unwrap_or_default();
    assert!(problem.starts_with("Can't find the sync folder"), "{problem}");
    assert_eq!(result.status.waiting, 2);
    assert_eq!(desktop.sync_status(at(TODAY, 611)).unwrap().waiting, 2);

    std::fs::rename(&unplugged, &shared).unwrap();
    let result = sync(&mut desktop, at(TODAY, 620));
    assert_eq!((result.status.waiting, result.status.problem.clone()), (0, None));
    sync(&mut laptop, at(TODAY, 621));
    assert_eq!(laptop.questions_in(only_topic(&laptop)).unwrap().len(), 2);
}

// sync.feature: A change file that's still being copied (R127)
#[test]
fn a_half_written_change_is_read_once_it_is_complete_and_a_broken_line_is_skipped() {
    let shared = folder("half");
    let mut desktop = computer("half", "desktop", 1);
    let (topic, _) = small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("half", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));

    desktop.save_question(question(topic, "Which cache is fastest?")).unwrap();
    sync(&mut desktop, at(TODAY, 610));
    let dir = shared.join("changes").join(desktop.store.device_id());
    let file = std::fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
    let whole = std::fs::read(&file).unwrap();
    // Cut the file off part way through its last line, as a sync app might mid-copy.
    let last_start = whole[..whole.len() - 1].iter().rposition(|byte| *byte == b'\n').unwrap() + 1;
    std::fs::write(&file, &whole[..last_start + 10]).unwrap();
    sync(&mut laptop, at(TODAY, 611));
    std::fs::write(&file, &whole).unwrap();
    // And a line that isn't a change at all, from some other program.
    let mut with_junk = whole.clone();
    with_junk.extend_from_slice(b"this is not json\n");
    std::fs::write(&file, &with_junk).unwrap();
    let result = sync(&mut laptop, at(TODAY, 612));
    assert_eq!(laptop.questions_in(only_topic(&laptop)).unwrap().len(), 1);
    assert_same(&desktop, &laptop);
    let problem = result.status.problem.unwrap_or_default();
    assert!(problem.contains("couldn't be read"), "{problem}");
}

// sync.feature: A computer on a newer version (R127)
#[test]
fn changes_from_a_newer_version_are_skipped_with_a_message_to_update() {
    let shared = folder("newer");
    let mut desktop = computer("newer", "desktop", 1);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    std::fs::create_dir_all(shared.join("changes").join("0badf00d")).unwrap();
    std::fs::write(shared.join("devices").join("0badf00d.json"), r#"{"format":2,"device":"0badf00d","name":"Laptop","seq":1,"seen_at":0,"app":"9.0.0"}"#).unwrap();
    std::fs::write(shared.join("changes").join("0badf00d").join("2026-11.jsonl"), "{\"seq\":1,\"at\":\"0001793000000000-0000-0badf00d\",\"kind\":\"modules\",\"uid\":\"x\",\"set\":{\"name\":\"From the future\"},\"new_thing\":true}\n").unwrap();
    let result = sync(&mut desktop, at(TODAY, 610));
    assert!(desktop.modules().unwrap().is_empty());
    assert_eq!(result.status.problem.as_deref(), Some("Laptop has a newer version of the app, so its changes are skipped. Update this one to sync with it."));
    assert!(result.status.devices.iter().any(|device| device.name == "Laptop" && device.newer_app));
}

// sync.feature: A quiz in progress (R121)
#[test]
fn the_automatic_sync_waits_while_a_quiz_or_review_is_in_progress() {
    let shared = folder("busy");
    let mut desktop = computer("busy", "desktop", 1);
    crate::seed::seed_profile(&mut desktop, TODAY).unwrap();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    desktop.start_review(None, TODAY).unwrap();
    desktop.grade_card(Rating::Good, 3, TODAY).unwrap();
    let result = desktop.sync_now(true, at(TODAY, 610)).unwrap();
    assert!(result.deferred);
    assert_eq!(result.sent, 0);
    desktop.leave_review();
    let result = desktop.sync_now(true, at(TODAY, 611)).unwrap();
    assert!(!result.deferred);
    assert!(result.sent > 0);
}

// sync.feature: Restoring a backup (R128)
#[test]
fn restoring_a_backup_pauses_sync_until_it_is_turned_back_on() {
    let shared = folder("restore");
    let mut desktop = computer("restore", "desktop", 1);
    small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let backup = desktop.back_up_now(at(TODAY, 601)).unwrap();
    desktop.create_topic("Added after the backup", None).unwrap();
    sync(&mut desktop, at(TODAY, 602));
    desktop.restore_backup(&backup.name, at(TODAY, 603)).unwrap();

    let status = desktop.sync_status(at(TODAY, 604)).unwrap();
    assert_eq!(status.paused.as_deref(), Some("Paused after restoring a backup"));
    let result = sync(&mut desktop, at(TODAY, 605));
    assert_eq!((result.sent, result.received), (0, 0));
    assert_eq!(desktop.topics().unwrap().len(), 1);

    // Turning it back on brings back what the folder still has, and nothing gets mixed up.
    desktop.sync_resume(at(TODAY, 606)).unwrap();
    assert_eq!(desktop.topics().unwrap().len(), 2);
    desktop.create_topic("After the restore", None).unwrap();
    sync(&mut desktop, at(TODAY, 607));
    let mut laptop = computer("restore", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 608));
    assert_same(&desktop, &laptop);
}

#[test]
fn sync_can_be_renamed_moved_and_turned_off() {
    let shared = folder("settings");
    let mut desktop = computer("settings", "desktop", 1);
    small_profile(&mut desktop);
    assert!(desktop.sync_setup(SyncSetup { folder: "relative/path".into(), name: "Desktop".into() }, at(TODAY, 600)).is_err());
    assert!(desktop.sync_setup(SyncSetup { folder: shared.join("nope").display().to_string(), name: "Desktop".into() }, at(TODAY, 600)).is_err());
    assert!(desktop.sync_setup(SyncSetup { folder: shared.display().to_string(), name: "  ".into() }, at(TODAY, 600)).is_err());
    assert!(!desktop.sync_status(at(TODAY, 600)).unwrap().on);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));

    desktop.sync_rename("Big desktop").unwrap();
    let mut laptop = computer("settings", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 610));
    assert!(laptop.sync_status(at(TODAY, 611)).unwrap().devices.iter().any(|device| device.name == "Big desktop"));

    // A new folder gets everything this computer has.
    let moved = folder("settings-moved");
    join(&mut desktop, &moved, "Big desktop", at(TODAY, 620));
    let mut other = computer("settings", "other", 3);
    join(&mut other, &moved, "Other", at(TODAY, 621));
    assert_same(&desktop, &other);

    let status = desktop.sync_off().unwrap();
    assert!(!status.on);
    desktop.create_topic("Not synced", None).unwrap();
    let result = sync(&mut desktop, at(TODAY, 630));
    assert_eq!(result.sent, 0);
}

#[test]
#[ignore = "timing, run with --ignored"]
fn the_large_profile_syncs_to_a_second_computer_in_reasonable_time() {
    let shared = folder("large");
    let mut desktop = computer("large", "desktop", 1);
    crate::seed_large::seed_large(&mut desktop, TODAY).unwrap();
    let started = std::time::Instant::now();
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let first = started.elapsed();
    let mut laptop = computer("large", "laptop", 2);
    let started = std::time::Instant::now();
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));
    let second = started.elapsed();
    let started = std::time::Instant::now();
    sync(&mut laptop, at(TODAY, 602));
    let quiet = started.elapsed();
    assert!(desktop.review_one_for_test());
    let started = std::time::Instant::now();
    sync(&mut desktop, at(TODAY, 603));
    sync(&mut laptop, at(TODAY, 604));
    let small = started.elapsed();
    eprintln!("first sync out {first:?}, in {second:?}, a quiet sync {quiet:?}, one review across {small:?}");
    let bytes: u64 = std::fs::read_dir(shared.join("changes").join(desktop.store.device_id())).unwrap().map(|entry| entry.unwrap().metadata().unwrap().len()).sum();
    eprintln!("desktop's change files: {:.1} MB", bytes as f64 / 1e6);
    assert_same(&desktop, &laptop);
}

impl App {
    fn review_one_for_test(&mut self) -> bool {
        review_one(self, Rating::Good, TODAY)
    }
}

// celebrations.feature: which moments you want follows you to your other computers.
#[test]
fn celebration_choices_sync() {
    let shared = folder("celebrations");
    let mut desktop = computer("celebrations", "desktop", 1);
    small_profile(&mut desktop);
    join(&mut desktop, &shared, "Desktop", at(TODAY, 600));
    let mut laptop = computer("celebrations", "laptop", 2);
    join(&mut laptop, &shared, "Laptop", at(TODAY, 601));
    desktop.set_celebrations(r#"{"streaks":false}"#.into()).unwrap();
    sync(&mut desktop, at(TODAY, 602));
    sync(&mut laptop, at(TODAY, 603));
    assert_eq!(laptop.celebrations().unwrap().as_deref(), Some(r#"{"streaks":false}"#));
}

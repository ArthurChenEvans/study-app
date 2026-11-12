//! Tests from the robustness audit (November 2026): hostile files, damaged settings, two
//! copies of the app on one database, and what a crash or a bad file leaves behind.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use serde_json::Value;

use super::*;
use crate::backup::BackupSettings;
use crate::practice::Clock;
use crate::sync::SyncSetup;
use crate::transfer::{ClashRule, ExportRequest, ImportChoice, pack, unpack};

const TODAY: i64 = 20_769;

fn at(day: i64, minute: u32) -> Clock {
    Clock { day, minute, unix: day * 86_400 + minute as i64 * 60 }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("study-api-audit-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seeded(seed: u64) -> App {
    let mut app = App::with_seed(Store::in_memory().unwrap(), seed);
    crate::seed::seed_profile(&mut app, TODAY).unwrap();
    app
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn leaves<'a>(value: &'a mut Value, out: &mut Vec<&'a mut Value>) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(|item| leaves(item, out)),
        Value::Object(map) => map.values_mut().for_each(|item| leaves(item, out)),
        other => out.push(other),
    }
}

/// Changes `count` leaves anywhere in the value to something hostile of the same kind.
fn corrupt(value: &mut Value, rng: &mut Rng, count: usize) {
    let numbers = [0i64, -1, 1, i64::MAX, i64::MIN, u32::MAX as i64, 3_000_000, -3_000_000, 20_769, 99_999_999];
    let strings = ["", " ", "x", "💥", "01a1a800-0000-7000-8000-000000000001", "0000000000000001-0000-aaaa0000", "9999999999999999-9999-ffffffff", "Single", "Multiple", "short", "{}", "[]", "null", "{\"a\":", "../../x", "Module", "Topic", "question:1"];
    let mut all = vec![];
    leaves(value, &mut all);
    if all.is_empty() {
        return;
    }
    for _ in 0..count {
        let index = rng.below(all.len());
        let leaf = &mut all[index];
        **leaf = match &**leaf {
            Value::Number(_) => Value::from(numbers[rng.below(numbers.len())]),
            Value::String(text) if rng.below(4) == 0 => Value::String(format!("{text}{}", "é".repeat(rng.below(3000)))),
            Value::String(_) => Value::String(strings[rng.below(strings.len())].into()),
            Value::Bool(flag) => Value::Bool(!flag),
            _ => Value::Null,
        };
    }
}

/// Calls every screen's reading command, returning the first panic message if one panics.
fn exercise(app: &mut App, today: i64) -> Option<String> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _ = app.topics();
        let _ = app.modules();
        let _ = app.history();
        let _ = app.overview(today);
        let _ = app.sources();
        let _ = app.recall_list(today);
        let _ = app.cards_home(today);
        let _ = app.card_suggestions(today);
        let _ = app.projects(today);
        let _ = app.issues();
        let _ = app.drills_due(today);
        let _ = app.topic_quizzes(today);
        let _ = app.quiz_queue(today);
        let _ = app.due_today(today);
        let _ = app.dates(today);
        let _ = app.calendar_month(today - 14, today + 27, today);
        for day in [today - 1, today, today + 1, today + 30] {
            let _ = app.calendar_day(day, today);
        }
        if let Ok(topics) = app.topics() {
            for topic in topics.iter().take(40) {
                let _ = app.topic_cards(topic.id, today);
                let _ = app.questions_in(topic.id);
            }
        }
        if let Ok(history) = app.history() {
            for row in history.iter().take(40) {
                let _ = app.results(row.id);
            }
        }
        if let Ok(rows) = app.recall_list(today) {
            for row in rows.iter().take(40) {
                let _ = app.checklist(row.id, today);
                let _ = app.recall_history(row.id);
            }
        }
        let _ = app.start_review(None, today);
        let _ = app.grade_card(study_core::cards::Rating::Good, 3, today);
        app.leave_review();
        let _ = app.export(&ExportRequest { topics: None, history: true }, today);
    }));
    outcome.err().map(|payload| payload.downcast_ref::<String>().cloned().or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string())).unwrap_or_default())
}

#[test]
#[ignore]
fn probe_hostile_imports() {
    let rounds: usize = std::env::var("ROUNDS").ok().and_then(|n| n.parse().ok()).unwrap_or(300);
    let (_, file) = seeded(1).export(&ExportRequest { topics: None, history: true }, TODAY).unwrap();
    let (manifest, snapshot) = unpack(&file).unwrap();
    let clean = serde_json::to_value(&snapshot).unwrap();
    let mut rng = Rng(0xA5A5_1234_5678_9ABC);
    std::panic::set_hook(Box::new(|_| {}));
    let (mut refused, mut accepted) = (0, 0);
    let mut findings: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    for round in 0..rounds {
        let mut value = clean.clone();
        let changes = 1 + rng.below(6);
        corrupt(&mut value, &mut rng, changes);
        let Ok(snapshot) = serde_json::from_value::<study_store::portable::Snapshot>(value) else { refused += 1; continue };
        let Ok(bytes) = pack(&snapshot, &manifest) else { continue };
        // Into an empty app and into one that already has the same data
        for (kind, mut app) in [("empty", App::with_seed(Store::in_memory().unwrap(), 2)), ("same data", seeded(1))] {
            let rule = [ClashRule::KeepMine, ClashRule::TakeTheirs, ClashRule::Newer][rng.below(3)];
            let outcome = catch_unwind(AssertUnwindSafe(|| app.import_preview(&bytes, "x.studyapp").and_then(|_| app.import_apply(ImportChoice { rule, theirs: vec![] }, at(TODAY, 600)))));
            let mut note = |what: String| {
                let what: String = what.split(" at line ").next().unwrap().split('`').enumerate().map(|(i, part)| if i % 2 == 1 && i == 1 { "…" } else { part }).collect::<Vec<_>>().join("`").chars().filter(|c| !c.is_ascii_digit()).take(120).collect();
                let entry = findings.entry(what).or_insert((0, format!("round {round}, into {kind}")));
                entry.0 += 1;
            };
            match outcome {
                Err(_) => note("PANIC while importing".into()),
                Ok(Err(_)) => refused += 1,
                Ok(Ok(_)) => {
                    accepted += 1;
                    for problem in app.store.health().unwrap() {
                        note(format!("after import: {}", problem.split(" row ").next().unwrap_or(&problem).to_string() + " … " + problem.rsplit(' ').next().unwrap_or("")));
                    }
                    if let Some(panic) = exercise(&mut app, TODAY) {
                        note(format!("PANIC on a screen after import: {}", panic.chars().take(140).collect::<String>()));
                    }
                    for (what, result) in [("topics", app.topics().map(|_| ())), ("history", app.history().map(|_| ())), ("cards", app.cards_home(TODAY).map(|_| ())), ("recall", app.recall_list(TODAY).map(|_| ())), ("overview", app.overview(TODAY).map(|_| ())), ("projects", app.projects(TODAY).map(|_| ())), ("calendar", app.calendar_day(TODAY, TODAY).map(|_| ())), ("search", app.search(study_store::search::SearchQuery { text: String::new(), tab: study_store::search::Tab::All, module: None, tag: None, sort: study_store::search::SortBy::Question, descending: false }).map(|_| ()))] {
                        if let Err(error) = result {
                            note(format!("screen '{what}' fails after import: {}", error.message.chars().take(110).collect::<String>()));
                        }
                    }
                }
            }
        }
    }
    let _ = std::panic::take_hook();
    println!("\n== hostile imports: {rounds} files, {accepted} imports accepted, {refused} refused");
    for (what, (count, example)) in &findings {
        println!("   [{count}x] {what}   ({example})");
    }
}

#[test]
#[ignore]
fn probe_damaged_backup_settings() {
    let dir = scratch("settings");
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    app.set_backups(dir.join("backups"));
    app.create_topic("Caches", None).unwrap();
    app.set_backup_settings(BackupSettings { keep_days: 60, keep_weeks: 4, keep_months: 12, copy_to: None }, at(TODAY - 40, 600)).unwrap();
    for ago in (0..40).rev() {
        app.back_up_now(at(TODAY - ago - 1, 600)).unwrap();
    }
    let before = app.backups().unwrap().len();
    std::fs::write(dir.join("backups").join("settings.json"), b"{\"keep_days\": 6").unwrap();
    app.back_up_now(at(TODAY, 600)).unwrap();
    let after = app.backups().unwrap().len();
    println!("\n== damaged settings.json: {before} backups before, {after} after one more backup (settings now read as {:?})", app.backup_status().unwrap().settings);
}

#[test]
#[ignore]
fn probe_restore_of_a_bad_backup() {
    let dir = scratch("restore");
    let mut app = App::with_seed(Store::open(&dir.join("study.sqlite")).unwrap(), 1);
    app.set_backups(dir.join("backups"));
    crate::seed::seed_profile(&mut app, TODAY).unwrap();
    let good = app.back_up_now(at(TODAY, 600)).unwrap();
    let topics = app.topics().unwrap().len();
    // 1: a real database that isn't this app's, claiming an old schema version
    let alien = dir.join("backups").join("study 2026-11-01 09-00 daily.sqlite");
    {
        let connection = study_store::backup::open_at_version(&alien, 3).unwrap();
        connection.execute_batch("DROP TABLE topics; CREATE TABLE topics (junk TEXT);").unwrap();
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| app.restore_backup("study 2026-11-01 09-00 daily.sqlite", at(TODAY, 601))));
    println!("\n== restore of a damaged old backup: {:?}", outcome.as_ref().map(|result| result.as_ref().map(|info| info.name.clone()).map_err(|error| error.message.clone())).map_err(|_| "PANIC"));
    println!("   afterwards: topics() = {:?}, schema = {:?}, health = {:?}", app.topics().map(|topics| topics.len()).map_err(|error| error.message), app.store.schema_version().ok(), app.store.health().map(|problems| problems.len()).map_err(|error| error.to_string()));
    println!("   (had {topics} topics before; good backup is {})", good.name);
    // 2: a backup with rows pointing nowhere
    let orphaned = dir.join("backups").join("study 2026-11-02 09-00 daily.sqlite");
    std::fs::copy(dir.join("backups").join(&good.name), &orphaned).unwrap();
    {
        let connection = study_store::backup::open_at_version(&orphaned, study_store::LATEST_SCHEMA).unwrap();
        connection.execute_batch("PRAGMA foreign_keys = OFF; DELETE FROM topics;").unwrap_or_else(|error| println!("   (couldn't orphan rows: {error})"));
    }
    let mut second = App::with_seed(Store::open(&dir.join("second.sqlite")).unwrap(), 1);
    second.set_backups(dir.join("backups"));
    let outcome = second.restore_backup("study 2026-11-02 09-00 daily.sqlite", at(TODAY, 602));
    println!("== restore of a backup with orphaned rows: {:?}, health problems after: {}", outcome.map(|info| info.name).map_err(|error| error.message), second.store.health().unwrap().len());
}

// Hazard 6: the desktop app and the browser version (or two windows) on one database.
#[test]
fn two_copies_of_the_app_on_one_database_lose_nothing() {
    let dir = scratch("two");
    let path = dir.join("study.sqlite");
    let first = Store::open(&path).unwrap();
    drop(first);
    let workers: Vec<_> = (0..2)
        .map(|worker| {
            let path = path.clone();
            std::thread::spawn(move || {
                let mut app = App::with_seed(Store::open(&path).unwrap(), worker);
                let mut failed = std::collections::BTreeMap::<String, usize>::new();
                let mut made = 0;
                for n in 0..400 {
                    match app.create_topic(&format!("w{worker}-{n}"), None).and_then(|topic| {
                        app.save_question(QuestionInput { topic, text: "Q".into(), multiple: false, options: vec![OptionInput { text: "a".into(), correct: true }, OptionInput { text: "b".into(), correct: false }, OptionInput { text: "c".into(), correct: false }], explanation: String::new(), tags: vec!["t".into()], citations: vec![], short: None })
                    }) {
                        Ok(_) => made += 1,
                        Err(error) => *failed.entry(error.message.chars().take(60).collect()).or_default() += 1,
                    }
                }
                (made, failed)
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|worker| worker.join().unwrap()).collect();
    let app = App::with_seed(Store::open(&path).unwrap(), 9);
    let topics = app.topics().unwrap();
    let complete = topics.iter().filter(|topic| topic.question_count == 1).count();
    let snapshot = app.store.snapshot(&study_store::portable::Scope { topics: None, history: true }).unwrap();
    let mut uids: Vec<&str> = snapshot.topics.iter().map(|topic| topic.uid.as_str()).chain(snapshot.questions.iter().map(|question| question.uid.as_str())).collect();
    let total = uids.len();
    uids.sort();
    uids.dedup();
    let mut stamps: Vec<&str> = snapshot.topics.iter().map(|topic| topic.changed.as_str()).chain(snapshot.questions.iter().map(|question| question.changed.as_str())).collect();
    let stamp_total = stamps.len();
    stamps.sort();
    stamps.dedup();
    for (made, failed) in &results {
        assert_eq!((*made, failed.len()), (400, 0), "every save went through: {failed:?}");
    }
    assert_eq!((topics.len(), complete), (800, 800), "every topic is there with its question");
    assert_eq!(total - uids.len(), 0, "no two rows share an ID");
    assert_eq!(stamp_total - stamps.len(), 0, "no two changes share a stamp");
    assert_eq!(app.store.health().unwrap(), Vec::<String>::new());
}

#[test]
#[ignore]
fn probe_hostile_sync_folder() {
    let rounds: usize = std::env::var("ROUNDS").ok().and_then(|n| n.parse().ok()).unwrap_or(150);
    let mut rng = Rng(0x1357_9BDF_2468_ACE0);
    std::panic::set_hook(Box::new(|_| {}));
    let mut findings: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    // One honest computer writes a folder full of real changes.
    let folder = scratch("sync-folder");
    let home = scratch("sync-a");
    let mut honest = App::with_seed(Store::in_memory().unwrap(), 1);
    honest.set_backups(home.join("backups"));
    honest.set_sync_home(&home);
    crate::seed::seed_profile(&mut honest, TODAY).unwrap();
    honest.sync_setup(SyncSetup { folder: folder.display().to_string(), name: "A".into() }, at(TODAY, 600)).unwrap();
    let change_dir = std::fs::read_dir(folder.join("changes")).unwrap().flatten().next().unwrap().path();
    let change_file = std::fs::read_dir(&change_dir).unwrap().flatten().next().unwrap().path();
    let clean = std::fs::read_to_string(&change_file).unwrap();
    let lines: Vec<&str> = clean.lines().collect();
    println!("\n== hostile sync folder: {} honest lines", lines.len());
    for round in 0..rounds {
        let mut changed: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        let kind = rng.below(6);
        match kind {
            0 => {
                for _ in 0..1 + rng.below(8) {
                    let index = rng.below(changed.len());
                    let mut value: Value = serde_json::from_str(&changed[index]).unwrap();
                    let count = 1 + rng.below(3);
                    corrupt(&mut value, &mut rng, count);
                    changed[index] = value.to_string();
                }
            }
            1 => {
                let cut = rng.below(clean.len());
                changed = clean.as_bytes()[..cut].split(|byte| *byte == b'\n').map(|line| String::from_utf8_lossy(line).to_string()).collect();
            }
            2 => {
                let index = rng.below(changed.len());
                changed.insert(index, "{not json at all".into());
                changed.insert(index, "\u{0}\u{0}\u{0}".into());
            }
            3 => {
                let copy = changed.clone();
                changed.extend(copy); // everything twice
            }
            4 => {
                let index = rng.below(changed.len());
                changed.swap(0, index);
                changed.reverse();
            }
            _ => {
                let drop = rng.below(changed.len());
                changed.remove(drop); // a line lost in the middle
            }
        }
        std::fs::write(&change_file, changed.join("\n") + "\n").unwrap();
        let home_b = scratch("sync-b");
        let mut other = App::with_seed(Store::in_memory().unwrap(), 2);
        other.set_backups(home_b.join("backups"));
        other.set_sync_home(&home_b);
        let mut note = |what: String| {
            let what: String = what.split(" at line ").next().unwrap().split('`').enumerate().map(|(i, part)| if i % 2 == 1 && i == 1 { "…" } else { part }).collect::<Vec<_>>().join("`").chars().filter(|c| !c.is_ascii_digit()).take(120).collect();
            let entry = findings.entry(what).or_insert((0, format!("round {round}, damage kind {kind}")));
            entry.0 += 1;
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let first = other.sync_setup(SyncSetup { folder: folder.display().to_string(), name: "B".into() }, at(TODAY, 601));
            let second = other.sync_now(false, at(TODAY, 602));
            (first.map(|_| ()), second.map(|_| ()))
        }));
        match outcome {
            Err(_) => note("PANIC while syncing".into()),
            Ok((first, second)) => {
                for result in [first, second] {
                    if let Err(error) = result {
                        note(format!("sync refused: {}", error.message.chars().take(90).collect::<String>()));
                    }
                }
                for problem in other.store.health().unwrap() {
                    note(format!("after sync: {} … {}", problem.split(" row ").next().unwrap_or(&problem), problem.rsplit(' ').next().unwrap_or("")));
                }
                if let Some(panic) = exercise(&mut other, TODAY) {
                    note(format!("PANIC on a screen after sync: {}", panic.chars().take(140).collect::<String>()));
                }
                for (what, result) in [("topics", other.topics().map(|_| ())), ("history", other.history().map(|_| ())), ("cards", other.cards_home(TODAY).map(|_| ())), ("recall", other.recall_list(TODAY).map(|_| ())), ("overview", other.overview(TODAY).map(|_| ())), ("calendar", other.calendar_day(TODAY, TODAY).map(|_| ()))] {
                    if let Err(error) = result {
                        note(format!("screen '{what}' fails after sync: {}", error.message.chars().take(110).collect::<String>()));
                    }
                }
            }
        }
        // Clean up what B wrote so the next round starts from A's folder alone.
        for entry in std::fs::read_dir(folder.join("changes")).unwrap().flatten() {
            if entry.path() != change_dir {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let _ = std::panic::take_hook();
    for (what, (count, example)) in &findings {
        println!("   [{count}x] {what}   ({example})");
    }
}

#[cfg(unix)]
#[test]
#[ignore]
fn probe_slide_conversion_leftovers() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("slides");
    let program = dir.join("soffice");
    // Like the real launcher: a shell script that starts the real program as a child.
    std::fs::write(&program, format!("#!/bin/sh\nfor a in \"$@\"; do last=\"$a\"; done\nstat -c '%a' \"$(dirname \"$last\")\" > {}/mode.txt\nsleep 31337 &\nwait\n", dir.display())).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let result = crate::convert::slides_to_pdf(b"PK", "deck.pptx", Some(&program), std::time::Duration::from_millis(600));
    std::thread::sleep(std::time::Duration::from_millis(300));
    let survivors = std::process::Command::new("pgrep").args(["-f", "sleep 31337"]).output().unwrap();
    let survivors = String::from_utf8_lossy(&survivors.stdout).lines().count();
    let mode = std::fs::read_to_string(dir.join("mode.txt")).unwrap_or_default();
    let left: Vec<String> = std::fs::read_dir(std::env::temp_dir()).unwrap().flatten().map(|entry| entry.file_name().to_string_lossy().to_string()).filter(|name| name.starts_with("study-app-convert-")).collect();
    println!("\n== slides: result {result:?}, {survivors} child process(es) still running, temp folder mode {}, folders left behind: {left:?}", mode.trim());
    let _ = std::process::Command::new("pkill").args(["-f", "sleep 31337"]).status();
}

// ---------- Hazard 1: a quiz in progress is saved as you go ----------

fn quiz_on(app: &mut App, topic: TopicId) -> QuizView {
    app.start_quiz(QuizSetupInput { topics: vec![topic], count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: false, total_minutes: None, per_question_seconds: None, easy: false, types: QuestionTypes::Mcq }).unwrap()
}

fn topic_with_questions(app: &mut App, count: usize) -> TopicId {
    let topic = app.create_topic("Resumable", None).unwrap();
    for n in 0..count {
        app.save_question(QuestionInput { topic, text: format!("Question {n}"), multiple: false, options: vec![OptionInput { text: "right".into(), correct: true }, OptionInput { text: "wrong".into(), correct: false }, OptionInput { text: "also wrong".into(), correct: false }], explanation: String::new(), tags: vec![], citations: vec![], short: None }).unwrap();
    }
    topic
}

#[test]
fn a_quiz_in_progress_survives_the_app_closing() {
    let dir = scratch("resume");
    let path = dir.join("study.sqlite");
    let topic = {
        let mut app = App::with_seed(Store::open(&path).unwrap(), 1);
        let topic = topic_with_questions(&mut app, 4);
        let first = quiz_on(&mut app, topic);
        assert_eq!((first.position, first.total), (1, 4));
        app.answer(vec![0], TODAY).unwrap();
        app.toggle_flag().unwrap();
        app.answer(vec![1], TODAY).unwrap();
        topic
        // The app goes away here without finishing: closed, crashed or killed.
    };
    let mut app = App::with_seed(Store::open(&path).unwrap(), 2);
    let resumed = app.current_question().expect("the quiz is still running after reopening");
    assert_eq!((resumed.position, resumed.total, resumed.topic), (3, 4, topic));
    assert_eq!(app.history().unwrap().len(), 0, "it isn't in history until it's finished");
    app.answer(vec![0], TODAY).unwrap();
    let last = app.answer(vec![0], TODAY).unwrap();
    let Next::Finished(attempt) = last.next else { panic!("four answers finish the quiz") };
    let results = app.results(attempt).unwrap();
    let tally = results.summary.overall;
    assert_eq!(tally.right_sure + tally.right_guessed + tally.wrong, 4, "the answers from before closing are counted: {tally:?}");
    assert_eq!(tally.unanswered, 0);
    drop(app);
    let app = App::with_seed(Store::open(&path).unwrap(), 3);
    assert!(app.current_question().is_err(), "a finished quiz doesn't come back");
    assert_eq!(app.history().unwrap().len(), 1);
}

#[test]
fn quitting_or_restoring_leaves_no_quiz_to_resume() {
    let dir = scratch("resume-quit");
    let path = dir.join("study.sqlite");
    {
        let mut app = App::with_seed(Store::open(&path).unwrap(), 1);
        app.set_backups(dir.join("backups"));
        let topic = topic_with_questions(&mut app, 3);
        let backup = app.back_up_now(at(TODAY, 600)).unwrap();
        quiz_on(&mut app, topic);
        app.answer(vec![0], TODAY).unwrap();
        app.quit_quiz(TODAY).unwrap();
        assert!(App::with_seed(Store::open(&path).unwrap(), 2).current_question().is_err(), "quit means gone");
        quiz_on(&mut app, topic);
        app.restore_backup(&backup.name, at(TODAY, 601)).unwrap();
    }
    let app = App::with_seed(Store::open(&path).unwrap(), 3);
    assert!(app.current_question().is_err(), "a restore doesn't bring back a quiz that was running");
}

#[test]
fn a_running_quiz_stays_on_this_computer() {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    let topic = topic_with_questions(&mut app, 2);
    let before = app.export(&ExportRequest { topics: None, history: true }, TODAY).unwrap().1;
    quiz_on(&mut app, topic);
    let (_, snapshot) = unpack(&app.export(&ExportRequest { topics: None, history: true }, TODAY).unwrap().1).unwrap();
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), serde_json::to_value(&unpack(&before).unwrap().1).unwrap(), "an export is the same with or without a quiz running");
}

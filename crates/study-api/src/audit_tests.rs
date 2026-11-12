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

// ---------- Hazard 7: damaged backup settings never thin out backups ----------

// backup.feature: Old backups are thinned out (R103). The settings file sits beside the
// backups. Cut short (a crash while it was being written, a sync app copying half of it)
// it used to read as "the defaults", and the next backup deleted everything the
// defaults don't keep.
#[test]
fn damaged_backup_settings_never_thin_out_backups() {
    let dir = scratch("settings");
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    app.set_backups(dir.join("backups"));
    app.create_topic("Caches", None).unwrap();
    let wanted = BackupSettings { keep_days: 60, keep_weeks: 4, keep_months: 12, copy_to: None };
    app.set_backup_settings(wanted.clone(), at(TODAY - 40, 600)).unwrap();
    for ago in (0..40).rev() {
        app.back_up_now(at(TODAY - ago - 1, 600)).unwrap();
    }
    assert_eq!(app.backups().unwrap().len(), 40);
    let file = dir.join("backups").join("settings.json");
    for damage in [&b"{\"keep_days\": 6"[..], b"", b"\0\0\0\0", b"[1, 2, 3]"] {
        std::fs::write(&file, damage).unwrap();
        app.back_up_now(at(TODAY, 600)).unwrap();
        assert!(app.backups().unwrap().len() >= 41, "nothing is deleted while the settings can't be read ({:?})", String::from_utf8_lossy(damage));
    }
    // Saving settings again makes the file whole, and thinning carries on from there.
    app.set_backup_settings(BackupSettings { keep_days: 5, keep_weeks: 0, keep_months: 0, copy_to: None }, at(TODAY, 601)).unwrap();
    assert!(app.backups().unwrap().len() < 20);
    assert_eq!(app.backup_status().unwrap().settings.keep_days, 5);
    assert!(std::fs::read_dir(dir.join("backups")).unwrap().flatten().all(|entry| !entry.file_name().to_string_lossy().contains("partial")), "settings are written whole, nothing half-written left");
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

// ---------- Hazard 2: nothing a command is given makes it panic ----------

fn no_panic<T>(what: &str, run: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|_| panic!("{what} panicked"))
}

#[test]
fn a_date_absurdly_far_away_is_refused_instead_of_breaking_the_calendar() {
    let mut app = seeded(1);
    let topic = app.topics().unwrap()[0].id;
    let date = |day: i64| crate::dates::DateSave { name: "Exam".into(), day, kind: "Exam".into(), module: None, topics: vec![topic], tighten: true };
    for day in [i64::MIN, i64::MAX, -1, TODAY + 400_000] {
        assert!(no_panic("add_date", || app.add_date(date(day))).is_err(), "day {day} is not a date anyone means");
        assert!(no_panic("preview_date", || app.preview_date(None, date(day), TODAY)).is_err());
    }
    let id = app.add_date(date(TODAY + 30)).unwrap();
    assert!(no_panic("update_date", || app.update_date(id, date(i64::MIN))).is_err());
    no_panic("dates", || app.dates(TODAY)).unwrap();
    no_panic("calendar_day", || app.calendar_day(TODAY, TODAY)).unwrap();
}

#[test]
fn a_quiz_timer_longer_than_time_itself_is_refused() {
    let mut app = seeded(1);
    let topic = app.topics().unwrap()[0].id;
    let setup = |total: Option<u64>, each: Option<u64>| QuizSetupInput { topics: vec![topic], count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: true, total_minutes: total, per_question_seconds: each, easy: false, types: QuestionTypes::Both };
    assert!(no_panic("start_quiz", || app.start_quiz(setup(Some(u64::MAX), None))).is_err());
    assert!(no_panic("start_quiz", || app.start_quiz(setup(None, Some(u64::MAX)))).is_err());
    assert!(no_panic("start_quiz", || app.start_quiz(setup(Some(20_000), None))).is_err(), "over a week");
    assert!(app.start_quiz(setup(Some(90), Some(30))).is_ok());
}

#[test]
fn a_clock_set_before_1970_doesnt_break_finishing_a_quiz_or_a_recall_check() {
    let mut app = seeded(1);
    let topic = app.topics().unwrap()[0].id;
    for day in [-1, -400] {
        app.start_quiz(QuizSetupInput { topics: vec![topic], count: Some(1), even: false, shuffle_questions: false, shuffle_options: false, exam: false, total_minutes: None, per_question_seconds: None, easy: false, types: QuestionTypes::Mcq }).unwrap();
        no_panic("quit_quiz", || app.quit_quiz(day)).unwrap();
        let checklist = app.recall_list(TODAY).unwrap()[0].id;
        let _ = no_panic("finish_check", || app.finish_check(checklist, None, vec![0], day));
        let _ = no_panic("preview_check", || app.preview_check(checklist, 1, day));
    }
}

#[test]
fn the_calendar_refuses_a_range_no_screen_would_ask_for() {
    let app = seeded(1);
    assert_eq!(app.calendar_month(TODAY - 14, TODAY + 27, TODAY).unwrap().len(), 42, "six weeks, as the month view asks");
    let started = std::time::Instant::now();
    for (from, to) in [(0, 9_000_000_000_000_000_000), (i64::MIN, i64::MAX), (TODAY, TODAY + 3_000_000), (TODAY, TODAY - 1)] {
        assert!(no_panic("calendar_month", || app.calendar_month(from, to, TODAY)).is_err(), "{from} to {to}");
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(1), "refused straight away, not after building the list");
}

#[test]
fn a_panic_in_one_command_doesnt_lock_the_app_for_good() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(seeded(1)));
    let held = state.clone();
    let _ = std::thread::spawn(move || {
        let _app = held.lock().unwrap();
        panic!("a bug in some command");
    })
    .join();
    assert!(state.lock().is_err(), "the lock is poisoned, which used to fail every later command");
    let app = crate::locked(&state);
    assert!(!app.topics().unwrap().is_empty(), "the app still answers");
}

// ---------- Hazard 10: a file with data the app can't read back is refused whole ----------

/// Every screen's list still loads. Returns what failed.
fn broken_screens(app: &mut App) -> Vec<String> {
    let mut broken = vec![];
    if let Some(panic) = exercise(app, TODAY) {
        broken.push(format!("PANIC: {}", panic.chars().take(140).collect::<String>()));
    }
    let all = study_store::search::SearchQuery { text: String::new(), tab: study_store::search::Tab::All, module: None, tag: None, sort: study_store::search::SortBy::Question, descending: false };
    for (what, result) in [("topics", app.topics().map(|_| ())), ("history", app.history().map(|_| ())), ("cards", app.cards_home(TODAY).map(|_| ())), ("recall", app.recall_list(TODAY).map(|_| ())), ("overview", app.overview(TODAY).map(|_| ())), ("projects", app.projects(TODAY).map(|_| ())), ("calendar", app.calendar_day(TODAY, TODAY).map(|_| ())), ("search", app.search(all).map(|_| ())), ("export", app.export(&ExportRequest { topics: None, history: true }, TODAY).map(|_| ()))] {
        if let Err(error) = result {
            broken.push(format!("{what}: {}", error.message.chars().take(110).collect::<String>()));
        }
    }
    broken
}

// transfer.feature: Files that can't be trusted (R118). Real exports with a few values
// damaged: wrong words where the app expects one of a set, negative counts, absurd days,
// text where a number goes. Whatever comes in, the app either refuses the file and keeps
// what it had, or imports it and every screen still works.
#[test]
fn a_damaged_import_is_refused_whole_or_leaves_every_screen_working() {
    let rounds: usize = std::env::var("ROUNDS").ok().and_then(|n| n.parse().ok()).unwrap_or(80);
    let (_, file) = seeded(1).export(&ExportRequest { topics: None, history: true }, TODAY).unwrap();
    let (manifest, snapshot) = unpack(&file).unwrap();
    let clean = serde_json::to_value(&snapshot).unwrap();
    let mut rng = Rng(0xA5A5_1234_5678_9ABC);
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
                Ok(Ok(_)) => accepted += 1,
            }
            for problem in app.store.health().unwrap() {
                note(format!("database: {problem}"));
            }
            for problem in broken_screens(&mut app) {
                note(format!("a screen fails afterwards, {problem}"));
            }
            if kind == "same data" && app.topics().map(|topics| topics.len()).unwrap_or(0) < 5 {
                note("topics that were there before are gone".into());
            }
        }
    }
    assert!(accepted > 20 && refused > 0, "the damage should hit both paths: {accepted} accepted, {refused} refused");
    assert!(findings.is_empty(), "{findings:#?}");
}

// ---------- Hazard 9: damaged lines in the sync folder never break this computer ----------

// sync.feature: lines another computer wrote arrive damaged, cut short, doubled, shuffled
// or with a line missing. Syncing may refuse them, but this computer's data stays sound
// and every screen still works.
#[test]
fn damaged_lines_in_the_sync_folder_never_break_this_computer() {
    let rounds: usize = std::env::var("ROUNDS").ok().and_then(|n| n.parse().ok()).unwrap_or(40);
    let mut rng = Rng(0x1357_9BDF_2468_ACE0);
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
            Ok(_) => {}
        }
        for problem in other.store.health().unwrap() {
            note(format!("database: {problem}"));
        }
        for problem in broken_screens(&mut other) {
            note(format!("a screen fails afterwards, {problem}"));
        }
        // Clean up what B wrote so the next round starts from A's folder alone.
        for entry in std::fs::read_dir(folder.join("changes")).unwrap().flatten() {
            if entry.path() != change_dir {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    assert!(findings.is_empty(), "{findings:#?}");
}

// ---------- Hazard 3: a backup that doesn't restore cleanly is undone ----------

// backup.feature: Restoring keeps a way back (R105). A backup file that is a database but
// not a sound one of this app's (damaged, or edited by hand) used to be reported as
// restored, leaving an app whose screens all failed.
#[test]
fn a_backup_that_doesnt_restore_cleanly_is_undone() {
    let dir = scratch("restore");
    let mut app = App::with_seed(Store::open(&dir.join("study.sqlite")).unwrap(), 1);
    app.set_backups(dir.join("backups"));
    crate::seed::seed_profile(&mut app, TODAY).unwrap();
    let before = serde_json::to_value(app.topics().unwrap()).unwrap();
    // A real SQLite file claiming an old version of this app, with a table that isn't ours.
    let bad = "study 2026-11-01 09-00 daily.sqlite";
    std::fs::create_dir_all(dir.join("backups")).unwrap();
    {
        let connection = study_store::backup::open_at_version(&dir.join("backups").join(bad), 3).unwrap();
        connection.execute_batch("DROP TABLE topics; CREATE TABLE topics (junk TEXT);").unwrap();
    }
    let error = app.restore_backup(bad, at(TODAY, 601)).expect_err("a backup that can't be used isn't reported as restored").message;
    assert!(error.contains("put back"), "{error}");
    assert_eq!(serde_json::to_value(app.topics().unwrap()).unwrap(), before, "everything is as it was");
    assert_eq!(app.store.health().unwrap(), Vec::<String>::new());
    assert_eq!(broken_screens(&mut app), Vec::<String>::new());
    // And a good backup still restores.
    let good = app.backups().unwrap().into_iter().find(|backup| backup.reason == crate::backup::Reason::BeforeRestore).expect("the copy made before the failed restore");
    app.restore_backup(&good.name, at(TODAY, 602)).unwrap();
    assert_eq!(serde_json::to_value(app.topics().unwrap()).unwrap(), before);
}


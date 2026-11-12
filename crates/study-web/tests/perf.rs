//! Performance on years of heavy use (hazard 21). Seeds a big database once (kept in the
//! temp folder: 3 years, about 170 topics, 4,600 questions, 1,600 quizzes, 43,000 card
//! reviews), then times every command a screen calls when it opens, and reads SQLite's
//! plan for each statement they run.
//!
//! Budgets, on the best of three runs so a busy machine doesn't fail it:
//! - a command a screen needs to open: 100 ms (PERF_SCREEN_MS)
//! - an action you asked for and wait on (a quiz across every topic, a backup, an
//!   export of everything): 2 seconds (PERF_ACTION_MS)
//!
//!   cargo test --release -p study-web --features trace --test perf -- --nocapture

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use study_api::App;
use study_api::practice::Clock;
use study_store::Store;
use study_web::dispatch::{Reply, dispatch_at};

const TODAY: i64 = 20_769;

fn years() -> i64 {
    std::env::var("PERF_YEARS").ok().and_then(|years| years.parse().ok()).unwrap_or(3)
}

fn big() -> App {
    let path = std::env::temp_dir().join(format!("study-perf-{}y.sqlite", years()));
    let fresh = !path.exists();
    let mut app = App::with_seed(Store::open(&path).unwrap(), 3);
    if fresh {
        let started = Instant::now();
        let summary = study_api::seed_large::seed_large_over(&mut app, TODAY, years() * 365).unwrap();
        println!("seeded in {:?}: {summary:?}", started.elapsed());
    }
    let dir = std::env::temp_dir().join(format!("study-perf-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    app.set_backups(dir.join("backups"));
    app.set_sync_home(&dir);
    app
}

fn run(app: &mut App, command: &str, args: Value) -> (Duration, usize, Value) {
    let clock = Clock { day: TODAY, minute: 600, unix: TODAY * 86_400 };
    let started = Instant::now();
    let reply = dispatch_at(app, command, &args, TODAY, clock).unwrap_or_else(|error| panic!("{command}: {}", error.message));
    let took = started.elapsed();
    match reply {
        Reply::Json(value) => (took, value.to_string().len(), value),
        Reply::Bytes(bytes) => (took, bytes.len(), Value::Null),
    }
}

#[test]
fn screens_on_years_of_use() {
    let mut app = big();
    app.store_mut().trace();
    let topics = run(&mut app, "topics", json!({})).2;
    let topic_ids: Vec<i64> = topics.as_array().unwrap().iter().map(|topic| topic["id"].as_i64().unwrap()).collect();
    let history = run(&mut app, "history", json!({})).2;
    let last_attempt = history.as_array().unwrap().iter().map(|row| row["id"].as_i64().unwrap()).max().unwrap();
    println!("{} topics, {} quizzes in history", topic_ids.len(), history.as_array().unwrap().len());
    Store::take_trace();

    let sync_folder = std::env::temp_dir().join(format!("study-perf-sync-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&sync_folder);
    std::fs::create_dir_all(&sync_folder).unwrap();
    let sync_folder = sync_folder.display().to_string();
    let search = |text: &str, tab: &str| json!({"query": {"text": text, "tab": tab, "module": null, "tag": null, "sort": "Question", "descending": false}});
    let calls: Vec<(&str, &str, Value)> = vec![
        ("Topics", "topics", json!({})),
        ("Topics", "modules", json!({})),
        ("History", "history", json!({})),
        ("Overview", "overview", json!({})),
        ("Overview", "due_today", json!({})),
        ("Results", "results", json!({"attempt": last_attempt})),
        ("Search", "search", search("", "All")),
        ("Search", "search", search("a", "All")),
        ("Search", "search", search("cache example", "Questions")),
        ("Bibliography", "sources", json!({})),
        ("Recall", "recall_list", json!({})),
        ("Recall", "recall_due_count", json!({})),
        ("Cards", "cards_home", json!({})),
        ("Cards", "cards_due_count", json!({})),
        ("Cards", "topic_cards", json!({"topic": topic_ids[0]})),
        ("Suggestions", "card_suggestions", json!({})),
        ("Projects", "projects", json!({})),
        ("Projects", "issues", json!({})),
        ("Projects", "drills_due", json!({})),
        ("Calendar", "calendar_month", json!({"from": TODAY - 14, "to": TODAY + 27})),
        ("Calendar", "calendar_day", json!({"day": TODAY})),
        ("Calendar", "calendar_day", json!({"day": TODAY + 3})),
        ("Calendar", "dates", json!({})),
        ("Quiz setup", "topic_quizzes", json!({})),
        ("Quiz setup", "quiz_queue", json!({})),
        ("Quiz", "start_quiz", json!({"setup": {"topics": topic_ids, "count": 20, "even": false, "shuffle_questions": true, "shuffle_options": true, "exam": false, "total_minutes": null, "per_question_seconds": null, "easy": false, "types": "Both"}})),
        ("Quiz", "quit_quiz", json!({})),
        ("Review", "start_review", json!({"scope": null})),
        ("Review", "grade_card", json!({"rating": "Good", "seconds": 5})),
        ("Your data", "backup_status", json!({})),
        ("Your data", "back_up_now", json!({})),
        ("Your data", "export_summary", json!({"request": {"topics": null, "history": true}})),
        ("Your data", "export_file", json!({"request": {"topics": null, "history": true}})),
        ("Your data", "sync_status", json!({})),
        // Turning sync on writes everything out once. After that each sync reads the
        // folder, merges, and reads every screen's data back before keeping the merge.
        ("Review", "leave_review", json!({})),
        ("Your data", "sync_setup", json!({"setup": {"folder": sync_folder, "name": "Perf"}})),
        ("Your data", "sync_now", json!({"auto": false})),
    ];
    let mut plans: BTreeMap<String, (Vec<String>, Duration, usize, Vec<String>)> = BTreeMap::new();
    println!("\n{:<13}{:<20}{:>10}{:>12}{:>8}", "screen", "command", "ms", "bytes", "stmts");
    let budget = |name: &str, default: u64| Duration::from_millis(std::env::var(name).ok().and_then(|ms| ms.parse().ok()).unwrap_or(default));
    let (screen_budget, action_budget) = (budget("PERF_SCREEN_MS", 100), budget("PERF_ACTION_MS", 2000));
    let mut over: Vec<String> = vec![];
    for (screen, command, args) in calls {
        // Commands that change something run once. The rest are timed three times.
        let once = ["start_quiz", "quit_quiz", "start_review", "grade_card", "back_up_now", "sync_setup"].contains(&command);
        let (mut took, bytes, _) = run(&mut app, command, args.clone());
        let statements = Store::take_trace();
        for _ in 0..if once { 0 } else { 2 } {
            took = took.min(run(&mut app, command, args.clone()).0);
            Store::take_trace();
        }
        let allowed = if command == "sync_setup" { Duration::from_secs(60) } else if ["start_quiz", "back_up_now", "export_summary", "export_file", "sync_now"].contains(&command) { action_budget } else { screen_budget };
        println!("{screen:<13}{command:<20}{:>10.1}{bytes:>12}{:>8}", took.as_secs_f64() * 1000.0, statements.len());
        if took > allowed {
            over.push(format!("{screen}: {command} took {:.0} ms, over its {} ms", took.as_secs_f64() * 1000.0, allowed.as_millis()));
        }
        for (sql, spent) in statements {
            let entry = plans.entry(sql.clone()).or_insert_with(|| (vec![], Duration::ZERO, 0, vec![]));
            entry.1 += spent;
            entry.2 += 1;
            if !entry.3.contains(&command.to_string()) {
                entry.3.push(command.to_string());
            }
            if entry.0.is_empty() && !sql.trim_start().to_uppercase().starts_with("VACUUM") {
                entry.0 = app.store_mut().query_plan(&sql).unwrap_or_else(|error| vec![format!("(no plan: {error})")]);
            }
        }
    }
    // The read-back check that guards imports, syncs that bring changes, and restores.
    let started = Instant::now();
    assert_eq!(app.check_data(TODAY), None);
    let took = started.elapsed();
    println!("{:<13}{:<20}{:>10.1}", "Your data", "read-back check", took.as_secs_f64() * 1000.0);
    if took > action_budget {
        over.push(format!("the read-back check took {:.0} ms, over its {} ms", took.as_secs_f64() * 1000.0, action_budget.as_millis()));
    }
    println!("\nover budget: {over:?}");
    let mut ranked: Vec<_> = plans.iter().collect();
    ranked.sort_by_key(|(_, (_, spent, _, _))| std::cmp::Reverse(*spent));
    println!("\nstatements by total time (top 25), with full scans marked:");
    for (sql, (plan, spent, count, commands)) in ranked.iter().take(25) {
        let scans: Vec<&String> = plan.iter().filter(|step| step.starts_with("SCAN")).collect();
        let short: String = sql.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(150).collect();
        println!("{:>9.1}ms x{count:<6} {commands:?}\n      {short}\n      scans: {scans:?}", spent.as_secs_f64() * 1000.0);
    }
    assert!(over.is_empty(), "{over:#?}");
}

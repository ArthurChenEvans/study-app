//! Every command with hostile arguments, on a seeded in-memory app (hazard 2). A command
//! may refuse what it's given, but it must never panic, since a panic while the app is
//! locked used to leave every later command failing until a restart, and never hang.
//!
//! The command list and argument names are read from dispatch.rs, so a new command is
//! covered as soon as it exists. Values start from valid shapes (and from whatever the
//! commands themselves return) and are then damaged: huge and negative numbers, empty
//! and enormous strings, odd Unicode, half-written cloze markers, wrong types.
//!
//! A longer run: FUZZ_ITERS=200000 FUZZ_SEED=7 cargo test -p study-web --test commands_fuzz -- --nocapture

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Value, json};
use study_api::App;
use study_api::practice::Clock;
use study_store::Store;
use study_web::dispatch::{Reply, dispatch_at};

const TODAY: i64 = 20_769;

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
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// command -> argument names, read from dispatch.rs itself so new commands are covered.
fn commands() -> BTreeMap<String, Vec<String>> {
    let source = include_str!("../src/dispatch.rs");
    let mut found = BTreeMap::new();
    let body = source.split("match command {").nth(1).unwrap();
    let mut current: Option<String> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('"') && trimmed.contains("=>") {
            let name = trimmed[1..].split('"').next().unwrap().to_string();
            found.insert(name.clone(), vec![]);
            current = Some(name);
        }
        let Some(name) = &current else { continue };
        for piece in line.split("(a, \"").skip(1) {
            let arg = piece.split('"').next().unwrap();
            let arg = if line.contains("base64(a, ") && piece.starts_with(arg) && line.split("base64(a, \"").nth(1).is_some_and(|rest| rest.starts_with(arg)) { "data" } else { arg };
            found.get_mut(name).unwrap().push(arg.to_string());
        }
    }
    found
}

fn nasty_strings() -> Vec<Value> {
    let mut strings: Vec<String> = ["", " ", "\u{0}", "💥", "é", "{{c1::x}}", "{{c", "{{c1::", "{{c1::é", "}}{{c9999999999999999999::a}}", "{{c0::a}}", "{{c-1::a}}", "$", "$$", "'; DROP TABLE topics; --", "../../etc/passwd", "/nonexistent", "/nonexistent/x", "\u{202e}abc", "a\u{301}\u{301}\u{301}", "%", "_", "\\", "\"", "*", "a b c", "Caches", "2026-11-12", "study 2026-11-04 09-12 daily.sqlite", "c1", "question:1", "note:1", "0", "-1", "\n\n", "\t", "ß", "İ", "ǅ", "ﬃ"]
        .into_iter()
        .map(String::from)
        .collect();
    strings.push("a".repeat(100_000));
    strings.push("é".repeat(5_000));
    strings.push("{{c1::".repeat(2_000));
    strings.push("word ".repeat(5_000));
    strings.into_iter().map(Value::String).collect()
}

fn numbers() -> Vec<Value> {
    vec![json!(0), json!(1), json!(2), json!(3), json!(4), json!(5), json!(7), json!(50), json!(-1), json!(-2), json!(i64::MAX), json!(i64::MIN), json!(u32::MAX), json!(u64::MAX), json!(1u64 << 53), json!(0.5), json!(0.9), json!(1.5), json!(-0.1), json!(1e30), json!(1e308), json!(TODAY), json!(TODAY + 1), json!(TODAY - 1), json!(TODAY + 40_000_000), json!(255), json!(256), json!(65_536), json!(100), json!(101), json!(3_000_000)]
}

fn templates() -> Vec<Value> {
    vec![
        json!({"topics": [1, 2], "count": 5, "even": false, "shuffle_questions": true, "shuffle_options": true, "exam": false, "total_minutes": null, "per_question_seconds": null, "easy": false, "types": "Both"}),
        json!({"topics": [1], "count": null, "even": true, "shuffle_questions": false, "shuffle_options": false, "exam": true, "total_minutes": 1, "per_question_seconds": 5, "easy": true, "types": "Short"}),
        json!({"topic": 1, "text": "What is a cache?", "multiple": false, "options": [{"text": "a", "correct": true}, {"text": "b", "correct": false}, {"text": "c", "correct": false}], "explanation": "because", "tags": ["x"], "citations": [{"source": 1, "location": "p. 3"}], "short": null}),
        json!({"topic": 1, "text": "Name it", "multiple": false, "options": [], "explanation": "", "tags": [], "citations": [], "short": {"check": {"Accepted": {"answers": ["cache"], "typos": true}}, "model_answer": "cache"}}),
        json!({"topic": 1, "text": "Explain", "multiple": false, "options": [], "explanation": "", "tags": [], "citations": [], "short": {"check": {"Keywords": {"keywords": [{"word": "fast", "also": ["quick"]}], "needed": 1}}, "model_answer": "fast"}}),
        json!({"Accepted": {"answers": ["cache"], "typos": true}}),
        json!({"Keywords": {"keywords": [{"word": "fast", "also": ["quick"]}], "needed": null}}),
        json!({"authors": "A", "year": "2020", "title": "T", "edition": null, "description": null, "publisher": null, "url": null}),
        json!({"Module": 1}),
        json!({"Topic": 1}),
        json!([{"text": "a point", "question": null, "note": null}, {"text": "another", "question": 1, "note": 1}]),
        json!([{"Question": 1}, {"Note": 1}]),
        json!({"note": {"topic": 1, "kind": "Basic", "front": "f", "back": "b", "cues": ["c"], "text": "", "extra": ""}, "suggestion": null}),
        json!({"note": {"topic": 1, "kind": "Cloze", "front": "", "back": "", "cues": [], "text": "a {{c1::b}} c {{c2::d}}", "extra": "more"}, "suggestion": null}),
        json!({"note": {"topic": 1, "kind": "Reversed", "front": "f", "back": "b", "cues": [], "text": "", "extra": ""}, "suggestion": "question:1"}),
        json!({"name": "Exam", "day": TODAY + 10, "kind": "Exam", "module": 1, "topics": [], "tighten": true}),
        json!({"name": "Test", "day": TODAY + 3, "kind": "Deadline", "module": null, "topics": [1, 2], "tighten": true}),
        json!({"issue": "pointers", "note": "why", "kind": "Apply", "topic": 1}),
        json!({"what": "pointers", "why": "hard", "kind": "Recall", "topic": null}),
        json!("Card"),
        json!("Nothing"),
        json!({"Checklist": {"checklist": 1}}),
        json!({"NewDrill": {"issue": "x", "task": "do", "hints": "", "topic": 1, "day": TODAY + 1}}),
        json!({"Drill": {"issue": "x", "task": "do", "hints": "", "topic": 1, "day": TODAY + 1}}),
        json!({"MoveDrill": {"issue": "x", "drill": 1, "day": TODAY + 1}}),
        json!({"topics": null, "history": true}),
        json!({"topics": [1], "history": false}),
        json!({"rule": "Newer", "theirs": []}),
        
        json!({"keep_days": 7, "keep_weeks": 4, "keep_months": 12, "copy_to": null}),
        json!({"text": "cache", "tab": "All", "module": null, "tag": null, "sort": "Question", "descending": false}),
        json!({"text": "", "tab": "Questions", "module": 1, "tag": "x", "sort": "Edited", "descending": true}),
        json!("Again"), json!("Hard"), json!("Good"), json!("Easy"),
        json!("Struggled"), json!("Shaky"), json!("Solid"),
        json!("Blank"), json!({"Other": "because"}),
        json!([0]), json!([0, 1]), json!([]), json!([1, 1, 1]), json!([9999999]),
        json!(true), json!(false), json!(null),
    ]
}

/// Valid shapes for an argument, by command and argument name, so most calls get past
/// argument parsing and reach the code behind it.
fn shapes(scratch: &str) -> BTreeMap<String, Vec<Value>> {
    let question = vec![
        json!({"topic": 1, "text": "What is a cache?", "multiple": false, "options": [{"text": "a", "correct": true}, {"text": "b", "correct": false}, {"text": "c", "correct": false}], "explanation": "because", "tags": ["x"], "citations": [{"source": 1, "location": "p. 3"}], "short": null}),
        json!({"topic": 1, "text": "Name it", "multiple": false, "options": [], "explanation": "", "tags": [], "citations": [], "short": {"check": {"Accepted": {"answers": ["cache"], "typos": true}}, "model_answer": "cache"}}),
        json!({"topic": 1, "text": "Explain", "multiple": false, "options": [], "explanation": "", "tags": [], "citations": [], "short": {"check": {"Keywords": {"keywords": [{"word": "fast", "also": ["quick"]}], "needed": 1}}, "model_answer": "fast"}}),
    ];
    let note = vec![
        json!({"note": {"topic": 1, "kind": "Basic", "front": "f", "back": "b", "cues": ["c"], "text": "", "extra": ""}, "suggestion": null}),
        json!({"note": {"topic": 1, "kind": "Cloze", "front": "", "back": "", "cues": [], "text": "a {{c1::b}} c {{c2::d}}", "extra": "more"}, "suggestion": null}),
        json!({"note": {"topic": 1, "kind": "Reversed", "front": "f", "back": "b", "cues": [], "text": "", "extra": ""}, "suggestion": "question:1"}),
    ];
    let quiz = vec![
        json!({"topics": [1, 2], "count": 5, "even": false, "shuffle_questions": true, "shuffle_options": true, "exam": false, "total_minutes": null, "per_question_seconds": null, "easy": false, "types": "Both"}),
        json!({"topics": [1], "count": null, "even": true, "shuffle_questions": false, "shuffle_options": false, "exam": true, "total_minutes": 1, "per_question_seconds": 5, "easy": true, "types": "Short"}),
    ];
    let date = vec![
        json!({"name": "Exam", "day": TODAY + 10, "kind": "Exam", "module": 1, "topics": [], "tighten": true}),
        json!({"name": "Test", "day": TODAY + 3, "kind": "Deadline", "module": null, "topics": [1, 2], "tighten": true}),
    ];
    let subject = vec![json!({"Module": 1}), json!({"Topic": 1}), json!(null)];
    let items = vec![json!([{"Question": 1}, {"Note": 1}]), json!([]), json!([{"Note": 2}])];
    let numbers_list = vec![json!([0]), json!([0, 1]), json!([]), json!([1, 1, 1]), json!([9999999])];
    let text = vec![json!("Caches"), json!("pointers"), json!(""), json!("x y")];
    let flag = vec![json!(true), json!(false)];
    let mut shapes: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut put = |keys: &[&str], values: &Vec<Value>| {
        for key in keys {
            shapes.insert(key.to_string(), values.clone());
        }
    };
    put(&["save_question.input", "update_question.input"], &question);
    put(&["save_note.input"], &note);
    put(&["log_struggle.input"], &vec![json!({"what": "pointers", "why": "hard", "kind": "Recall", "topic": null}), json!({"what": "loops", "why": "", "kind": "Apply", "topic": 1})]);
    put(&["start_quiz.setup"], &quiz);
    put(&["sync_setup.setup"], &vec![json!({"folder": format!("{scratch}/sync"), "name": "Laptop"}), json!({"folder": "/nonexistent/sync", "name": ""})]);
    put(&["add_date.date", "update_date.date", "preview_date.date"], &date);
    put(&["create_checklist.subject", "start_review.scope"], &subject);
    put(&["save_points.points"], &vec![json!([{"text": "a point", "question": null, "note": null}, {"text": "another", "question": 1, "note": 1}]), json!([])]);
    put(&["add_from_items.add", "add_from_items.remove"], &items);
    put(&["try_short_answer.check"], &vec![json!({"Accepted": {"answers": ["cache"], "typos": true}}), json!({"Keywords": {"keywords": [{"word": "fast", "also": ["quick"]}], "needed": null}})]);
    put(&["add_source.fields", "update_source.fields"], &vec![json!({"authors": "A", "year": "2020", "title": "T", "edition": null, "description": null, "publisher": null, "url": null})]);
    put(&["give_reason.reason"], &vec![json!("Blank"), json!("CarelessSlip"), json!({"Other": "because"})]);
    put(&["grade_card.rating"], &vec![json!("Again"), json!("Hard"), json!("Good"), json!("Easy")]);
    put(&["rate_drill.rating"], &vec![json!("Struggled"), json!("Shaky"), json!("Solid")]);
    put(&["review_struggle.choice"], &vec![json!("Card"), json!("Nothing"), json!({"Checklist": {"checklist": 1}}), json!({"NewDrill": {"issue": "x", "task": "do", "hints": "", "topic": 1, "day": TODAY + 1}}), json!({"Drill": {"issue": "x", "task": "do", "hints": "", "topic": 1, "day": TODAY + 1}}), json!({"MoveDrill": {"issue": "x", "drill": 1, "day": TODAY + 1}})]);
    put(&["import_apply.choice"], &vec![json!({"rule": "Newer", "theirs": []}), json!({"rule": "EachOne", "theirs": ["x"]})]);
    put(&["export_summary.request", "export_file.request"], &vec![json!({"topics": null, "history": true}), json!({"topics": [1], "history": false})]);
    put(&["search.query"], &vec![json!({"text": "cache", "tab": "All", "module": null, "tag": null, "sort": "Question", "descending": false}), json!({"text": "", "tab": "Questions", "module": 1, "tag": "x", "sort": "Edited", "descending": true})]);
    put(&["set_backup_settings.settings"], &vec![json!({"keep_days": 7, "keep_weeks": 4, "keep_months": 12, "copy_to": null}), json!({"keep_days": 1, "keep_weeks": 0, "keep_months": 0, "copy_to": format!("{scratch}/copies")})]);
    put(&["answer.picked", "finish_check.recalled"], &numbers_list);
    put(&["set_topic_tags.tags"], &vec![json!(["a", "b"]), json!([])]);
    put(&["archive_card.archived", "sync_now.auto", "sync_settle.other"], &flag);
    put(&["create_module.name", "create_topic.name", "create_project.name", "create_project.kind", "answer_text.typed", "try_short_answer.typed", "finish_check.dump", "rate_drill.note", "known_drill.issue", "dismiss_suggestion.key", "sync_rename.name", "restore_preview.name", "restore_backup.name", "import_preview.name"], &text);
    shapes
}

/// Changes one leaf (or shape) somewhere in the value.
fn mutate(value: &mut Value, rng: &mut Rng, strings: &[Value], numbers: &[Value], depth: usize) {
    match value {
        Value::Number(_) => *value = rng.pick(numbers).clone(),
        Value::String(_) => *value = rng.pick(strings).clone(),
        Value::Bool(flag) => *flag = !*flag,
        Value::Null => {
            if rng.below(2) == 0 {
                *value = rng.pick(numbers).clone();
            }
        }
        Value::Array(items) => match rng.below(5) {
            0 => items.clear(),
            1 if !items.is_empty() => {
                let copy = items[0].clone();
                let many = [2usize, 6, 50, 3000][rng.below(4)];
                items.extend(std::iter::repeat_n(copy, many));
            }
            _ if !items.is_empty() && depth < 6 => {
                let index = rng.below(items.len());
                mutate(&mut items[index], rng, strings, numbers, depth + 1);
            }
            _ => items.push(rng.pick(numbers).clone()),
        },
        Value::Object(map) => {
            if map.is_empty() || depth >= 6 {
                return;
            }
            let key = map.keys().nth(rng.below(map.len())).unwrap().clone();
            mutate(map.get_mut(&key).unwrap(), rng, strings, numbers, depth + 1);
        }
    }
}

fn harvest(value: &Value, pool: &mut Vec<Value>, depth: usize) {
    if pool.len() > 4000 || depth > 3 {
        return;
    }
    match value {
        Value::Object(map) => {
            if value.to_string().len() < 4000 {
                pool.push(value.clone());
            }
            for inner in map.values() {
                harvest(inner, pool, depth + 1);
            }
        }
        Value::Array(items) => {
            if !items.is_empty() && value.to_string().len() < 4000 {
                pool.push(value.clone());
            }
            for inner in items.iter().take(3) {
                harvest(inner, pool, depth + 1);
            }
        }
        _ => {}
    }
}

fn fresh(tag: &str) -> App {
    let dir = std::env::temp_dir().join(format!("study-fuzz-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = App::with_seed(Store::in_memory().unwrap(), 7);
    app.set_backups(dir.join("backups"));
    app.set_sync_home(&dir);
    study_api::seed::seed_profile(&mut app, TODAY).unwrap();
    app
}

#[test]
fn no_command_panics_or_hangs_on_hostile_arguments() {
    fuzz(false, 0x9E37_79B9_7F4A_7C15);
}

/// The same with the computer's clock wrong: a day early or late, years out, or reset to 1970.
#[test]
fn no_command_panics_when_the_clock_is_wrong() {
    fuzz(true, 777);
}

fn fuzz(odd_clock: bool, default_seed: u64) {
    let iterations: usize = std::env::var("FUZZ_ITERS").ok().and_then(|n| n.parse().ok()).unwrap_or(12_000);
    // fit_scheduler trains a model, which takes seconds by design.
    let skip: Vec<String> = std::env::var("FUZZ_SKIP").unwrap_or_else(|_| "fit_scheduler".into()).split(',').map(String::from).collect();
    let seed: u64 = std::env::var("FUZZ_SEED").ok().and_then(|n| n.parse().ok()).unwrap_or(default_seed);
    let commands = commands();
    let names: Vec<&String> = commands.keys().filter(|name| !skip.contains(name)).collect();
    let strings = nasty_strings();
    let numbers = numbers();
    let fixed = templates();
    let mut pool: Vec<Value> = vec![];
    let mut arg_errors: BTreeMap<String, String> = BTreeMap::new();
    let scratch = std::env::temp_dir().join(format!("study-fuzz-{}-{}-folders", std::process::id(), if odd_clock { "clock" } else { "args" }));
    std::fs::create_dir_all(scratch.join("sync")).unwrap();
    std::fs::create_dir_all(scratch.join("copies")).unwrap();
    let mut learned: BTreeMap<String, Vec<Value>> = shapes(&scratch.display().to_string());
    let mut rng = Rng(seed);
    let tag = if odd_clock { "clock" } else { "args" };
    let mut app = fresh(tag);
    // Settings and schedules: start from what the app itself hands out.
    for (getter, setter) in [("card_settings", "set_card_settings.settings"), ("recall_schedule", "set_recall_schedule.schedule"), ("quiz_schedule", "set_quiz_schedule.schedule")] {
        if let Ok(Reply::Json(value)) = dispatch_at(&mut app, getter, &json!({}), TODAY, Clock { day: TODAY, minute: 600, unix: TODAY * 86_400 }) {
            learned.insert(setter.to_string(), vec![value]);
        }
    }
    let mut panics: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut slow: BTreeMap<String, (u128, String)> = BTreeMap::new();
    let mut reached: BTreeMap<String, usize> = BTreeMap::new();
    let trace = std::env::temp_dir().join(format!("study-fuzz-last-call-{tag}.txt"));
    let days = [TODAY, TODAY, TODAY, TODAY, TODAY + 1, TODAY - 1, TODAY + 400, TODAY - 400, TODAY + 36_500, 0, -1, 3_000_000];
    for iteration in 0..iterations {
        let name = *rng.pick(&names);
        let mut args = serde_json::Map::new();
        for arg in &commands[name] {
            let key = format!("{name}.{arg}");
            let id_like = ["id", "project", "topic", "module", "attempt", "index", "n", "day", "from", "to", "count", "seconds", "recalled", "drill"].contains(&arg.as_str());
            let mut value = match rng.below(10) {
                _ if id_like && rng.below(10) < 8 => rng.pick(&numbers).clone(),
                _ if learned.get(&key).is_some_and(|known: &Vec<Value>| !known.is_empty()) && rng.below(4) != 0 => rng.pick(&learned[&key]).clone(),
                0 => rng.pick(&strings).clone(),
                1 | 2 => rng.pick(&numbers).clone(),
                3..=6 => rng.pick(&fixed).clone(),
                _ if pool.is_empty() => rng.pick(&fixed).clone(),
                _ => rng.pick(&pool).clone(),
            };
            if rng.below(3) == 0 {
                mutate(&mut value, &mut rng, &strings, &numbers, 0);
            }
            args.insert(arg.clone(), value);
        }
        let args = Value::Object(args);
        let sent = args.clone();
        let day = if odd_clock { *rng.pick(&days) } else { TODAY };
        let clock = Clock { day, minute: if odd_clock { [0u32, 1439, 1440, u32::MAX][rng.below(4)] } else { 600 }, unix: day.wrapping_mul(86_400) };
        let shown: String = args.to_string().chars().take(600).collect();
        if iteration % 50 == 0 || std::env::var("FUZZ_TRACE").is_ok() {
            let _ = std::fs::write(&trace, format!("{name} day={day} {shown}"));
        }
        let started = std::time::Instant::now();
        let outcome = catch_unwind(AssertUnwindSafe(|| dispatch_at(&mut app, name, &args, day, clock)));
        let took = started.elapsed().as_millis();
        if took > 2000 && slow.get(name).is_none_or(|(worst, _)| took > *worst) {
            slow.insert(name.clone(), (took, format!("day={day} {shown}")));
        }
        let parsed = !matches!(&outcome, Ok(Err(error)) if error.message.contains("argument is missing or wrong"));
        if parsed && let Value::Object(map) = &sent {
            for (arg, value) in map {
                let known = learned.entry(format!("{name}.{arg}")).or_default();
                if known.len() < 40 && value.to_string().len() < 3000 {
                    known.push(value.clone());
                }
            }
        }
        match outcome {
            Ok(Ok(Reply::Json(value))) => {
                *reached.entry(name.clone()).or_default() += 1;
                harvest(&value, &mut pool, 0);
            }
            Ok(Ok(Reply::Bytes(_))) => *reached.entry(name.clone()).or_default() += 1,
            Ok(Err(error)) => {
                if !error.message.contains("argument is missing or wrong") {
                    *reached.entry(name.clone()).or_default() += 1;
                } else {
                    arg_errors.insert(name.clone(), error.message.chars().take(200).collect());
                }
            }
            Err(payload) => {
                let message = payload.downcast_ref::<String>().cloned().or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string())).unwrap_or_default();
                let entry = panics.entry(format!("{name}: {}", message.chars().take(160).collect::<String>())).or_insert((0, format!("day={day} {shown}")));
                entry.0 += 1;
                // State may be half-changed after a panic, so start again from a clean app.
                app = fresh(tag);
            }
        }
    }
    let never: Vec<&&String> = names.iter().filter(|name| !reached.contains_key(**name)).collect();
    println!("\n== {} commands, {iterations} calls, pool {}", names.len(), pool.len());
    println!("== never got past argument parsing: {never:?}");
    for name in &never {
        println!("   {name}: {}", arg_errors.get(**name).cloned().unwrap_or_default());
    }
    println!("== slow calls (>2s):");
    for (name, (took, example)) in &slow {
        println!("   {name} {took}ms  {example}");
    }
    println!("== panics: {}", panics.len());
    for (what, (count, example)) in &panics {
        println!("   [{count}x] {what}\n        {example}");
    }
    assert!(panics.is_empty(), "{} kinds of panic, the first: {:?}", panics.len(), panics.iter().next());
    assert!(slow.is_empty(), "calls that took over 2 seconds: {slow:?}");
    assert!(never.is_empty(), "the fuzzer never got a valid call through to {never:?}, teach it their argument shapes");
}

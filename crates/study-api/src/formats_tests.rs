//! import-export.feature: spreadsheets, JSON, Markdown checklists and Moodle questions
//! (R174 to R176), through the command layer.

use super::*;
use crate::anki::DeckTarget;
use crate::cards::NoteInput;
use crate::formats::{CsvKind, CsvMapping, ItemsChoice, Role};
use crate::practice::Clock;
use crate::transfer::{ExportFormat, ExportRequest};
use study_core::cards::{Note, NoteKind};
use study_core::short::{Check, Keyword, ShortAnswer};

const TODAY: i64 = 20_771;

fn at(day: i64) -> Clock {
    Clock { day, minute: 600, unix: day * 86_400 + 36_000 }
}

fn app() -> App {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 9);
    app.set_backups(std::env::temp_dir().join(format!("study-formats-{}-{}", std::process::id(), rand::random::<u32>())));
    app
}

fn mcq(topic: TopicId, text: &str, options: &[(&str, bool)], explanation: &str) -> QuestionInput {
    QuestionInput {
        topic,
        text: text.into(),
        multiple: options.iter().filter(|(_, right)| *right).count() > 1,
        options: options.iter().map(|(text, correct)| OptionInput { text: (*text).into(), correct: *correct }).collect(),
        explanation: explanation.into(),
        tags: vec![],
        citations: vec![],
        short: None,
    }
}

fn written() -> App {
    let mut app = app();
    let module = app.create_module("Computer Architecture").unwrap();
    let cpu = app.create_topic("CPU", Some(module)).unwrap();
    app.save_question(mcq(cpu, "Which design reads instructions and data at once?", &[("Von Neumann", false), ("Harvard", true), ("RISC", false)], "Separate buses")).unwrap();
    app.save_question(mcq(cpu, "Which are registers?", &[("PC", true), ("ACC", true), ("ALU", false), ("Bus", false)], "")).unwrap();
    let mut short = mcq(cpu, "What does ALU stand for?", &[], "");
    short.short = Some(ShortAnswer { check: Check::Keywords { keywords: vec![Keyword { word: "arithmetic".into(), also: vec![] }, Keyword { word: "logic".into(), also: vec!["logical".into()] }], needed: None }, model_answer: "Arithmetic Logic Unit".into() });
    app.save_question(short).unwrap();
    app.save_note(None, NoteInput { note: Note { topic: cpu, kind: NoteKind::Cloze, front: String::new(), back: String::new(), cues: vec![], text: "The {{c1::program counter}} holds the next address".into(), extra: String::new(), tags: vec![] }, suggestion: None }, TODAY).unwrap();
    app.save_note(None, NoteInput { note: Note { topic: cpu, kind: NoteKind::Basic, front: "Clock speed unit?".into(), back: "Hertz".into(), cues: vec![], text: String::new(), extra: String::new(), tags: vec![] }, suggestion: None }, TODAY).unwrap();
    let checklist = app.create_checklist(study_store::recall::Subject::Topic(cpu), TODAY).unwrap();
    app.save_points(checklist, vec![crate::recall::PointInput { text: "Fetch, decode, execute".into(), question: None, note: None }]).unwrap();
    app
}

fn export(app: &App, format: ExportFormat) -> (String, String) {
    let (name, bytes) = app.export(&ExportRequest { topics: None, history: false, format, quizzes_as_cards: false }, TODAY).unwrap();
    (name, String::from_utf8(bytes).unwrap())
}

fn import(into: &mut App, name: &str, text: &str, mapping: Option<CsvMapping>) -> crate::formats::ItemsPreview {
    let preview = into.items_preview(text.as_bytes(), name, mapping).unwrap();
    into.items_apply(ItemsChoice { targets: vec![] }, at(TODAY)).unwrap();
    preview
}

fn questions_in(app: &App, topic: &str) -> Vec<QuestionInput> {
    let id = app.topics().unwrap().into_iter().find(|row| row.name == topic).unwrap_or_else(|| panic!("no topic {topic}")).id;
    app.questions_in(id).unwrap().into_iter().map(|(question, _)| app.question(question).unwrap()).collect()
}

#[test]
fn json_carries_questions_cards_and_checklists_both_ways() {
    let app = written();
    let (name, text) = export(&app, ExportFormat::Json);
    assert!(name.ends_with(".json"));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["format"], "shudhana");
    let mut other = self::app();
    let preview = import(&mut other, &name, &text, None);
    assert_eq!(preview.format, "json");
    assert_eq!((preview.topics[0].name.as_str(), preview.topics[0].questions, preview.topics[0].cards), ("CPU", 3, 2));
    assert_eq!(preview.topics[0].target, DeckTarget::NewTopic { name: "CPU".into(), module: Some("Computer Architecture".into()) });
    let questions = questions_in(&other, "CPU");
    assert_eq!(questions.len(), 3);
    assert!(questions.iter().any(|question| question.text == "What does ALU stand for?" && question.short.is_some()));
    let again = other.items_preview(text.as_bytes(), &name, None).unwrap();
    assert_eq!(again.already_here, again.ready, "importing it again adds nothing new");
}

#[test]
fn a_question_spreadsheet_is_matched_by_its_column_names() {
    let csv = "question,a,b,c,answer,why,notes\n\
        Which design reads instructions and data at once?,Von Neumann,Harvard,RISC,B,Separate buses,lecture 3\n\
        What does the ALU do?,Arithmetic and logic,Stores data,Fetches,A,,\n\
        Which register holds the next address?,MAR,PC,ACC,2,Program counter,check\n\
        Speedup of 60% sped up 4x?,1.82,2.5,1.45,1.82,Amdahl's law,\n\
        Which cache level is fastest?,L1,L2,L3,E,,\n\
        ,a,b,c,A,,\n";
    let mut app = app();
    let cpu = app.create_topic("CPU", None).unwrap();
    let preview = app.items_preview(csv.as_bytes(), "cpu-questions.csv", None).unwrap();
    let info = preview.csv.as_ref().unwrap();
    assert_eq!((info.kind, info.header, info.total_rows), (CsvKind::Mcq, true, 6));
    assert_eq!(info.roles, vec![Role::Question, Role::Option, Role::Option, Role::Option, Role::Correct, Role::Explanation, Role::Ignore]);
    assert_eq!(preview.ready, 4);
    let problems: Vec<(String, String)> = preview.problems.iter().map(|problem| (problem.at.clone(), problem.why.clone())).collect();
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].0 == "Row 6" && problems[0].1.contains("E") && problems[0].1.contains("3 options"), "{problems:?}");
    assert!(problems[1].0 == "Row 7" && problems[1].1.contains("question is empty"), "{problems:?}");
    app.items_apply(ItemsChoice { targets: vec![("cpu-questions".into(), DeckTarget::Topic { topic: cpu })] }, at(TODAY)).unwrap();
    let questions = questions_in(&app, "CPU");
    assert_eq!(questions.len(), 4);
    let harvard = questions.iter().find(|question| question.text.starts_with("Which design")).unwrap();
    assert_eq!(harvard.options.iter().filter(|option| option.correct).map(|option| option.text.as_str()).collect::<Vec<_>>(), vec!["Harvard"]);
    assert_eq!(harvard.explanation, "Separate buses");
    let pc = questions.iter().find(|question| question.text.starts_with("Which register")).unwrap();
    assert!(pc.options.iter().any(|option| option.text == "PC" && option.correct), "a number counts from 1");
    let amdahl = questions.iter().find(|question| question.text.starts_with("Speedup")).unwrap();
    assert!(amdahl.options.iter().any(|option| option.text == "1.82" && option.correct), "or the answer itself");

    // Fixed and imported again: what came in the first time isn't doubled up.
    let fixed = csv.replace(",L1,L2,L3,E,", ",L1,L2,L3,A,");
    app.items_preview(fixed.as_bytes(), "cpu-questions.csv", None).unwrap();
    app.items_apply(ItemsChoice { targets: vec![("cpu-questions".into(), DeckTarget::Topic { topic: cpu })] }, at(TODAY)).unwrap();
    assert_eq!(questions_in(&app, "CPU").len(), 5);
}

#[test]
fn columns_can_be_chosen_by_hand_and_rows_can_be_cards() {
    let csv = "TLB,A cache of page table entries\nThrashing,Paging more than running\n";
    let mut app = app();
    let preview = app.items_preview(csv.as_bytes(), "memory.csv", Some(CsvMapping { kind: CsvKind::Card, header: false, roles: vec![Role::Front, Role::Back] })).unwrap();
    assert_eq!((preview.ready, preview.topics[0].cards), (2, 2));
    app.items_apply(ItemsChoice { targets: vec![] }, at(TODAY)).unwrap();
    let memory = app.topics().unwrap().into_iter().find(|row| row.name == "memory").unwrap().id;
    assert_eq!(app.topic_cards(memory, TODAY).unwrap().cards.len(), 2);
}

#[test]
fn spreadsheets_export_questions_and_cards_that_import_back() {
    let app = written();
    let (name, questions) = export(&app, ExportFormat::QuestionsCsv);
    assert!(name.ends_with(".csv"));
    assert!(questions.starts_with("question,a,b,c,d,e,answer,explanation,type,model answer,keywords,topic,module"), "{questions}");
    let mut other = self::app();
    let preview = import(&mut other, &name, &questions, None);
    assert_eq!(preview.ready, 3, "{:?}", preview.problems);
    assert_eq!(questions_in(&other, "CPU").len(), 3);
    let (_, cards) = export(&app, ExportFormat::CardsCsv);
    assert!(cards.starts_with("kind,front,back,text,extra,topic,module"), "{cards}");
    let preview = import(&mut other, "cards.csv", &cards, None);
    assert_eq!(preview.ready, 2, "{:?}", preview.problems);
}

#[test]
fn markdown_checklists_round_trip() {
    let text = "# Paging\n\n- Pages are fixed size\n- The TLB caches translations\n\n# Scheduling\n\n* Round robin uses a time slice\n1. Priority can starve\n";
    let mut app = app();
    let preview = import(&mut app, "checklists.md", text, None);
    assert_eq!(preview.format, "markdown");
    assert_eq!(preview.topics.iter().map(|topic| (topic.name.as_str(), topic.checklists)).collect::<Vec<_>>(), vec![("Paging", 1), ("Scheduling", 1)]);
    let (name, exported) = export(&app, ExportFormat::Markdown);
    assert!(name.ends_with(".md"));
    assert!(exported.contains("# Paging\n\n- Pages are fixed size\n- The TLB caches translations"), "{exported}");
    assert!(exported.contains("- Priority can starve"), "{exported}");
}

#[test]
fn moodle_gift_and_aiken_both_ways() {
    let app = written();
    let (name, gift) = export(&app, ExportFormat::Gift);
    assert!(name.ends_with(".txt"));
    assert!(gift.contains("$CATEGORY: Computer Architecture/CPU"), "{gift}");
    assert!(gift.contains("=Harvard") && gift.contains("~Von Neumann") && gift.contains("####Separate buses"), "{gift}");
    let mut other = self::app();
    let preview = import(&mut other, &name, &gift, None);
    assert_eq!(preview.format, "gift");
    assert_eq!(preview.ready, 3, "{:?}", preview.problems);
    assert_eq!(preview.topics[0].target, DeckTarget::NewTopic { name: "CPU".into(), module: Some("Computer Architecture".into()) });
    let registers = questions_in(&other, "CPU").into_iter().find(|question| question.text == "Which are registers?").unwrap();
    assert_eq!(registers.options.iter().filter(|option| option.correct).count(), 2);

    let (_, aiken) = export(&app, ExportFormat::Aiken);
    assert!(aiken.contains("Which design reads instructions and data at once?\nA. Von Neumann\nB. Harvard\nC. RISC\nANSWER: B"), "{aiken}");
    assert!(!aiken.contains("Which are registers?"), "Aiken only holds one right answer");
    let mut third = self::app();
    let preview = import(&mut third, "moodle.txt", &aiken, None);
    assert_eq!((preview.format.as_str(), preview.ready), ("aiken", 1));
}

#[test]
fn a_file_it_cant_read_says_so() {
    let mut app = app();
    let error = app.items_preview(b"\x00\x01binary", "lecture-notes.zip", None).unwrap_err();
    assert!(error.message.contains("isn't a kind of file"), "{}", error.message);
    let error = app.items_preview(b"{ not json", "broken.json", None).unwrap_err();
    assert!(error.message.contains("couldn't be read"), "{}", error.message);
}

// import-export.feature: Undo the last import (R177)
#[test]
fn the_last_import_can_be_undone_from_the_backup_made_before_it() {
    let mut app = app();
    app.create_topic("Mine", None).unwrap();
    assert!(app.last_import().unwrap().is_none());
    import(&mut app, "checklists.md", "# Paging\n\n- Pages are fixed size\n", None);
    let last = app.last_import().unwrap().expect("the import is remembered");
    assert_eq!(last.file_name, "checklists.md");
    assert!(last.summary.contains("1 topic") && last.summary.contains("1 recall checklist"), "{}", last.summary);
    assert_eq!(app.topics().unwrap().len(), 2);
    app.undo_last_import(at(TODAY)).unwrap();
    assert_eq!(app.topics().unwrap().iter().map(|topic| topic.name.as_str()).collect::<Vec<_>>(), vec!["Mine"]);
    assert!(app.last_import().unwrap().is_none(), "nothing left to undo");
    assert!(app.undo_last_import(at(TODAY)).is_err());
}

//! dates.feature through the command layer (R130 to R137).

use super::*;
use crate::cards::NoteInput;
use crate::dates::DateSave;
use study_core::cards::{Memory, Note, NoteKind, Schedule};
use study_core::recall::Position;
use study_store::dates::Covers;
use study_store::practice::QuizMove;

// Tue 3 Nov is TODAY, Mon 23 Nov is EXAM.
const TODAY: i64 = 20_760;
const EXAM: i64 = TODAY + 20;

fn app() -> App {
    App::with_seed(Store::in_memory().unwrap(), 7)
}

struct Networks {
    app: App,
    module: ModuleId,
    osi: TopicId,
    tcp: TopicId,
    subnetting: TopicId,
}

/// Networks with OSI model and TCP/IP quizzed on `last`, due on the days given, and
/// Subnetting never quizzed.
fn networks(last: i64, osi_due: i64, tcp_due: i64) -> Networks {
    let mut app = app();
    let module = app.create_module("Networks").unwrap();
    let osi = app.create_topic("OSI model", Some(module)).unwrap();
    let tcp = app.create_topic("TCP/IP", Some(module)).unwrap();
    let subnetting = app.create_topic("Subnetting", Some(module)).unwrap();
    for (attempt, topic, due) in [(1, osi, osi_due), (2, tcp, tcp_due)] {
        app.store.save_quiz_move(&QuizMove { attempt: AttemptId(attempt), topic, day: last, from: None, score: 90, passed: true, to: Position { step: 2, rep: 0 }, next_due: due }).unwrap();
    }
    Networks { app, module, osi, tcp, subnetting }
}

fn exam(module: ModuleId) -> DateSave {
    DateSave { name: "Networks exam".into(), day: EXAM, kind: "Exam".into(), module: Some(module), topics: vec![], tighten: true }
}

fn due_of(app: &App, topic: TopicId, today: i64) -> (i64, Option<String>) {
    let quiz = app.topic_quizzes(today).unwrap().into_iter().find(|q| q.topic == topic).unwrap();
    (quiz.due, quiz.tightened_for)
}

// dates.feature: Add a date, Edit and delete a date
#[test]
fn dates_are_listed_with_what_they_cover() {
    let Networks { mut app, module, osi, tcp, .. } = networks(TODAY, TODAY + 13, TODAY + 25);
    let deadline = app.add_date(DateSave { name: "Study app write-up".into(), day: TODAY + 10, kind: "Deadline".into(), module: None, topics: vec![], tighten: true }).unwrap();
    app.add_date(exam(module)).unwrap();
    app.add_date(DateSave { name: "Mock".into(), day: TODAY + 5, kind: "Mock exam".into(), module: None, topics: vec![osi, tcp], tighten: false }).unwrap();
    let dates = app.dates(TODAY).unwrap();
    let shown: Vec<(&str, i64, &str, bool)> = dates.iter().map(|d| (d.name.as_str(), d.days_left, d.covers_label.as_str(), d.tighten)).collect();
    assert_eq!(shown, vec![("Mock", 5, "OSI model and TCP/IP", false), ("Study app write-up", 10, "", false), ("Networks exam", 20, "Networks module", true)]);
    assert_eq!(dates[2].topics.len(), 3);

    app.update_date(deadline, DateSave { name: "Study app write-up".into(), day: TODAY + 13, kind: "Deadline".into(), module: None, topics: vec![], tighten: false }).unwrap();
    assert_eq!(app.dates(TODAY).unwrap().iter().find(|d| d.id == deadline).unwrap().day, TODAY + 13);
    app.delete_date(deadline).unwrap();
    assert_eq!(app.dates(TODAY).unwrap().len(), 2);
    assert!(app.add_date(DateSave { name: " ".into(), day: TODAY, kind: String::new(), module: None, topics: vec![], tighten: false }).is_err());
}

// dates.feature: A quiz is pulled in to half the days that were left, The tighter date wins,
// After the date things go back to normal, Deleting the date puts everything back
#[test]
fn topic_quizzes_are_tightened() {
    let Networks { mut app, module, osi, tcp, .. } = networks(TODAY, TODAY + 5, TODAY + 25);
    let id = app.add_date(exam(module)).unwrap();
    assert_eq!(due_of(&app, tcp, TODAY), (TODAY + 10, Some("Networks exam".into())));
    assert_eq!(due_of(&app, osi, TODAY), (TODAY + 5, None), "already soon enough");
    let quiz = app.topic_quizzes(TODAY).unwrap().into_iter().find(|q| q.topic == tcp).unwrap();
    assert_eq!(quiz.normal_due, TODAY + 25);

    let mock = app.add_date(DateSave { name: "Networks mock".into(), day: TODAY + 10, kind: "Mock exam".into(), module: None, topics: vec![tcp], tighten: true }).unwrap();
    assert_eq!(due_of(&app, tcp, TODAY), (TODAY + 5, Some("Networks mock".into())));
    app.delete_date(mock).unwrap();

    assert_eq!(due_of(&app, tcp, EXAM + 1), (TODAY + 25, None), "back to normal the day after");
    app.delete_date(id).unwrap();
    assert_eq!(due_of(&app, tcp, TODAY), (TODAY + 25, None));
}

// dates.feature: Something done long ago comes up straight away
#[test]
fn something_done_long_ago_is_due_today() {
    let Networks { mut app, module, tcp, .. } = networks(TODAY - 29, TODAY + 32, TODAY + 32);
    app.add_date(exam(module)).unwrap();
    let (due, _) = due_of(&app, tcp, TODAY);
    assert!(due <= TODAY);
    assert_eq!(app.due_today(TODAY).unwrap().quizzes, 2);
}

/// A card reviewed on `last`, due on `due`.
fn reviewed_card(app: &mut App, topic: TopicId, front: &str, last: i64, due: i64) {
    let note = app.save_note(None, NoteInput { note: Note { topic, kind: NoteKind::Basic, front: front.into(), back: "Back".into(), cues: vec![], text: String::new(), extra: String::new(), tags: vec![] }, suggestion: None }, last).unwrap();
    let card = app.store.cards_of(note).unwrap()[0].id;
    let schedule = Schedule { memory: Some(Memory { stability: 30.0, difficulty: 5.0 }), due, last_review: Some(last), reps: 3, lapses: 0 };
    app.store.record_review(card, &schedule, &study_store::cards::ReviewRow { card, day: last, rating: 3, elapsed: 10, was_new: false, seconds: 5 }).unwrap();
}

// dates.feature: Cards and drills are tightened, checklists aren't
#[test]
fn cards_and_drills_are_tightened_but_not_checklists() {
    let Networks { mut app, module, tcp, .. } = networks(TODAY, TODAY + 25, TODAY + 25);
    reviewed_card(&mut app, tcp, "Three-way handshake", TODAY, TODAY + 30);
    let drill = app.store.add_drill(None, Some(tcp), "Draw the handshake", "", TODAY, TODAY + 30).unwrap();
    let checklist = app.create_checklist(study_store::recall::Subject::Module(module), TODAY).unwrap();
    app.save_points(checklist, vec![crate::recall::PointInput { text: "Layers".into(), question: None, note: None }]).unwrap();
    let checklist_due = app.store.checklists().unwrap()[0].due;

    app.add_date(exam(module)).unwrap();
    assert_eq!(app.topic_cards(tcp, TODAY).unwrap().cards[0].due, TODAY + 10);
    assert_eq!(app.cards_due_count(TODAY + 10).unwrap(), 1);
    let drills = app.drills_due(TODAY + 10).unwrap();
    assert_eq!((drills.len(), drills[0].id, drills[0].due), (1, drill, Some(TODAY + 10)));
    assert_eq!(app.store.checklists().unwrap()[0].due, checklist_due);
    let day = app.calendar_day(TODAY + 10, TODAY).unwrap();
    assert_eq!((day.cards.due.review, day.drills.len(), day.quizzes.len()), (1, 1, 2));
    assert_eq!(day.quizzes[0].tightened_for.as_deref(), Some("Networks exam"));
    let month = app.calendar_month(TODAY, TODAY + 30, TODAY).unwrap();
    let on = month.iter().find(|d| d.day == TODAY + 10).unwrap();
    assert_eq!((on.cards, on.quizzes, on.drills), (1, 2, 1));
}

// dates.feature: Preview before saving
#[test]
fn previews_what_moves() {
    let Networks { mut app, module, tcp, .. } = networks(TODAY, TODAY + 8, TODAY + 25);
    reviewed_card(&mut app, tcp, "Handshake", TODAY, TODAY + 30);
    reviewed_card(&mut app, tcp, "Ports", TODAY, TODAY + 4);
    let preview = app.preview_date(None, exam(module), TODAY).unwrap();
    let moves: Vec<(&str, i64, i64)> = preview.quizzes.iter().map(|m| (m.name.as_str(), m.from, m.to)).collect();
    assert_eq!(moves, vec![("TCP/IP", TODAY + 25, TODAY + 10)], "OSI model is already soon enough");
    assert_eq!(preview.unquizzed.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["Subnetting"]);
    assert_eq!(preview.cards, 1);

    let mut plain = exam(module);
    plain.tighten = false;
    assert!(app.preview_date(None, plain, TODAY).unwrap().quizzes.is_empty());

    // Editing compares with how things are without the date being edited.
    let id = app.add_date(exam(module)).unwrap();
    let later = DateSave { day: TODAY + 30, ..exam(module) };
    let moves: Vec<(i64, i64)> = app.preview_date(Some(id), later, TODAY).unwrap().quizzes.iter().map(|m| (m.from, m.to)).collect();
    assert_eq!(moves, vec![(TODAY + 25, TODAY + 15)]);
}

// dates.feature: The exam's day, The week before
#[test]
fn the_exams_day_and_the_week_before() {
    let Networks { mut app, module, osi, tcp, subnetting } = networks(TODAY, TODAY + 8, TODAY + 25);
    reviewed_card(&mut app, tcp, "Handshake", TODAY, TODAY + 30);
    reviewed_card(&mut app, tcp, "Ports", TODAY, TODAY + 4);
    app.add_date(exam(module)).unwrap();
    app.add_date(DateSave { name: "Mum's birthday".into(), day: EXAM, kind: String::new(), module: None, topics: vec![], tighten: false }).unwrap();

    let day = app.calendar_day(EXAM, TODAY).unwrap();
    assert_eq!(day.dates.len(), 2);
    let panel = &day.dates.iter().find(|p| p.date.name == "Networks exam").unwrap();
    assert_eq!((panel.date.days_left, panel.date.covers_label.as_str()), (20, "Networks module"));
    let quizzes: Vec<(TopicId, i64, i64)> = panel.quizzes.iter().map(|m| (m.topic, m.from, m.to)).collect();
    assert_eq!(quizzes, vec![(osi, TODAY + 8, TODAY + 8), (tcp, TODAY + 25, TODAY + 10)]);
    assert_eq!(panel.unquizzed.iter().map(|t| t.topic).collect::<Vec<_>>(), vec![subnetting]);
    assert_eq!((panel.cards_before, panel.cards_pulled), (2, 1));

    let week_before = app.calendar_day(EXAM - 7, TODAY).unwrap();
    assert!(week_before.dates.is_empty());
    assert_eq!(week_before.soon.iter().map(|d| (d.name.as_str(), d.day)).collect::<Vec<_>>(), vec![("Networks exam", EXAM)], "only dates covering something");
}

// dates.feature: Tightened next quizzes are labelled (results)
#[test]
fn results_show_the_tightened_next_quiz() {
    let mut app = app();
    let module = app.create_module("Networks").unwrap();
    let tcp = app.create_topic("TCP/IP", Some(module)).unwrap();
    for text in ["SYN?", "ACK?"] {
        app.save_question(QuestionInput {
            topic: tcp,
            text: text.into(),
            multiple: false,
            options: ["Right", "Wrong", "Also wrong"].iter().enumerate().map(|(i, t)| OptionInput { text: (*t).into(), correct: i == 0 }).collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
            short: None,
        })
        .unwrap();
    }
    // Already on a long step, so passing moves it further out.
    app.store.save_quiz_move(&QuizMove { attempt: AttemptId(0), topic: tcp, day: TODAY - 7, from: None, score: 90, passed: true, to: Position { step: 2, rep: 0 }, next_due: TODAY }).unwrap();
    app.add_date(DateSave { name: "Networks exam".into(), day: TODAY + 10, kind: "Exam".into(), module: Some(module), topics: vec![], tighten: true }).unwrap();
    let setup = QuizSetupInput { topics: vec![tcp], count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: true, total_minutes: None, per_question_seconds: None, easy: false, types: study_core::assembly::QuestionTypes::Both };
    app.start_quiz(setup).unwrap();
    app.answer(vec![0], TODAY).unwrap();
    let Next::Finished(attempt) = app.answer(vec![0], TODAY).unwrap().next else { panic!("two questions") };
    let moved = &app.move_views(attempt).unwrap()[0];
    assert!(moved.next_due > TODAY + 10, "{moved:?}");
    assert_eq!((moved.tightened_due, moved.tightened_for.as_deref()), (Some(TODAY + 5), Some("Networks exam")));
}

// dates.feature: Dates sync (they're in a full export, which sync is built on)
#[test]
fn dates_go_into_a_full_export() {
    let Networks { mut app, module, osi, .. } = networks(TODAY, TODAY + 13, TODAY + 25);
    app.add_date(exam(module)).unwrap();
    app.add_date(DateSave { name: "Mock".into(), day: TODAY + 5, kind: String::new(), module: None, topics: vec![osi], tighten: true }).unwrap();
    let snapshot = app.store.snapshot(&study_store::portable::Scope { topics: None, history: true }).unwrap();
    let mut other = self::app();
    other.store.apply(&snapshot, &study_store::portable::Plan { today: TODAY, ..Default::default() }).unwrap();
    let dates = other.dates(TODAY).unwrap();
    let shown: Vec<(&str, &str, bool)> = dates.iter().map(|d| (d.name.as_str(), d.covers_label.as_str(), d.tighten)).collect();
    assert_eq!(shown, vec![("Mock", "OSI model", true), ("Networks exam", "Networks module", true)]);
    assert!(matches!(dates[1].covers, Covers::Module(_)));
}

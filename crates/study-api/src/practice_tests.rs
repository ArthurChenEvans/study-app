//! projects.feature and calendar.feature through the command layer (R87 to R100).

use super::*;
use crate::calendar::When;
use crate::cards::{NoteInput, SuggestionKind};
use crate::practice::{Clock, OutcomeView, ReviewChoice, StruggleInput};
use crate::recall::PointInput;
use study_core::assembly::QuestionTypes;
use study_core::cards::{Note, NoteKind};
use study_core::practice::{DrillRating, StruggleKind};
use study_store::recall::Subject;

const TODAY: i64 = 20_760;

fn app() -> App {
    App::with_seed(Store::in_memory().unwrap(), 7)
}

fn at(day: i64, minute: u32) -> Clock {
    Clock { day, minute, unix: day * 86_400 + minute as i64 * 60 }
}

fn struggle(what: &str, kind: StruggleKind) -> StruggleInput {
    StruggleInput { what: what.into(), why: "habit from the C course".into(), kind, topic: None }
}

fn mcq(topic: TopicId, text: &str) -> QuestionInput {
    QuestionInput {
        topic,
        text: text.into(),
        multiple: false,
        options: ["Right", "Wrong", "Also wrong"].iter().enumerate().map(|(i, t)| OptionInput { text: (*t).into(), correct: i == 0 }).collect(),
        explanation: String::new(),
        tags: vec![],
        citations: vec![],
        short: None,
    }
}

/// Takes a quiz over the topics, answering the first `right` questions right.
fn quiz(app: &mut App, topics: Vec<TopicId>, right: usize, day: i64) -> AttemptId {
    let setup = QuizSetupInput { topics, count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: true, total_minutes: None, per_question_seconds: None, easy: false, types: QuestionTypes::Both };
    app.start_quiz(setup).unwrap();
    let mut answered = 0;
    loop {
        let picked = if answered < right { vec![0] } else { vec![1] };
        answered += 1;
        if let Next::Finished(id) = app.answer(picked, day).unwrap().next {
            return id;
        }
    }
}

// ---------- Projects ----------

// projects.feature: Work on a project in sessions, Log a struggle during a session
#[test]
fn sessions_and_struggles() {
    let mut app = app();
    let project = app.create_project("  Study app ", "Coding", TODAY).unwrap();
    assert!(app.create_project("  ", "", TODAY).is_err());
    assert_eq!(app.log_struggle(project, struggle("x", StruggleKind::Apply), at(TODAY, 600)).unwrap_err().message, "start a session before logging a struggle");

    let view = app.start_session(project, at(TODAY, 600)).unwrap();
    assert_eq!(view.row.name, "Study app");
    assert!(view.row.session.is_some());
    assert_eq!(app.start_session(project, at(TODAY, 601)).unwrap_err().message, "a session is already running on this project");
    assert_eq!(app.log_struggle(project, struggle("  ", StruggleKind::Apply), at(TODAY, 610)).unwrap_err().message, "write what tripped you up");
    let logged = app.log_struggle(project, struggle("Loop ran one too many times", StruggleKind::Apply), at(TODAY, 860)).unwrap();
    assert_eq!((logged.day, logged.minute, logged.outcome.clone()), (TODAY, 860, None));

    let row = &app.projects(TODAY).unwrap()[0];
    assert_eq!((row.struggles, row.to_review, row.last_day), (1, 1, Some(TODAY)));
    app.end_session(project, at(TODAY, 900)).unwrap();
    assert!(app.projects(TODAY).unwrap()[0].session.is_none());
    assert!(app.end_session(project, at(TODAY, 901)).is_err(), "nothing to end");
}

// projects.feature: A recall miss becomes a card suggestion, Finish a review later
#[test]
fn reviews_make_card_suggestions_and_can_wait() {
    let mut app = app();
    let project = app.create_project("Study app", "Coding", TODAY).unwrap();
    app.start_session(project, at(TODAY, 600)).unwrap();
    let first = app.log_struggle(project, struggle("Forgot how to slice from the end", StruggleKind::Recall), at(TODAY, 610)).unwrap();
    app.log_struggle(project, struggle("Second", StruggleKind::Unsure), at(TODAY, 620)).unwrap();
    app.log_struggle(project, struggle("Third", StruggleKind::Apply), at(TODAY, 630)).unwrap();
    app.end_session(project, at(TODAY, 700)).unwrap();

    let to_review = app.struggles_to_review(project).unwrap();
    assert_eq!(to_review.iter().map(|s| s.what.as_str()).collect::<Vec<_>>(), vec!["Forgot how to slice from the end", "Second", "Third"], "oldest first");
    let reviewed = app.review_struggle(first.id, ReviewChoice::Card, TODAY).unwrap();
    assert_eq!(reviewed.outcome, Some(OutcomeView::Card));
    assert_eq!(app.review_struggle(first.id, ReviewChoice::Nothing, TODAY).unwrap_err().message, "that struggle has already been reviewed");
    assert_eq!(app.projects(TODAY).unwrap()[0].to_review, 2, "the rest wait");

    let suggestion = app.card_suggestions(TODAY).unwrap().into_iter().find(|s| s.kind == SuggestionKind::Struggle).unwrap();
    assert_eq!((suggestion.text.as_str(), suggestion.from.as_str(), suggestion.answer.clone()), ("Forgot how to slice from the end", "Project: Study app", None));
    assert_eq!(suggestion.note, "Why it wasn't obvious: habit from the C course");
}

// projects.feature: A struggle becomes a recall checklist point
#[test]
fn a_struggle_can_be_a_checklist_point() {
    let mut app = app();
    let rust = app.create_topic("Rust", None).unwrap();
    let checklist = app.create_checklist(Subject::Topic(rust), TODAY).unwrap();
    app.save_points(checklist, vec![PointInput { text: "Ownership".into(), question: None, note: None }]).unwrap();
    let project = app.create_project("Study app", "Coding", TODAY).unwrap();
    app.start_session(project, at(TODAY, 0)).unwrap();
    let logged = app.log_struggle(project, struggle("Borrowing in closures", StruggleKind::Recall), at(TODAY, 5)).unwrap();
    let reviewed = app.review_struggle(logged.id, ReviewChoice::Checklist { checklist }, TODAY).unwrap();
    assert_eq!(reviewed.outcome, Some(OutcomeView::Checklist { id: checklist, name: "Rust".into() }));
    let points: Vec<String> = app.checklist(checklist, TODAY).unwrap().points.into_iter().map(|p| p.text).collect();
    assert_eq!(points, vec!["Ownership", "Borrowing in closures"]);
}

// projects.feature: A struggle with a new issue gets a new drill, A known issue offers its drill again
#[test]
fn drills_from_reviews_and_known_issues() {
    let mut app = app();
    let a = app.create_project("Study app", "Coding", TODAY).unwrap();
    let b = app.create_project("Networks past paper", "Past paper", TODAY).unwrap();
    app.start_session(a, at(TODAY, 0)).unwrap();
    let first = app.log_struggle(a, struggle("Loop ran one too many times", StruggleKind::Apply), at(TODAY, 5)).unwrap();
    let new_drill = ReviewChoice::NewDrill { issue: "Off-by-one in loops".into(), task: "Loop over a slice".into(), hints: "0..len".into(), topic: None, day: TODAY + 1 };
    assert!(app.review_struggle(first.id, ReviewChoice::NewDrill { issue: " ".into(), task: "t".into(), hints: String::new(), topic: None, day: TODAY + 1 }, TODAY).is_err(), "needs an issue");
    assert!(app.review_struggle(first.id, ReviewChoice::NewDrill { issue: "x".into(), task: "t".into(), hints: String::new(), topic: None, day: TODAY - 1 }, TODAY).is_err(), "not in the past");
    let reviewed = app.review_struggle(first.id, new_drill, TODAY).unwrap();
    let Some(OutcomeView::Drill { id: drill, due }) = reviewed.outcome else { panic!("expected a drill") };
    assert_eq!((due, reviewed.issue.as_deref()), (Some(TODAY + 1), Some("Off-by-one in loops")));
    assert_eq!(app.projects(TODAY + 1).unwrap().iter().find(|p| p.id == a).unwrap().drills_due, 1);

    // Done and ticked off, then the same issue comes up in another project.
    app.rate_drill(drill, DrillRating::Solid, 120, "", TODAY + 1).unwrap();
    assert_eq!(app.drill_view(drill).unwrap().due, None);
    let known = app.known_drill("  OFF-BY-ONE in loops").unwrap().expect("the issue has a drill");
    assert_eq!((known.id, known.task.as_str()), (drill, "Loop over a slice"));
    assert_eq!(app.known_drill("Something new").unwrap(), None);

    app.start_session(b, at(TODAY + 2, 0)).unwrap();
    let again = app.log_struggle(b, struggle("Counted one extra host", StruggleKind::Apply), at(TODAY + 2, 5)).unwrap();
    app.review_struggle(again.id, ReviewChoice::DrillAgain { issue: "off-by-one in loops".into(), drill, day: TODAY + 3 }, TODAY + 2).unwrap();
    assert_eq!(app.drill_view(drill).unwrap().due, Some(TODAY + 3));

    let issues = app.issues().unwrap();
    assert_eq!(issues.len(), 1, "shared, matched ignoring case");
    assert_eq!((issues[0].seen, issues[0].projects.clone()), (2, vec!["Networks past paper".to_string(), "Study app".to_string()]));
    let view = app.drill_view(drill).unwrap();
    assert_eq!(view.from.iter().map(|s| s.what.as_str()).collect::<Vec<_>>(), vec!["Counted one extra host", "Loop ran one too many times"]);
    assert_eq!(view.runs, 1);
}

// projects.feature: Rating a drill
#[test]
fn drills_come_back_until_solid_and_can_move() {
    let mut app = app();
    let project = app.create_project("Blender mug", "Blender", TODAY).unwrap();
    app.start_session(project, at(TODAY, 0)).unwrap();
    let logged = app.log_struggle(project, struggle("Seam split", StruggleKind::Apply), at(TODAY, 1)).unwrap();
    let reviewed = app.review_struggle(logged.id, ReviewChoice::NewDrill { issue: "Mirror seams".into(), task: "Mirror a mug".into(), hints: String::new(), topic: None, day: TODAY }, TODAY).unwrap();
    let Some(OutcomeView::Drill { id, .. }) = reviewed.outcome else { panic!() };
    assert_eq!(app.drills_due(TODAY).unwrap().len(), 1);
    assert_eq!(app.rate_drill(id, DrillRating::Struggled, 60, " shaky seam ", TODAY).unwrap().due, Some(TODAY + 1));
    assert!(app.drills_due(TODAY).unwrap().is_empty());
    assert_eq!(app.drills_due(TODAY + 3).unwrap().len(), 1, "overdue still counts");
    assert_eq!(app.move_drill(id, TODAY + 4, TODAY + 3).unwrap().due, Some(TODAY + 4));
    assert!(app.move_drill(id, TODAY, TODAY + 3).is_err());
    let view = app.rate_drill(id, DrillRating::Shaky, 60, "", TODAY + 4).unwrap();
    assert_eq!((view.due, view.last.as_ref().map(|r| r.rating)), (Some(TODAY + 5), Some(DrillRating::Shaky)));
    assert_eq!(app.rate_drill(id, DrillRating::Solid, 60, "", TODAY + 5).unwrap().due, None);
    assert_eq!(app.drill_view(id).unwrap().runs, 3);
}

// ---------- Quiz scheduling ----------

// calendar.feature: A topic's quiz schedule moves with its score
#[test]
fn quizzes_move_each_topic_on_its_own_score() {
    let mut app = app();
    let cpu = app.create_topic("CPU", None).unwrap();
    let osi = app.create_topic("OSI model", None).unwrap();
    for n in 0..2 {
        app.save_question(mcq(cpu, &format!("CPU {n}"))).unwrap();
        app.save_question(mcq(osi, &format!("OSI {n}"))).unwrap();
    }
    assert!(app.topic_quizzes(TODAY).unwrap().is_empty(), "starts the first time a topic is quizzed");

    // Unshuffled: CPU's two questions come first, both right. OSI's two are wrong.
    let first = quiz(&mut app, vec![cpu, osi], 2, TODAY);
    let rows = app.topic_quizzes(TODAY).unwrap();
    let cpu_row = rows.iter().find(|r| r.topic == cpu).unwrap();
    let osi_row = rows.iter().find(|r| r.topic == osi).unwrap();
    assert_eq!((cpu_row.step.as_str(), cpu_row.due), ("Day", TODAY + 1));
    assert_eq!((osi_row.step.as_str(), osi_row.due), ("Day", TODAY + 1));
    let moves = app.results(first).unwrap().moves;
    assert_eq!(moves.iter().map(|m| (m.name.as_str(), m.score, m.passed, m.from.clone())).collect::<Vec<_>>(), vec![("CPU", 100, true, None), ("OSI model", 0, false, None)]);

    let second = quiz(&mut app, vec![cpu], 2, TODAY + 1);
    let cpu_row = app.topic_quizzes(TODAY + 1).unwrap().into_iter().find(|r| r.topic == cpu).unwrap();
    assert_eq!((cpu_row.step.as_str(), cpu_row.due, cpu_row.due_today), ("Week", TODAY + 8, false));
    assert_eq!(app.results(second).unwrap().moves[0].from.as_deref(), Some("Day"));
}

// calendar.feature: Retries and quits don't move the schedule
#[test]
fn retries_and_quits_leave_schedules_alone() {
    let mut app = app();
    let cpu = app.create_topic("CPU", None).unwrap();
    for n in 0..3 {
        app.save_question(mcq(cpu, &format!("CPU {n}"))).unwrap();
    }
    let first = quiz(&mut app, vec![cpu], 1, TODAY);
    let before = app.topic_quizzes(TODAY).unwrap();
    app.retry(first).unwrap();
    while let Ok(view) = app.current_question() {
        let _ = view;
        if let Next::Finished(id) = app.answer(vec![0], TODAY).unwrap().next {
            assert!(app.results(id).unwrap().moves.is_empty());
            break;
        }
    }
    let setup = QuizSetupInput { topics: vec![cpu], count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: true, total_minutes: None, per_question_seconds: None, easy: false, types: QuestionTypes::Both };
    app.start_quiz(setup).unwrap();
    app.answer(vec![0], TODAY).unwrap();
    app.quit_quiz(TODAY).unwrap();
    assert_eq!(app.topic_quizzes(TODAY).unwrap(), before);
}

// calendar.feature: Overruling redoes the move
#[test]
fn overruling_redoes_the_latest_move() {
    use study_core::question::{Check, ShortAnswer};
    let mut app = app();
    let osi = app.create_topic("OSI model", None).unwrap();
    app.save_question(QuestionInput {
        short: Some(ShortAnswer { check: Check::Accepted { answers: vec!["Transport".into()], typos: true }, model_answer: String::new() }),
        ..mcq(osi, "Which layer adds ports?")
    })
    .unwrap();
    let setup = QuizSetupInput { topics: vec![osi], count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: true, total_minutes: None, per_question_seconds: None, easy: false, types: QuestionTypes::Both };
    app.start_quiz(setup).unwrap();
    let Next::Finished(attempt) = app.answer_text("network".into(), TODAY).unwrap().next else { panic!() };
    assert!(!app.results(attempt).unwrap().moves[0].passed);
    let results = app.overrule_result(attempt, 0).unwrap();
    assert_eq!((results.moves[0].passed, results.moves[0].score), (true, 100));
    assert_eq!(app.topic_quizzes(TODAY).unwrap()[0].due, TODAY + 1, "Day is the first step either way");
}

// ---------- The calendar ----------

// calendar.feature: See what's when
#[test]
fn the_calendar_shows_whats_due_and_what_was_done() {
    let mut app = app();
    let cpu = app.create_topic("CPU", None).unwrap();
    app.save_question(mcq(cpu, "q")).unwrap();
    quiz(&mut app, vec![cpu], 1, TODAY - 2); // CPU next due TODAY - 1, so overdue today
    let checklist = app.create_checklist(Subject::Topic(cpu), TODAY + 3).unwrap();
    app.save_points(checklist, vec![PointInput { text: "Pipelines".into(), question: None, note: None }]).unwrap();
    let note = NoteInput { note: Note { topic: cpu, kind: NoteKind::Basic, front: "f".into(), back: "b".into(), cues: vec![], text: String::new(), extra: String::new() }, suggestion: None };
    app.save_note(None, note, TODAY).unwrap();
    let project = app.create_project("Study app", "Coding", TODAY - 2).unwrap();
    app.start_session(project, at(TODAY - 2, 0)).unwrap();
    let logged = app.log_struggle(project, struggle("Loop", StruggleKind::Apply), at(TODAY - 2, 5)).unwrap();
    app.review_struggle(logged.id, ReviewChoice::NewDrill { issue: "Off-by-one".into(), task: "Loop over a slice".into(), hints: String::new(), topic: Some(cpu), day: TODAY + 2 }, TODAY - 2).unwrap();

    let month = app.calendar_month(TODAY - 2, TODAY + 3, TODAY).unwrap();
    let day = |d: i64| month.iter().find(|c| c.day == d).copied().unwrap();
    assert_eq!((day(TODAY - 2).when, day(TODAY - 2).quizzes, day(TODAY - 2).struggles), (When::Past, 1, 1));
    assert_eq!((day(TODAY - 1).quizzes, day(TODAY - 1).when), (0, When::Past), "past days show what was done, not what was due");
    let today = day(TODAY);
    assert_eq!((today.when, today.quizzes, today.cards, today.drills), (When::Today, 1, 1, 0), "overdue quiz and today's new card");
    assert_eq!(day(TODAY + 2).drills, 1);
    assert_eq!(day(TODAY + 3).checklists, 1);
    assert_eq!(app.due_today(TODAY).unwrap().total, 2);

    let past = app.calendar_day(TODAY - 2, TODAY).unwrap();
    assert_eq!((past.attempts.len(), past.attempts[0].topics.clone(), past.struggles.len()), (1, vec!["CPU".to_string()], 1));
    assert_eq!(past.attempts[0].moves, vec!["CPU moved up to Day"]);
    let now = app.calendar_day(TODAY, TODAY).unwrap();
    assert_eq!((now.quizzes.len(), now.quizzes[0].step.as_str(), now.cards.due.new), (1, "Day", 1));
    let future = app.calendar_day(TODAY + 2, TODAY).unwrap();
    assert_eq!((future.when, future.drills.len(), future.drills[0].task.as_str()), (When::Future, 1, "Loop over a slice"));
    assert!(future.struggles.is_empty());
}

// lists.feature: Quizzes are capped a day (R146)
#[test]
fn topic_quizzes_are_capped_a_day_most_overdue_first() {
    use study_core::recall::Position;
    use study_store::practice::QuizMove;
    let mut app = app();
    let mut topics = vec![];
    for index in 0..8 {
        let topic = app.create_topic(&format!("Topic {index}"), None).unwrap();
        // Topic 0 is the most overdue.
        app.store.save_quiz_move(&QuizMove { attempt: AttemptId(index + 1), topic, day: TODAY - 30, from: None, score: 90, passed: true, to: Position { step: 3, rep: 0 }, next_due: TODAY - 10 + index }).unwrap();
        topics.push(topic);
    }
    let queue = app.quiz_queue(TODAY).unwrap();
    assert_eq!(queue.daily, 5);
    assert_eq!(queue.today, topics[..5].to_vec());
    assert_eq!(queue.waiting, topics[5..].to_vec());
    assert_eq!(queue.clears_by, Some(TODAY + 1));
    assert_eq!(app.due_today(TODAY).unwrap().quizzes, 5);
    let views = app.topic_quizzes(TODAY).unwrap();
    let waiting: Vec<TopicId> = views.iter().filter(|v| v.waiting).map(|v| v.topic).collect();
    assert_eq!(waiting, topics[5..].to_vec());
    assert!(views.iter().filter(|v| v.waiting).all(|v| !v.due_today && v.step == views[0].step), "waiting keeps its step");
    let day = app.calendar_day(TODAY, TODAY).unwrap();
    assert_eq!((day.quizzes.len(), day.waiting.len(), day.clears_by), (5, 3, Some(TODAY + 1)));
    assert_eq!(day.quizzes[0].topic, topics[0]);

    app.set_quizzes_a_day(2).unwrap();
    let queue = app.quiz_queue(TODAY).unwrap();
    assert_eq!((queue.today.len(), queue.waiting.len(), queue.clears_by), (2, 6, Some(TODAY + 3)));
    assert!(app.set_quizzes_a_day(0).is_err());
}

// R185: a project's name and kind can be changed
#[test]
fn projects_can_be_renamed() {
    let mut app = app();
    let project = app.create_project("Shell", "Rust", 0).unwrap();
    app.update_project(project, " Tiny shell ", " C ").unwrap();
    let row = app.projects(0).unwrap().into_iter().next().unwrap();
    assert_eq!((row.name.as_str(), row.kind.as_str()), ("Tiny shell", "C"));
    assert!(app.update_project(project, "  ", "C").is_err());
    assert!(app.update_project(study_store::practice::ProjectId(999), "X", "").is_err());
}

// R185: a drill's task and hints can be edited and the drill deleted, an issue renamed and
// deleted, and the projects that pointed at them still open
#[test]
fn drills_and_issues_can_be_edited_and_deleted() {
    let mut app = app();
    let project = app.create_project("Study app", "Coding", TODAY).unwrap();
    app.start_session(project, at(TODAY, 0)).unwrap();
    let logged = app.log_struggle(project, struggle("Loop ran one too many times", StruggleKind::Apply), at(TODAY, 5)).unwrap();
    let choice = ReviewChoice::NewDrill { issue: "Off-by-one in loops".into(), task: "Loop over a slice".into(), hints: "0..len".into(), topic: None, day: TODAY + 1 };
    let Some(OutcomeView::Drill { id: drill, .. }) = app.review_struggle(logged.id, choice, TODAY).unwrap().outcome else { panic!("expected a drill") };

    app.update_drill(drill, " Loop over a slice backwards ", " (0..len).rev() ").unwrap();
    let edited = app.drill_view(drill).unwrap();
    assert_eq!((edited.task.as_str(), edited.hints.as_str()), ("Loop over a slice backwards", "(0..len).rev()"));
    assert!(app.update_drill(drill, "  ", "").is_err());

    let issue = app.issues().unwrap()[0].id;
    let other = app.log_struggle(project, struggle("Another", StruggleKind::Apply), at(TODAY, 9)).unwrap();
    app.review_struggle(other.id, ReviewChoice::NewDrill { issue: "Borrowing".into(), task: "t".into(), hints: String::new(), topic: None, day: TODAY + 1 }, TODAY).unwrap();
    app.rename_issue(issue, "Off by one").unwrap();
    assert!(app.issues().unwrap().iter().any(|i| i.name == "Off by one"));
    assert_eq!(app.rename_issue(issue, "  borrowing ").unwrap_err().message, "there's already an issue called Borrowing");
    assert!(app.rename_issue(issue, " ").is_err());

    app.delete_drill(drill).unwrap();
    assert!(app.drill_view(drill).is_err());
    assert!(app.issues().unwrap().iter().find(|i| i.id == issue).unwrap().drill.is_none());
    app.project_view(project, TODAY).unwrap();

    let borrowing = app.issues().unwrap().into_iter().find(|i| i.name == "Borrowing").unwrap();
    let its_drill = borrowing.drill.unwrap().id;
    app.delete_issue(borrowing.id).unwrap();
    assert!(app.drill_view(its_drill).is_err(), "an issue's drills go with it");
    app.delete_issue(issue).unwrap();
    assert!(app.issues().unwrap().iter().all(|i| i.id != issue));
    assert_eq!(app.project_view(project, TODAY).unwrap().struggles.iter().find(|s| s.id == logged.id).unwrap().issue, None);
}

// R185: a logged struggle's words can be fixed
#[test]
fn struggles_can_be_edited() {
    let mut app = app();
    let project = app.create_project("Study app", "Coding", TODAY).unwrap();
    app.start_session(project, at(TODAY, 0)).unwrap();
    let logged = app.log_struggle(project, struggle("Lifetme error", StruggleKind::Apply), at(TODAY, 5)).unwrap();
    app.update_struggle(logged.id, " Lifetime error ", " didn't see the borrow ").unwrap();
    let view = app.project_view(project, TODAY).unwrap();
    let edited = view.struggles.iter().find(|s| s.id == logged.id).unwrap();
    assert_eq!((edited.what.as_str(), edited.why.as_str()), ("Lifetime error", "didn't see the borrow"));
    assert!(app.update_struggle(logged.id, " ", "").is_err());
}

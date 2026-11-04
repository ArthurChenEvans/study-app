//! short-answer.feature through the command layer (R78 to R86).

use super::*;
use crate::recall::Item;
use study_core::question::{Check, Keyword, ShortAnswer};
use study_core::short::Detail;
use study_store::recall::Subject;

const TODAY: i64 = 20_760;

fn app() -> App {
    App::with_seed(Store::in_memory().unwrap(), 7)
}

fn short_input(topic: TopicId, text: &str, check: Check, model: &str) -> QuestionInput {
    QuestionInput {
        topic,
        text: text.into(),
        multiple: false,
        // Left over from switching type in the editor: ignored for short answers.
        options: vec![OptionInput { text: "Network".into(), correct: true }],
        explanation: "Ports let apps share an address.".into(),
        tags: vec!["layers".into()],
        citations: vec![],
        short: Some(ShortAnswer { check, model_answer: model.into() }),
    }
}

fn transport() -> Check {
    Check::Accepted { answers: vec!["Transport layer".into(), "Layer 4".into()], typos: true }
}

fn handshake() -> Check {
    Check::Keywords {
        keywords: vec![
            Keyword { word: "SYN".into(), also: vec![] },
            Keyword { word: "SYN-ACK".into(), also: vec!["synack".into()] },
            Keyword { word: "ACK".into(), also: vec![] },
        ],
        needed: None,
    }
}

fn setup(topics: Vec<TopicId>, exam: bool, types: QuestionTypes) -> QuizSetupInput {
    QuizSetupInput { topics, count: None, even: false, shuffle_questions: false, shuffle_options: false, exam, total_minutes: None, per_question_seconds: None, easy: false, types }
}

/// An OSI topic with a short answer (accepted answers) and one multiple choice question.
fn with_short() -> (App, TopicId, QuestionId) {
    let mut app = app();
    let osi = app.create_topic("OSI model", None).unwrap();
    let id = app.save_question(short_input(osi, "Which layer adds port numbers?", transport(), "The **transport** layer")).unwrap();
    app.save_question(QuestionInput {
        topic: osi,
        text: "Which layer routes packets?".into(),
        multiple: false,
        options: ["Network", "Transport", "Session"].iter().enumerate().map(|(i, t)| OptionInput { text: (*t).into(), correct: i == 0 }).collect(),
        explanation: String::new(),
        tags: vec![],
        citations: vec![],
        short: None,
    })
    .unwrap();
    (app, osi, id)
}

fn finished(next: &Next) -> AttemptId {
    match next {
        Next::Finished(id) => *id,
        Next::Question(_) => panic!("expected the quiz to finish"),
    }
}

fn feedback(result: &AnswerResult) -> &Feedback {
    result.feedback.as_ref().expect("study mode gives feedback")
}

// short-answer.feature: A short answer question needs something to check against, Switching type keeps what's shared
#[test]
fn short_answer_questions_save_and_come_back_for_editing() {
    let mut app = app();
    let osi = app.create_topic("OSI model", None).unwrap();
    let error = app.save_question(short_input(osi, "Q", Check::Accepted { answers: vec![], typos: true }, "")).unwrap_err();
    assert_eq!(error.message, "add at least one accepted answer, so the app can check it");

    let id = app.save_question(short_input(osi, "Which layer adds port numbers?", transport(), "The **transport** layer")).unwrap();
    let back = app.question(id).unwrap();
    assert!(back.options.is_empty(), "options don't apply to short answers");
    assert_eq!(back.short, Some(ShortAnswer { check: transport(), model_answer: "The **transport** layer".into() }));
    assert_eq!((back.explanation.as_str(), back.tags.as_slice()), ("Ports let apps share an address.", &["layers".to_string()][..]));
    let topic = &app.topics().unwrap()[0];
    assert_eq!((topic.question_count, topic.short_count), (1, 1));
}

// short-answer.feature: Try an answer before saving
#[test]
fn try_an_answer_marks_without_recording() {
    let marking = try_short_answer(&transport(), "the transprot layer");
    assert!(marking.right);
    assert_eq!(marking.detail, Detail::Matched { answer: "Transport layer".into(), typo: Some("transprot".into()) });
    assert!(!try_short_answer(&handshake(), "SYN, then the server sends SYN-ACK").right);
}

// short-answer.feature: Study mode shows the verdict, Choose the question types for a quiz
#[test]
fn a_study_quiz_marks_typed_answers() {
    let (mut app, osi, _) = with_short();
    let view = app.start_quiz(setup(vec![osi], false, QuestionTypes::Short)).unwrap();
    assert_eq!((view.total, view.short, view.options.len()), (1, true, 0));
    assert_eq!(app.answer(vec![0], TODAY).unwrap_err().message, "type your answer for this question");
    assert_eq!(app.answer_text("  ".into(), TODAY).unwrap_err().message, "write an answer before submitting");

    let result = app.answer_text("network layer".into(), TODAY).unwrap();
    let given = feedback(&result);
    assert!(!given.right);
    let short = given.short.as_ref().unwrap();
    assert_eq!(short.typed, "network layer");
    assert_eq!(short.marking.detail, Detail::NoMatch);
    assert_eq!(short.shown_answer, "The **transport** layer");
    assert_eq!(short.also_accepted, vec!["Transport layer", "Layer 4"]);
    assert!(!short.overruled);
    assert_eq!(given.explanation, "Ports let apps share an address.");
    assert!(matches!(result.next, Next::Finished(_)));

    let view = app.start_quiz(setup(vec![osi], false, QuestionTypes::Both)).unwrap();
    assert_eq!(view.total, 2);
    let mcq_only = app.start_quiz(setup(vec![osi], false, QuestionTypes::Mcq)).unwrap();
    assert_eq!((mcq_only.total, mcq_only.short), (1, false));
}

// short-answer.feature: Overrule a verdict
#[test]
fn overruling_in_the_quiz_and_on_the_results() {
    let (mut app, osi, question) = with_short();
    app.start_quiz(setup(vec![osi], false, QuestionTypes::Both)).unwrap();
    let result = app.answer_text("network layer".into(), TODAY).unwrap();
    let index = feedback(&result).index;
    let overruled = app.overrule_answer(None, index).unwrap();
    assert!(overruled.right && overruled.short.as_ref().unwrap().overruled);
    let back = app.overrule_answer(None, index).unwrap();
    assert!(!back.right && !back.short.unwrap().overruled, "undo");
    app.overrule_answer(None, index).unwrap();
    let Next::Question(_) = result.next else { panic!("one more question") };
    let attempt = finished(&app.answer(vec![1], TODAY).unwrap().next);

    let results = app.results(attempt).unwrap();
    assert_eq!(results.summary.overall.right_sure, 1, "the overruled answer counts as right");
    let item = results.review.iter().find(|item| item.short.is_some()).unwrap();
    assert_eq!(item.status, ReviewStatus::Right);
    let short = item.short.as_ref().unwrap();
    assert!(short.overruled && short.typed == "network layer" && !short.marking.right);
    assert_eq!(results.retry_count, 1, "only the wrong MCQ");
    assert!(app.card_suggestions(TODAY).unwrap().iter().all(|s| s.key != format!("quiz:{}", question.0)), "not suggested once overruled to right");

    // Undo from the results page, give a reason, then overrule again: the reason goes.
    let short_index = item.index;
    let results = app.overrule_result(attempt, short_index).unwrap();
    assert_eq!(results.summary.overall.right_sure, 0);
    app.give_reason(attempt, short_index, GivenReason::Misremembered).unwrap();
    let suggestion = app.card_suggestions(TODAY).unwrap().into_iter().find(|s| s.key == format!("quiz:{}", question.0)).unwrap();
    assert_eq!(suggestion.note, "Wrong, you wrote “network layer”");
    assert_eq!(suggestion.answer.as_deref(), Some("The **transport** layer"));
    let results = app.overrule_result(attempt, short_index).unwrap();
    assert_eq!(results.review.iter().find(|i| i.index == short_index).unwrap().reason, None, "no reason on a right answer");
    assert_eq!(app.history().unwrap()[0].summary.overall.right_sure, 1, "history uses the overruled outcome");

    let mcq_index = results.review.iter().find(|i| i.short.is_none()).unwrap().index;
    assert_eq!(app.overrule_result(attempt, mcq_index).unwrap_err().message, "only short answers that were written and marked can be overruled");
}

#[test]
fn overruling_the_last_answer_of_a_study_quiz_updates_the_saved_attempt() {
    let (mut app, osi, _) = with_short();
    app.start_quiz(setup(vec![osi], false, QuestionTypes::Short)).unwrap();
    let result = app.answer_text("network".into(), TODAY).unwrap();
    let attempt = finished(&result.next);
    let given = app.overrule_answer(Some(attempt), feedback(&result).index).unwrap();
    assert!(given.right);
    assert_eq!(app.results(attempt).unwrap().summary.overall.right_sure, 1);
}

#[test]
fn exam_mode_keeps_verdicts_for_the_results() {
    let (mut app, osi, _) = with_short();
    app.start_quiz(setup(vec![osi], true, QuestionTypes::Short)).unwrap();
    let result = app.answer_text("Layer 4".into(), TODAY).unwrap();
    assert!(result.feedback.is_none());
    let results = app.results(finished(&result.next)).unwrap();
    let short = results.review[0].short.as_ref().unwrap();
    assert_eq!(short.marking.detail, Detail::Matched { answer: "Layer 4".into(), typo: None });
    assert_eq!(results.review[0].status, ReviewStatus::Right);
}

// short-answer.feature: Short answers feed recall checklists and cards
#[test]
fn short_answers_can_become_checklist_points() {
    let (mut app, osi, question) = with_short();
    let keywords = app.save_question(short_input(osi, "What opens a TCP connection?", handshake(), "")).unwrap();
    let checklist = app.create_checklist(Subject::Topic(osi), TODAY).unwrap();
    let candidates = app.recall_candidates(checklist).unwrap();
    let short: Vec<_> = candidates.iter().filter(|c| c.kind == "Short answer").map(|c| (c.item, c.answer.as_str())).collect();
    assert_eq!(short, vec![(Item::Question(question), "The **transport** layer"), (Item::Question(keywords), "SYN, SYN-ACK, ACK")]);
    let points = app.add_from_items(checklist, vec![Item::Question(question)], vec![]).unwrap();
    assert_eq!(points[0].text, "Which layer adds port numbers? → The **transport** layer");
}

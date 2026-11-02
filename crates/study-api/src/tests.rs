use super::*;

const TODAY: i64 = 20_394;

fn app() -> App {
    App::with_seed(Store::in_memory().unwrap(), 7)
}

fn input(topic: TopicId, text: &str, multiple: bool, correct: &[usize]) -> QuestionInput {
    QuestionInput {
        topic,
        text: text.into(),
        multiple,
        options: (0..4)
            .map(|index| OptionInput {
                text: format!("{text} option {index}"),
                correct: correct.contains(&index),
            })
            .collect(),
        explanation: format!("Because of {text}"),
        tags: vec!["hardware".into()],
        citations: vec![],
    }
}

fn setup(topics: Vec<TopicId>) -> QuizSetupInput {
    QuizSetupInput {
        topics,
        count: None,
        even: false,
        shuffle_questions: false,
        shuffle_options: false,
        exam: false,
        total_minutes: None,
        per_question_seconds: None,
        easy: false,
    }
}

/// A CPU topic with three questions: q1 single (0), q2 multiple (0, 2), q3 single (1).
fn with_questions() -> (App, TopicId) {
    let mut app = app();
    let cpu = app.create_topic("CPU", None).unwrap();
    app.save_question(input(cpu, "q1", false, &[0])).unwrap();
    app.save_question(input(cpu, "q2", true, &[0, 2])).unwrap();
    app.save_question(input(cpu, "q3", false, &[1])).unwrap();
    (app, cpu)
}

fn finished(next: &Next) -> AttemptId {
    match next {
        Next::Finished(id) => *id,
        Next::Question(_) => panic!("expected the quiz to finish"),
    }
}

// ---------- Topics ----------

#[test]
fn create_and_list_topics_and_modules() {
    let mut app = app();
    let module = app.create_module("Computer Architecture").unwrap();
    let cpu = app.create_topic("CPU", Some(module)).unwrap();
    app.set_topic_tags(cpu, vec!["exam-1".into()]).unwrap();
    let topics = app.topics().unwrap();
    assert_eq!(
        (topics[0].id, topics[0].module, topics[0].tags.clone()),
        (cpu, Some(module), vec!["exam-1".to_string()])
    );
    assert_eq!(app.modules().unwrap()[0].name, "Computer Architecture");
}

#[test]
fn blank_topic_name_is_refused() {
    assert_eq!(
        app().create_topic("   ", None).unwrap_err().message,
        "give the topic a name"
    );
}

// ---------- Questions ----------

#[test]
fn question_rules_come_back_as_readable_errors() {
    let mut app = app();
    let cpu = app.create_topic("CPU", None).unwrap();
    let error = app.save_question(input(cpu, "q", false, &[])).unwrap_err();
    assert_eq!(
        error.message,
        "mark at least one correct option before saving"
    );
}

#[test]
fn questions_round_trip_for_editing() {
    let (mut app, cpu) = with_questions();
    let (id, text) = app.questions_in(cpu).unwrap()[0].clone();
    assert_eq!(text, "q1");
    let mut edited = app.question(id).unwrap();
    edited.text = "q1 edited".into();
    app.update_question(id, edited).unwrap();
    assert_eq!(app.question(id).unwrap().text, "q1 edited");
    app.delete_question(id).unwrap();
    assert_eq!(app.questions_in(cpu).unwrap().len(), 2);
}

// ---------- Quiz: the view never leaks the answer ----------

#[test]
fn quiz_view_does_not_reveal_correct_answers() {
    let (mut app, cpu) = with_questions();
    let view = app
        .start_quiz(QuizSetupInput {
            exam: true,
            ..setup(vec![cpu])
        })
        .unwrap();
    let json = serde_json_like(&view);
    assert!(!json.contains("correct"), "{json}");
    assert_eq!(
        (view.position, view.total, view.text.as_str()),
        (1, 3, "q1")
    );
}

fn serde_json_like(view: &QuizView) -> String {
    format!("{view:?}").to_lowercase()
}

#[test]
fn study_mode_gives_feedback_with_the_answer() {
    let (mut app, cpu) = with_questions();
    app.start_quiz(setup(vec![cpu])).unwrap();
    let result = app.answer(vec![1], TODAY).unwrap();
    let feedback = result.feedback.unwrap();
    assert!(!feedback.right);
    assert_eq!(feedback.correct, vec![0]);
    assert_eq!(feedback.explanation, "Because of q1");
    assert!(matches!(result.next, Next::Question(ref view) if view.position == 2));
}

#[test]
fn exam_mode_gives_no_feedback() {
    let (mut app, cpu) = with_questions();
    app.start_quiz(QuizSetupInput {
        exam: true,
        ..setup(vec![cpu])
    })
    .unwrap();
    assert_eq!(app.answer(vec![0], TODAY).unwrap().feedback, None);
}

#[test]
fn shuffled_options_are_answered_by_their_original_index() {
    let (mut app, cpu) = with_questions();
    let view = app
        .start_quiz(QuizSetupInput {
            shuffle_options: true,
            ..setup(vec![cpu])
        })
        .unwrap();
    let mut shown: Vec<usize> = view.options.iter().map(|option| option.index).collect();
    assert_ne!(shown, vec![0, 1, 2, 3], "seed 7 shuffles");
    shown.sort();
    assert_eq!(shown, vec![0, 1, 2, 3]);
    assert!(app.answer(vec![0], TODAY).unwrap().feedback.unwrap().right);
}

#[test]
fn easy_mode_shows_how_many_to_pick() {
    let (mut app, cpu) = with_questions();
    app.start_quiz(QuizSetupInput {
        easy: true,
        ..setup(vec![cpu])
    })
    .unwrap();
    let next = app.answer(vec![0], TODAY).unwrap().next;
    assert!(
        matches!(next, Next::Question(ref view) if view.multiple && view.answers_to_pick == Some(2))
    );
}

#[test]
fn study_track_shows_results_so_far_exam_track_does_not() {
    let (mut app, cpu) = with_questions();
    app.start_quiz(setup(vec![cpu])).unwrap();
    app.toggle_flag().unwrap();
    let Next::Question(view) = app.answer(vec![0], TODAY).unwrap().next else {
        panic!()
    };
    assert_eq!(
        view.track,
        vec![
            TrackCell::RightGuessed,
            TrackCell::Current,
            TrackCell::Upcoming
        ]
    );

    app.start_quiz(QuizSetupInput {
        exam: true,
        ..setup(vec![cpu])
    })
    .unwrap();
    let Next::Question(view) = app.answer(vec![1], TODAY).unwrap().next else {
        panic!()
    };
    assert_eq!(
        view.track,
        vec![TrackCell::Answered, TrackCell::Current, TrackCell::Upcoming]
    );
}

#[test]
fn finishing_saves_the_attempt_to_history() {
    let (mut app, cpu) = with_questions();
    app.start_quiz(setup(vec![cpu])).unwrap();
    app.answer(vec![0], TODAY).unwrap();
    app.answer(vec![0, 2], TODAY).unwrap();
    let id = finished(&app.answer(vec![0], TODAY).unwrap().next);
    let history = app.history().unwrap();
    assert_eq!(
        (
            history[0].id,
            history[0].day,
            history[0].summary.overall.right()
        ),
        (id, TODAY, 2)
    );
    assert!(
        app.current_question().is_err(),
        "no quiz is running any more"
    );
}

#[test]
fn quitting_and_timeouts_save_the_attempt() {
    let (mut app, cpu) = with_questions();
    app.start_quiz(QuizSetupInput {
        per_question_seconds: Some(30),
        ..setup(vec![cpu])
    })
    .unwrap();
    let after_timeout = app.question_timed_out(TODAY).unwrap();
    assert_eq!(
        after_timeout.feedback.unwrap().auto_reason,
        Some(AutoReason::TimedOut)
    );
    let id = app.quit_quiz(TODAY).unwrap();
    assert_eq!(app.results(id).unwrap().status, Status::Incomplete);

    app.start_quiz(setup(vec![cpu])).unwrap();
    let id = app.total_timed_out(TODAY).unwrap();
    assert_eq!(app.results(id).unwrap().summary.overall.unanswered, 3);
}

#[test]
fn quiz_setup_errors_are_readable() {
    let (mut app, _) = with_questions();
    assert_eq!(
        app.start_quiz(setup(vec![])).unwrap_err().message,
        "pick at least one topic to start"
    );
    assert!(app.answer(vec![0], TODAY).is_err(), "no quiz started");
}

// ---------- Results, reasons, retry ----------

fn finished_with_mistakes() -> (App, AttemptId) {
    let (mut app, cpu) = with_questions();
    app.start_quiz(setup(vec![cpu])).unwrap();
    app.toggle_flag().unwrap();
    app.answer(vec![0], TODAY).unwrap(); // right, guessed
    app.answer(vec![0], TODAY).unwrap(); // incomplete
    let id = finished(&app.answer(vec![0], TODAY).unwrap().next); // wrong
    (app, id)
}

#[test]
fn results_review_wrong_and_guessed_first_and_prompt_for_reasons() {
    let (app, id) = finished_with_mistakes();
    let results = app.results(id).unwrap();
    assert_eq!(
        results
            .review
            .iter()
            .map(|item| item.index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(results.review[0].status, ReviewStatus::RightGuessed);
    assert_eq!(results.review[1].auto_reason, Some(AutoReason::Incomplete));
    assert_eq!(results.review[2].correct, vec![1]);
    assert_eq!(results.to_prompt, vec![2]);
    assert_eq!(results.retry_count, 3);
}

#[test]
fn giving_a_reason_is_saved() {
    let (mut app, id) = finished_with_mistakes();
    let results = app.give_reason(id, 2, GivenReason::Misremembered).unwrap();
    assert!(results.to_prompt.is_empty());
    assert_eq!(
        app.results(id).unwrap().review[2].reason,
        Some(GivenReason::Misremembered)
    );
    assert_eq!(
        app.give_reason(id, 0, GivenReason::Blank)
            .unwrap_err()
            .message,
        "only wrong answers get a reason"
    );
}

#[test]
fn retry_starts_a_quiz_of_wrong_and_guessed() {
    let (mut app, id) = finished_with_mistakes();
    let view = app.retry(id).unwrap();
    assert_eq!(view.total, 3);
    app.answer(vec![0], TODAY).unwrap();
    app.answer(vec![0, 2], TODAY).unwrap();
    let retry_id = finished(&app.answer(vec![1], TODAY).unwrap().next);
    assert_eq!(app.history().unwrap()[0].retry_of, Some(id));
    assert_eq!(app.results(retry_id).unwrap().retry_count, 0);
}

// ---------- Overview, bibliography, search ----------

#[test]
fn overview_is_first_run_until_a_quiz_is_saved() {
    let (app, _) = with_questions();
    assert_eq!(app.overview(TODAY).unwrap(), Overview::FirstRun);
    let (app, _) = finished_with_mistakes();
    assert!(
        matches!(app.overview(TODAY).unwrap(), Overview::Progress(ref p) if p.accuracy == Some(33))
    );
}

#[test]
fn sources_and_search_pass_through() {
    let (mut app, _) = with_questions();
    app.add_source(SourceFields {
        authors: "Kirk".into(),
        title: "Parallel".into(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(app.sources().unwrap()[0].apa, "Kirk. (n.d.). *Parallel*.");
    assert_eq!(
        app.search(SearchQuery {
            text: "q2".into(),
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
}

// R43, R44, R45
#[test]
fn modules_and_sources_can_be_edited_and_deleted() {
    let mut app = app();
    let arch = app.create_module("Arch").unwrap();
    let gpu = app.create_topic("GPU", Some(arch)).unwrap();
    app.delete_module(arch).unwrap();
    assert_eq!(app.topics().unwrap()[0].module, None);

    let fields = SourceFields { authors: "Kirk".into(), title: "Parallel".into(), ..Default::default() };
    let source = app.add_source(fields.clone()).unwrap();
    app.update_source(source, SourceFields { title: "Massively parallel".into(), ..fields.clone() }).unwrap();
    assert_eq!(app.sources().unwrap()[0].fields.title, "Massively parallel");
    assert!(app.update_source(source, SourceFields { title: " ".into(), ..fields }).is_err());

    let mut cited = input(gpu, "Q", false, &[0]);
    cited.citations = vec![CitationInput { source, location: "p. 1".into() }];
    app.save_question(cited).unwrap();
    assert_eq!(app.delete_source(source).unwrap(), 1);
    assert!(app.sources().unwrap().is_empty());
}

// R38
#[test]
fn empty_topic_cant_start_a_quiz() {
    let mut app = app();
    let empty = app.create_topic("Misc", None).unwrap();
    let error = app.start_quiz(setup(vec![empty])).unwrap_err();
    assert_eq!(error.message, "a chosen topic has no questions yet");
}

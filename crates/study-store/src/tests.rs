use super::*;
use study_core::assembly::{Mode, QuizItem, QuizSettings};
use study_core::attempt::Outcome;
use study_core::results::GivenReason;

fn store() -> Store {
    Store::in_memory().unwrap()
}

fn question(topic: TopicId, text: &str) -> Question {
    Question::new(QuestionDraft {
        topic,
        text: text.into(),
        kind: AnswerKind::Multiple,
        options: vec![
            AnswerOption {
                text: "All threads run the same instruction".into(),
                correct: true,
            },
            AnswerOption {
                text: "Each thread has its own program counter".into(),
                correct: false,
            },
            AnswerOption {
                text: "Divergent branches are serialised".into(),
                correct: true,
            },
        ],
        explanation: "A warp runs in SIMT lockstep, with $\\LaTeX$ and `code`.".into(),
        tags: vec!["hardware".into(), "warps".into()],
        citations: vec![],
        short: None,
    })
    .unwrap()
}

fn kirk() -> SourceFields {
    SourceFields {
        authors: "Kirk, D. B., & Hwu, W. W.".into(),
        year: Some("2022".into()),
        title: "Programming massively parallel processors".into(),
        edition: Some("4th".into()),
        publisher: Some("Morgan Kaufmann".into()),
        ..Default::default()
    }
}

fn finished_attempt(store: &Store, topic: TopicId) -> Attempt {
    let questions = store.questions_in(&[topic]).unwrap();
    let items = questions
        .iter()
        .map(|(id, q)| QuizItem {
            question: *id,
            option_order: (0..q.options().len()).collect(),
        })
        .collect();
    let lookup = |id: QuestionId| {
        questions
            .iter()
            .find(|(qid, _)| *qid == id)
            .unwrap()
            .1
            .clone()
    };
    let mut attempt = Attempt::start(
        QuizSettings {
            mode: Mode::Study,
            ..QuizSettings::for_topics(vec![topic])
        },
        items,
        lookup,
    );
    attempt.submit(&[0, 2]).unwrap();
    attempt.submit(&[1]).unwrap();
    attempt
}

// ---------- Schema ----------

#[test]
fn new_database_is_migrated_to_the_latest_schema() {
    assert_eq!(store().schema_version().unwrap(), schema::LATEST);
}

#[test]
fn reopening_a_database_keeps_its_data() {
    let dir = std::env::temp_dir().join(format!("study-store-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("reopen.sqlite");
    let _ = std::fs::remove_file(&path);
    {
        let mut store = Store::open(&path).unwrap();
        store.create_topic("CPU", None).unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.topics().unwrap()[0].name, "CPU");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn refuses_a_database_from_a_newer_app() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "user_version", schema::LATEST + 1)
        .unwrap();
    assert!(matches!(
        Store::from_connection(connection),
        Err(StoreError::NewerSchema { .. })
    ));
}

// ---------- topics.feature ----------

#[test]
fn create_topics_in_a_module_or_none() {
    let mut store = store();
    let architecture = store.create_module("Computer Architecture").unwrap();
    let cpu = store.create_topic("CPU", Some(architecture)).unwrap();
    let misc = store.create_topic("Misc", None).unwrap();
    let topics = store.topics().unwrap();
    assert_eq!(
        topics.iter().find(|t| t.id == cpu).unwrap().module,
        Some(architecture)
    );
    assert_eq!(topics.iter().find(|t| t.id == misc).unwrap().module, None);
    assert_eq!(
        store.modules().unwrap(),
        vec![ModuleRow {
            id: architecture,
            name: "Computer Architecture".into()
        }]
    );
}

#[test]
fn move_a_topic_to_another_module() {
    let mut store = store();
    let architecture = store.create_module("Computer Architecture").unwrap();
    let hardware = store.create_module("Hardware").unwrap();
    let cpu = store.create_topic("CPU", Some(architecture)).unwrap();
    store.move_topic(cpu, Some(hardware)).unwrap();
    assert_eq!(store.topics().unwrap()[0].module, Some(hardware));
}

#[test]
fn tag_a_topic() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    store.set_topic_tags(gpu, &["hardware", "exam-1"]).unwrap();
    assert_eq!(
        store.topics().unwrap()[0].tags,
        vec!["exam-1".to_string(), "hardware".to_string()]
    );
}

#[test]
fn topic_list_counts_questions() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    store.save_question(&question(gpu, "Q1")).unwrap();
    store.save_question(&question(gpu, "Q2")).unwrap();
    assert_eq!(store.topics().unwrap()[0].question_count, 2);
}

#[test]
fn delete_a_topic_that_is_in_history() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    store
        .save_question(&question(gpu, "Which describe a warp?"))
        .unwrap();
    store.save_question(&question(gpu, "Second")).unwrap();
    let attempt = finished_attempt(&store, gpu);
    let saved = store
        .save_attempt(&attempt, 100, &Reasons::default())
        .unwrap();

    store.delete_topic(gpu).unwrap();
    assert!(store.topics().unwrap().is_empty());
    assert!(store.questions_in(&[gpu]).unwrap().is_empty());
    let (from_history, _) = store.attempt(saved).unwrap();
    assert_eq!(
        from_history.asked()[0].snapshot.text(),
        "Which describe a warp?"
    );
}

// ---------- questions.feature ----------

#[test]
fn a_question_round_trips_with_options_tags_and_rich_text() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    let original = question(gpu, "Which describe a warp?");
    let id = store.save_question(&original).unwrap();
    assert_eq!(store.question(id).unwrap(), original);
}

#[test]
fn questions_come_back_for_the_chosen_topics_only() {
    let mut store = store();
    let cpu = store.create_topic("CPU", None).unwrap();
    let gpu = store.create_topic("GPU", None).unwrap();
    store.save_question(&question(cpu, "cpu q")).unwrap();
    store.save_question(&question(gpu, "gpu q")).unwrap();
    let got = store.questions_in(&[gpu]).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].1.text(), "gpu q");
}

#[test]
fn delete_a_question_that_is_in_history() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    let first = store
        .save_question(&question(gpu, "Which describe a warp?"))
        .unwrap();
    store.save_question(&question(gpu, "Second")).unwrap();
    let saved = store
        .save_attempt(&finished_attempt(&store, gpu), 100, &Reasons::default())
        .unwrap();
    store.delete_question(first).unwrap();
    assert!(matches!(
        store.question(first),
        Err(StoreError::NotFound(_))
    ));
    assert_eq!(
        store.attempt(saved).unwrap().0.asked()[0].snapshot.text(),
        "Which describe a warp?"
    );
}

// ---------- history.feature: History shows the question as it was ----------

#[test]
fn editing_a_question_does_not_change_history() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    let first = store
        .save_question(&question(gpu, "What does the ALU do?"))
        .unwrap();
    store.save_question(&question(gpu, "Second")).unwrap();
    let saved = store
        .save_attempt(&finished_attempt(&store, gpu), 100, &Reasons::default())
        .unwrap();
    store
        .update_question(first, &question(gpu, "What is the ALU's job?"))
        .unwrap();
    assert_eq!(
        store.question(first).unwrap().text(),
        "What is the ALU's job?"
    );
    assert_eq!(
        store.attempt(saved).unwrap().0.asked()[0].snapshot.text(),
        "What does the ALU do?"
    );
}

// ---------- sources.feature ----------

#[test]
fn bibliography_lists_sources_in_apa_with_the_topics_that_use_them() {
    let mut store = store();
    let cpu = store.create_topic("CPU", None).unwrap();
    let gpu = store.create_topic("GPU", None).unwrap();
    let source = store.add_source(&kirk()).unwrap();
    for topic in [cpu, gpu] {
        let mut draft_question = question(topic, "cites kirk");
        draft_question = Question::new(QuestionDraft {
            topic,
            text: draft_question.text().into(),
            kind: draft_question.kind(),
            options: draft_question.options().to_vec(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![Citation {
                source,
                location: "p. 88".into(),
            }],
            short: None,
        })
        .unwrap();
        store.save_question(&draft_question).unwrap();
    }
    let rows = store.sources().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].apa,
        "Kirk, D. B., & Hwu, W. W. (2022). *Programming massively parallel processors* (4th ed.). Morgan Kaufmann."
    );
    assert_eq!(rows[0].used_by, vec![cpu, gpu]);
    assert_eq!(rows[0].cited_by_questions, 2);
    assert_eq!(rows[0].fields, kirk());
}

// ---------- history.feature / retry.feature / results.feature ----------

#[test]
fn attempts_are_listed_newest_first_with_retries_and_incomplete_marked() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    store.save_question(&question(gpu, "Q1")).unwrap();
    store.save_question(&question(gpu, "Q2")).unwrap();
    let first = finished_attempt(&store, gpu);
    let first_id = store
        .save_attempt(&first, 100, &Reasons::default())
        .unwrap();
    let mut retry = first.retry(first_id.0 as usize).unwrap();
    retry.quit();
    store
        .save_attempt(&retry, 101, &Reasons::default())
        .unwrap();

    let history = store.history().unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(
        (history[0].day, history[0].status, history[0].retry_of),
        (101, Status::Incomplete, Some(first_id))
    );
    assert_eq!(
        (history[1].day, history[1].status, history[1].retry_of),
        (100, Status::Finished, None)
    );
    assert_eq!(history[1].summary.overall.right(), 1);
    assert_eq!(history[1].topics, vec![gpu]);
}

#[test]
fn reasons_are_saved_and_feed_the_overview() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    store.save_question(&question(gpu, "Q1")).unwrap();
    store.save_question(&question(gpu, "Q2")).unwrap();
    let attempt = finished_attempt(&store, gpu);
    assert_eq!(attempt.asked()[1].outcome, Some(Outcome::Wrong(None)));
    let id = store
        .save_attempt(&attempt, 100, &Reasons::default())
        .unwrap();

    let mut reasons = Reasons::default();
    reasons
        .give(&attempt, 1, GivenReason::Misremembered)
        .unwrap();
    store.save_reasons(id, &reasons).unwrap();

    assert_eq!(store.attempt(id).unwrap().1, reasons);
    let records = store.attempt_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].reason_labels, vec!["Misremembered".to_string()]);
    assert_eq!(records[0].day, 100);
    assert_eq!(records[0].sequence, id.0 as u64);
}

// R43: deleting a module moves its topics to "No module"
#[test]
fn deleting_a_module_keeps_its_topics() {
    let mut store = store();
    let arch = store.create_module("Computer Architecture").unwrap();
    let gpu = store.create_topic("GPU", Some(arch)).unwrap();
    store.save_question(&question(gpu, "Q1")).unwrap();
    store.delete_module(arch).unwrap();
    assert!(store.modules().unwrap().is_empty());
    let topics = store.topics().unwrap();
    assert_eq!(topics.len(), 1);
    assert_eq!(topics[0].module, None);
    assert_eq!(topics[0].question_count, 1);
    assert!(matches!(store.delete_module(arch), Err(StoreError::NotFound(_))));
}

fn cited(topic: TopicId, source: SourceId) -> Question {
    Question::new(QuestionDraft {
        topic,
        text: "Cited".into(),
        kind: AnswerKind::Single,
        options: (0..3)
            .map(|i| AnswerOption { text: format!("O{i}"), correct: i == 0 })
            .collect(),
        explanation: String::new(),
        tags: vec![],
        citations: vec![study_core::question::Citation { source, location: "p. 4".into() }],
        short: None,
    })
    .unwrap()
}

// R45: editing a source updates it
#[test]
fn editing_a_source_updates_it() {
    let mut store = store();
    let id = store.add_source(&kirk()).unwrap();
    let mut fields = kirk();
    fields.year = Some("2023".into());
    store.update_source(id, &fields).unwrap();
    assert_eq!(store.sources().unwrap()[0].fields.year.as_deref(), Some("2023"));
}

// R46: a source needs a title
#[test]
fn a_source_needs_a_title() {
    let mut store = store();
    let mut fields = kirk();
    fields.title = "  ".into();
    assert!(matches!(store.add_source(&fields), Err(StoreError::Invalid(_))));
    let id = store.add_source(&kirk()).unwrap();
    assert!(matches!(store.update_source(id, &fields), Err(StoreError::Invalid(_))));
}

// R44: deleting a cited source removes the citations and says how many
#[test]
fn deleting_a_source_removes_its_citations() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    let source = store.add_source(&kirk()).unwrap();
    let q = store.save_question(&cited(gpu, source)).unwrap();
    assert_eq!(store.delete_source(source).unwrap(), 1);
    assert!(store.sources().unwrap().is_empty());
    assert!(store.question(q).unwrap().citations().is_empty());
}

// questions.feature: Add an image by pasting or picking ("the image is kept with the question")
#[test]
fn keeps_images_and_reuses_a_repeated_paste() {
    use study_core::image::{ImageFormat, ImageId};
    let mut store = store();
    let png = b"\x89PNG\r\n\x1a\npipeline".to_vec();
    let first = store.save_image(ImageFormat::Png, &png).unwrap();
    let again = store.save_image(ImageFormat::Png, &png).unwrap();
    let other = store.save_image(ImageFormat::Gif, b"GIF89a-other").unwrap();
    assert_eq!(first, again);
    assert_ne!(first, other);
    assert_eq!(store.image(first).unwrap(), (ImageFormat::Png, png));
    assert!(matches!(store.image(ImageId(999)), Err(StoreError::NotFound("image"))));
}

#[test]
fn images_survive_reopening_the_database() {
    use study_core::image::ImageFormat;
    let path = std::env::temp_dir().join(format!("study-images-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let id = Store::open(&path).unwrap().save_image(ImageFormat::Jpeg, &[0xFF, 0xD8, 0xFF, 1]).unwrap();
    let (format, bytes) = Store::open(&path).unwrap().image(id).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!((format, bytes), (ImageFormat::Jpeg, vec![0xFF, 0xD8, 0xFF, 1]));
}

// ---------- short-answer.feature ----------

fn short_question(topic: TopicId, check: study_core::question::Check) -> Question {
    Question::new(QuestionDraft {
        topic,
        text: "Which OSI layer adds port numbers?".into(),
        kind: AnswerKind::Short,
        options: vec![],
        explanation: "Ports let apps share an address.".into(),
        tags: vec!["layers".into()],
        citations: vec![],
        short: Some(study_core::question::ShortAnswer { check, model_answer: "The **transport** layer".into() }),
    })
    .unwrap()
}

#[test]
fn short_answer_questions_round_trip_and_can_switch_type() {
    use study_core::question::{Check, Keyword};
    let mut store = store();
    let osi = store.create_topic("OSI model", None).unwrap();
    let accepted = short_question(osi, Check::Accepted { answers: vec!["Transport layer".into(), "Layer 4".into()], typos: false });
    let id = store.save_question(&accepted).unwrap();
    assert_eq!(store.question(id).unwrap(), accepted);

    let keywords = short_question(osi, Check::Keywords { keywords: vec![Keyword { word: "SYN-ACK".into(), also: vec!["synack".into()] }], needed: Some(1) });
    store.update_question(id, &keywords).unwrap();
    assert_eq!(store.question(id).unwrap(), keywords);

    let mcq = question(osi, "Now multiple choice");
    store.update_question(id, &mcq).unwrap();
    assert_eq!(store.question(id).unwrap(), mcq);
    store.update_question(id, &accepted).unwrap();
    assert_eq!(store.question(id).unwrap().options(), &[], "the old options are gone");
}

// short-answer.feature: Choose the question types for a quiz (topic rows show how many of each)
#[test]
fn topics_count_short_answer_questions_separately() {
    use study_core::question::Check;
    let mut store = store();
    let osi = store.create_topic("OSI model", None).unwrap();
    store.save_question(&question(osi, "Q1")).unwrap();
    store.save_question(&short_question(osi, Check::Accepted { answers: vec!["A".into()], typos: true })).unwrap();
    let row = &store.topics().unwrap()[0];
    assert_eq!((row.question_count, row.short_count), (2, 1));
}

// short-answer.feature: Overrule a verdict (on the results page, after the quiz is saved)
#[test]
fn a_saved_attempt_can_be_updated() {
    let mut store = store();
    let gpu = store.create_topic("GPU", None).unwrap();
    store.save_question(&question(gpu, "Q1")).unwrap();
    store.save_question(&question(gpu, "Q2")).unwrap();
    let mut attempt = finished_attempt(&store, gpu);
    attempt.overrule(0).ok();
    let id = store.save_attempt(&attempt, 20_760, &Reasons::default()).unwrap();
    let (saved, reasons) = store.attempt(id).unwrap();
    assert_eq!(saved, attempt);
    let mut saved = first_two_asked_differently(saved);
    store.update_attempt(id, &saved).unwrap();
    saved = store.attempt(id).unwrap().0.clone();
    assert_eq!(store.attempt(id).unwrap(), (saved.clone(), reasons));
    assert_ne!(saved, attempt);
    assert!(matches!(store.update_attempt(AttemptId(99), &attempt), Err(StoreError::NotFound(_))));
}

#[test]
fn migrating_to_short_answers_keeps_questions_and_their_links() {
    let connection = Connection::open_in_memory().unwrap();
    schema::migrate_to(&connection, 5).unwrap();
    connection
        .execute_batch(
            "INSERT INTO topics (id, name) VALUES (1, 'CPU');
             INSERT INTO questions (id, topic_id, kind, text, explanation) VALUES (7, 1, 'single', 'Which stage reads registers?', 'Decode');
             INSERT INTO options (question_id, position, text, correct) VALUES (7, 0, 'Fetch', 0), (7, 1, 'Decode', 1), (7, 2, 'Execute', 0);
             INSERT INTO question_tags (question_id, tag) VALUES (7, 'pipeline');
             INSERT INTO recall_checklists (id, topic_id, subject_name, subject_kind, created, due) VALUES (1, 1, 'CPU', 'Topic', 0, 0);
             INSERT INTO recall_points (checklist_id, position, text, question_id) VALUES (1, 0, 'Which stage reads registers? → Decode', 7);",
        )
        .unwrap();
    let mut store = Store::from_connection(connection).unwrap();
    assert_eq!(store.schema_version().unwrap(), schema::LATEST);
    let question = store.question(QuestionId(7)).unwrap();
    assert_eq!((question.text(), question.correct_indices(), question.tags()), ("Which stage reads registers?", vec![1], &["pipeline".to_string()][..]));
    assert_eq!(store.points(study_store_recall_id(1)).unwrap()[0].question, Some(QuestionId(7)), "recall points still point at it");
    store.delete_question(QuestionId(7)).unwrap();
    let options: i64 = store.connection.query_row("SELECT COUNT(*) FROM options", [], |row| row.get(0)).unwrap();
    assert_eq!(options, 0, "foreign keys are on again, so deleting cascades");
    assert_eq!(store.points(study_store_recall_id(1)).unwrap()[0].question, None);
}

fn study_store_recall_id(id: i64) -> crate::recall::ChecklistId {
    crate::recall::ChecklistId(id)
}

/// A changed copy of an attempt, to check the update is written.
fn first_two_asked_differently(attempt: Attempt) -> Attempt {
    let mut json = serde_json::to_value(&attempt).unwrap();
    json["asked"][0]["flagged_guessed"] = serde_json::Value::Bool(true);
    serde_json::from_value(json).unwrap()
}

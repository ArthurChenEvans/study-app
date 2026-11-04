//! transfer.feature through the command layer (R112 to R118).

use super::*;
use crate::cards::{NoteInput, ReviewStep};
use crate::practice::Clock;
use crate::transfer::{ClashRule, ExportRequest, ImportChoice, unpack};
use serde_json::Value;
use study_core::cards::{Note, NoteKind, Rating};

const TODAY: i64 = 20_761;

fn at(day: i64) -> Clock {
    Clock { day, minute: 600, unix: day * 86_400 + 36_000 }
}

fn seeded() -> App {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 3);
    crate::seed::seed_profile(&mut app, TODAY).unwrap();
    app
}

fn everything() -> ExportRequest {
    ExportRequest { topics: None, history: true }
}

fn topic_named(app: &App, name: &str) -> TopicId {
    app.topics().unwrap().into_iter().find(|topic| topic.name == name).unwrap_or_else(|| panic!("no topic {name}")).id
}

fn import(app: &mut App, bytes: &[u8], rule: ClashRule) -> crate::transfer::ImportResult {
    app.import_preview(bytes, "file.studyapp").unwrap();
    app.import_apply(ImportChoice { rule, theirs: vec![] }, at(TODAY)).unwrap()
}

/// A view with the local row numbers taken out, so two databases can be compared.
fn without_ids<T: serde::Serialize>(view: &T) -> Value {
    fn strip(value: &mut Value) {
        match value {
            // A map keyed by row numbers (per-topic scores) becomes its values in order.
            Value::Object(fields) if !fields.is_empty() && fields.keys().all(|key| key.parse::<i64>().is_ok()) => {
                let mut values: Vec<Value> = fields.values().cloned().collect();
                values.iter_mut().for_each(strip);
                values.sort_by_key(|value| value.to_string());
                *value = Value::Array(values);
            }
            Value::Object(fields) => {
                for key in ["id", "attempt", "topic", "topics", "question", "source", "retry_of", "card", "note", "checklist", "project", "drill", "module", "issue", "session"] {
                    fields.remove(key);
                }
                fields.values_mut().for_each(strip);
            }
            Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(view).unwrap();
    strip(&mut value);
    value
}

// transfer.feature: Moving everything to another computer (R112, R114)
#[test]
fn everything_comes_back_the_same_in_a_database_that_already_has_other_things() {
    let mut original = seeded();
    // A dismissed quiz mistake has to stay dismissed after the move.
    let quiz_mistake = original.card_suggestions(TODAY).unwrap().into_iter().find(|suggestion| suggestion.key.starts_with("quiz:")).expect("a quiz mistake to dismiss");
    original.dismiss_suggestion(&quiz_mistake.key).unwrap();
    let (name, file) = original.export(&everything(), TODAY).unwrap();
    assert_eq!(name, "Study App 2026-11-04.studyapp");

    // Something here first, so every row number the file's records get is different.
    let mut other = App::with_seed(Store::in_memory().unwrap(), 4);
    let unrelated = other.create_module("Unrelated").unwrap();
    let topic = other.create_topic("Already here", Some(unrelated)).unwrap();
    other.save_note(None, NoteInput { note: Note { topic, kind: NoteKind::Basic, front: "Mine".into(), back: "Mine".into(), cues: vec![], text: String::new(), extra: String::new() }, suggestion: None }, TODAY).unwrap();
    let question = other.save_question(QuestionInput { topic, text: "Mine too".into(), multiple: false, options: ["A", "B", "C"].iter().enumerate().map(|(i, text)| OptionInput { text: (*text).into(), correct: i == 0 }).collect(), explanation: String::new(), tags: vec![], citations: vec![], short: None }).unwrap();
    let mine: std::collections::HashSet<String> = [other.store.stamp_of("modules", unrelated.0).unwrap().uid, other.store.stamp_of("topics", topic.0).unwrap().uid, other.store.stamp_of("questions", question.0).unwrap().uid].into();

    let preview = other.import_preview(&file, &name).unwrap();
    assert!(preview.everything && preview.history && !preview.from_here);
    assert!(preview.clashes.is_empty());
    assert!(preview.rows.iter().all(|row| row.same == 0 && row.changed == 0), "{:?}", preview.rows);
    let added = other.import_apply(ImportChoice { rule: preview.suggested, theirs: vec![] }, at(TODAY)).unwrap();
    assert!(added.added.iter().any(|count| count.what == "card reviews" && count.count > 250), "{:?}", added.added);

    // Export again: record for record, the same as the original.
    let (_, again) = other.export(&everything(), TODAY).unwrap();
    let (_, first) = unpack(&file).unwrap();
    let (_, mut second) = unpack(&again).unwrap();
    second.modules.retain(|module| !mine.contains(&module.uid));
    second.topics.retain(|topic| !mine.contains(&topic.uid));
    second.notes.retain(|note| note.note["front"] != "Mine");
    second.questions.retain(|question| !mine.contains(&question.uid));
    assert_eq!(first.topics, second.topics);
    assert_eq!(first.questions, second.questions);
    assert_eq!(first.notes, second.notes);
    assert_eq!(first.reviews, second.reviews);
    assert_eq!(first.checklists, second.checklists);
    assert_eq!(first.checks, second.checks);
    assert_eq!(first.attempts, second.attempts, "quizzes, with their questions, topics and sources");
    assert_eq!(first.struggles, second.struggles, "struggles, with what became of them");
    assert_eq!(first.settings, second.settings);
    assert_eq!(first.suggestions_done, second.suggestions_done);
    assert_eq!(first, second);

    // And what the screens show is the same.
    assert_eq!(without_ids(&original.history().unwrap()), without_ids(&other.history().unwrap()));
    for (from, to) in original.history().unwrap().iter().zip(other.history().unwrap().iter()) {
        assert_eq!(without_ids(&original.results(from.id).unwrap()), without_ids(&other.results(to.id).unwrap()));
    }
    // Suggestions from quiz mistakes follow old quizzes back to today's questions.
    let suggestions = |app: &App| {
        let mut list: Vec<(String, String)> = app.card_suggestions(TODAY).unwrap().into_iter().map(|suggestion| (suggestion.text, format!("{} {:?} {:?}", suggestion.from, suggestion.answer, suggestion.day))).collect();
        list.sort();
        list
    };
    assert!(!suggestions(&original).is_empty());
    assert_eq!(suggestions(&original), suggestions(&other));
    let names = |app: &App| app.recall_list(TODAY).unwrap().into_iter().map(|row| (row.subject_name, row.due)).collect::<Vec<_>>();
    assert_eq!(names(&original), names(&other));
    let projects = |app: &App| app.projects(TODAY).unwrap().into_iter().map(|row| (row.name, row.struggles, row.to_review, row.drills_due)).collect::<Vec<_>>();
    assert_eq!(projects(&original), projects(&other));
    let (theirs, mine) = (original.cards_home(TODAY).unwrap().due, other.cards_home(TODAY).unwrap().due);
    assert_eq!((theirs.new + 1, theirs.learning, theirs.review), (mine.new, mine.learning, mine.review), "plus the one new card that was here already");
}

#[test]
fn importing_the_same_file_twice_adds_nothing_the_second_time() {
    let original = seeded();
    let (_, file) = original.export(&everything(), TODAY).unwrap();
    let mut other = App::with_seed(Store::in_memory().unwrap(), 4);
    import(&mut other, &file, ClashRule::Newer);
    let preview = other.import_preview(&file, "again").unwrap();
    assert!(preview.rows.iter().all(|row| row.new == 0 && row.changed == 0), "{:?}", preview.rows);
    let result = other.import_apply(ImportChoice { rule: ClashRule::TakeTheirs, theirs: vec![] }, at(TODAY)).unwrap();
    assert!(result.added.is_empty() && result.replaced == 0, "{result:?}");
    assert_eq!(other.topics().unwrap().len(), original.topics().unwrap().len());
}

// transfer.feature: Sharing some topics (R113)
#[test]
fn sharing_topics_takes_their_questions_cards_checklists_and_sources_but_not_history() {
    let original = seeded();
    let osi = topic_named(&original, "OSI model");
    let request = ExportRequest { topics: Some(vec![osi]), history: false };
    let summary = original.export_summary(&request, TODAY).unwrap();
    assert_eq!(summary.name, "OSI model.studyapp");
    let (_, file) = original.export(&request, TODAY).unwrap();
    let (manifest, snapshot) = unpack(&file).unwrap();
    assert!(!manifest.everything && !manifest.history);
    assert_eq!(snapshot.topics.iter().map(|topic| topic.name.as_str()).collect::<Vec<_>>(), ["OSI model"]);
    assert_eq!(snapshot.modules.iter().map(|module| module.name.as_str()).collect::<Vec<_>>(), ["Networks"]);
    assert!(!snapshot.questions.is_empty() && !snapshot.notes.is_empty());
    assert!(snapshot.checklists.iter().any(|checklist| checklist.subject_name == "OSI model"));
    assert!(snapshot.reviews.is_empty() && snapshot.checks.is_empty() && snapshot.attempts.is_empty() && snapshot.projects.is_empty() && snapshot.settings.is_empty());
    assert!(snapshot.notes.iter().flat_map(|note| &note.cards).all(|card| card.schedule.is_none()));
    assert!(snapshot.topics[0].schedule.is_none());
    assert_eq!(summary.counts.iter().find(|count| count.what == "Questions").unwrap().count, snapshot.questions.len());

    let mut friend = App::with_seed(Store::in_memory().unwrap(), 5);
    let preview = friend.import_preview(&file, "OSI model.studyapp").unwrap();
    assert_eq!(preview.suggested, ClashRule::KeepMine);
    friend.import_apply(ImportChoice { rule: preview.suggested, theirs: vec![] }, at(TODAY)).unwrap();
    let home = friend.cards_home(TODAY).unwrap();
    assert_eq!(home.total_cards, snapshot.notes.iter().map(|note| note.cards.len()).sum::<usize>());
    assert_eq!(home.due.review, 0, "cards start as new for a friend");
    assert!(friend.history().unwrap().is_empty());
}

// transfer.feature: An updated version of a shared topic updates it (R115, R116)
#[test]
fn a_newer_version_of_a_shared_topic_updates_it_instead_of_copying_and_clashes_are_settled_as_chosen() {
    let mut original = seeded();
    let osi = topic_named(&original, "OSI model");
    let share = ExportRequest { topics: Some(vec![osi]), history: false };
    let (_, first) = original.export(&share, TODAY).unwrap();
    let mut friend = App::with_seed(Store::in_memory().unwrap(), 5);
    import(&mut friend, &first, ClashRule::KeepMine);
    let questions_before = friend.store.questions_in(&[topic_named(&friend, "OSI model")]).unwrap().len();

    // The original edits a question, then shares again.
    let (question, _) = original.store.questions_in(&[osi]).unwrap()[0].clone();
    let mut edited = original.question(question).unwrap();
    edited.text = "Which layer adds port numbers? (edited)".into();
    original.update_question(question, edited).unwrap();
    let (_, second) = original.export(&share, TODAY).unwrap();

    let preview = friend.import_preview(&second, "OSI model.studyapp").unwrap();
    assert_eq!(preview.clashes.len(), 1, "{:?}", preview.clashes);
    let clash = &preview.clashes[0];
    assert!(clash.theirs_newer && clash.label.starts_with("Which layer adds port numbers? (edited)") && clash.what == "Question");
    let row = preview.rows.iter().find(|row| row.what == "Questions").unwrap();
    assert_eq!((row.new, row.changed), (0, 1));

    // Keep mine: nothing changes.
    friend.import_apply(ImportChoice { rule: ClashRule::KeepMine, theirs: vec![] }, at(TODAY)).unwrap();
    let friend_osi = topic_named(&friend, "OSI model");
    let texts = |app: &App| app.store.questions_in(&[friend_osi]).unwrap().into_iter().map(|(id, _)| app.question(id).unwrap().text).collect::<Vec<_>>();
    assert!(!texts(&friend).iter().any(|text| text.ends_with("(edited)")));
    // Look at each one, taking theirs: the question is updated, not copied.
    let preview = friend.import_preview(&second, "OSI model.studyapp").unwrap();
    friend.import_apply(ImportChoice { rule: ClashRule::EachOne, theirs: vec![preview.clashes[0].uid.clone()] }, at(TODAY)).unwrap();
    assert!(texts(&friend).iter().any(|text| text.ends_with("(edited)")));
    assert_eq!(texts(&friend).len(), questions_before);
    assert_eq!(friend.topics().unwrap().len(), 1);
}

#[test]
fn whichever_changed_last_wins_with_the_newer_rule() {
    let mut original = seeded();
    let osi = topic_named(&original, "OSI model");
    let share = ExportRequest { topics: Some(vec![osi]), history: false };
    let (_, first) = original.export(&share, TODAY).unwrap();
    let mut friend = App::with_seed(Store::in_memory().unwrap(), 5);
    import(&mut friend, &first, ClashRule::KeepMine);
    let friend_osi = topic_named(&friend, "OSI model");

    original.set_topic_tags(osi, vec!["theirs".into()]).unwrap();
    let (_, second) = original.export(&share, TODAY).unwrap();
    friend.set_topic_tags(friend_osi, vec!["mine, later".into()]).unwrap();
    let preview = friend.import_preview(&second, "x").unwrap();
    assert_eq!(preview.clashes.len(), 1);
    assert!(!preview.clashes[0].theirs_newer, "the friend's edit is later");
    friend.import_apply(ImportChoice { rule: ClashRule::Newer, theirs: vec![] }, at(TODAY)).unwrap();
    assert_eq!(friend.topics().unwrap()[0].tags, ["mine, later"]);
    let preview = friend.import_preview(&second, "x").unwrap();
    friend.import_apply(ImportChoice { rule: ClashRule::TakeTheirs, theirs: vec![] }, at(TODAY)).unwrap();
    assert_eq!(preview.clashes.len(), 1);
    assert_eq!(friend.topics().unwrap()[0].tags, ["theirs"]);
}

// transfer.feature: Both computers reviewed a card (R117)
#[test]
fn the_side_reviewed_more_recently_keeps_a_cards_schedule() {
    let mut original = seeded();
    let (_, file) = original.export(&everything(), TODAY).unwrap();
    let mut other = App::with_seed(Store::in_memory().unwrap(), 4);
    import(&mut other, &file, ClashRule::Newer);
    let schedule = |app: &App| app.store.cards().unwrap().into_iter().map(|card| (card.schedule.last_review, card.schedule.due)).collect::<Vec<_>>();

    // The original reviews today; the copy hasn't.
    let ReviewStep::Card(card) = original.start_review(None, TODAY).unwrap() else { panic!("something due today") };
    original.grade_card(Rating::Good, 5, TODAY).unwrap();
    original.leave_review();
    let (_, later) = original.export(&everything(), TODAY).unwrap();
    let before = schedule(&other);
    let result = import(&mut other, &later, ClashRule::Newer);
    assert!(result.added.iter().any(|count| count.what == "card reviews" && count.count == 1), "{result:?}");
    let after = schedule(&other);
    assert_ne!(before, after);
    assert_eq!(after, schedule(&original), "the copy takes the newer schedule");
    let _ = card;

    // Importing the older file back into the original keeps the original's newer schedule.
    let mine = schedule(&original);
    import(&mut original, &file, ClashRule::Newer);
    assert_eq!(schedule(&original), mine);
}

// transfer.feature: Images go with the questions that show them (R113)
#[test]
fn images_travel_with_the_questions_and_point_at_the_right_image_after() {
    let mut original = App::with_seed(Store::in_memory().unwrap(), 6);
    let topic = original.create_topic("Diagrams", None).unwrap();
    let png = b"\x89PNG\r\n\x1a\nthe diagram";
    let spare = original.add_image(b"\x89PNG\r\n\x1a\na spare one").unwrap();
    let image = original.add_image(png).unwrap();
    assert_ne!(spare, image);
    original
        .save_question(QuestionInput {
            topic,
            text: format!("What does this show? ![diagram](image:{})", image.0),
            multiple: false,
            options: ["A cache", "A pipeline", "A bus"].iter().enumerate().map(|(i, text)| OptionInput { text: (*text).into(), correct: i == 0 }).collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
            short: None,
        })
        .unwrap();
    let (_, file) = original.export(&ExportRequest { topics: Some(vec![topic]), history: false }, TODAY).unwrap();
    let (_, snapshot) = unpack(&file).unwrap();
    assert_eq!(snapshot.images.len(), 1, "only the image that's used");
    assert!(snapshot.questions[0].text.contains(&format!("image:{}", snapshot.images[0].uid)));

    let mut friend = App::with_seed(Store::in_memory().unwrap(), 7);
    friend.add_image(b"\x89PNG\r\n\x1a\nthe friend's own").unwrap();
    import(&mut friend, &file, ClashRule::KeepMine);
    let (question, _) = friend.store.questions_in(&[topic_named(&friend, "Diagrams")]).unwrap()[0].clone();
    let text = friend.question(question).unwrap().text;
    let id: i64 = text.split("image:").nth(1).unwrap().trim_end_matches(')').parse().unwrap();
    assert_eq!(friend.image(study_core::image::ImageId(id)).unwrap(), png.to_vec());
}

// transfer.feature: An import makes a backup first (R114)
#[test]
fn an_import_backs_up_first_and_a_bad_file_changes_nothing() {
    let dir = std::env::temp_dir().join(format!("study-transfer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let original = seeded();
    let (_, file) = original.export(&ExportRequest { topics: Some(vec![topic_named(&original, "OSI model")]), history: false }, TODAY).unwrap();
    let mut friend = App::with_seed(Store::in_memory().unwrap(), 5);
    friend.set_backups(dir.join("backups"));
    friend.create_topic("Mine", None).unwrap();

    assert!(friend.import_preview(b"not a zip file at all", "junk.studyapp").is_err());
    assert!(friend.import_apply(ImportChoice { rule: ClashRule::KeepMine, theirs: vec![] }, at(TODAY)).is_err(), "nothing waiting after a bad file");
    assert!(friend.backups().unwrap().is_empty());

    friend.import_preview(&file, "OSI model.studyapp").unwrap();
    let result = friend.import_apply(ImportChoice { rule: ClashRule::KeepMine, theirs: vec![] }, at(TODAY)).unwrap();
    let backup = result.backup.expect("a backup").name;
    assert!(backup.ends_with("before-import.sqlite"), "{backup}");
    assert_eq!(friend.restore_preview(&backup).unwrap().undone.iter().find(|undone| undone.one == "topic added").map(|undone| undone.count), Some(1));
}


// transfer.feature: Files that can't be trusted (R118)
#[test]
fn broken_or_hostile_files_are_refused_with_a_reason() {
    use crate::transfer::{Manifest, pack};
    use study_store::portable::{Image, Question, Snapshot, Topic};
    let manifest = |version: u32| Manifest { format: "studyapp".into(), version, app_version: "0.1.0".into(), exported: TODAY, device: "aaaa0000".into(), everything: false, history: false, counts: vec![] };
    let topic = Topic { uid: "01a1a800-0000-7000-8000-000000000001".into(), changed: "0000000000000001-0000-aaaa0000".into(), name: "Shared".into(), module: None, tags: vec![], schedule: None };
    let question = |topic: &str| Question { uid: "01a1a800-0000-7000-8000-000000000002".into(), changed: "0000000000000001-0000-aaaa0000".into(), topic: topic.into(), kind: "Single".into(), text: "Q".into(), explanation: String::new(), short: None, edited_at: "2026-11-04 10:00:00".into(), options: vec![], tags: vec![], citations: vec![] };
    let refused = |bytes: Vec<u8>| -> String {
        let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
        let error = app.import_preview(&bytes, "x.studyapp").unwrap_err().message;
        assert!(app.topics().unwrap().is_empty());
        error
    };

    let good = Snapshot { topics: vec![topic.clone()], questions: vec![question(&topic.uid)], ..Snapshot::default() };
    App::with_seed(Store::in_memory().unwrap(), 1).import_preview(&pack(&good, &manifest(1)).unwrap(), "x").unwrap();

    assert!(refused(pack(&good, &manifest(99)).unwrap()).contains("newer version of the app"));
    assert!(refused(b"PK\x03\x04 not really a zip".to_vec()).contains("isn't a .studyapp file"));
    let orphan = Snapshot { topics: vec![topic.clone()], questions: vec![question("someone else's topic")], ..Snapshot::default() };
    assert!(refused(pack(&orphan, &manifest(1)).unwrap()).contains("a topic that isn't in it"));
    let twice = Snapshot { topics: vec![topic.clone(), topic.clone()], ..Snapshot::default() };
    assert!(refused(pack(&twice, &manifest(1)).unwrap()).contains("the same ID"));
    let fake_image = Snapshot { topics: vec![topic.clone()], images: vec![Image { uid: "01a1a800-0000-7000-8000-000000000003".into(), changed: "0000000000000001-0000-aaaa0000".into(), format: "\"Png\"".into(), bytes: b"#!/bin/sh\nrm -rf ~".to_vec() }], ..Snapshot::default() };
    assert!(refused(pack(&fake_image, &manifest(1)).unwrap()).contains("isn't a picture"));
    let traversal = Snapshot { topics: vec![topic], images: vec![Image { uid: "../../etc/passwd".into(), changed: "0000000000000001-0000-aaaa0000".into(), format: "\"Png\"".into(), bytes: b"\x89PNG\r\n\x1a\nx".to_vec() }], ..Snapshot::default() };
    assert!(refused(pack(&traversal, &manifest(1)).unwrap()).contains("broken name"));
}

#[test]
fn a_file_that_unpacks_to_far_more_than_it_says_is_refused() {
    use std::io::Write;
    // A small file holding a huge run of zeros, the shape of a zip bomb.
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(br#"{"format":"studyapp","version":1,"app_version":"0","exported":0,"device":"x","everything":true,"history":true,"counts":[]}"#).unwrap();
    zip.start_file("data/topics.jsonl", options).unwrap();
    let zeros = vec![b' '; 16 * 1024 * 1024];
    for _ in 0..5 {
        zip.write_all(&zeros).unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    assert!(bytes.len() < 5 * 1024 * 1024, "{} bytes", bytes.len());
    let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
    assert!(app.import_preview(&bytes, "bomb.studyapp").unwrap_err().message.contains("more than the app will read"));
}

#[test]
#[ignore = "slow: times export and import of the large test profile"]
fn the_large_profile_exports_and_imports_in_reasonable_time() {
    let mut big = App::with_seed(Store::in_memory().unwrap(), 3);
    crate::seed_large::seed_large(&mut big, TODAY).unwrap();
    let started = std::time::Instant::now();
    let (_, file) = big.export(&everything(), TODAY).unwrap();
    let exported = started.elapsed();
    let started = std::time::Instant::now();
    let mut other = App::with_seed(Store::in_memory().unwrap(), 4);
    other.import_preview(&file, "big").unwrap();
    let previewed = started.elapsed();
    other.import_apply(ImportChoice { rule: ClashRule::Newer, theirs: vec![] }, at(TODAY)).unwrap();
    eprintln!("file {:.1} MB, export {exported:?}, preview {previewed:?}, import {:?}", file.len() as f64 / 1e6, started.elapsed());
    assert_eq!(other.topics().unwrap().len(), big.topics().unwrap().len());
}

#[test]
fn keeping_my_version_of_a_note_keeps_my_cards_too() {
    let mut original = App::with_seed(Store::in_memory().unwrap(), 6);
    let topic = original.create_topic("Caches", None).unwrap();
    let cloze = |text: &str| NoteInput { note: Note { topic, kind: NoteKind::Cloze, front: String::new(), back: String::new(), cues: vec![], text: text.into(), extra: String::new() }, suggestion: None };
    let note = original.save_note(None, cloze("L1 takes {{c1::4}} cycles"), TODAY).unwrap();
    let share = ExportRequest { topics: Some(vec![topic]), history: false };
    let (_, first) = original.export(&share, TODAY).unwrap();
    let mut friend = App::with_seed(Store::in_memory().unwrap(), 7);
    import(&mut friend, &first, ClashRule::KeepMine);
    assert_eq!(friend.store.cards().unwrap().len(), 1);

    original.save_note(Some(note), cloze("L1 takes {{c1::4}} cycles, L2 {{c2::12}}"), TODAY).unwrap();
    let (_, second) = original.export(&share, TODAY).unwrap();
    import(&mut friend, &second, ClashRule::KeepMine);
    assert_eq!(friend.store.cards().unwrap().len(), 1, "my note, my one card");
    import(&mut friend, &second, ClashRule::TakeTheirs);
    assert_eq!(friend.store.cards().unwrap().len(), 2, "their note, their two cards");
}

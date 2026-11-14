//! import-export.feature: Anki decks (R171, R172), read from files Anki itself wrote
//! (tests/fixtures/anki, made by scripts/make-anki-fixtures.py).

use super::*;
use crate::anki::{AnkiChoice, DeckTarget};
use crate::practice::Clock;
use study_core::cards::NoteKind;

const TODAY: i64 = 20_771;
const MODERN: &[u8] = include_bytes!("../tests/fixtures/anki/networks-modern.apkg");
const LEGACY: &[u8] = include_bytes!("../tests/fixtures/anki/networks-legacy.apkg");
const COLLECTION: &[u8] = include_bytes!("../tests/fixtures/anki/whole-collection.colpkg");

fn at(day: i64) -> Clock {
    Clock { day, minute: 600, unix: day * 86_400 + 36_000 }
}

fn app() -> App {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 7);
    app.set_backups(std::env::temp_dir().join(format!("study-anki-{}-{}", std::process::id(), rand::random::<u32>())));
    app
}

fn deck<'a>(preview: &'a crate::anki::AnkiPreview, name: &str) -> &'a crate::anki::DeckRow {
    preview.decks.iter().find(|deck| deck.name == name).unwrap_or_else(|| panic!("no deck {name} in {:?}", preview.decks))
}

fn notes_in(app: &App, topic: &str) -> Vec<study_core::cards::Note> {
    let id = app.topics().unwrap().into_iter().find(|row| row.name == topic).unwrap_or_else(|| panic!("no topic {topic}")).id;
    let cards = app.topic_cards(id, TODAY).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    cards.cards.iter().filter(|card| seen.insert(card.note)).map(|card| app.note_view(card.note, TODAY).unwrap().note).collect()
}

#[test]
fn a_new_anki_deck_shows_its_decks_card_types_and_media_before_anything_is_added() {
    let mut app = app();
    let preview = app.anki_preview(MODERN, "Networks.apkg").unwrap();
    assert_eq!(preview.notes, 6);
    assert_eq!(deck(&preview, "Networks::DNS").notes, 2);
    assert_eq!(deck(&preview, "Networks::TLS").notes, 2);
    assert_eq!(deck(&preview, "Misc").notes, 2);
    assert_eq!(deck(&preview, "Networks::DNS").target, DeckTarget::NewTopic { name: "DNS".into(), module: Some("Networks".into()) });
    assert_eq!(deck(&preview, "Misc").target, DeckTarget::NewTopic { name: "Misc".into(), module: None });
    let types: Vec<(String, usize, String, bool)> = preview.types.iter().map(|row| (row.name.clone(), row.notes, row.becomes.clone(), row.check)).collect();
    assert!(types.contains(&("Basic".into(), 3, "Basic".into(), false)), "{types:?}");
    assert!(types.contains(&("Basic (and reversed card)".into(), 1, "Basic and reversed".into(), false)), "{types:?}");
    assert!(types.contains(&("Cloze".into(), 1, "Cloze".into(), false)), "{types:?}");
    assert!(types.contains(&("Vocab".into(), 1, "Basic".into(), true)), "an unknown type comes in as Basic, flagged: {types:?}");
    assert_eq!((preview.images, preview.sounds), (1, 1));
    assert!(preview.has_progress);
    assert_eq!(preview.already_here, 0);
    assert!(app.topics().unwrap().is_empty(), "nothing is added by looking");
}

#[test]
fn the_older_anki_format_and_a_whole_collection_read_the_same() {
    let mut app = app();
    let modern = app.anki_preview(MODERN, "Networks.apkg").unwrap();
    let legacy = app.anki_preview(LEGACY, "Networks.apkg").unwrap();
    let collection = app.anki_preview(COLLECTION, "collection.colpkg").unwrap();
    for other in [&legacy, &collection] {
        assert_eq!(other.notes, modern.notes);
        assert_eq!(other.decks, modern.decks);
        assert_eq!(other.types, modern.types);
        assert_eq!((other.images, other.sounds), (modern.images, modern.sounds));
    }
}

#[test]
fn importing_puts_each_deck_where_you_chose_converting_html_images_and_cloze() {
    let mut app = app();
    let tls = app.create_topic("TLS handshakes", None).unwrap();
    app.anki_preview(MODERN, "Networks.apkg").unwrap();
    let result = app
        .anki_apply(
            AnkiChoice {
                decks: vec![
                    ("Networks::DNS".into(), DeckTarget::NewTopic { name: "DNS".into(), module: Some("Networks".into()) }),
                    ("Networks::TLS".into(), DeckTarget::Topic { topic: tls }),
                    ("Misc".into(), DeckTarget::Skip),
                ],
                keep_progress: false,
            },
            at(TODAY),
        )
        .unwrap();
    assert!(result.backup.is_some(), "a backup is made first");

    let dns = notes_in(&app, "DNS");
    assert_eq!(dns.len(), 2);
    assert_eq!((dns[0].kind, dns[0].front.as_str(), dns[0].back.as_str()), (NoteKind::Basic, "What resolves a name to an **IP address**?", "DNS\nDomain Name System"));
    assert!(dns[1].back.starts_with("The TLB ![tlb](image:"), "{}", dns[1].back);
    let image: i64 = dns[1].back.split("image:").nth(1).unwrap().trim_end_matches(')').parse().unwrap();
    assert!(app.image(study_core::image::ImageId(image)).unwrap().starts_with(b"\x89PNG"), "the image came across");
    assert_eq!(app.modules().unwrap().iter().filter(|module| module.name == "Networks").count(), 1);

    let handshakes = notes_in(&app, "TLS handshakes");
    let kinds: Vec<NoteKind> = handshakes.iter().map(|note| note.kind).collect();
    assert!(kinds.contains(&NoteKind::Reversed) && kinds.contains(&NoteKind::Cloze), "{kinds:?}");
    let cloze = handshakes.iter().find(|note| note.kind == NoteKind::Cloze).unwrap();
    assert_eq!(cloze.text, "The handshake agrees a {{c1::cipher suite}} and swaps {{c2::keys::what}}.");
    assert_eq!(cloze.extra, "From lecture 5");
    assert_eq!(app.topic_cards(tls, TODAY).unwrap().cards.len(), 4, "a reversed card and a 2-gap cloze make 2 cards each");
    assert!(app.topics().unwrap().iter().all(|topic| topic.name != "Misc"), "a skipped deck isn't imported");
}

#[test]
fn importing_the_same_deck_again_adds_nothing_new() {
    let mut app = app();
    let choice = || AnkiChoice { decks: vec![], keep_progress: false };
    app.anki_preview(MODERN, "Networks.apkg").unwrap();
    app.anki_apply(choice(), at(TODAY)).unwrap();
    let topics = app.topics().unwrap().len();
    let again = app.anki_preview(MODERN, "Networks.apkg").unwrap();
    assert_eq!(again.already_here, 6);
    assert_eq!(deck(&again, "Networks::DNS").target, DeckTarget::NewTopic { name: "DNS".into(), module: Some("Networks".into()) });
    app.anki_apply(choice(), at(TODAY)).unwrap();
    assert_eq!(app.topics().unwrap().len(), topics);
    assert_eq!(notes_in(&app, "DNS").len(), 2, "not doubled up");
}

#[test]
fn unknown_note_types_and_sounds_come_in_as_text() {
    let mut app = app();
    app.anki_preview(MODERN, "Networks.apkg").unwrap();
    app.anki_apply(AnkiChoice { decks: vec![], keep_progress: false }, at(TODAY)).unwrap();
    let misc = notes_in(&app, "Misc");
    let vocab = misc.iter().find(|note| note.front == "จำ").expect("the Vocab note");
    assert_eq!((vocab.kind, vocab.back.as_str()), (NoteKind::Basic, "to remember"));
    let sound = misc.iter().find(|note| note.front.starts_with("Sound check")).unwrap();
    assert_eq!((sound.front.as_str(), sound.back.as_str()), ("Sound check", "It made a click & a beep"));
}

#[test]
fn keeping_progress_carries_a_reviewed_card_on_and_otherwise_cards_start_new() {
    let mut fresh = app();
    fresh.anki_preview(MODERN, "Networks.apkg").unwrap();
    fresh.anki_apply(AnkiChoice { decks: vec![], keep_progress: false }, at(TODAY)).unwrap();
    let dns = fresh.topics().unwrap().into_iter().find(|row| row.name == "DNS").unwrap().id;
    assert!(fresh.topic_cards(dns, TODAY).unwrap().cards.iter().all(|card| card.is_new));

    let mut kept = app();
    kept.anki_preview(MODERN, "Networks.apkg").unwrap();
    kept.anki_apply(AnkiChoice { decks: vec![], keep_progress: true }, at(TODAY)).unwrap();
    let dns = kept.topics().unwrap().into_iter().find(|row| row.name == "DNS").unwrap().id;
    let cards = kept.topic_cards(dns, TODAY).unwrap().cards;
    assert_eq!(cards.iter().filter(|card| !card.is_new).count(), 1, "the one card reviewed in Anki keeps going");
}

#[test]
fn a_file_that_isnt_an_anki_deck_says_so() {
    let mut app = app();
    for bytes in [b"plain text".as_slice(), include_bytes!("../tests/fixtures/anki/networks-modern.apkg").get(..200).unwrap()] {
        let error = app.anki_preview(bytes, "notes.apkg").unwrap_err();
        assert!(error.message.contains("isn't an Anki deck"), "{}", error.message);
    }
}

// ---------- Exporting to Anki (R173) ----------

use crate::cards::NoteInput;
use crate::transfer::{ExportFormat, ExportRequest};
use crate::{OptionInput, QuestionInput};
use study_core::cards::Note;

fn note(topic: TopicId, kind: NoteKind, front: &str, back: &str, text: &str) -> NoteInput {
    NoteInput { note: Note { topic, kind, front: front.into(), back: back.into(), cues: vec![], text: text.into(), extra: String::new() }, suggestion: None }
}

fn written() -> (App, TopicId) {
    let mut app = app();
    let module = app.create_module("Networks").unwrap();
    let dns = app.create_topic("DNS", Some(module)).unwrap();
    let image = app.add_image(b"\x89PNG\r\n\x1a\nnot really a picture").unwrap();
    app.save_note(None, note(dns, NoteKind::Basic, "What resolves **names**?", &format!("DNS ![diagram](image:{})\n\nMaths: $x^2$", image.0), ""), TODAY).unwrap();
    app.save_note(None, note(dns, NoteKind::Reversed, "TTL", "Time to live", ""), TODAY).unwrap();
    app.save_note(None, note(dns, NoteKind::Cloze, "", "", "Port {{c1::53}} over {{c2::UDP}}"), TODAY).unwrap();
    app.save_question(QuestionInput {
        topic: dns,
        text: "Which record maps a name to an IPv6 address?".into(),
        multiple: false,
        options: vec![OptionInput { text: "A".into(), correct: false }, OptionInput { text: "AAAA".into(), correct: true }, OptionInput { text: "MX".into(), correct: false }],
        explanation: "Four times the A.".into(),
        tags: vec![],
        citations: vec![],
        short: None,
    })
    .unwrap();
    (app, dns)
}

fn exported(app: &App, quizzes: bool) -> (String, Vec<u8>) {
    let request = ExportRequest { topics: None, history: true, format: ExportFormat::Anki, quizzes_as_cards: quizzes };
    app.export(&request, TODAY).unwrap()
}

#[test]
fn an_anki_export_reads_back_with_the_same_cards_decks_and_images() {
    let (app, _) = written();
    let (name, bytes) = exported(&app, false);
    assert!(name.ends_with(".apkg"), "{name}");
    let mut other = self::app();
    let preview = other.anki_preview(&bytes, &name).unwrap();
    assert_eq!(preview.notes, 3);
    assert_eq!(preview.decks, vec![crate::anki::DeckRow { name: "Networks::DNS".into(), notes: 3, target: DeckTarget::NewTopic { name: "DNS".into(), module: Some("Networks".into()) } }]);
    assert_eq!(preview.images, 1);
    other.anki_apply(AnkiChoice { decks: vec![], keep_progress: false }, at(TODAY)).unwrap();
    let notes = notes_in(&other, "DNS");
    let basic = notes.iter().find(|note| note.kind == NoteKind::Basic).unwrap();
    assert_eq!(basic.front, "What resolves **names**?");
    assert!(basic.back.starts_with("DNS ![diagram](image:") && basic.back.ends_with("Maths: $x^2$"), "{}", basic.back);
    assert!(notes.iter().any(|note| note.kind == NoteKind::Reversed && note.front == "TTL"));
    assert!(notes.iter().any(|note| note.kind == NoteKind::Cloze && note.text == "Port {{c1::53}} over {{c2::UDP}}"));
}

#[test]
fn quiz_questions_can_come_along_as_cards() {
    let (app, _) = written();
    let (_, without) = exported(&app, false);
    let (_, with) = exported(&app, true);
    let mut other = self::app();
    assert_eq!(other.anki_preview(&without, "x.apkg").unwrap().notes, 3);
    assert_eq!(other.anki_preview(&with, "x.apkg").unwrap().notes, 4);
    other.anki_apply(AnkiChoice { decks: vec![], keep_progress: false }, at(TODAY)).unwrap();
    let quiz = notes_in(&other, "DNS").into_iter().find(|note| note.front.starts_with("Which record")).unwrap();
    assert!(quiz.front.contains("A") && quiz.front.contains("AAAA") && quiz.front.contains("MX"), "{}", quiz.front);
    assert!(quiz.back.contains("AAAA") && quiz.back.contains("Four times the A."), "{}", quiz.back);
}

#[test]
fn reimporting_its_own_export_finds_the_notes_already_here() {
    let (mut app, _) = written();
    let (_, bytes) = exported(&app, false);
    let preview = app.anki_preview(&bytes, "x.apkg").unwrap();
    assert_eq!(preview.already_here, 3, "each note keeps its own ID through Anki");
}

/// Writes an export for scripts/check-anki-export.py to open in Anki itself:
/// `ANKI_OUT=/tmp/x.apkg cargo test -p study-api write_an_anki_export -- --ignored`
#[test]
#[ignore]
fn write_an_anki_export() {
    let (app, _) = written();
    let (_, bytes) = exported(&app, true);
    std::fs::write(std::env::var("ANKI_OUT").expect("set ANKI_OUT"), bytes).unwrap();
}

// import-export.feature: a big import shows progress and can be stopped (R178)
#[test]
fn an_import_reports_progress_and_stopping_adds_nothing() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let mut app = app();
    let seen = Arc::new(Mutex::new(vec![]));
    let stop = Arc::new(AtomicBool::new(false));
    let log = seen.clone();
    app.set_progress(Some(Progress { report: Arc::new(move |what: &str, done, total| log.lock().unwrap().push((what.to_string(), done, total))), stop: stop.clone() }));
    app.anki_preview(MODERN, "Networks.apkg").unwrap();
    app.anki_apply(AnkiChoice { decks: vec![], keep_progress: false }, at(TODAY)).unwrap();
    assert_eq!(seen.lock().unwrap().last(), Some(&("Notes".to_string(), 6, 6)));

    let mut other = self::app();
    stop.store(true, Ordering::SeqCst);
    other.set_progress(Some(Progress { report: Arc::new(|_: &str, _, _| {}), stop }));
    other.anki_preview(MODERN, "Networks.apkg").unwrap();
    let error = other.anki_apply(AnkiChoice { decks: vec![], keep_progress: false }, at(TODAY)).unwrap_err();
    assert!(error.message.contains("nothing was imported"));
    assert!(other.topics().unwrap().is_empty());
}

#[test]
fn a_target_reads_and_writes_as_the_window_sends_it() {
    let target = DeckTarget::Topic { topic: TopicId(12) };
    assert_eq!(serde_json::to_value(&target).unwrap(), serde_json::json!({ "kind": "Topic", "topic": 12 }));
    let choice: AnkiChoice = serde_json::from_value(serde_json::json!({ "decks": [["Networks::DNS", { "kind": "Skip" }], ["Misc", { "kind": "NewTopic", "name": "Misc", "module": null }]], "keep_progress": true })).unwrap();
    assert_eq!(choice.decks[0].1, DeckTarget::Skip);
}

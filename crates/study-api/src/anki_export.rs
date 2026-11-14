//! import-export.feature: Anki decks out (R173).
//!
//! Writes the older `.apkg` layout (an SQLite `collection.anki2`, media listed as JSON),
//! which every version of Anki imports. Note types, deck options and settings start from
//! Anki's own defaults (anki_defaults.json). Each note's Anki ID is its own ID here, so
//! importing the deck back (into either app) updates rather than doubles up. Modules
//! become parent decks (`Networks::DNS`).

use std::collections::HashMap;
use std::io::Write;

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use study_store::portable::{Note, Question, Snapshot};

use crate::{ApiError, ApiResult};

const DEFAULTS: &str = include_str!("anki_defaults.json");
const SCHEMA: &str = "
CREATE TABLE col (id integer PRIMARY KEY, crt integer NOT NULL, mod integer NOT NULL, scm integer NOT NULL, ver integer NOT NULL, dty integer NOT NULL, usn integer NOT NULL, ls integer NOT NULL, conf text NOT NULL, models text NOT NULL, decks text NOT NULL, dconf text NOT NULL, tags text NOT NULL);
CREATE TABLE notes (id integer PRIMARY KEY, guid text NOT NULL, mid integer NOT NULL, mod integer NOT NULL, usn integer NOT NULL, tags text NOT NULL, flds text NOT NULL, sfld integer NOT NULL, csum integer NOT NULL, flags integer NOT NULL, data text NOT NULL);
CREATE TABLE cards (id integer PRIMARY KEY, nid integer NOT NULL, did integer NOT NULL, ord integer NOT NULL, mod integer NOT NULL, usn integer NOT NULL, type integer NOT NULL, queue integer NOT NULL, due integer NOT NULL, ivl integer NOT NULL, factor integer NOT NULL, reps integer NOT NULL, lapses integer NOT NULL, left integer NOT NULL, odue integer NOT NULL, odid integer NOT NULL, flags integer NOT NULL, data text NOT NULL);
CREATE TABLE revlog (id integer PRIMARY KEY, cid integer NOT NULL, usn integer NOT NULL, ease integer NOT NULL, ivl integer NOT NULL, lastIvl integer NOT NULL, factor integer NOT NULL, time integer NOT NULL, type integer NOT NULL);
CREATE TABLE graves (usn integer NOT NULL, oid integer NOT NULL, type integer NOT NULL);
CREATE INDEX ix_notes_usn ON notes (usn);
CREATE INDEX ix_cards_usn ON cards (usn);
CREATE INDEX ix_revlog_usn ON revlog (usn);
CREATE INDEX ix_cards_nid ON cards (nid);
CREATE INDEX ix_cards_sched ON cards (did, queue, due);
CREATE INDEX ix_revlog_cid ON revlog (cid);
CREATE INDEX ix_notes_csum ON notes (csum);
";

/// What to put in the deck.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnkiOptions {
    /// Quiz questions as Basic cards: the question and options on the front, the answer
    /// and explanation on the back.
    pub quizzes_as_cards: bool,
    /// Each card's place in its schedule (only when the snapshot has history).
    pub keep_progress: bool,
}

fn failed(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(format!("couldn't make the Anki deck: {error}"))
}

/// The app's Markdown as Anki's HTML. Images become files in the deck, named by `image`.
pub fn markdown_to_html(markdown: &str, image: &mut dyn FnMut(&str) -> Option<String>) -> String {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    // Maths in Anki's MathJax delimiters, before Markdown sees the dollars.
    let mut text = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(at) = rest.find('$') {
        text.push_str(&rest[..at]);
        let display = rest[at..].starts_with("$$");
        let open = if display { 2 } else { 1 };
        let body = &rest[at + open..];
        let close = if display { body.find("$$") } else { body.find('$') };
        match close {
            Some(end) => {
                let (left, right) = if display { ("\\\\[", "\\\\]") } else { ("\\\\(", "\\\\)") };
                text.push_str(left);
                text.push_str(&body[..end]);
                text.push_str(right);
                rest = &body[end + open..];
            }
            None => {
                text.push_str(&rest[at..at + open]);
                rest = body;
            }
        }
    }
    text.push_str(rest);
    let events = Parser::new_ext(&text, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH).map(|event| match event {
        Event::Start(Tag::Image { dest_url, title, id, link_type }) => {
            let swapped = dest_url.strip_prefix("image:").and_then(|uid| image(uid)).unwrap_or_else(|| dest_url.to_string());
            Event::Start(Tag::Image { link_type, dest_url: swapped.into(), title, id })
        }
        Event::SoftBreak => Event::HardBreak,
        other => other,
    });
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events);
    // One paragraph needs no <p> around it (Anki shows the field as it is).
    let trimmed = html.trim();
    match trimmed.strip_prefix("<p>").and_then(|inner| inner.strip_suffix("</p>")) {
        Some(inner) if !inner.contains("<p>") => inner.to_string(),
        _ => trimmed.to_string(),
    }
}

fn checksum(text: &str) -> i64 {
    // Anki's: the first 8 hex digits of the SHA-1 of the field without HTML. Only used for
    // finding duplicates, so a stable hash of the text does the job.
    let plain: String = crate::markup::decode_entities(&text.split('<').map(|part| part.split_once('>').map(|(_, after)| after).unwrap_or(part)).collect::<String>());
    i64::from(plain.bytes().fold(0x811c_9dc5u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)) & 0x7fff_ffff)
}

struct Writer<'a> {
    snapshot: &'a Snapshot,
    db: Connection,
    media: Vec<(String, Vec<u8>)>,
    media_names: HashMap<String, String>,
    next_id: i64,
    now: i64,
    start_day: i64,
    today: i64,
    keep_progress: bool,
}

impl Writer<'_> {
    fn id(&mut self) -> i64 {
        self.next_id += 1;
        self.next_id
    }

    fn html(&mut self, markdown: &str) -> String {
        let images: HashMap<&str, &study_store::portable::Image> = self.snapshot.images.iter().map(|image| (image.uid.as_str(), image)).collect();
        let mut wanted = vec![];
        let names = &mut self.media_names;
        let html = markdown_to_html(markdown, &mut |uid| {
            let image = images.get(uid)?;
            let extension = match image.format.trim_matches('"') {
                "Jpeg" => "jpg",
                "Gif" => "gif",
                "Webp" => "webp",
                _ => "png",
            };
            let name = names.entry(uid.to_string()).or_insert_with(|| {
                wanted.push((format!("{uid}.{extension}"), image.bytes.clone()));
                format!("{uid}.{extension}")
            });
            Some(name.clone())
        });
        self.media.extend(wanted);
        html
    }

    fn add_note(&mut self, guid: &str, model: i64, deck: i64, fields: &[String], tags: &[String], schedules: &[Option<&study_store::portable::CardSchedule>]) -> ApiResult<()> {
        let note = self.id();
        let flds = fields.join("\u{1f}");
        let sort = fields.first().cloned().unwrap_or_default();
        let tags = if tags.is_empty() { String::new() } else { format!(" {} ", tags.iter().map(|tag| tag.replace(' ', "_")).collect::<Vec<_>>().join(" ")) };
        self.db
            .execute("INSERT INTO notes VALUES (?1, ?2, ?3, ?4, -1, ?5, ?6, ?7, ?8, 0, '')", params![note, guid, model, self.now, tags, flds, sort, checksum(&sort)])
            .map_err(failed)?;
        for (ord, schedule) in schedules.iter().enumerate() {
            let card = self.id();
            let (kind, queue, due, interval, factor, reps, lapses) = match schedule.filter(|_| self.keep_progress) {
                Some(schedule) if schedule.reps > 0 => {
                    let interval = schedule.stability.unwrap_or(1.0).round().max(1.0) as i64;
                    let factor = schedule.difficulty.map(|difficulty| (3500.0 - (difficulty - 1.0) / 9.0 * 2200.0).round() as i64).unwrap_or(2500);
                    (2, 2, schedule.due - self.start_day, interval, factor, schedule.reps, schedule.lapses)
                }
                _ => (0, 0, note, 0, 0, 0, 0),
            };
            let _ = self.today;
            self.db
                .execute("INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, -1, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, 0, 0, 0, '')", params![card, note, deck, ord as i64, self.now, kind, queue, due, interval, factor, reps, lapses])
                .map_err(failed)?;
        }
        Ok(())
    }
}

fn question_fields(question: &Question, html: &mut dyn FnMut(&str) -> String) -> (String, String) {
    let letters = ["A", "B", "C", "D", "E"];
    if question.kind == "short" || question.short.is_some() {
        let model = question.short.as_ref().and_then(|short| short.get("model_answer")).and_then(Value::as_str).unwrap_or("").to_string();
        let back = [model, question.explanation.clone()].into_iter().filter(|part| !part.trim().is_empty()).collect::<Vec<_>>().join("\n\n");
        return (html(&question.text), html(&back));
    }
    let options: Vec<String> = question.options.iter().enumerate().map(|(index, option)| format!("{}. {}", letters.get(index).unwrap_or(&"-"), option.text)).collect();
    let front = format!("{}\n\n{}", question.text, options.iter().map(|line| format!("- {line}")).collect::<Vec<_>>().join("\n"));
    let right: Vec<String> = question.options.iter().enumerate().filter(|(_, option)| option.correct).map(|(index, option)| format!("{}. {}", letters.get(index).unwrap_or(&"-"), option.text)).collect();
    let back = [right.join(", "), question.explanation.clone()].into_iter().filter(|part| !part.trim().is_empty()).collect::<Vec<_>>().join("\n\n");
    (html(&front), html(&back))
}

/// The `.apkg` for `snapshot`.
pub fn build(snapshot: &Snapshot, options: AnkiOptions, today: i64) -> ApiResult<Vec<u8>> {
    let defaults: Value = serde_json::from_str(DEFAULTS).map_err(failed)?;
    let now = today * 86_400;
    let mut writer = Writer { snapshot, db: Connection::open_in_memory().map_err(failed)?, media: vec![], media_names: HashMap::new(), next_id: now * 1000, now, start_day: today - 30, today, keep_progress: options.keep_progress };
    writer.db.execute_batch(SCHEMA).map_err(failed)?;

    // Decks: one per topic, under its module.
    let modules: HashMap<&str, &str> = snapshot.modules.iter().map(|module| (module.uid.as_str(), module.name.as_str())).collect();
    let mut decks = serde_json::Map::new();
    decks.insert("1".into(), defaults["deck"].clone());
    let mut deck_ids: HashMap<&str, i64> = HashMap::new();
    let mut named: HashMap<String, i64> = HashMap::new();
    let mut deck = |name: String, writer: &mut Writer| -> i64 {
        *named.entry(name.clone()).or_insert_with(|| {
            let id = writer.id();
            let mut deck = defaults["deck"].clone();
            deck["id"] = json!(id);
            deck["name"] = json!(name);
            deck["mod"] = json!(writer.now);
            decks.insert(id.to_string(), deck);
            id
        })
    };
    for topic in &snapshot.topics {
        let clean = |name: &str| name.replace("::", ":");
        let name = match topic.module.as_deref().and_then(|module| modules.get(module)) {
            Some(module) => {
                deck(clean(module), &mut writer);
                format!("{}::{}", clean(module), clean(&topic.name))
            }
            None => clean(&topic.name),
        };
        deck_ids.insert(topic.uid.as_str(), deck(name, &mut writer));
    }

    // Note types: Anki's Basic, Basic (and reversed card) and Cloze.
    let mut models = serde_json::Map::new();
    let mut model_ids = HashMap::new();
    // Fixed IDs and names of their own: importing a second deck reuses them, and they
    // don't clash with the Basic and Cloze already in someone's Anki.
    for (key, id, name) in [("Basic", 1_786_000_000_001i64, "Shudhana Basic"), ("Reversed", 1_786_000_000_002, "Shudhana Basic (and reversed card)"), ("Cloze", 1_786_000_000_003, "Shudhana Cloze")] {
        let mut model = defaults["models"][key].clone();
        model["id"] = json!(id);
        model["name"] = json!(name);
        model["mod"] = json!(now);
        model["did"] = json!(1);
        models.insert(id.to_string(), model);
        model_ids.insert(key, id);
    }

    fn schedules(note: &Note) -> Vec<Option<&study_store::portable::CardSchedule>> {
        note.cards.iter().map(|card| card.schedule.as_ref()).collect()
    }
    for note in &snapshot.notes {
        let Some(&deck) = deck_ids.get(note.topic.as_str()) else { continue };
        let text = |key: &str| note.note.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        let kind = note.note.get("kind").and_then(Value::as_str).unwrap_or("Basic");
        let (model, fields) = match kind {
            "Cloze" => (model_ids["Cloze"], vec![writer.html(&text("text")), writer.html(&text("extra"))]),
            "Reversed" => (model_ids["Reversed"], vec![writer.html(&text("front")), writer.html(&text("back"))]),
            _ => (model_ids["Basic"], vec![writer.html(&text("front")), writer.html(&text("back"))]),
        };
        // A Basic note's extra cues become their own Basic notes: Anki's Basic has no cues.
        let all = schedules(note);
        let cards_now: Vec<_> = if kind == "Basic" { all.iter().take(1).cloned().collect() } else { all.clone() };
        // R192: the note's own tags go to Anki as its tags.
        let tags: Vec<String> = note.note.get("tags").and_then(Value::as_array).map(|tags| tags.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
        writer.add_note(&note.uid, model, deck, &fields, &tags, &cards_now)?;
        if kind == "Basic" {
            let cues: Vec<String> = note.note.get("cues").and_then(Value::as_array).map(|cues| cues.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
            for (index, cue) in cues.iter().enumerate() {
                let fields = vec![writer.html(cue), fields[1].clone()];
                writer.add_note(&format!("{}:cue:{index}", note.uid), model, deck, &fields, &tags, &[all.get(index + 1).cloned().flatten()])?;
            }
        }
    }
    if options.quizzes_as_cards {
        for question in &snapshot.questions {
            let Some(&deck) = deck_ids.get(question.topic.as_str()) else { continue };
            let (front, back) = question_fields(question, &mut |markdown| writer.html(markdown));
            writer.add_note(&format!("{}:quiz", question.uid), model_ids["Basic"], deck, &[front, back], &question.tags, &[None])?;
        }
    }

    let mut conf = defaults["conf"].clone();
    conf["curModel"] = json!(model_ids["Basic"]);
    let dconf = defaults["dconf"].clone();
    writer
        .db
        .execute(
            "INSERT INTO col VALUES (1, ?1, ?2, ?3, 11, 0, 0, 0, ?4, ?5, ?6, ?7, '{}')",
            params![writer.start_day * 86_400, now * 1000, now * 1000, conf.to_string(), Value::Object(models).to_string(), Value::Object(decks).to_string(), dconf.to_string()],
        )
        .map_err(failed)?;

    // The database as bytes, then the zip.
    let file = tempfile::NamedTempFile::new().map_err(failed)?;
    writer.db.execute("VACUUM INTO ?1", [file.path().to_string_lossy()]).map_err(failed)?;
    let database = std::fs::read(file.path()).map_err(failed)?;
    let mut zipped = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zipped.start_file("collection.anki2", stored).map_err(failed)?;
    zipped.write_all(&database).map_err(failed)?;
    let mut listing = serde_json::Map::new();
    for (index, (name, bytes)) in writer.media.iter().enumerate() {
        zipped.start_file(index.to_string(), stored).map_err(failed)?;
        zipped.write_all(bytes).map_err(failed)?;
        listing.insert(index.to_string(), json!(name));
    }
    zipped.start_file("media", stored).map_err(failed)?;
    zipped.write_all(Value::Object(listing).to_string().as_bytes()).map_err(failed)?;
    Ok(zipped.finish().map_err(failed)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_becomes_anki_html_with_mathjax_and_media_names() {
        let html = markdown_to_html("What is **DNS**?\nA *lookup* ![d](image:abc-1) and $x^2$", &mut |uid| Some(format!("{uid}.png")));
        assert_eq!(html, "What is <strong>DNS</strong>?<br />\nA <em>lookup</em> <img src=\"abc-1.png\" alt=\"d\" /> and \\(x^2\\)");
        assert_eq!(markdown_to_html("$$E=mc^2$$", &mut |_| None), "\\[E=mc^2\\]");
        assert_eq!(markdown_to_html("Port {{c1::53}}", &mut |_| None), "Port {{c1::53}}");
        assert_eq!(markdown_to_html("One\n\nTwo", &mut |_| None), "<p>One</p>\n<p>Two</p>");
    }
}

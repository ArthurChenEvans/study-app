//! Records in a form that means the same on any computer, for export and import
//! (R112 to R118). Every reference is an ID (`uid`) rather than a local row number,
//! including the ones inside stored JSON (a note's topic, a quiz's questions) and image
//! references in text (`image:12` becomes `image:<uid>`).

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Result, Store, StoreError};

// ---------- The records ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub uid: String,
    pub changed: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuizStep {
    pub step: i64,
    pub rep: i64,
    pub due: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    pub uid: String,
    pub changed: String,
    pub name: String,
    pub module: Option<String>,
    pub tags: Vec<String>,
    /// Only with history.
    pub schedule: Option<QuizStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub uid: String,
    pub changed: String,
    pub authors: String,
    pub year: Option<String>,
    pub title: String,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub uid: String,
    pub changed: String,
    /// The stored format, as the app writes it (`"Png"`).
    pub format: String,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionOption {
    pub text: String,
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cited {
    pub source: String,
    pub location: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub uid: String,
    pub changed: String,
    pub topic: String,
    pub kind: String,
    pub text: String,
    pub explanation: String,
    pub short: Option<Value>,
    pub edited_at: String,
    pub options: Vec<QuestionOption>,
    pub tags: Vec<String>,
    pub citations: Vec<Cited>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardSchedule {
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub due: i64,
    pub last_review: Option<i64>,
    pub reps: i64,
    pub lapses: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub uid: String,
    pub changed: String,
    pub key: String,
    pub front: String,
    pub back: String,
    pub archived: bool,
    /// Only with history. Without it the card starts as new.
    pub schedule: Option<CardSchedule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub uid: String,
    pub changed: String,
    pub topic: String,
    /// The note as the app stores it, without its topic.
    pub note: Value,
    /// The suggestion it was made from, with IDs in place of row numbers.
    pub source: Option<String>,
    pub created: i64,
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub uid: String,
    pub changed: String,
    pub card: String,
    pub day: i64,
    pub rating: i64,
    pub elapsed: i64,
    pub was_new: bool,
    pub seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub text: String,
    pub question: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChecklistStep {
    pub step: Option<i64>,
    pub rep: Option<i64>,
    pub due: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checklist {
    pub uid: String,
    pub changed: String,
    pub module: Option<String>,
    pub topic: Option<String>,
    pub subject_name: String,
    pub subject_kind: String,
    pub created: i64,
    /// Only with history. Without it the checklist starts again from the first step.
    pub schedule: Option<ChecklistStep>,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub uid: String,
    pub changed: String,
    pub checklist: String,
    pub day: i64,
    pub dump: Option<String>,
    pub points: Value,
    pub score: i64,
    pub passed: bool,
    pub from_label: Option<String>,
    pub to_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuizMove {
    pub topic: String,
    pub day: i64,
    pub from_step: Option<i64>,
    pub from_rep: Option<i64>,
    pub from_due: Option<i64>,
    pub score: i64,
    pub passed: bool,
    pub to_step: i64,
    pub to_rep: i64,
    pub next_due: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    pub uid: String,
    pub changed: String,
    pub day: i64,
    pub status: Value,
    pub retry_of: Option<String>,
    pub attempt: Value,
    pub reasons: Value,
    pub moves: Vec<QuizMove>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub uid: String,
    pub changed: String,
    pub name: String,
    pub kind: String,
    pub created: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub uid: String,
    pub changed: String,
    pub project: String,
    pub day: i64,
    pub started: i64,
    pub ended: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub uid: String,
    pub changed: String,
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drill {
    pub uid: String,
    pub changed: String,
    pub issue: Option<String>,
    pub topic: Option<String>,
    pub task: String,
    pub hints: String,
    pub created: i64,
    pub due: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrillRun {
    pub uid: String,
    pub changed: String,
    pub drill: String,
    pub day: i64,
    pub rating: String,
    pub seconds: i64,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Struggle {
    pub uid: String,
    pub changed: String,
    pub project: String,
    pub session: Option<String>,
    pub day: i64,
    pub minute: i64,
    pub what: String,
    pub why: String,
    pub kind: String,
    pub topic: Option<String>,
    pub issue: Option<String>,
    pub outcome: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub changed: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuggestionDone {
    pub key: String,
    pub status: String,
    pub changed: String,
}

/// Everything in an export, in the order it has to be put back.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub modules: Vec<Module>,
    pub topics: Vec<Topic>,
    pub sources: Vec<Source>,
    pub images: Vec<Image>,
    pub questions: Vec<Question>,
    pub notes: Vec<Note>,
    pub reviews: Vec<Review>,
    pub checklists: Vec<Checklist>,
    pub checks: Vec<Check>,
    pub attempts: Vec<Attempt>,
    pub projects: Vec<Project>,
    pub sessions: Vec<Session>,
    pub issues: Vec<Issue>,
    pub drills: Vec<Drill>,
    pub drill_runs: Vec<DrillRun>,
    pub struggles: Vec<Struggle>,
    pub settings: Vec<Setting>,
    pub suggestions_done: Vec<SuggestionDone>,
}

/// What to export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// `None` is everything. Otherwise these topics and what belongs to them.
    pub topics: Option<Vec<i64>>,
    pub history: bool,
}

// ---------- References inside JSON and text ----------

/// The `image:` references in `text`, swapped with `swap`. Unknown ones stay as they are.
pub fn swap_images(text: &str, swap: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("image:") {
        out.push_str(&rest[..at + 6]);
        rest = &rest[at + 6..];
        let end = rest.find(|letter: char| !(letter.is_ascii_hexdigit() || letter == '-')).unwrap_or(rest.len());
        let token = &rest[..end];
        match (!token.is_empty()).then(|| swap(token)).flatten() {
            Some(swapped) => out.push_str(&swapped),
            None => out.push_str(token),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Runs `swap` over every image reference in a record, by way of its JSON.
pub fn with_images<T: Serialize + DeserializeOwned>(record: &T, swap: &dyn Fn(&str) -> Option<String>) -> Result<T> {
    let json = serde_json::to_string(record)?;
    Ok(serde_json::from_str(&swap_images(&json, swap))?)
}

/// Calls `change` on the value at `path` inside `value`, where `*` is every item of a list.
fn at_path(value: &mut Value, path: &[&str], change: &mut dyn FnMut(&mut Value)) {
    match path.split_first() {
        None => change(value),
        Some((&"*", rest)) => {
            if let Value::Array(items) = value {
                for item in items {
                    at_path(item, rest, change);
                }
            }
        }
        Some((key, rest)) => {
            if let Some(inner) = value.get_mut(*key) {
                at_path(inner, rest, change);
            }
        }
    }
}

/// The places in a stored quiz attempt that hold row numbers, and what they point at.
const ATTEMPT_REFS: &[(&[&str], &str)] = &[
    (&["retry_of"], "attempts"),
    (&["settings", "topics", "*"], "topics"),
    (&["asked", "*", "item", "question"], "questions"),
    (&["asked", "*", "snapshot", "topic"], "topics"),
    (&["asked", "*", "snapshot", "citations", "*", "source"], "sources"),
];

/// Row numbers to IDs for every table, read once per export.
struct ToUid(HashMap<&'static str, HashMap<i64, String>>);

impl ToUid {
    fn read(connection: &Connection) -> Result<Self> {
        let mut maps = HashMap::new();
        for table in crate::ids::STAMPED {
            let mut statement = connection.prepare(&format!("SELECT id, uid FROM {table}"))?;
            let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
            maps.insert(*table, rows.collect::<rusqlite::Result<HashMap<_, _>>>()?);
        }
        Ok(Self(maps))
    }

    fn of(&self, table: &str, id: i64) -> Option<String> {
        self.0.get(table)?.get(&id).cloned()
    }

    fn opt(&self, table: &str, id: Option<i64>) -> Option<String> {
        id.and_then(|id| self.of(table, id))
    }

    /// A number in JSON to its ID. Null stays null, and a record that's gone (history
    /// keeps snapshots of deleted questions) becomes an empty ID.
    fn json(&self, table: &str, value: &mut Value) {
        if let Some(id) = value.as_i64() {
            *value = Value::String(self.of(table, id).unwrap_or_default());
        }
    }

    /// `quiz:12`, `recall:3:text` and `struggle:9` suggestion keys, with IDs.
    fn key(&self, key: &str) -> Option<String> {
        let (kind, rest) = key.split_once(':')?;
        match kind {
            "quiz" => Some(format!("quiz:{}", self.of("questions", rest.parse().ok()?)?)),
            "struggle" => Some(format!("struggle:{}", self.of("struggles", rest.parse().ok()?)?)),
            "recall" => {
                let (checklist, text) = rest.split_once(':')?;
                Some(format!("recall:{}:{text}", self.of("recall_checklists", checklist.parse().ok()?)?))
            }
            _ => None,
        }
    }
}

/// IDs to row numbers here, filled in as an import adds records.
#[derive(Default)]
pub struct ToLocal(HashMap<&'static str, HashMap<String, i64>>);

impl ToLocal {
    fn read(connection: &Connection) -> Result<Self> {
        let mut maps = HashMap::new();
        for table in crate::ids::STAMPED {
            let mut statement = connection.prepare(&format!("SELECT uid, id FROM {table} WHERE uid IS NOT NULL"))?;
            let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
            maps.insert(*table, rows.collect::<rusqlite::Result<HashMap<_, _>>>()?);
        }
        Ok(Self(maps))
    }

    pub fn of(&self, table: &str, uid: &str) -> Option<i64> {
        self.0.get(table)?.get(uid).copied()
    }

    fn need(&self, table: &str, uid: &str) -> Result<i64> {
        self.of(table, uid).ok_or(StoreError::Invalid("the file refers to something that isn't in it"))
    }

    fn opt(&self, table: &str, uid: Option<&str>) -> Option<i64> {
        uid.and_then(|uid| self.of(table, uid))
    }

    fn set(&mut self, table: &'static str, uid: &str, id: i64) {
        self.0.entry(table).or_default().insert(uid.to_string(), id);
    }

    /// An ID in JSON back to a row number here. Null stays null, and an ID that isn't
    /// here becomes 0, which matches nothing.
    fn json(&self, table: &str, value: &mut Value) {
        if let Some(uid) = value.as_str() {
            *value = Value::from(self.of(table, uid).unwrap_or(0));
        }
    }

    fn key(&self, key: &str) -> Option<String> {
        let (kind, rest) = key.split_once(':')?;
        match kind {
            "quiz" => Some(format!("quiz:{}", self.of("questions", rest)?)),
            "struggle" => Some(format!("struggle:{}", self.of("struggles", rest)?)),
            "recall" => {
                let (checklist, text) = rest.split_once(':')?;
                Some(format!("recall:{}:{text}", self.of("recall_checklists", checklist)?))
            }
            _ => None,
        }
    }
}

fn parse_json(text: &str) -> Result<Value> {
    Ok(serde_json::from_str(text)?)
}

// ---------- Reading ----------

/// (id, topic, uid, changed, note, source, created)
type NoteRow = (i64, i64, String, String, String, Option<String>, i64);
/// (id, uid, changed, day, status, retry_of, attempt, reasons)
type AttemptRow = (i64, String, String, i64, String, Option<i64>, String, String);

impl Store {
    /// The records in `scope`, ready to write to a file (R112, R113).
    pub fn snapshot(&self, scope: &Scope) -> Result<Snapshot> {
        let connection = &self.connection;
        let uids = ToUid::read(connection)?;
        let everything = scope.topics.is_none();
        let history = scope.history;
        let picked: Option<HashSet<i64>> = scope.topics.as_ref().map(|topics| topics.iter().copied().collect());
        let topic_in = |id: i64| picked.as_ref().is_none_or(|picked| picked.contains(&id));
        let mut snapshot = Snapshot::default();

        // Topics, and the modules they're in.
        let mut module_ids = HashSet::new();
        let mut statement = connection.prepare("SELECT id, uid, changed, name, module_id FROM topics ORDER BY id")?;
        let topics: Vec<(i64, String, String, String, Option<i64>)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?.collect::<rusqlite::Result<_>>()?;
        for (id, uid, changed, name, module) in topics.into_iter().filter(|topic| topic_in(topic.0)) {
            if let Some(module) = module {
                module_ids.insert(module);
            }
            let tags = connection.prepare("SELECT tag FROM topic_tags WHERE topic_id = ?1 ORDER BY tag")?.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            let schedule = if history {
                connection.query_row("SELECT step, rep, due FROM topic_schedule WHERE topic_id = ?1", [id], |row| Ok(QuizStep { step: row.get(0)?, rep: row.get(1)?, due: row.get(2)? })).optional()?
            } else {
                None
            };
            snapshot.topics.push(Topic { uid, changed, name, module: uids.opt("modules", module), tags, schedule });
        }
        let mut statement = connection.prepare("SELECT id, uid, changed, name FROM modules ORDER BY id")?;
        snapshot.modules = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, Module { uid: row.get(1)?, changed: row.get(2)?, name: row.get(3)? })))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(id, _)| everything || module_ids.contains(id))
            .map(|(_, module)| module)
            .collect();

        // Questions, with their options, tags and citations.
        let mut source_ids = HashSet::new();
        let mut question_ids = HashSet::new();
        let mut statement = connection.prepare("SELECT id, uid, changed, topic_id, kind, text, explanation, short, edited_at FROM questions ORDER BY id")?;
        let rows: Vec<(i64, i64, Question)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(3)?, Question {
                    uid: row.get(1)?, changed: row.get(2)?, topic: String::new(), kind: row.get(4)?, text: row.get(5)?, explanation: row.get(6)?,
                    short: row.get::<_, Option<String>>(7)?.map(|json| serde_json::from_str(&json).unwrap_or(Value::Null)), edited_at: row.get(8)?,
                    options: vec![], tags: vec![], citations: vec![],
                }))
            })?
            .collect::<rusqlite::Result<_>>()?;
        for (id, topic, mut question) in rows.into_iter().filter(|row| topic_in(row.1)) {
            question.topic = uids.of("topics", topic).unwrap_or_default();
            question.options = connection.prepare("SELECT text, correct FROM options WHERE question_id = ?1 ORDER BY position")?.query_map([id], |row| Ok(QuestionOption { text: row.get(0)?, correct: row.get(1)? }))?.collect::<rusqlite::Result<_>>()?;
            question.tags = connection.prepare("SELECT tag FROM question_tags WHERE question_id = ?1 ORDER BY tag")?.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            let cited: Vec<(i64, String)> = connection.prepare("SELECT source_id, location FROM citations WHERE question_id = ?1 ORDER BY position")?.query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            question.citations = cited.into_iter().filter_map(|(source, location)| { source_ids.insert(source); Some(Cited { source: uids.of("sources", source)?, location }) }).collect();
            question_ids.insert(id);
            snapshot.questions.push(question);
        }
        let mut statement = connection.prepare("SELECT id, uid, changed, authors, year, title, edition, description, publisher, url FROM sources ORDER BY id")?;
        snapshot.sources = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, Source { uid: row.get(1)?, changed: row.get(2)?, authors: row.get(3)?, year: row.get(4)?, title: row.get(5)?, edition: row.get(6)?, description: row.get(7)?, publisher: row.get(8)?, url: row.get(9)? })))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(id, _)| everything || source_ids.contains(id))
            .map(|(_, source)| source)
            .collect();

        // Notes and their cards.
        let mut note_ids = HashSet::new();
        let mut card_ids = HashSet::new();
        let mut statement = connection.prepare("SELECT id, uid, changed, topic_id, note, source, created FROM notes ORDER BY id")?;
        let rows: Vec<NoteRow> = statement.query_map([], |row| Ok((row.get(0)?, row.get(3)?, row.get(1)?, row.get(2)?, row.get(4)?, row.get(5)?, row.get(6)?)))?.collect::<rusqlite::Result<_>>()?;
        for (id, topic, uid, changed, json, source, created) in rows.into_iter().filter(|row| topic_in(row.1)) {
            let mut note = parse_json(&json)?;
            if let Value::Object(fields) = &mut note {
                fields.remove("topic");
            }
            let mut statement = connection.prepare("SELECT id, uid, changed, key, front, back, archived, stability, difficulty, due, last_review, reps, lapses FROM cards WHERE note_id = ?1 ORDER BY id")?;
            let cards = statement
                .query_map([id], |row| {
                    Ok((row.get::<_, i64>(0)?, Card {
                        uid: row.get(1)?, changed: row.get(2)?, key: row.get(3)?, front: row.get(4)?, back: row.get(5)?, archived: row.get(6)?,
                        schedule: history.then(|| -> rusqlite::Result<CardSchedule> { Ok(CardSchedule { stability: row.get(7)?, difficulty: row.get(8)?, due: row.get(9)?, last_review: row.get(10)?, reps: row.get(11)?, lapses: row.get(12)? }) }).transpose()?,
                    }))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            card_ids.extend(cards.iter().map(|(id, _)| *id));
            note_ids.insert(id);
            let source = if everything { source.and_then(|key| uids.key(&key)) } else { None };
            snapshot.notes.push(Note { uid, changed, topic: uids.of("topics", topic).unwrap_or_default(), note, source, created, cards: cards.into_iter().map(|(_, card)| card).collect() });
        }
        if history {
            let mut statement = connection.prepare("SELECT uid, changed, card_id, day, rating, elapsed, was_new, seconds FROM card_reviews ORDER BY id")?;
            snapshot.reviews = statement
                .query_map([], |row| Ok((row.get::<_, i64>(2)?, Review { uid: row.get(0)?, changed: row.get(1)?, card: String::new(), day: row.get(3)?, rating: row.get(4)?, elapsed: row.get(5)?, was_new: row.get(6)?, seconds: row.get(7)? })))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .filter(|(card, _)| card_ids.contains(card))
                .map(|(card, review)| Review { card: uids.of("cards", card).unwrap_or_default(), ..review })
                .collect();
        }

        // Recall checklists: about a picked topic, or a module whose topics are all picked.
        let whole_modules: HashSet<i64> = if everything {
            HashSet::new()
        } else {
            module_ids
                .iter()
                .copied()
                .filter(|module| {
                    connection
                        .prepare("SELECT id FROM topics WHERE module_id = ?1")
                        .and_then(|mut statement| statement.query_map([module], |row| row.get::<_, i64>(0))?.collect::<rusqlite::Result<Vec<_>>>())
                        .map(|topics| topics.iter().all(|topic| topic_in(*topic)))
                        .unwrap_or(false)
                })
                .collect()
        };
        let mut checklist_ids = HashSet::new();
        let mut statement = connection.prepare("SELECT id, uid, changed, module_id, topic_id, subject_name, subject_kind, created, step, rep, due FROM recall_checklists ORDER BY id")?;
        let rows: Vec<(i64, Option<i64>, Option<i64>, Checklist)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(3)?, row.get(4)?, Checklist {
                    uid: row.get(1)?, changed: row.get(2)?, module: None, topic: None, subject_name: row.get(5)?, subject_kind: row.get(6)?, created: row.get(7)?,
                    schedule: history.then(|| -> rusqlite::Result<ChecklistStep> { Ok(ChecklistStep { step: row.get(8)?, rep: row.get(9)?, due: row.get(10)? }) }).transpose()?, points: vec![],
                }))
            })?
            .collect::<rusqlite::Result<_>>()?;
        for (id, module, topic, mut checklist) in rows {
            let wanted = everything || topic.is_some_and(&topic_in) || module.is_some_and(|module| whole_modules.contains(&module));
            if !wanted {
                continue;
            }
            checklist.module = uids.opt("modules", module);
            checklist.topic = uids.opt("topics", topic);
            let points: Vec<(String, Option<i64>, Option<i64>)> = connection.prepare("SELECT text, question_id, note_id FROM recall_points WHERE checklist_id = ?1 ORDER BY position")?.query_map([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
            checklist.points = points
                .into_iter()
                .map(|(text, question, note)| Point {
                    text,
                    question: question.filter(|id| everything || question_ids.contains(id)).and_then(|id| uids.of("questions", id)),
                    note: note.filter(|id| everything || note_ids.contains(id)).and_then(|id| uids.of("notes", id)),
                })
                .collect();
            checklist_ids.insert(id);
            snapshot.checklists.push(checklist);
        }
        if history {
            let mut statement = connection.prepare("SELECT uid, changed, checklist_id, day, dump, points, score, passed, from_label, to_label FROM recall_checks ORDER BY id")?;
            let rows: Vec<(i64, Check, String)> = statement
                .query_map([], |row| Ok((row.get(2)?, Check { uid: row.get(0)?, changed: row.get(1)?, checklist: String::new(), day: row.get(3)?, dump: row.get(4)?, points: Value::Null, score: row.get(6)?, passed: row.get(7)?, from_label: row.get(8)?, to_label: row.get(9)? }, row.get(5)?)))?
                .collect::<rusqlite::Result<_>>()?;
            for (checklist, check, points) in rows.into_iter().filter(|row| checklist_ids.contains(&row.0)) {
                snapshot.checks.push(Check { checklist: uids.of("recall_checklists", checklist).unwrap_or_default(), points: parse_json(&points)?, ..check });
            }
        }

        if everything {
            self.snapshot_the_rest(&uids, history, &mut snapshot)?;
        }

        // Images used by anything in the export.
        let used: HashSet<String> = {
            let text = serde_json::to_string(&snapshot)?;
            let mut used = HashSet::new();
            let mut rest = text.as_str();
            while let Some(at) = rest.find("image:") {
                rest = &rest[at + 6..];
                let end = rest.find(|letter: char| !letter.is_ascii_digit()).unwrap_or(rest.len());
                used.insert(rest[..end].to_string());
            }
            used
        };
        let mut statement = connection.prepare("SELECT id, uid, changed, format, bytes FROM images ORDER BY id")?;
        snapshot.images = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, Image { uid: row.get(1)?, changed: row.get(2)?, format: row.get(3)?, bytes: row.get(4)? })))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(id, _)| everything || used.contains(&id.to_string()))
            .map(|(_, image)| image)
            .collect();

        // Last, every image reference in text becomes the image's ID.
        let swap = |token: &str| token.parse::<i64>().ok().and_then(|id| uids.of("images", id));
        let images = std::mem::take(&mut snapshot.images);
        let mut snapshot = with_images(&snapshot, &swap)?;
        snapshot.images = images;
        Ok(snapshot)
    }

    /// Quizzes, projects, settings and dismissed suggestions, for a full export (R112).
    fn snapshot_the_rest(&self, uids: &ToUid, history: bool, snapshot: &mut Snapshot) -> Result<()> {
        let connection = &self.connection;
        if history {
            let mut statement = connection.prepare("SELECT id, uid, changed, day, status, retry_of, attempt, reasons FROM attempts ORDER BY id")?;
            let rows: Vec<AttemptRow> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)))?.collect::<rusqlite::Result<_>>()?;
            for (id, uid, changed, day, status, retry_of, attempt, reasons) in rows {
                let mut attempt = parse_json(&attempt)?;
                for (path, table) in ATTEMPT_REFS {
                    at_path(&mut attempt, path, &mut |value| uids.json(table, value));
                }
                let mut statement = connection.prepare("SELECT topic_id, day, from_step, from_rep, from_due, score, passed, to_step, to_rep, next_due FROM quiz_moves WHERE attempt_id = ?1 ORDER BY topic_id")?;
                let moves = statement
                    .query_map([id], |row| Ok((row.get::<_, i64>(0)?, QuizMove { topic: String::new(), day: row.get(1)?, from_step: row.get(2)?, from_rep: row.get(3)?, from_due: row.get(4)?, score: row.get(5)?, passed: row.get(6)?, to_step: row.get(7)?, to_rep: row.get(8)?, next_due: row.get(9)? })))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .filter_map(|(topic, quiz_move)| Some(QuizMove { topic: uids.of("topics", topic)?, ..quiz_move }))
                    .collect();
                snapshot.attempts.push(Attempt { uid, changed, day, status: parse_json(&status)?, retry_of: uids.opt("attempts", retry_of), attempt, reasons: parse_json(&reasons)?, moves });
            }
        }
        let mut statement = connection.prepare("SELECT uid, changed, name, kind, created FROM projects ORDER BY id")?;
        snapshot.projects = statement.query_map([], |row| Ok(Project { uid: row.get(0)?, changed: row.get(1)?, name: row.get(2)?, kind: row.get(3)?, created: row.get(4)? }))?.collect::<rusqlite::Result<_>>()?;
        let mut statement = connection.prepare("SELECT uid, changed, project_id, day, started, ended FROM project_sessions ORDER BY id")?;
        snapshot.sessions = statement.query_map([], |row| Ok(Session { uid: row.get(0)?, changed: row.get(1)?, project: uids.of("projects", row.get(2)?).unwrap_or_default(), day: row.get(3)?, started: row.get(4)?, ended: row.get(5)? }))?.collect::<rusqlite::Result<_>>()?;
        let mut statement = connection.prepare("SELECT uid, changed, name, key FROM issues ORDER BY id")?;
        snapshot.issues = statement.query_map([], |row| Ok(Issue { uid: row.get(0)?, changed: row.get(1)?, name: row.get(2)?, key: row.get(3)? }))?.collect::<rusqlite::Result<_>>()?;
        let mut statement = connection.prepare("SELECT uid, changed, issue_id, topic_id, task, hints, created, due FROM drills ORDER BY id")?;
        snapshot.drills = statement.query_map([], |row| Ok(Drill { uid: row.get(0)?, changed: row.get(1)?, issue: uids.opt("issues", row.get(2)?), topic: uids.opt("topics", row.get(3)?), task: row.get(4)?, hints: row.get(5)?, created: row.get(6)?, due: row.get(7)? }))?.collect::<rusqlite::Result<_>>()?;
        if history {
            let mut statement = connection.prepare("SELECT uid, changed, drill_id, day, rating, seconds, note FROM drill_runs ORDER BY id")?;
            snapshot.drill_runs = statement.query_map([], |row| Ok(DrillRun { uid: row.get(0)?, changed: row.get(1)?, drill: uids.of("drills", row.get(2)?).unwrap_or_default(), day: row.get(3)?, rating: row.get(4)?, seconds: row.get(5)?, note: row.get(6)? }))?.collect::<rusqlite::Result<_>>()?;
        }
        let mut statement = connection.prepare("SELECT uid, changed, project_id, session_id, day, minute, what, why, kind, topic_id, issue_id, outcome FROM struggles ORDER BY id")?;
        let rows: Vec<(Struggle, Option<String>)> = statement
            .query_map([], |row| {
                Ok((Struggle {
                    uid: row.get(0)?, changed: row.get(1)?, project: uids.of("projects", row.get(2)?).unwrap_or_default(), session: uids.opt("project_sessions", row.get(3)?),
                    day: row.get(4)?, minute: row.get(5)?, what: row.get(6)?, why: row.get(7)?, kind: row.get(8)?, topic: uids.opt("topics", row.get(9)?), issue: uids.opt("issues", row.get(10)?), outcome: None,
                }, row.get(11)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        for (struggle, outcome) in rows {
            let outcome = outcome.map(|json| parse_json(&json)).transpose()?.map(|mut outcome| {
                at_path(&mut outcome, &["Checklist"], &mut |value| uids.json("recall_checklists", value));
                at_path(&mut outcome, &["Drill"], &mut |value| uids.json("drills", value));
                outcome
            });
            snapshot.struggles.push(Struggle { outcome, ..struggle });
        }
        let mut statement = connection.prepare("SELECT key, value, changed FROM settings ORDER BY key")?;
        snapshot.settings = statement.query_map([], |row| Ok(Setting { key: row.get(0)?, value: row.get(1)?, changed: row.get::<_, Option<String>>(2)?.unwrap_or_default() }))?.collect::<rusqlite::Result<_>>()?;
        let mut statement = connection.prepare("SELECT key, status, changed FROM card_suggestions_done ORDER BY key")?;
        snapshot.suggestions_done = statement
            .query_map([], |row| Ok(SuggestionDone { key: row.get(0)?, status: row.get(1)?, changed: row.get::<_, Option<String>>(2)?.unwrap_or_default() }))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|done| Some(SuggestionDone { key: uids.key(&done.key)?, ..done }))
            .collect();
        Ok(())
    }
}

// ---------- Writing ----------

/// What to do with a record that's here already and differs (R116).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Take {
    Mine,
    Theirs,
}

/// An import, decided: which records that are here already take the file's version.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// IDs of records here that the file's version replaces.
    pub theirs: HashSet<String>,
    /// The day new cards and checklists start from when the file has no history.
    pub today: i64,
}

/// How many records were added, by kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Added {
    pub counts: Vec<(String, usize)>,
    pub replaced: usize,
}

impl Store {
    /// Every record's `changed` stamp here, by kind and ID, for an import preview.
    pub fn stamps_here(&self) -> Result<HashMap<String, String>> {
        let mut stamps = HashMap::new();
        for table in crate::ids::STAMPED {
            let mut statement = self.connection.prepare(&format!("SELECT uid, changed FROM {table} WHERE uid IS NOT NULL"))?;
            for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))? {
                let (uid, changed) = row?;
                stamps.insert(uid, changed.unwrap_or_default());
            }
        }
        for (table, prefix) in [("settings", "settings:"), ("card_suggestions_done", "suggestion:")] {
            let mut statement = self.connection.prepare(&format!("SELECT key, changed FROM {table}"))?;
            for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))? {
                let (key, changed) = row?;
                stamps.insert(format!("{prefix}{key}"), changed.unwrap_or_default());
            }
        }
        Ok(stamps)
    }

    /// Adds what's new in `snapshot` and replaces what `plan` says, all or nothing (R114
    /// to R117). Records here that the file doesn't have are left alone, and nothing is
    /// deleted.
    pub fn apply(&mut self, snapshot: &Snapshot, plan: &Plan) -> Result<Added> {
        let transaction = self.connection.transaction()?;
        let added = write(&transaction, snapshot, plan)?;
        transaction.commit()?;
        crate::ids::catch_up(&self.connection, &self.clock)?;
        Ok(added)
    }
}

struct Writer<'a> {
    tx: &'a Transaction<'a>,
    plan: &'a Plan,
    local: ToLocal,
    added: HashMap<&'static str, usize>,
    replaced: usize,
}

enum Fate {
    New,
    Replace(i64),
    Keep(i64),
}

impl Writer<'_> {
    fn fate(&self, table: &str, uid: &str) -> Fate {
        match self.local.of(table, uid) {
            None => Fate::New,
            Some(id) if self.plan.theirs.contains(uid) => Fate::Replace(id),
            Some(id) => Fate::Keep(id),
        }
    }

    fn count(&mut self, kind: &'static str) {
        *self.added.entry(kind).or_default() += 1;
    }

    fn inserted(&mut self, table: &'static str, kind: &'static str, uid: &str) -> i64 {
        let id = self.tx.last_insert_rowid();
        self.local.set(table, uid, id);
        self.count(kind);
        id
    }

    /// Sets a record's stamp to the file's, after its parts were written (writing the
    /// parts moves the parent's stamp).
    fn stamp(&self, table: &str, id: i64, changed: &str) -> Result<()> {
        // Setting a stamp to the value it has already would count as an edit and move it.
        self.tx.execute(&format!("UPDATE {table} SET changed = ?2 WHERE id = ?1 AND changed IS NOT ?2"), params![id, changed])?;
        Ok(())
    }
}

fn write(tx: &Transaction, snapshot: &Snapshot, plan: &Plan) -> Result<Added> {
    let mut writer = Writer { tx, plan, local: ToLocal::read(tx)?, added: HashMap::new(), replaced: 0 };

    // Images first, so references to them in text can be put back as row numbers. An
    // image that's here already under another ID is used as it is.
    for image in &snapshot.images {
        if writer.local.of("images", &image.uid).is_some() {
            continue;
        }
        let same: Option<i64> = tx.query_row("SELECT id FROM images WHERE bytes = ?1", [&image.bytes], |row| row.get(0)).optional()?;
        match same {
            Some(id) => writer.local.set("images", &image.uid, id),
            None => {
                tx.execute("INSERT INTO images (format, bytes, uid, changed) VALUES (?1, ?2, ?3, ?4)", params![image.format, image.bytes, image.uid, image.changed])?;
                writer.inserted("images", "images", &image.uid);
            }
        }
    }
    let images: HashMap<String, i64> = writer.local.0.get("images").cloned().unwrap_or_default();
    let swap = move |token: &str| images.get(token).map(|id| id.to_string());
    let snapshot = &with_images(snapshot, &swap)?;

    for module in &snapshot.modules {
        match writer.fate("modules", &module.uid) {
            Fate::New => {
                tx.execute("INSERT INTO modules (name, uid, changed) VALUES (?1, ?2, ?3)", params![module.name, module.uid, module.changed])?;
                writer.inserted("modules", "modules", &module.uid);
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE modules SET name = ?2, changed = ?3 WHERE id = ?1", params![id, module.name, module.changed])?;
                writer.replaced += 1;
            }
            Fate::Keep(_) => {}
        }
    }

    for topic in &snapshot.topics {
        let module = writer.local.opt("modules", topic.module.as_deref());
        let fate = writer.fate("topics", &topic.uid);
        let is_new = matches!(fate, Fate::New);
        let id = match fate {
            Fate::New => {
                tx.execute("INSERT INTO topics (name, module_id, uid, changed) VALUES (?1, ?2, ?3, ?4)", params![topic.name, module, topic.uid, topic.changed])?;
                Some(writer.inserted("topics", "topics", &topic.uid))
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE topics SET name = ?2, module_id = ?3 WHERE id = ?1", params![id, topic.name, module])?;
                tx.execute("DELETE FROM topic_tags WHERE topic_id = ?1", [id])?;
                later_quiz_step(tx, id, topic.schedule.as_ref())?;
                writer.replaced += 1;
                Some(id)
            }
            Fate::Keep(id) => {
                later_quiz_step(tx, id, topic.schedule.as_ref())?;
                None
            }
        };
        if let Some(id) = id {
            for tag in &topic.tags {
                tx.execute("INSERT OR IGNORE INTO topic_tags (topic_id, tag) VALUES (?1, ?2)", params![id, tag])?;
            }
            if let (Some(schedule), true) = (&topic.schedule, is_new) {
                tx.execute("INSERT INTO topic_schedule (topic_id, step, rep, due) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(topic_id) DO UPDATE SET step = excluded.step, rep = excluded.rep, due = excluded.due", params![id, schedule.step, schedule.rep, schedule.due])?;
            }
            writer.stamp("topics", id, &topic.changed)?;
        }
    }

    for source in &snapshot.sources {
        match writer.fate("sources", &source.uid) {
            Fate::New => {
                tx.execute("INSERT INTO sources (authors, year, title, edition, description, publisher, url, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", params![source.authors, source.year, source.title, source.edition, source.description, source.publisher, source.url, source.uid, source.changed])?;
                writer.inserted("sources", "sources", &source.uid);
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE sources SET authors = ?2, year = ?3, title = ?4, edition = ?5, description = ?6, publisher = ?7, url = ?8, changed = ?9 WHERE id = ?1", params![id, source.authors, source.year, source.title, source.edition, source.description, source.publisher, source.url, source.changed])?;
                writer.replaced += 1;
            }
            Fate::Keep(_) => {}
        }
    }

    for question in &snapshot.questions {
        let topic = writer.local.need("topics", &question.topic)?;
        let short = question.short.as_ref().filter(|short| !short.is_null()).map(serde_json::to_string).transpose()?;
        let id = match writer.fate("questions", &question.uid) {
            Fate::New => {
                tx.execute("INSERT INTO questions (topic_id, kind, text, explanation, short, edited_at, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![topic, question.kind, question.text, question.explanation, short, question.edited_at, question.uid, question.changed])?;
                writer.inserted("questions", "questions", &question.uid)
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE questions SET topic_id = ?2, kind = ?3, text = ?4, explanation = ?5, short = ?6, edited_at = ?7 WHERE id = ?1", params![id, topic, question.kind, question.text, question.explanation, short, question.edited_at])?;
                for table in ["options", "question_tags", "citations"] {
                    tx.execute(&format!("DELETE FROM {table} WHERE question_id = ?1"), [id])?;
                }
                writer.replaced += 1;
                id
            }
            Fate::Keep(_) => continue,
        };
        for (position, option) in question.options.iter().enumerate() {
            tx.execute("INSERT INTO options (question_id, position, text, correct) VALUES (?1, ?2, ?3, ?4)", params![id, position as i64, option.text, option.correct])?;
        }
        for tag in &question.tags {
            tx.execute("INSERT OR IGNORE INTO question_tags (question_id, tag) VALUES (?1, ?2)", params![id, tag])?;
        }
        for (position, cited) in question.citations.iter().enumerate() {
            if let Some(source) = writer.local.of("sources", &cited.source) {
                tx.execute("INSERT INTO citations (question_id, position, source_id, location) VALUES (?1, ?2, ?3, ?4)", params![id, position as i64, source, cited.location])?;
            }
        }
        writer.stamp("questions", id, &question.changed)?;
    }

    for note in &snapshot.notes {
        let topic = writer.local.need("topics", &note.topic)?;
        let mut stored = note.note.clone();
        if let Value::Object(fields) = &mut stored {
            fields.insert("topic".into(), Value::from(topic));
        }
        let json = serde_json::to_string(&stored)?;
        let source = note.source.as_deref().and_then(|key| writer.local.key(key));
        let (id, taking) = match writer.fate("notes", &note.uid) {
            Fate::New => {
                tx.execute("INSERT INTO notes (topic_id, note, source, created, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![topic, json, source, note.created, note.uid, note.changed])?;
                (writer.inserted("notes", "notes", &note.uid), true)
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE notes SET topic_id = ?2, note = ?3, changed = ?4 WHERE id = ?1", params![id, topic, json, note.changed])?;
                // The note's cards become the file's: ones it no longer has go.
                let keep: HashSet<&str> = note.cards.iter().map(|card| card.uid.as_str()).collect();
                let here: Vec<(i64, String)> = tx.prepare("SELECT id, uid FROM cards WHERE note_id = ?1")?.query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
                for (card, uid) in here.into_iter().filter(|(_, uid)| !keep.contains(uid.as_str())) {
                    tx.execute("DELETE FROM cards WHERE id = ?1", [card])?;
                    writer.local.0.entry("cards").or_default().remove(&uid);
                }
                writer.replaced += 1;
                (id, true)
            }
            Fate::Keep(id) => (id, false),
        };
        for card in &note.cards {
            let schedule = card.schedule.clone().unwrap_or(CardSchedule { stability: None, difficulty: None, due: plan.today, last_review: None, reps: 0, lapses: 0 });
            match writer.local.of("cards", &card.uid) {
                // A note kept as it is here keeps its own set of cards.
                None if !taking => {}
                None => {
                    tx.execute(
                        "INSERT INTO cards (note_id, key, front, back, archived, stability, difficulty, due, last_review, reps, lapses, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                        params![id, card.key, card.front, card.back, card.archived, schedule.stability, schedule.difficulty, schedule.due, schedule.last_review, schedule.reps, schedule.lapses, card.uid, card.changed],
                    )?;
                    writer.inserted("cards", "cards", &card.uid);
                }
                Some(card_id) => {
                    if taking {
                        tx.execute("UPDATE cards SET key = ?2, front = ?3, back = ?4, archived = ?5, changed = ?6 WHERE id = ?1", params![card_id, card.key, card.front, card.back, card.archived, card.changed])?;
                    }
                    if let Some(theirs) = &card.schedule {
                        // The side reviewed more recently keeps its schedule (R117).
                        let mine: Option<i64> = tx.query_row("SELECT last_review FROM cards WHERE id = ?1", [card_id], |row| row.get(0))?;
                        if theirs.last_review > mine {
                            tx.execute("UPDATE cards SET stability = ?2, difficulty = ?3, due = ?4, last_review = ?5, reps = ?6, lapses = ?7 WHERE id = ?1", params![card_id, theirs.stability, theirs.difficulty, theirs.due, theirs.last_review, theirs.reps, theirs.lapses])?;
                        }
                    }
                }
            }
        }
    }

    for review in &snapshot.reviews {
        if writer.local.of("card_reviews", &review.uid).is_some() {
            continue;
        }
        let Some(card) = writer.local.of("cards", &review.card) else { continue };
        tx.execute("INSERT INTO card_reviews (card_id, day, rating, elapsed, was_new, seconds, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![card, review.day, review.rating, review.elapsed, review.was_new, review.seconds, review.uid, review.changed])?;
        writer.inserted("card_reviews", "card reviews", &review.uid);
    }

    for checklist in &snapshot.checklists {
        let module = writer.local.opt("modules", checklist.module.as_deref());
        let topic = writer.local.opt("topics", checklist.topic.as_deref());
        let schedule = checklist.schedule.clone().unwrap_or(ChecklistStep { step: None, rep: None, due: plan.today });
        let id = match writer.fate("recall_checklists", &checklist.uid) {
            Fate::New => {
                tx.execute(
                    "INSERT INTO recall_checklists (module_id, topic_id, subject_name, subject_kind, created, step, rep, due, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![module, topic, checklist.subject_name, checklist.subject_kind, checklist.created, schedule.step, schedule.rep, schedule.due, checklist.uid, checklist.changed],
                )?;
                writer.inserted("recall_checklists", "checklists", &checklist.uid)
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE recall_checklists SET module_id = ?2, topic_id = ?3, subject_name = ?4, subject_kind = ?5 WHERE id = ?1", params![id, module, topic, checklist.subject_name, checklist.subject_kind])?;
                tx.execute("DELETE FROM recall_points WHERE checklist_id = ?1", [id])?;
                writer.replaced += 1;
                id
            }
            Fate::Keep(id) => {
                later_checklist_step(tx, id, &checklist.uid, snapshot, checklist.schedule.as_ref())?;
                continue;
            }
        };
        for (position, point) in checklist.points.iter().enumerate() {
            tx.execute(
                "INSERT INTO recall_points (checklist_id, position, text, question_id, note_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, position as i64, point.text, writer.local.opt("questions", point.question.as_deref()), writer.local.opt("notes", point.note.as_deref())],
            )?;
        }
        writer.stamp("recall_checklists", id, &checklist.changed)?;
    }

    for check in &snapshot.checks {
        if writer.local.of("recall_checks", &check.uid).is_some() {
            continue;
        }
        let Some(checklist) = writer.local.of("recall_checklists", &check.checklist) else { continue };
        tx.execute(
            "INSERT INTO recall_checks (checklist_id, day, dump, points, score, passed, from_label, to_label, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![checklist, check.day, check.dump, serde_json::to_string(&check.points)?, check.score, check.passed, check.from_label, check.to_label, check.uid, check.changed],
        )?;
        writer.inserted("recall_checks", "recall checks", &check.uid);
    }

    for attempt in &snapshot.attempts {
        if writer.local.of("attempts", &attempt.uid).is_some() {
            continue;
        }
        let mut stored = attempt.attempt.clone();
        for (path, table) in ATTEMPT_REFS {
            at_path(&mut stored, path, &mut |value| writer.local.json(table, value));
        }
        let retry_of = writer.local.opt("attempts", attempt.retry_of.as_deref());
        if let (Some(retry_of), Value::Object(fields)) = (retry_of, &mut stored) {
            fields.insert("retry_of".into(), Value::from(retry_of));
        }
        tx.execute(
            "INSERT INTO attempts (day, status, retry_of, attempt, reasons, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![attempt.day, serde_json::to_string(&attempt.status)?, retry_of, serde_json::to_string(&stored)?, serde_json::to_string(&attempt.reasons)?, attempt.uid, attempt.changed],
        )?;
        let id = writer.inserted("attempts", "quizzes", &attempt.uid);
        for quiz_move in &attempt.moves {
            let Some(topic) = writer.local.of("topics", &quiz_move.topic) else { continue };
            tx.execute(
                "INSERT OR IGNORE INTO quiz_moves (attempt_id, topic_id, day, from_step, from_rep, from_due, score, passed, to_step, to_rep, next_due) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![id, topic, quiz_move.day, quiz_move.from_step, quiz_move.from_rep, quiz_move.from_due, quiz_move.score, quiz_move.passed, quiz_move.to_step, quiz_move.to_rep, quiz_move.next_due],
            )?;
        }
    }

    for project in &snapshot.projects {
        match writer.fate("projects", &project.uid) {
            Fate::New => {
                tx.execute("INSERT INTO projects (name, kind, created, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5)", params![project.name, project.kind, project.created, project.uid, project.changed])?;
                writer.inserted("projects", "projects", &project.uid);
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE projects SET name = ?2, kind = ?3, changed = ?4 WHERE id = ?1", params![id, project.name, project.kind, project.changed])?;
                writer.replaced += 1;
            }
            Fate::Keep(_) => {}
        }
    }
    for session in &snapshot.sessions {
        if writer.local.of("project_sessions", &session.uid).is_some() {
            continue;
        }
        let Some(project) = writer.local.of("projects", &session.project) else { continue };
        tx.execute("INSERT INTO project_sessions (project_id, day, started, ended, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![project, session.day, session.started, session.ended, session.uid, session.changed])?;
        writer.inserted("project_sessions", "sessions", &session.uid);
    }
    for issue in &snapshot.issues {
        match writer.fate("issues", &issue.uid) {
            Fate::New => {
                // Issues are matched by name too, so one named on both computers stays one (R94).
                let same: Option<i64> = tx.query_row("SELECT id FROM issues WHERE key = ?1", [&issue.key], |row| row.get(0)).optional()?;
                match same {
                    Some(id) => writer.local.set("issues", &issue.uid, id),
                    None => {
                        tx.execute("INSERT INTO issues (name, key, uid, changed) VALUES (?1, ?2, ?3, ?4)", params![issue.name, issue.key, issue.uid, issue.changed])?;
                        writer.inserted("issues", "issues", &issue.uid);
                    }
                }
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE issues SET name = ?2, changed = ?3 WHERE id = ?1", params![id, issue.name, issue.changed])?;
                writer.replaced += 1;
            }
            Fate::Keep(_) => {}
        }
    }
    for drill in &snapshot.drills {
        let issue = writer.local.opt("issues", drill.issue.as_deref());
        let topic = writer.local.opt("topics", drill.topic.as_deref());
        match writer.fate("drills", &drill.uid) {
            Fate::New => {
                tx.execute("INSERT INTO drills (issue_id, topic_id, task, hints, created, due, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![issue, topic, drill.task, drill.hints, drill.created, drill.due, drill.uid, drill.changed])?;
                writer.inserted("drills", "drills", &drill.uid);
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE drills SET issue_id = ?2, topic_id = ?3, task = ?4, hints = ?5, due = ?6, changed = ?7 WHERE id = ?1", params![id, issue, topic, drill.task, drill.hints, drill.due, drill.changed])?;
                writer.replaced += 1;
            }
            Fate::Keep(_) => {}
        }
    }
    for run in &snapshot.drill_runs {
        if writer.local.of("drill_runs", &run.uid).is_some() {
            continue;
        }
        let Some(drill) = writer.local.of("drills", &run.drill) else { continue };
        tx.execute("INSERT INTO drill_runs (drill_id, day, rating, seconds, note, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![drill, run.day, run.rating, run.seconds, run.note, run.uid, run.changed])?;
        writer.inserted("drill_runs", "drill runs", &run.uid);
    }
    for struggle in &snapshot.struggles {
        let Some(project) = writer.local.of("projects", &struggle.project) else { continue };
        let outcome = struggle
            .outcome
            .clone()
            .map(|mut outcome| {
                at_path(&mut outcome, &["Checklist"], &mut |value| writer.local.json("recall_checklists", value));
                at_path(&mut outcome, &["Drill"], &mut |value| writer.local.json("drills", value));
                serde_json::to_string(&outcome)
            })
            .transpose()?;
        let session = writer.local.opt("project_sessions", struggle.session.as_deref());
        let topic = writer.local.opt("topics", struggle.topic.as_deref());
        let issue = writer.local.opt("issues", struggle.issue.as_deref());
        match writer.fate("struggles", &struggle.uid) {
            Fate::New => {
                tx.execute(
                    "INSERT INTO struggles (project_id, session_id, day, minute, what, why, kind, topic_id, issue_id, outcome, uid, changed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![project, session, struggle.day, struggle.minute, struggle.what, struggle.why, struggle.kind, topic, issue, outcome, struggle.uid, struggle.changed],
                )?;
                writer.inserted("struggles", "struggles", &struggle.uid);
            }
            Fate::Replace(id) => {
                tx.execute("UPDATE struggles SET what = ?2, why = ?3, kind = ?4, topic_id = ?5, issue_id = ?6, outcome = ?7, changed = ?8 WHERE id = ?1", params![id, struggle.what, struggle.why, struggle.kind, topic, issue, outcome, struggle.changed])?;
                writer.replaced += 1;
            }
            Fate::Keep(_) => {}
        }
    }

    for setting in &snapshot.settings {
        let here: Option<String> = tx.query_row("SELECT changed FROM settings WHERE key = ?1", [&setting.key], |row| row.get(0)).optional()?.flatten();
        let take = here.is_none() || plan.theirs.contains(&format!("settings:{}", setting.key));
        if take {
            tx.execute("INSERT INTO settings (key, value, changed) VALUES (?1, ?2, ?3) ON CONFLICT(key) DO UPDATE SET value = excluded.value, changed = excluded.changed", params![setting.key, setting.value, setting.changed])?;
            if here.is_none() {
                writer.count("settings");
            } else {
                writer.replaced += 1;
            }
        }
    }
    for done in &snapshot.suggestions_done {
        let Some(key) = writer.local.key(&done.key) else { continue };
        tx.execute("INSERT OR IGNORE INTO card_suggestions_done (key, status, changed) VALUES (?1, ?2, ?3)", params![key, done.status, done.changed])?;
    }

    let order = ["modules", "topics", "sources", "images", "questions", "notes", "cards", "card reviews", "checklists", "recall checks", "quizzes", "projects", "sessions", "issues", "drills", "drill runs", "struggles", "settings"];
    Ok(Added { counts: order.iter().filter_map(|kind| writer.added.get(kind).map(|count| (kind.to_string(), *count))).collect(), replaced: writer.replaced })
}

/// A topic both sides have: the side quizzed more recently keeps its quiz step (R117).
fn later_quiz_step(tx: &Transaction, id: i64, theirs: Option<&QuizStep>) -> Result<()> {
    let Some(theirs) = theirs else { return Ok(()) };
    let mine: Option<i64> = tx.query_row("SELECT due FROM topic_schedule WHERE topic_id = ?1", [id], |row| row.get(0)).optional()?;
    if mine.is_none_or(|mine| theirs.due > mine) {
        tx.execute("INSERT INTO topic_schedule (topic_id, step, rep, due) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(topic_id) DO UPDATE SET step = excluded.step, rep = excluded.rep, due = excluded.due", params![id, theirs.step, theirs.rep, theirs.due])?;
    }
    Ok(())
}

/// A checklist both sides have: the side checked more recently keeps its step (R117).
fn later_checklist_step(tx: &Transaction, id: i64, uid: &str, snapshot: &Snapshot, theirs: Option<&ChecklistStep>) -> Result<()> {
    let Some(theirs) = theirs else { return Ok(()) };
    let mine_last: Option<i64> = tx.query_row("SELECT MAX(day) FROM recall_checks WHERE checklist_id = ?1", [id], |row| row.get(0))?;
    let theirs_last = snapshot.checks.iter().filter(|check| check.checklist == uid).map(|check| check.day).max();
    if theirs_last > mine_last {
        tx.execute("UPDATE recall_checklists SET step = ?2, rep = ?3, due = ?4 WHERE id = ?1", params![id, theirs.step, theirs.rep, theirs.due])?;
    }
    Ok(())
}

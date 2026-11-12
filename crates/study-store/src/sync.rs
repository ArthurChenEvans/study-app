//! Sync through a folder (R119 to R128): the part that lives in the database.
//!
//! Each computer keeps a "shadow" of every record as it was last synced, flattened into
//! fields, with who changed each field and when (a [`Tick`]). Syncing is then:
//!
//! 1. **Collect.** Compare the database with the shadow and write each difference as a
//!    numbered change line (only the fields that changed), into the outbox. The folder
//!    side copies the outbox into this computer's change file.
//! 2. **Merge.** Read the other computers' new lines and merge them into the shadow field
//!    by field. A line says which of every computer's lines it had seen, so two changes to
//!    the same field are either one after the other (the later one wins quietly) or made
//!    at the same time (the newer stamp wins and the pair is listed to check).
//! 3. **Write.** Put the merged records back in the database with the import's writer,
//!    hold back any that refer to something that hasn't arrived, and delete what was
//!    deleted.
//!
//! The shadow always matches the database after a sync, so writing merged records never
//! looks like a new local change, and syncing again writes nothing.

use std::collections::{BTreeMap, HashMap, HashSet};

use base64::Engine;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use study_core::cards::Schedule;

use crate::portable::{Plan, Scope, Snapshot, ToLocal};
use crate::{Result, Store, StoreError};

/// Which of each computer's lines a line had seen, by device ID.
pub type Seen = BTreeMap<String, u64>;

/// Every kind of record, in the order they have to be written (parents first).
pub const KINDS: [&str; 20] = [
    "modules", "topics", "sources", "images", "questions", "notes", "cards", "reviews", "checklists", "checks", "attempts", "projects", "sessions", "issues", "drills", "drill_runs", "struggles", "dates", "settings", "suggestions_done",
];

/// Fields worked out from history. The newer one is used without being listed to check,
/// and a card reviewed on two computers at once is worked out again (R125).
const SCHEDULES: [(&str, &str); 4] = [("topics", "schedule"), ("cards", "schedule"), ("checklists", "schedule"), ("drills", "due")];

/// A card's own copy of what its note says. A clash there is shown once, on the note.
const FROM_NOTE: [&str; 3] = ["key", "front", "back"];

/// The field a deletion's tick is kept under, and a deletion's "field" in things to check.
pub const DELETED: &str = "~del";
/// The tick of a record as first seen, for fields it hasn't changed since.
const CREATED: &str = "*";

fn kind_index(kind: &str) -> usize {
    KINDS.iter().position(|known| *known == kind).unwrap_or(KINDS.len())
}

fn known_kind(kind: &str) -> Option<&'static str> {
    KINDS.iter().copied().find(|known| *known == kind)
}

fn is_schedule(kind: &str, field: &str) -> bool {
    SCHEDULES.contains(&(kind, field))
}

pub fn table_of(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "reviews" => "card_reviews",
        "checklists" => "recall_checklists",
        "checks" => "recall_checks",
        "sessions" => "project_sessions",
        "settings" | "suggestions_done" => return None,
        other => known_kind(other)?,
    })
}

/// One change, as a line in a change file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub seq: u64,
    pub at: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub seen: Seen,
    pub kind: String,
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub del: bool,
}

/// Who changed a field and when: the line's stamp, its computer and its number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick(pub String, pub String, pub u64);

impl Tick {
    fn at(&self) -> &str {
        &self.0
    }
    fn by(&self) -> &str {
        &self.1
    }
    fn seq(&self) -> u64 {
        self.2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Here = 0,
    Deleted = 1,
    /// Matched to a record here with another ID (the same image, an issue with the same
    /// name), so it isn't written or deleted on its own.
    Alias = 2,
    /// Refers to something that hasn't arrived yet (R126).
    Waiting = 3,
}

impl State {
    fn from(value: i64) -> Self {
        match value {
            1 => Self::Deleted,
            2 => Self::Alias,
            3 => Self::Waiting,
            _ => Self::Here,
        }
    }
}

#[derive(Debug, Clone)]
struct Row {
    body: Option<Map<String, Value>>,
    clocks: BTreeMap<String, Tick>,
    state: State,
    dirty: bool,
}

impl Row {
    fn clock(&self, field: &str) -> Option<&Tick> {
        self.clocks.get(field).or_else(|| self.clocks.get(CREATED))
    }

    /// When the record was last edited: its newest field, leaving out schedules, which
    /// move with reviews and checks rather than edits (R110).
    fn changed(&self, kind: &str) -> String {
        let Some(body) = &self.body else {
            return self.clocks.iter().filter(|(field, _)| field.as_str() != DELETED && !is_schedule(kind, field)).map(|(_, tick)| tick.at()).max().unwrap_or_default().to_string();
        };
        // Each field's own tick, or the one it was made with.
        body.keys().filter(|field| !is_schedule(kind, field)).filter_map(|field| self.clock(field)).map(Tick::at).max().unwrap_or_default().to_string()
    }
}

type Key = (String, String);

fn load_shadow(connection: &Connection) -> Result<HashMap<Key, Row>> {
    let mut statement = connection.prepare("SELECT kind, uid, body, clocks, state FROM sync_shadow")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?)))?;
    let mut shadow = HashMap::new();
    for row in rows {
        let (kind, uid, body, clocks, state) = row?;
        let body = body.map(|body| serde_json::from_str(&body)).transpose()?;
        shadow.insert((kind, uid), Row { body, clocks: serde_json::from_str(&clocks)?, state: State::from(state), dirty: false });
    }
    Ok(shadow)
}

fn save_shadow(tx: &Transaction, shadow: &HashMap<Key, Row>) -> Result<()> {
    let mut statement = tx.prepare("INSERT OR REPLACE INTO sync_shadow (kind, uid, body, clocks, state) VALUES (?1, ?2, ?3, ?4, ?5)")?;
    for ((kind, uid), row) in shadow.iter().filter(|(_, row)| row.dirty) {
        let body = row.body.as_ref().map(serde_json::to_string).transpose()?;
        statement.execute(params![kind, uid, body, serde_json::to_string(&row.clocks)?, row.state as i64])?;
    }
    Ok(())
}

fn meta(connection: &Connection, key: &str) -> Result<Option<String>> {
    Ok(connection.query_row("SELECT value FROM sync_meta WHERE key = ?1", [key], |row| row.get(0)).optional()?)
}

fn set_meta(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection.execute("INSERT INTO sync_meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value])?;
    Ok(())
}

fn last_seq(connection: &Connection) -> Result<u64> {
    let outbox: i64 = connection.query_row("SELECT coalesce(MAX(seq), 0) FROM sync_outbox", [], |row| row.get(0))?;
    let floor: u64 = meta(connection, "seq_floor")?.and_then(|value| value.parse().ok()).unwrap_or(0);
    Ok((outbox as u64).max(floor))
}

fn applied(connection: &Connection) -> Result<HashMap<String, u64>> {
    let mut statement = connection.prepare("SELECT device, seq FROM sync_peers")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn save_applied(tx: &Transaction, device: &str, seq: u64) -> Result<()> {
    tx.execute("INSERT INTO sync_peers (device, seq) VALUES (?1, ?2) ON CONFLICT(device) DO UPDATE SET seq = excluded.seq", params![device, seq as i64])?;
    Ok(())
}

// ---------- Records as fields ----------

/// A record from the database, as fields.
struct Flat {
    kind: &'static str,
    uid: String,
    changed: String,
    body: Map<String, Value>,
}

/// Objects whose fields are merged one by one rather than as a whole: a note's front and
/// back can change on different computers (R123).
fn spread(kind: &str) -> Option<&'static str> {
    (kind == "notes").then_some("note")
}

fn id_field(kind: &str) -> &'static str {
    if matches!(kind, "settings" | "suggestions_done") { "key" } else { "uid" }
}

fn flatten(snapshot: &Snapshot) -> Result<Vec<Flat>> {
    let Value::Object(mut all) = serde_json::to_value(snapshot)? else { return Err(StoreError::Invalid("couldn't read the database for syncing")) };
    let mut cards = vec![];
    let mut flats = vec![];
    for kind in KINDS {
        let items = if kind == "cards" { std::mem::take(&mut cards) } else { all.remove(kind).and_then(|items| if let Value::Array(items) = items { Some(items) } else { None }).unwrap_or_default() };
        for (index, item) in items.into_iter().enumerate() {
            let Value::Object(mut body) = item else { continue };
            let Some(uid) = body.remove(id_field(kind)).and_then(|uid| uid.as_str().map(String::from)) else { continue };
            let changed = body.remove("changed").and_then(|changed| changed.as_str().map(String::from)).unwrap_or_default();
            if kind == "notes"
                && let Some(Value::Array(note_cards)) = body.remove("cards")
            {
                for card in note_cards {
                    if let Value::Object(mut card) = card {
                        card.insert("note".into(), Value::from(uid.clone()));
                        cards.push(Value::Object(card));
                    }
                }
            }
            if let Some(field) = spread(kind)
                && let Some(Value::Object(parts)) = body.remove(field)
            {
                for (part, value) in parts {
                    body.insert(format!("{field}.{part}"), value);
                }
            }
            if kind == "images"
                && let Some(image) = snapshot.images.get(index)
            {
                body.insert("bytes".into(), Value::from(base64::engine::general_purpose::STANDARD.encode(&image.bytes)));
            }
            flats.push(Flat { kind, uid, changed, body });
        }
    }
    Ok(flats)
}

/// Records as fields back into a snapshot the import's writer can put in the database.
/// Cards go inside their notes, so every card's note has to be among `records`.
fn unflatten(records: &[(String, String, String, Map<String, Value>)]) -> Result<Snapshot> {
    let mut all: Map<String, Value> = KINDS.iter().filter(|kind| **kind != "cards").map(|kind| (kind.to_string(), Value::Array(vec![]))).collect();
    let mut cards: HashMap<String, Vec<Value>> = HashMap::new();
    let mut images: HashMap<String, Vec<u8>> = HashMap::new();
    for (kind, uid, changed, body) in records {
        let mut body = body.clone();
        if let Some(field) = spread(kind) {
            let prefix = format!("{field}.");
            let parts: Map<String, Value> = body.iter().filter_map(|(key, value)| key.strip_prefix(&prefix).map(|part| (part.to_string(), value.clone()))).collect();
            body.retain(|key, _| !key.starts_with(&prefix));
            body.insert(field.into(), Value::Object(parts));
        }
        body.insert(id_field(kind).into(), Value::from(uid.clone()));
        body.insert("changed".into(), Value::from(changed.clone()));
        if kind == "images"
            && let Some(Value::String(bytes)) = body.remove("bytes")
        {
            images.insert(uid.clone(), base64::engine::general_purpose::STANDARD.decode(bytes).map_err(|_| StoreError::Invalid("an image in a change couldn't be read"))?);
        }
        if kind == "cards" {
            let note = body.remove("note").and_then(|note| note.as_str().map(String::from)).unwrap_or_default();
            cards.entry(note).or_default().push(Value::Object(body));
            continue;
        }
        if let Some(Value::Array(items)) = all.get_mut(kind.as_str()) {
            items.push(Value::Object(body));
        }
    }
    if let Some(Value::Array(notes)) = all.get_mut("notes") {
        for note in notes.iter_mut() {
            let uid = note.get("uid").and_then(Value::as_str).unwrap_or_default().to_string();
            note["cards"] = Value::Array(cards.remove(&uid).unwrap_or_default());
        }
    }
    let mut snapshot: Snapshot = serde_json::from_value(Value::Object(all))?;
    for image in &mut snapshot.images {
        image.bytes = images.remove(&image.uid).unwrap_or_default();
    }
    Ok(snapshot)
}

// ---------- References ----------

/// What a record refers to: (kind, ID, whether deleting it would take this record with it
/// or be blocked by it).
fn refs_of(kind: &str, body: &Map<String, Value>) -> Vec<(&'static str, String, bool)> {
    let mut refs = vec![];
    let one = |field: &str, target: &'static str, holds: bool, refs: &mut Vec<(&'static str, String, bool)>| {
        if let Some(uid) = body.get(field).and_then(Value::as_str) {
            refs.push((target, uid.to_string(), holds));
        }
    };
    let each = |field: &str, part: &str, target: &'static str, holds: bool, refs: &mut Vec<(&'static str, String, bool)>| {
        if let Some(Value::Array(items)) = body.get(field) {
            for item in items {
                if let Some(uid) = item.get(part).and_then(Value::as_str) {
                    refs.push((target, uid.to_string(), holds));
                }
            }
        }
    };
    match kind {
        "topics" => one("module", "modules", true, &mut refs),
        "questions" => {
            one("topic", "topics", true, &mut refs);
            each("citations", "source", "sources", true, &mut refs);
        }
        "notes" => one("topic", "topics", true, &mut refs),
        "cards" => one("note", "notes", true, &mut refs),
        "reviews" => one("card", "cards", true, &mut refs),
        "checklists" => {
            one("module", "modules", false, &mut refs);
            one("topic", "topics", false, &mut refs);
            each("points", "question", "questions", false, &mut refs);
            each("points", "note", "notes", false, &mut refs);
        }
        "checks" => one("checklist", "checklists", true, &mut refs),
        "attempts" => {
            one("retry_of", "attempts", false, &mut refs);
            each("moves", "topic", "topics", false, &mut refs);
        }
        "sessions" => one("project", "projects", true, &mut refs),
        "drills" => {
            one("issue", "issues", false, &mut refs);
            one("topic", "topics", false, &mut refs);
        }
        "drill_runs" => one("drill", "drills", true, &mut refs),
        "dates" => {
            one("module", "modules", false, &mut refs);
            if let Some(Value::Array(topics)) = body.get("topics") {
                refs.extend(topics.iter().filter_map(Value::as_str).map(|uid| ("topics", uid.to_string(), false)));
            }
        }
        "struggles" => {
            one("project", "projects", true, &mut refs);
            one("session", "sessions", false, &mut refs);
            one("topic", "topics", false, &mut refs);
            one("issue", "issues", false, &mut refs);
        }
        _ => {}
    }
    refs
}

// ---------- Before and after ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    /// The line was made after it had seen the field's current value.
    LineAfter,
    /// The field's current value was made after it had seen the line.
    ClockAfter,
    /// Neither had seen the other.
    Together,
}

/// Which lines each line had seen, read as needed.
struct Seens<'a> {
    connection: &'a Connection,
    cache: HashMap<(String, u64), Seen>,
}

impl Seens<'_> {
    fn of(&mut self, device: &str, seq: u64) -> Result<Seen> {
        if let Some(seen) = self.cache.get(&(device.to_string(), seq)) {
            return Ok(seen.clone());
        }
        let json: Option<String> = self.connection.query_row("SELECT seen FROM sync_lines WHERE device = ?1 AND seq = ?2", params![device, seq as i64], |row| row.get(0)).optional()?;
        let seen: Seen = json.map(|json| serde_json::from_str(&json)).transpose()?.unwrap_or_default();
        self.cache.insert((device.to_string(), seq), seen.clone());
        Ok(seen)
    }

    fn order(&mut self, clock: &Tick, by: &str, line: &Line) -> Result<Order> {
        if clock.by() == by {
            return Ok(if line.seq > clock.seq() { Order::LineAfter } else { Order::ClockAfter });
        }
        if line.seen.get(clock.by()).copied().unwrap_or(0) >= clock.seq() {
            return Ok(Order::LineAfter);
        }
        if self.of(clock.by(), clock.seq())?.get(by).copied().unwrap_or(0) >= line.seq {
            return Ok(Order::ClockAfter);
        }
        Ok(Order::Together)
    }
}

/// A pair of changes to check (R123), or a delete that lost to a change (R124).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conflict {
    pub n: i64,
    pub kind: String,
    pub uid: String,
    /// The field, or [`DELETED`] for a delete that was undone.
    pub field: String,
    pub kept: Option<Value>,
    pub other: Option<Value>,
    pub kept_by: String,
    pub kept_at: String,
    pub other_by: String,
    pub other_at: String,
}

/// What a merge did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merged {
    /// Lines merged.
    pub changes: usize,
    /// Lines of a kind this version doesn't know.
    pub skipped: usize,
    pub conflicts: usize,
    /// Records waiting for something they refer to.
    pub waiting: usize,
    /// Cards reviewed on two computers at once, to work out again.
    pub replay: Vec<String>,
}

/// How far a computer's lines have been read: the last line number merged, and the file
/// and offset the next read starts from.
pub type ReadSoFar = (u64, Option<String>, u64);

/// Where the next read of a computer's change files starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub device: String,
    pub file: String,
    pub offset: u64,
}

struct Merge<'a> {
    me: String,
    shadow: HashMap<Key, Row>,
    seens: Seens<'a>,
    conflicts: Vec<Conflict>,
    settled: Vec<(String, String, Option<String>)>,
    touched: HashSet<Key>,
    deleted: Vec<Key>,
    replay: HashSet<String>,
    /// Records by what they refer to, for deletes (R124).
    children: HashMap<String, Vec<Key>>,
}

impl Merge<'_> {
    fn conflict(&mut self, kind: &str, uid: &str, field: &str, kept: (Option<Value>, &Tick), other: (Option<Value>, &Tick)) {
        self.conflicts.push(Conflict { n: 0, kind: kind.into(), uid: uid.into(), field: field.into(), kept: kept.0, other: other.0, kept_by: kept.1.by().into(), kept_at: kept.1.at().into(), other_by: other.1.by().into(), other_at: other.1.at().into() });
    }

    fn index(&mut self, key: &Key) {
        if let Some(body) = self.shadow.get(key).and_then(|row| row.body.as_ref()) {
            for (_, target, holds) in refs_of(&key.0, body) {
                if holds {
                    self.children.entry(target).or_default().push(key.clone());
                }
            }
        }
    }

    /// Records here, or waiting, that would go with `uid` or block deleting it.
    fn has_children(&self, uid: &str) -> bool {
        self.children.get(uid).is_some_and(|keys| {
            keys.iter().any(|key| {
                self.shadow.get(key).is_some_and(|row| matches!(row.state, State::Here | State::Waiting) && row.body.as_ref().is_some_and(|body| refs_of(&key.0, body).iter().any(|(_, target, holds)| *holds && target == uid)))
            })
        })
    }

    fn set(&mut self, kind: &'static str, by: &str, line: &Line, set: &Map<String, Value>) -> Result<()> {
        let key = (kind.to_string(), line.uid.clone());
        let tick = Tick(line.at.clone(), by.to_string(), line.seq);
        let Some(row) = self.shadow.get(&key) else {
            self.shadow.insert(key.clone(), Row { body: Some(set.clone()), clocks: [(CREATED.to_string(), tick)].into(), state: State::Here, dirty: true });
            self.index(&key);
            self.touched.insert(key);
            return Ok(());
        };
        let mut row = row.clone();
        let mut take_all = false;
        if row.state == State::Deleted {
            let Some(deletion) = row.clocks.get(DELETED).cloned() else { return Ok(()) };
            match self.seens.order(&deletion, by, line)? {
                // Deleted after this change was made: it stays deleted, but its fields are kept
                // in case something added to it on another computer brings it back (R124).
                Order::ClockAfter => {
                    let body = row.body.get_or_insert_with(Map::new);
                    for (field, value) in set {
                        let newer = row.clocks.get(field.as_str()).is_none_or(|clock| line.at.as_str() > clock.at());
                        if newer || !body.contains_key(field) {
                            body.insert(field.clone(), value.clone());
                            row.clocks.insert(field.clone(), tick.clone());
                        }
                    }
                    if !row.clocks.contains_key(CREATED) {
                        row.clocks.insert(CREATED.to_string(), tick.clone());
                    }
                    row.dirty = true;
                    self.shadow.insert(key, row);
                    return Ok(());
                }
                order => {
                    // Changed after or alongside the delete: it comes back (R124).
                    if order == Order::Together {
                        self.conflict(kind, &line.uid, DELETED, (None, &tick), (None, &deletion));
                    }
                    row.state = State::Here;
                    row.clocks.remove(DELETED);
                    take_all = true;
                }
            }
        }
        let mut body = row.body.take().unwrap_or_default();
        for (field, value) in set {
            let clock = row.clock(field).cloned();
            let order = match &clock {
                None => Order::LineAfter,
                Some(_) if take_all => Order::LineAfter,
                // A field it doesn't have yet: an edit arrived before the record was made.
                Some(_) if !body.contains_key(field) => Order::LineAfter,
                Some(clock) => self.seens.order(clock, by, line)?,
            };
            let take = match order {
                Order::LineAfter => true,
                Order::ClockAfter => false,
                Order::Together => clock.as_ref().is_none_or(|clock| line.at.as_str() > clock.at()),
            };
            let differs = body.get(field) != Some(value);
            if order == Order::Together && differs
                && let Some(clock) = &clock
            {
                if is_schedule(kind, field) {
                    if kind == "cards" {
                        self.replay.insert(line.uid.clone());
                    }
                } else if !(kind == "cards" && FROM_NOTE.contains(&field.as_str())) {
                    let mine = (body.get(field).cloned(), clock);
                    let theirs = (Some(value.clone()), &tick);
                    if take { self.conflict(kind, &line.uid, field, theirs, mine) } else { self.conflict(kind, &line.uid, field, mine, theirs) }
                }
            }
            if order == Order::LineAfter {
                self.settled.push((kind.to_string(), line.uid.clone(), Some(field.clone())));
            }
            if take {
                body.insert(field.clone(), value.clone());
                row.clocks.insert(field.clone(), tick.clone());
            }
        }
        row.body = Some(body);
        row.dirty = true;
        self.shadow.insert(key.clone(), row);
        self.index(&key);
        self.touched.insert(key);
        Ok(())
    }

    fn delete(&mut self, kind: &'static str, by: &str, line: &Line) -> Result<()> {
        let key = (kind.to_string(), line.uid.clone());
        let tick = Tick(line.at.clone(), by.to_string(), line.seq);
        let Some(row) = self.shadow.get(&key).cloned() else {
            self.shadow.insert(key, Row { body: None, clocks: [(DELETED.to_string(), tick)].into(), state: State::Deleted, dirty: true });
            return Ok(());
        };
        if row.state == State::Deleted {
            return Ok(());
        }
        // Changed, or added to, here before the deleting computer saw it: keep it (R124).
        let mut changed_since = false;
        for (field, clock) in &row.clocks {
            if field != DELETED && self.seens.order(clock, by, line)? == Order::Together {
                changed_since = true;
            }
        }
        if (changed_since || self.has_children(&line.uid)) && row.state != State::Alias {
            let newest = row.clocks.values().max_by(|left, right| left.at().cmp(right.at())).cloned().unwrap_or_else(|| Tick(String::new(), self.me.clone(), 0));
            self.conflict(kind, &line.uid, DELETED, (None, &newest), (None, &tick));
            return Ok(());
        }
        let mut row = row;
        row.state = State::Deleted;
        row.clocks.insert(DELETED.to_string(), tick);
        row.dirty = true;
        self.shadow.insert(key.clone(), row);
        self.settled.push((kind.to_string(), line.uid.clone(), None));
        self.touched.remove(&key);
        self.deleted.push(key);
        Ok(())
    }
}

/// Deletes a record here, with what goes with it.
fn delete_here(tx: &Transaction, kind: &str, uid: &str) -> Result<()> {
    match kind {
        "settings" => {
            tx.execute("DELETE FROM settings WHERE key = ?1", [uid])?;
        }
        "suggestions_done" => {
            if let Some(key) = ToLocal::read(tx)?.key(uid) {
                tx.execute("DELETE FROM card_suggestions_done WHERE key = ?1", [key])?;
            }
        }
        _ => {
            if let Some(table) = table_of(kind) {
                tx.execute(&format!("DELETE FROM {table} WHERE uid = ?1"), [uid])?;
            }
        }
    }
    Ok(())
}

/// Whether a record has every field it needs. One only heard about through an edit, with
/// its creation still on its way, doesn't yet.
fn complete(kind: &str, uid: &str, body: &Map<String, Value>) -> bool {
    if kind == "cards" {
        let mut card = body.clone();
        card.remove("note");
        card.insert("uid".into(), Value::from(uid));
        card.insert("changed".into(), Value::from(""));
        return body.contains_key("note") && serde_json::from_value::<crate::portable::Card>(Value::Object(card)).is_ok();
    }
    unflatten(&[(kind.to_string(), uid.to_string(), String::new(), body.clone())]).is_ok()
}

/// Writes records from the shadow into the database: each card with its note and all the
/// note's other cards, since the writer puts a note's cards back as a set.
fn write_records(tx: &Transaction, shadow: &HashMap<Key, Row>, keys: &HashSet<Key>, today: i64) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let mut notes: HashSet<String> = HashSet::new();
    for (kind, uid) in keys {
        match kind.as_str() {
            "notes" => {
                notes.insert(uid.clone());
            }
            "cards" => {
                if let Some(note) = shadow.get(&(kind.clone(), uid.clone())).and_then(|row| row.body.as_ref()).and_then(|body| body.get("note")).and_then(Value::as_str) {
                    notes.insert(note.to_string());
                }
            }
            _ => {}
        }
    }
    let mut wanted: Vec<Key> = keys.iter().filter(|(kind, _)| kind != "cards" && kind != "notes").cloned().collect();
    wanted.extend(notes.iter().map(|note| ("notes".to_string(), note.clone())));
    for ((kind, uid), row) in shadow {
        if kind == "cards" && row.state == State::Here && row.body.as_ref().and_then(|body| body.get("note")).and_then(Value::as_str).is_some_and(|note| notes.contains(note)) {
            wanted.push((kind.clone(), uid.clone()));
        }
    }
    wanted.sort_by_key(|(kind, _)| kind_index(kind));
    let records: Vec<(String, String, String, Map<String, Value>)> = wanted
        .iter()
        .filter_map(|key| shadow.get(key).filter(|row| row.state == State::Here).and_then(|row| row.body.clone().map(|body| (key.0.clone(), key.1.clone(), row.changed(&key.0), body))))
        .collect();
    // Writing a stamp equal to the one already there counts as an edit to the triggers,
    // which would give the record a new stamp, so clear it first.
    for (kind, uid, _, _) in &records {
        match table_of(kind) {
            Some(table) => tx.execute(&format!("UPDATE {table} SET changed = NULL WHERE uid = ?1"), [uid])?,
            None if kind == "settings" => tx.execute("UPDATE settings SET changed = NULL WHERE key = ?1", [uid])?,
            None => 0,
        };
    }
    let snapshot = unflatten(&records)?;
    let theirs: HashSet<String> = records.iter().map(|(kind, uid, _, _)| if kind == "settings" { format!("settings:{uid}") } else { uid.clone() }).collect();
    crate::portable::write(tx, &snapshot, &Plan { theirs, today, history_edits: true })?;

    // The writer only moves a schedule forward, sync sets it to the merged one.
    for (kind, uid, _, body) in &records {
        let schedule = body.get("schedule").filter(|schedule| !schedule.is_null());
        match (kind.as_str(), schedule) {
            ("topics", None) => {
                tx.execute("DELETE FROM topic_schedule WHERE topic_id = (SELECT id FROM topics WHERE uid = ?1)", [uid])?;
            }
            ("topics", Some(step)) => {
                tx.execute(
                    "INSERT INTO topic_schedule (topic_id, step, rep, due) SELECT id, ?2, ?3, ?4 FROM topics WHERE uid = ?1 ON CONFLICT(topic_id) DO UPDATE SET step = excluded.step, rep = excluded.rep, due = excluded.due",
                    params![uid, step["step"].as_i64(), step["rep"].as_i64(), step["due"].as_i64()],
                )?;
            }
            ("cards", Some(card)) => {
                tx.execute(
                    "UPDATE cards SET stability = ?2, difficulty = ?3, due = ?4, last_review = ?5, reps = ?6, lapses = ?7 WHERE uid = ?1",
                    params![uid, card["stability"].as_f64(), card["difficulty"].as_f64(), card["due"].as_i64(), card["last_review"].as_i64(), card["reps"].as_i64(), card["lapses"].as_i64()],
                )?;
            }
            ("checklists", Some(step)) => {
                tx.execute("UPDATE recall_checklists SET step = ?2, rep = ?3, due = ?4 WHERE uid = ?1", params![uid, step["step"].as_i64(), step["rep"].as_i64(), step["due"].as_i64()])?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_here(tx: &Transaction, kind: &str, uid: &str) -> Result<bool> {
    let Some(table) = table_of(kind) else { return Ok(true) };
    Ok(tx.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE uid = ?1"), [uid], |row| row.get::<_, i64>(0))? > 0)
}

impl Store {
    /// A cheap summary of everything that can change, so a sync with nothing new here
    /// doesn't have to read the whole database.
    fn fingerprint(&self) -> Result<String> {
        let mut parts = vec![];
        for table in crate::ids::STAMPED {
            parts.push(format!("SELECT COUNT(*) || ':' || ifnull(MAX(changed), '') FROM {table}"));
        }
        parts.push("SELECT COUNT(*) || ':' || ifnull(MAX(changed), '') FROM settings".into());
        parts.push("SELECT COUNT(*) || ':' || ifnull(MAX(changed), '') FROM card_suggestions_done".into());
        parts.push("SELECT COUNT(*) || ':' || total(step) || ':' || total(rep) || ':' || total(due) FROM topic_schedule".into());
        parts.push("SELECT total(due) || ':' || total(reps) || ':' || total(lapses) || ':' || total(stability) FROM cards".into());
        parts.push("SELECT total(step) || ':' || total(rep) || ':' || total(due) FROM recall_checklists".into());
        parts.push("SELECT total(due) FROM drills".into());
        parts.push("SELECT ifnull(MAX(at), '') FROM deleted".into());
        let mut out = String::new();
        for sql in parts {
            out.push_str(&format!("{:?}", self.connection.query_row(&sql, [], |row| row.get::<_, rusqlite::types::Value>(0))?));
            out.push('|');
        }
        Ok(out)
    }

    fn fresh_stamp(&self) -> String {
        self.clock.lock().map(|mut clock| clock.stamp()).unwrap_or_default()
    }

    /// Never number a line below `seq`: this computer's lines already in the folder, which
    /// can be ahead of the database after a backup is restored.
    pub fn sync_seq_floor(&mut self, seq: u64) -> Result<()> {
        if seq > last_seq(&self.connection)? {
            set_meta(&self.connection, "seq_floor", &seq.to_string())?;
        }
        Ok(())
    }

    /// The number of this computer's newest line.
    pub fn sync_last_seq(&self) -> Result<u64> {
        last_seq(&self.connection)
    }

    /// This computer's lines after `seq`, as JSON, oldest first.
    pub fn sync_outbox(&self, after: u64) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare("SELECT line FROM sync_outbox WHERE seq > ?1 ORDER BY seq")?;
        let rows = statement.query_map([after as i64], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn sync_meta(&self, key: &str) -> Result<Option<String>> {
        meta(&self.connection, key)
    }

    pub fn set_sync_meta(&mut self, key: &str, value: &str) -> Result<()> {
        set_meta(&self.connection, key, value)
    }

    /// How far each computer's lines have been read: (line number, file, offset in it).
    pub fn sync_cursors(&self) -> Result<HashMap<String, ReadSoFar>> {
        let mut statement = self.connection.prepare("SELECT device, seq, file, offset FROM sync_peers")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, (row.get::<_, i64>(1)? as u64, row.get::<_, Option<String>>(2)?, row.get::<_, i64>(3)? as u64))))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_sync_cursor(&mut self, cursor: &Cursor) -> Result<()> {
        self.connection.execute(
            "INSERT INTO sync_peers (device, file, offset) VALUES (?1, ?2, ?3) ON CONFLICT(device) DO UPDATE SET file = excluded.file, offset = excluded.offset",
            params![cursor.device, cursor.file, cursor.offset as i64],
        )?;
        Ok(())
    }

    /// Writes what changed here since the last sync as lines in the outbox (R119). Returns
    /// how many.
    pub fn sync_collect(&mut self) -> Result<usize> {
        let fingerprint = self.fingerprint()?;
        if meta(&self.connection, "fingerprint")?.as_deref() == Some(fingerprint.as_str()) {
            return Ok(0);
        }
        let snapshot = self.snapshot(&Scope { topics: None, history: true })?;
        let flats = flatten(&snapshot)?;
        let me = self.device_id();
        let clock = std::sync::Arc::clone(&self.clock);
        let stamp = move || clock.lock().map(|mut clock| clock.stamp()).unwrap_or_default();
        let tx = self.connection.transaction()?;
        let mut shadow = load_shadow(&tx)?;
        let mut seen: Seen = applied(&tx)?.into_iter().filter(|(device, seq)| *device != me && *seq > 0).collect();
        seen.remove(&me);
        let mut seq = last_seq(&tx)?;
        let mut lines: Vec<Line> = vec![];
        let mut present: HashSet<Key> = HashSet::new();
        let mut settled: Vec<(String, String, Option<String>)> = vec![];

        for flat in flats {
            let key = (flat.kind.to_string(), flat.uid.clone());
            present.insert(key.clone());
            let row = shadow.get(&key);
            match row {
                Some(row) if row.state == State::Here => {
                    let old = row.body.clone().unwrap_or_default();
                    let mut content = Map::new();
                    let mut schedule = Map::new();
                    for (field, value) in &flat.body {
                        if old.get(field) != Some(value) {
                            if is_schedule(flat.kind, field) { &mut schedule } else { &mut content }.insert(field.clone(), value.clone());
                        }
                    }
                    for field in old.keys().filter(|field| !flat.body.contains_key(*field)) {
                        content.insert(field.clone(), Value::Null);
                    }
                    let newest = content.keys().filter_map(|field| row.clock(field)).map(|tick| tick.at().to_string()).max();
                    for (fields, fresh) in [(content, false), (schedule, true)] {
                        if fields.is_empty() {
                            continue;
                        }
                        // An edit keeps the time it was made, even if the folder was away.
                        let at = if !fresh && !flat.changed.is_empty() && newest.as_ref().is_none_or(|newest| flat.changed > *newest) { flat.changed.clone() } else { stamp() };
                        seq += 1;
                        let row = shadow.get_mut(&key).expect("row");
                        for (field, value) in &fields {
                            row.body.get_or_insert_with(Map::new).insert(field.clone(), value.clone());
                            row.clocks.insert(field.clone(), Tick(at.clone(), me.clone(), seq));
                            settled.push((key.0.clone(), key.1.clone(), Some(field.clone())));
                        }
                        row.dirty = true;
                        lines.push(Line { seq, at, seen: seen.clone(), kind: key.0.clone(), uid: key.1.clone(), set: Some(fields), del: false });
                    }
                }
                _ => {
                    // New here, or back after being deleted.
                    let at = if flat.changed.is_empty() { stamp() } else { flat.changed.clone() };
                    seq += 1;
                    shadow.insert(key.clone(), Row { body: Some(flat.body.clone()), clocks: [(CREATED.to_string(), Tick(at.clone(), me.clone(), seq))].into(), state: State::Here, dirty: true });
                    lines.push(Line { seq, at, seen: seen.clone(), kind: key.0.clone(), uid: key.1.clone(), set: Some(flat.body), del: false });
                }
            }
        }

        // Deleted here: what goes with a record is deleted first, so a computer reading the
        // lines in order never sees a parent go while its children are still there.
        let mut gone: Vec<Key> = shadow.iter().filter(|(key, row)| row.state == State::Here && !present.contains(*key)).map(|(key, _)| key.clone()).collect();
        gone.sort_by_key(|(kind, uid)| (std::cmp::Reverse(kind_index(kind)), uid.clone()));
        for key in gone {
            let at = stamp();
            seq += 1;
            let row = shadow.get_mut(&key).expect("row");
            row.state = State::Deleted;
            row.clocks.insert(DELETED.to_string(), Tick(at.clone(), me.clone(), seq));
            row.dirty = true;
            settled.push((key.0.clone(), key.1.clone(), None));
            lines.push(Line { seq, at, seen: seen.clone(), kind: key.0, uid: key.1, set: None, del: true });
        }

        {
            let mut outbox = tx.prepare("INSERT INTO sync_outbox (seq, line) VALUES (?1, ?2)")?;
            let mut seens = tx.prepare("INSERT OR REPLACE INTO sync_lines (device, seq, seen) VALUES (?1, ?2, ?3)")?;
            for line in &lines {
                outbox.execute(params![line.seq as i64, serde_json::to_string(line)?])?;
                seens.execute(params![me, line.seq as i64, serde_json::to_string(&line.seen)?])?;
            }
        }
        // Changing a field here settles a clash over it (R123).
        settle(&tx, &settled)?;
        save_shadow(&tx, &shadow)?;
        if seq > 0 {
            save_applied(&tx, &me, seq)?;
        }
        set_meta(&tx, "fingerprint", &fingerprint)?;
        tx.commit()?;
        Ok(lines.len())
    }

    /// Merges other computers' lines into this database (R122 to R126). `cursors` are
    /// saved with it, so a sync that fails part way reads the same lines again.
    pub fn sync_merge(&mut self, incoming: &[(String, Line)], cursors: &[Cursor], today: i64) -> Result<Merged> {
        let me = self.device_id();
        let clock = std::sync::Arc::clone(&self.clock);
        let tx = self.connection.transaction()?;
        let mut done = applied(&tx)?;
        let mut merged = Merged::default();
        let shadow = load_shadow(&tx)?;
        let mut merge = Merge { me: me.clone(), shadow, seens: Seens { connection: &tx, cache: HashMap::new() }, conflicts: vec![], settled: vec![], touched: HashSet::new(), deleted: vec![], replay: HashSet::new(), children: HashMap::new() };
        let keys: Vec<Key> = merge.shadow.keys().cloned().collect();
        for key in &keys {
            merge.index(key);
        }

        let mut seen_lines: Vec<(String, u64, String)> = vec![];
        for (device, line) in incoming {
            if line.seq <= done.get(device).copied().unwrap_or(0) {
                continue;
            }
            done.insert(device.clone(), line.seq);
            seen_lines.push((device.clone(), line.seq, serde_json::to_string(&line.seen)?));
            merge.seens.cache.insert((device.clone(), line.seq), line.seen.clone());
            if let Ok(mut clock) = clock.lock() {
                clock.catch_up(&line.at);
            }
            let Some(kind) = known_kind(&line.kind) else {
                merged.skipped += 1;
                continue;
            };
            merged.changes += 1;
            if line.del {
                merge.delete(kind, device, line)?;
            } else if let Some(set) = &line.set {
                merge.set(kind, device, line, set)?;
            }
        }

        // Records waiting from before are tried again, then everything is checked for what
        // it refers to (R126). A record waits until what it refers to is here.
        for (key, row) in &merge.shadow {
            if row.state == State::Waiting {
                merge.touched.insert(key.clone());
            }
        }
        let mut pending: Vec<Key> = merge.touched.iter().filter(|key| merge.shadow.get(*key).is_some_and(|row| matches!(row.state, State::Here | State::Waiting))).cloned().collect();
        pending.sort_by_key(|(kind, uid)| (kind_index(kind), uid.clone()));
        let mut ready: HashSet<Key> = HashSet::new();
        let mut waiting: HashSet<Key> = pending.iter().cloned().collect();
        loop {
            let mut progress = false;
            let mut still = vec![];
            for key in std::mem::take(&mut pending) {
                let refs = merge.shadow.get(&key).and_then(|row| row.body.as_ref()).map(|body| refs_of(&key.0, body)).unwrap_or_default();
                let mut blocked = !merge.shadow.get(&key).and_then(|row| row.body.as_ref()).is_some_and(|body| complete(&key.0, &key.1, body));
                for (target_kind, target, _) in refs {
                    let target_key = (target_kind.to_string(), target.clone());
                    let available = match merge.shadow.get(&target_key) {
                        Some(row) if row.state == State::Alias => true,
                        Some(row) if row.state == State::Here => !waiting.contains(&target_key),
                        Some(row) if row.state == State::Deleted && row.body.as_ref().is_some_and(|body| complete(target_kind, &target, body)) => {
                            // Something was added to a deleted record before its computer saw the
                            // delete: the record comes back (R124).
                            let mut row = row.clone();
                            row.state = State::Here;
                            let deletion = row.clocks.remove(DELETED);
                            row.dirty = true;
                            merge.shadow.insert(target_key.clone(), row);
                            if let Some(deletion) = deletion {
                                let added = merge.shadow.get(&key).and_then(|row| row.clocks.values().max_by(|left, right| left.at().cmp(right.at())).cloned()).unwrap_or_else(|| Tick(String::new(), me.clone(), 0));
                                merge.conflict(target_kind, &target, DELETED, (None, &added), (None, &deletion));
                            }
                            waiting.insert(target_key.clone());
                            still.push(target_key.clone());
                            merge.touched.insert(target_key.clone());
                            progress = true;
                            false
                        }
                        _ => false,
                    };
                    blocked |= !available;
                }
                if blocked {
                    still.push(key);
                } else {
                    waiting.remove(&key);
                    ready.insert(key);
                    progress = true;
                }
            }
            pending = still;
            if !progress {
                break;
            }
        }
        for key in &pending {
            if let Some(row) = merge.shadow.get_mut(key) {
                row.state = State::Waiting;
                row.dirty = true;
            }
        }
        for key in &ready {
            if let Some(row) = merge.shadow.get_mut(key)
                && row.state == State::Waiting
            {
                row.state = State::Here;
                row.dirty = true;
            }
        }

        write_records(&tx, &merge.shadow, &ready, today)?;
        // Records matched to one here under another ID aren't written on their own.
        for key in &ready {
            if !is_here(&tx, &key.0, &key.1)?
                && let Some(row) = merge.shadow.get_mut(key)
            {
                row.state = State::Alias;
                row.dirty = true;
            }
        }
        // Only what's still deleted: a later line in the same batch can bring a record back.
        let mut deleted: Vec<Key> = merge.deleted.iter().filter(|key| merge.shadow.get(*key).is_some_and(|row| row.state == State::Deleted)).cloned().collect();
        deleted.dedup();
        deleted.sort_by_key(|(kind, uid)| (std::cmp::Reverse(kind_index(kind)), uid.clone()));
        for (kind, uid) in &deleted {
            delete_here(&tx, kind, uid)?;
        }

        settle(&tx, &merge.settled)?;
        {
            let mut statement = tx.prepare(
                "INSERT INTO sync_conflicts (kind, uid, field, kept, other, kept_by, kept_at, other_by, other_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(kind, uid, field) DO UPDATE SET kept = excluded.kept, other = excluded.other, kept_by = excluded.kept_by, kept_at = excluded.kept_at, other_by = excluded.other_by, other_at = excluded.other_at",
            )?;
            for conflict in &merge.conflicts {
                let json = |value: &Option<Value>| value.as_ref().map(serde_json::to_string).transpose();
                statement.execute(params![conflict.kind, conflict.uid, conflict.field, json(&conflict.kept)?, json(&conflict.other)?, conflict.kept_by, conflict.kept_at, conflict.other_by, conflict.other_at])?;
            }
        }
        {
            let mut statement = tx.prepare("INSERT OR REPLACE INTO sync_lines (device, seq, seen) VALUES (?1, ?2, ?3)")?;
            for (device, seq, seen) in &seen_lines {
                statement.execute(params![device, *seq as i64, seen])?;
            }
        }
        save_shadow(&tx, &merge.shadow)?;
        for (device, seq) in &done {
            save_applied(&tx, device, *seq)?;
        }
        for cursor in cursors {
            tx.execute(
                "INSERT INTO sync_peers (device, file, offset) VALUES (?1, ?2, ?3) ON CONFLICT(device) DO UPDATE SET file = excluded.file, offset = excluded.offset",
                params![cursor.device, cursor.file, cursor.offset as i64],
            )?;
        }
        merged.conflicts = merge.conflicts.len();
        merged.waiting = merge.shadow.values().filter(|row| row.state == State::Waiting).count();
        let mut replay: Vec<String> = merge.replay.into_iter().collect();
        replay.sort();
        merged.replay = replay;
        drop(merge.seens);
        tx.commit()?;
        crate::ids::catch_up(&self.connection, &self.clock)?;
        // The database matches the shadow again, so the next collect has nothing to read.
        let fingerprint = self.fingerprint()?;
        set_meta(&self.connection, "fingerprint", &fingerprint)?;
        Ok(merged)
    }

    /// Works out each card's schedule again from all its reviews, oldest first (R125).
    pub fn sync_replay(&mut self, cards: &[String]) -> Result<()> {
        let settings = self.card_settings()?;
        let tx = self.connection.transaction()?;
        for uid in cards {
            let reviews: Vec<(i64, i64)> = tx
                .prepare("SELECT r.day, r.rating FROM card_reviews r JOIN cards c ON c.id = r.card_id WHERE c.uid = ?1 ORDER BY r.day, r.changed, r.uid")?
                .query_map([uid], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let Some((first, _)) = reviews.first() else { continue };
            let mut schedule = Schedule::new(*first);
            for (day, rating) in &reviews {
                schedule = settings.grade(&schedule, crate::cards::rating_from(*rating), *day).map_err(|_| StoreError::Invalid("couldn't work out a card's schedule"))?;
            }
            tx.execute(
                "UPDATE cards SET stability = ?2, difficulty = ?3, due = ?4, last_review = ?5, reps = ?6, lapses = ?7 WHERE uid = ?1",
                params![uid, schedule.memory.map(|memory| memory.stability as f64), schedule.memory.map(|memory| memory.difficulty as f64), schedule.due, schedule.last_review, schedule.reps, schedule.lapses],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Things to check, oldest first (R123, R124).
    pub fn sync_conflicts(&self) -> Result<Vec<Conflict>> {
        let mut statement = self.connection.prepare("SELECT n, kind, uid, field, kept, other, kept_by, kept_at, other_by, other_at FROM sync_conflicts ORDER BY n")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                [row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?, row.get::<_, String>(9)?],
            ))
        })?;
        let mut conflicts = vec![];
        for row in rows {
            let (n, kind, uid, field, kept, other, [kept_by, kept_at, other_by, other_at]) = row?;
            let parse = |json: Option<String>| json.map(|json| serde_json::from_str::<Value>(&json)).transpose();
            conflicts.push(Conflict { n, kind, uid, field, kept: parse(kept)?, other: parse(other)?, kept_by, kept_at, other_by, other_at });
        }
        Ok(conflicts)
    }

    pub fn sync_dismiss(&mut self, n: i64) -> Result<()> {
        self.connection.execute("DELETE FROM sync_conflicts WHERE n = ?1", [n])?;
        Ok(())
    }

    /// A record as last synced, as fields, if it's here.
    pub fn sync_record(&self, kind: &str, uid: &str) -> Result<Option<Map<String, Value>>> {
        let row: Option<(Option<String>, i64)> = self.connection.query_row("SELECT body, state FROM sync_shadow WHERE kind = ?1 AND uid = ?2", params![kind, uid], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
        Ok(match row {
            Some((Some(body), state)) if State::from(state) != State::Deleted => Some(serde_json::from_str(&body)?),
            _ => None,
        })
    }

    /// Puts the other version of a thing to check back here, as a new change, so it goes
    /// to the other computers too. For a delete that was undone, deletes it after all.
    pub fn sync_take_other(&mut self, n: i64, today: i64) -> Result<()> {
        let conflict = self.sync_conflicts()?.into_iter().find(|conflict| conflict.n == n).ok_or(StoreError::NotFound("thing to check"))?;
        let stamp = self.fresh_stamp();
        let tx = self.connection.transaction()?;
        if conflict.field == DELETED {
            delete_here(&tx, &conflict.kind, &conflict.uid)?;
        } else {
            let mut shadow = load_shadow(&tx)?;
            let key = (conflict.kind.clone(), conflict.uid.clone());
            let row = shadow.get_mut(&key).filter(|row| row.state == State::Here).ok_or(StoreError::NotFound("record"))?;
            row.body.get_or_insert_with(Map::new).insert(conflict.field.clone(), conflict.other.clone().unwrap_or(Value::Null));
            // A stamp from now, so the record counts as changed here and goes out.
            row.clocks.insert(conflict.field.clone(), Tick(stamp, String::new(), 0));
            let mut copy: HashMap<Key, Row> = shadow.clone();
            copy.retain(|_, row| row.state == State::Here);
            write_records(&tx, &copy, &[key].into(), today)?;
        }
        tx.execute("DELETE FROM sync_conflicts WHERE n = ?1", [n])?;
        tx.commit()?;
        Ok(())
    }

    /// The row number here of a record, by kind and ID.
    pub fn local_id(&self, kind: &str, uid: &str) -> Result<Option<i64>> {
        let Some(table) = table_of(kind) else { return Ok(None) };
        Ok(self.connection.query_row(&format!("SELECT id FROM {table} WHERE uid = ?1"), [uid], |row| row.get(0)).optional()?)
    }

    /// Lines merged from each computer, and how many records are waiting (R126).
    pub fn sync_waiting(&self) -> Result<usize> {
        Ok(self.connection.query_row("SELECT COUNT(*) FROM sync_shadow WHERE state = 3", [], |row| row.get::<_, i64>(0))? as usize)
    }

    pub fn sync_applied(&self) -> Result<HashMap<String, u64>> {
        applied(&self.connection)
    }
}

/// Clashes that a later change settled: a field, or every field of a deleted record.
fn settle(tx: &Transaction, settled: &[(String, String, Option<String>)]) -> Result<()> {
    let mut one = tx.prepare("DELETE FROM sync_conflicts WHERE kind = ?1 AND uid = ?2 AND field = ?3")?;
    let mut all = tx.prepare("DELETE FROM sync_conflicts WHERE kind = ?1 AND uid = ?2")?;
    for (kind, uid, field) in settled {
        match field {
            Some(field) => one.execute(params![kind, uid, field])?,
            None => all.execute(params![kind, uid])?,
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_come_back_the_same_after_being_split_into_fields() {
        let mut store = Store::in_memory().unwrap();
        let module = store.create_module("Networks").unwrap();
        let topic = store.create_topic("OSI model", Some(module)).unwrap();
        let _ = topic;
        let snapshot = store.snapshot(&Scope { topics: None, history: true }).unwrap();
        let flats = flatten(&snapshot).unwrap();
        let records: Vec<_> = flats.into_iter().map(|flat| (flat.kind.to_string(), flat.uid, flat.changed, flat.body)).collect();
        assert_eq!(unflatten(&records).unwrap(), snapshot);
    }

    #[test]
    fn a_line_says_only_what_it_needs() {
        let line = Line { seq: 3, at: "0001".into(), seen: Seen::new(), kind: "modules".into(), uid: "u".into(), set: None, del: true };
        assert_eq!(serde_json::to_string(&line).unwrap(), r#"{"seq":3,"at":"0001","kind":"modules","uid":"u","del":true}"#);
    }
}

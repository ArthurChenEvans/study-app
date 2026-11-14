//! import-export.feature: Anki decks in (R171, R172).
//!
//! An `.apkg` (a deck) or `.colpkg` (a whole collection) is a zip holding Anki's SQLite
//! database and its media. Newer Anki compresses the database and each media file with
//! zstd (`collection.anki21b`, media listed in a small protobuf); older Anki doesn't
//! (`collection.anki21` or `.anki2`, media listed as JSON). Both are read the same way into
//! a `Package`, shown as an `AnkiPreview`, then turned into the same records a `.studyapp`
//! import uses, so it gets the same backup, "already here" check and undo.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use study_core::cards::{Note as CardNote, NoteKind};
use study_store::portable::{self, CardSchedule, Snapshot};

use crate::markup::{Dropped, html_to_markdown};
use crate::practice::Clock;
use crate::{ApiError, ApiResult, App, ModuleId, TopicId};

const NOT_ANKI: &str = "that file isn't an Anki deck (.apkg) or collection (.colpkg)";
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
/// Deck names in newer Anki separate levels with this rather than `::`.
const LEVEL: char = '\u{1f}';
const IMAGE_TYPES: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

fn not_anki() -> ApiError {
    ApiError::new(NOT_ANKI)
}

// ---------- Reading the package ----------

#[derive(Debug, Clone)]
struct NoteType {
    name: String,
    cloze: bool,
    fields: usize,
    templates: usize,
}

#[derive(Debug, Clone)]
struct RawNote {
    guid: String,
    kind: i64,
    modified: i64,
    tags: Vec<String>,
    fields: Vec<String>,
}

#[derive(Debug, Clone)]
struct RawCard {
    note: i64,
    deck: i64,
    #[allow(dead_code)]
    ord: i64,
    kind: i64,
    due: i64,
    interval: i64,
    factor: i64,
    reps: i64,
    lapses: i64,
    data: String,
}

/// A read package, kept between the preview and the import.
#[derive(Debug, Clone, Default)]
pub struct Package {
    types: HashMap<i64, NoteType>,
    decks: HashMap<i64, String>,
    notes: BTreeMap<i64, RawNote>,
    cards: Vec<RawCard>,
    media: HashMap<String, Vec<u8>>,
    /// When the collection began, in days since 1970: review due dates count from it.
    start_day: i64,
}

fn unzstd(bytes: Vec<u8>) -> ApiResult<Vec<u8>> {
    if bytes.starts_with(&ZSTD_MAGIC) {
        zstd::stream::decode_all(bytes.as_slice()).map_err(|_| not_anki())
    } else {
        Ok(bytes)
    }
}

/// Reads protobuf varints and length-delimited fields, enough for Anki's media list and
/// note type settings.
fn proto_fields(mut bytes: &[u8]) -> Vec<(u64, ProtoValue<'_>)> {
    fn varint(bytes: &mut &[u8]) -> Option<u64> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let (&byte, rest) = bytes.split_first()?;
            *bytes = rest;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }
    let mut fields = vec![];
    while !bytes.is_empty() {
        let Some(key) = varint(&mut bytes) else { break };
        let value = match key & 7 {
            0 => match varint(&mut bytes) {
                Some(number) => ProtoValue::Number(number),
                None => break,
            },
            2 => {
                let Some(length) = varint(&mut bytes).map(|length| length as usize).filter(|length| *length <= bytes.len()) else { break };
                let (inner, rest) = bytes.split_at(length);
                bytes = rest;
                ProtoValue::Bytes(inner)
            }
            1 if bytes.len() >= 8 => {
                bytes = &bytes[8..];
                continue;
            }
            5 if bytes.len() >= 4 => {
                bytes = &bytes[4..];
                continue;
            }
            _ => break,
        };
        fields.push((key >> 3, value));
    }
    fields
}

#[derive(Debug, Clone, Copy)]
enum ProtoValue<'a> {
    Number(u64),
    Bytes(&'a [u8]),
}

fn read_media(zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>) -> ApiResult<HashMap<String, Vec<u8>>> {
    let list = match zip.by_name("media") {
        Ok(mut file) => {
            let mut bytes = vec![];
            file.read_to_end(&mut bytes).map_err(|_| not_anki())?;
            unzstd(bytes)?
        }
        Err(_) => return Ok(HashMap::new()),
    };
    // Older Anki: {"0": "tlb.png"}. Newer: a protobuf list of entries, numbered by position.
    let names: Vec<(String, String)> = match serde_json::from_slice::<HashMap<String, String>>(&list) {
        Ok(map) => map.into_iter().collect(),
        Err(_) => proto_fields(&list)
            .into_iter()
            .filter(|(field, _)| *field == 1)
            .enumerate()
            .filter_map(|(index, (_, entry))| match entry {
                ProtoValue::Bytes(inner) => proto_fields(inner).into_iter().find_map(|(field, value)| match (field, value) {
                    (1, ProtoValue::Bytes(name)) => Some((index.to_string(), String::from_utf8_lossy(name).into_owned())),
                    _ => None,
                }),
                ProtoValue::Number(_) => None,
            })
            .collect(),
    };
    let mut media = HashMap::new();
    for (entry, name) in names {
        let wanted = name.rsplit('.').next().is_some_and(|extension| IMAGE_TYPES.contains(&extension.to_ascii_lowercase().as_str()));
        if !wanted {
            // Only its name matters (to count dropped sounds).
            media.insert(name, vec![]);
            continue;
        }
        if let Ok(mut file) = zip.by_name(&entry) {
            let mut bytes = vec![];
            if file.read_to_end(&mut bytes).is_ok() {
                media.insert(name, unzstd(bytes)?);
            }
        }
    }
    Ok(media)
}

fn read_database(bytes: &[u8]) -> ApiResult<Package> {
    let mut file = tempfile::NamedTempFile::new().map_err(|error| ApiError::new(format!("couldn't read the deck: {error}")))?;
    std::io::Write::write_all(&mut file, bytes).map_err(|error| ApiError::new(format!("couldn't read the deck: {error}")))?;
    let connection = Connection::open_with_flags(file.path(), OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| not_anki())?;
    // Newer Anki sorts some names with its own rule.
    connection
        .create_collation("unicase", |left, right| left.to_lowercase().cmp(&right.to_lowercase()))
        .map_err(|_| not_anki())?;
    let failed = |_: rusqlite::Error| not_anki();
    let (crt, models, decks_json): (i64, String, String) = connection
        .query_row("SELECT crt, models, decks FROM col", [], |row| Ok((row.get(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default(), row.get::<_, Option<String>>(2)?.unwrap_or_default())))
        .map_err(failed)?;
    let modern: bool = connection.query_row("SELECT count(*) > 0 FROM sqlite_master WHERE name = 'notetypes'", [], |row| row.get(0)).map_err(failed)?;

    let mut types = HashMap::new();
    let mut decks = HashMap::new();
    if modern {
        let mut statement = connection.prepare("SELECT id, name, config FROM notetypes").map_err(failed)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Vec<u8>>(2)?))).map_err(failed)?;
        for row in rows {
            let (id, name, config) = row.map_err(failed)?;
            let cloze = proto_fields(&config).iter().any(|(field, value)| *field == 1 && matches!(value, ProtoValue::Number(1)));
            let count = |table: &str| -> ApiResult<usize> { connection.query_row(&format!("SELECT count(*) FROM {table} WHERE ntid = ?1"), [id], |row| row.get::<_, i64>(0)).map(|count| count as usize).map_err(failed) };
            types.insert(id, NoteType { name, cloze, fields: count("fields")?, templates: count("templates")? });
        }
        let mut statement = connection.prepare("SELECT id, name FROM decks").map_err(failed)?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(failed)? {
            let (id, name) = row.map_err(failed)?;
            decks.insert(id, name.replace(LEVEL, "::"));
        }
    } else {
        let models: HashMap<String, Value> = serde_json::from_str(&models).map_err(|_| not_anki())?;
        for (id, model) in models {
            let count = |key: &str| model.get(key).and_then(Value::as_array).map(Vec::len).unwrap_or(0);
            let id: i64 = id.parse().map_err(|_| not_anki())?;
            types.insert(id, NoteType { name: model["name"].as_str().unwrap_or("Note").to_string(), cloze: model["type"].as_i64() == Some(1), fields: count("flds"), templates: count("tmpls") });
        }
        let parsed: HashMap<String, Value> = serde_json::from_str(&decks_json).map_err(|_| not_anki())?;
        for (id, deck) in parsed {
            decks.insert(id.parse().map_err(|_| not_anki())?, deck["name"].as_str().unwrap_or("Deck").to_string());
        }
    }

    let mut notes = BTreeMap::new();
    let mut statement = connection.prepare("SELECT id, guid, mid, mod, tags, flds FROM notes").map_err(failed)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, RawNote { guid: row.get(1)?, kind: row.get(2)?, modified: row.get(3)?, tags: row.get::<_, String>(4)?.split_whitespace().map(String::from).collect(), fields: row.get::<_, String>(5)?.split('\u{1f}').map(String::from).collect() })))
        .map_err(failed)?;
    for row in rows {
        let (id, note) = row.map_err(failed)?;
        notes.insert(id, note);
    }
    let mut statement = connection.prepare("SELECT nid, did, ord, type, due, ivl, factor, reps, lapses, data FROM cards ORDER BY nid, ord").map_err(failed)?;
    let cards = statement
        .query_map([], |row| Ok(RawCard { note: row.get(0)?, deck: row.get(1)?, ord: row.get(2)?, kind: row.get(3)?, due: row.get(4)?, interval: row.get(5)?, factor: row.get(6)?, reps: row.get(7)?, lapses: row.get(8)?, data: row.get::<_, Option<String>>(9)?.unwrap_or_default() }))
        .map_err(failed)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(failed)?;
    Ok(Package { types, decks, notes, cards, media: HashMap::new(), start_day: crt.div_euclid(86_400) })
}

/// Reads an `.apkg` or `.colpkg`.
pub fn read(bytes: &[u8]) -> ApiResult<Package> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| not_anki())?;
    // The newest database first: newer files also carry an old one that only says to update Anki.
    let name = ["collection.anki21b", "collection.anki21", "collection.anki2"].into_iter().find(|name| zip.by_name(name).is_ok()).ok_or_else(not_anki)?;
    let mut database = vec![];
    zip.by_name(name).map_err(|_| not_anki())?.read_to_end(&mut database).map_err(|_| not_anki())?;
    let mut package = read_database(&unzstd(database)?)?;
    package.media = read_media(&mut zip)?;
    Ok(package)
}

// ---------- The preview ----------

/// Where an Anki deck's notes go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeckTarget {
    /// A topic named after the deck, in a module named after its parent deck. If they're
    /// here already (a module of that name, or this deck from an earlier import), they're used.
    NewTopic { name: String, module: Option<String> },
    Topic { topic: TopicId },
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeckRow {
    /// The full name, like `Networks::DNS`.
    pub name: String,
    pub notes: usize,
    pub target: DeckTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeRow {
    pub name: String,
    pub notes: usize,
    /// "Basic", "Basic and reversed" or "Cloze".
    pub becomes: String,
    /// Doesn't map exactly, so worth a look after importing.
    pub check: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnkiPreview {
    pub file_name: String,
    pub notes: usize,
    pub decks: Vec<DeckRow>,
    pub types: Vec<TypeRow>,
    pub images: usize,
    pub sounds: usize,
    pub has_progress: bool,
    /// Notes that are here from an earlier import, which stay as they are.
    pub already_here: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnkiChoice {
    /// Decks left out get their suggested target.
    #[serde(default)]
    pub decks: Vec<(String, DeckTarget)>,
    #[serde(default)]
    pub keep_progress: bool,
}

/// How a note type maps onto the app's: (what it becomes, whether it needs a look).
fn mapping(kind: &NoteType) -> (NoteKind, bool) {
    if kind.cloze {
        (NoteKind::Cloze, kind.fields > 2)
    } else if kind.fields == 2 && kind.templates == 1 {
        (NoteKind::Basic, false)
    } else if kind.fields == 2 && kind.templates == 2 {
        (NoteKind::Reversed, false)
    } else {
        (NoteKind::Basic, true)
    }
}

fn kind_label(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Basic => "Basic",
        NoteKind::Reversed => "Basic and reversed",
        NoteKind::Cloze => "Cloze",
    }
}

impl Package {
    /// Each note's deck: where its first card is.
    fn deck_of(&self) -> HashMap<i64, i64> {
        let mut decks = HashMap::new();
        for card in &self.cards {
            decks.entry(card.note).or_insert(card.deck);
        }
        decks
    }

    fn uid_of(note: &RawNote) -> String {
        // A deck exported by this app keeps the note's own ID as its Anki ID.
        if note.guid.len() == 36 && note.guid.chars().filter(|letter| *letter == '-').count() == 4 { note.guid.clone() } else { format!("anki-{}", note.guid) }
    }
}

fn suggested(name: &str) -> DeckTarget {
    match name.rsplit_once("::") {
        Some((parent, leaf)) => {
            let module = parent.split("::").next().unwrap_or(parent);
            let topic = if parent.contains("::") { format!("{} / {leaf}", parent.split_once("::").map(|(_, rest)| rest).unwrap_or(parent).replace("::", " / ")) } else { leaf.to_string() };
            DeckTarget::NewTopic { name: topic, module: Some(module.to_string()) }
        }
        None => DeckTarget::NewTopic { name: name.to_string(), module: None },
    }
}

impl App {
    /// Reads an Anki file and says what importing it would do, changing nothing (R171).
    pub fn anki_preview(&mut self, bytes: &[u8], file_name: &str) -> ApiResult<AnkiPreview> {
        self.pending_anki = None;
        let package = read(bytes)?;
        let here = self.store.stamps_here()?;
        let deck_of = package.deck_of();
        let mut per_deck: BTreeMap<String, usize> = BTreeMap::new();
        let mut per_type: BTreeMap<String, (usize, NoteKind, bool)> = BTreeMap::new();
        let mut images = HashSet::new();
        let mut sounds = 0;
        let mut already_here = 0;
        for (id, note) in &package.notes {
            let deck = deck_of.get(id).and_then(|deck| package.decks.get(deck)).cloned().unwrap_or_else(|| "Default".into());
            *per_deck.entry(deck).or_default() += 1;
            let kind = package.types.get(&note.kind).cloned().unwrap_or(NoteType { name: "Note".into(), cloze: false, fields: note.fields.len(), templates: 1 });
            let (becomes, check) = mapping(&kind);
            per_type.entry(kind.name).or_insert((0, becomes, check)).0 += 1;
            for field in &note.fields {
                let mut dropped = Dropped::default();
                html_to_markdown(field, &mut |name, _| { images.insert(name.to_string()); None }, &mut dropped);
                sounds += dropped.sounds;
            }
            if here.contains_key(&Package::uid_of(note)) {
                already_here += 1;
            }
        }
        let images = images.iter().filter(|name| package.media.get(*name).is_some_and(|bytes| !bytes.is_empty())).count();
        let has_progress = package.cards.iter().any(|card| card.kind == 2 && card.interval > 0) || package.cards.iter().any(|card| card.reps > 0);
        let preview = AnkiPreview {
            file_name: file_name.to_string(),
            notes: package.notes.len(),
            decks: per_deck.into_iter().map(|(name, notes)| DeckRow { target: suggested(&name), name, notes }).collect(),
            types: per_type.into_iter().map(|(name, (notes, becomes, check))| TypeRow { name, notes, becomes: kind_label(becomes).into(), check }).collect(),
            images,
            sounds,
            has_progress,
            already_here,
        };
        self.pending_anki = Some((file_name.to_string(), package));
        Ok(preview)
    }

    /// Imports the previewed Anki file as chosen: a backup first, then everything at once (R172).
    pub fn anki_apply(&mut self, choice: AnkiChoice, clock: Clock) -> ApiResult<crate::transfer::ImportResult> {
        let (file_name, package) = self.pending_anki.take().ok_or_else(|| ApiError::new("pick the Anki file to import again"))?;
        let snapshot = self.anki_snapshot(&package, &choice, clock)?;
        self.step("Notes", package.notes.len(), package.notes.len())?;
        let result = self.apply_snapshot(snapshot, HashSet::new(), clock)?;
        self.remember_import(&file_name, &result, clock);
        Ok(result)
    }

    fn anki_snapshot(&self, package: &Package, choice: &AnkiChoice, clock: Clock) -> ApiResult<Snapshot> {
        let chosen: HashMap<&str, &DeckTarget> = choice.decks.iter().map(|(name, target)| (name.as_str(), target)).collect();
        let deck_of = package.deck_of();
        let device = "anki";
        let stamp = |ms: i64| format!("{:016}-0000-{device}", ms.max(0));
        let mut snapshot = Snapshot::default();
        let modules_here: HashMap<String, ModuleId> = self.modules()?.into_iter().map(|module| (module.name.to_lowercase(), module.id)).collect();
        let mut topic_uids: HashMap<String, Option<String>> = HashMap::new();
        let mut module_uids: HashMap<String, String> = HashMap::new();
        let mut image_uids: HashMap<String, String> = HashMap::new();

        let mut topic_for = |deck: &str, snapshot: &mut Snapshot| -> ApiResult<Option<String>> {
            if let Some(found) = topic_uids.get(deck) {
                return Ok(found.clone());
            }
            let target = chosen.get(deck).map(|target| (*target).clone()).unwrap_or_else(|| suggested(deck));
            let uid = match target {
                DeckTarget::Skip => None,
                DeckTarget::Topic { topic: id } => Some(self.store.uid_of("topics", id.0)?.ok_or_else(|| ApiError::new("one of the chosen topics is gone, pick again"))?),
                DeckTarget::NewTopic { name, module } => {
                    let module = match module {
                        None => None,
                        Some(module) => Some(match modules_here.get(&module.to_lowercase()) {
                            Some(id) => self.store.uid_of("modules", id.0)?.ok_or_else(|| ApiError::new("one of the modules is gone, pick again"))?,
                            None => module_uids
                                .entry(module.to_lowercase())
                                .or_insert_with(|| {
                                    let uid = format!("anki-module-{}", module.to_lowercase());
                                    snapshot.modules.push(portable::Module { uid: uid.clone(), changed: stamp(0), name: module.clone() });
                                    uid
                                })
                                .clone(),
                        }),
                    };
                    let uid = format!("anki-deck-{deck}");
                    snapshot.topics.push(portable::Topic { uid: uid.clone(), changed: stamp(0), name: name.trim().to_string(), module, tags: vec![], schedule: None });
                    Some(uid)
                }
            };
            topic_uids.insert(deck.to_string(), uid.clone());
            Ok(uid)
        };

        let cards_of: HashMap<i64, Vec<&RawCard>> = package.cards.iter().fold(HashMap::new(), |mut map, card| {
            map.entry(card.note).or_insert_with(Vec::new).push(card);
            map
        });
        let total = package.notes.len();
        for (done, (id, raw)) in package.notes.iter().enumerate() {
            self.step("Notes", done, total)?;
            let deck = deck_of.get(id).and_then(|deck| package.decks.get(deck)).cloned().unwrap_or_else(|| "Default".into());
            let Some(topic) = topic_for(&deck, &mut snapshot)? else { continue };
            let kind = package.types.get(&raw.kind).cloned().unwrap_or(NoteType { name: "Note".into(), cloze: false, fields: raw.fields.len(), templates: 1 });
            let (note_kind, _) = mapping(&kind);
            let mut field = |index: usize, snapshot: &mut Snapshot| -> String {
                let mut dropped = Dropped::default();
                let html = raw.fields.get(index).map(String::as_str).unwrap_or("");
                html_to_markdown(
                    html,
                    &mut |name, alt| {
                        let bytes = package.media.get(name).filter(|bytes| !bytes.is_empty())?;
                        let format = study_core::image::check_image(bytes).ok()?;
                        let uid = image_uids
                            .entry(name.to_string())
                            .or_insert_with(|| {
                                let uid = stable_uid(&format!("anki-media:{name}"));
                                snapshot.images.push(portable::Image { uid: uid.clone(), changed: stamp(0), format: serde_json::to_string(&format).unwrap_or_default(), bytes: bytes.clone() });
                                uid
                            })
                            .clone();
                        let alt = alt.filter(|alt| !alt.trim().is_empty()).unwrap_or_else(|| name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name)).replace(['[', ']'], "");
                        Some(format!("![{alt}](image:{uid})"))
                    },
                    &mut dropped,
                )
            };
            let mut note = CardNote { topic: TopicId(0), kind: note_kind, front: String::new(), back: String::new(), cues: vec![], text: String::new(), extra: String::new() };
            match note_kind {
                NoteKind::Cloze => {
                    note.text = field(0, &mut snapshot);
                    note.extra = field(1, &mut snapshot);
                }
                _ => {
                    note.front = field(0, &mut snapshot);
                    note.back = field(1, &mut snapshot);
                }
            }
            let Ok(faces) = note.faces() else { continue };
            let note_uid = Package::uid_of(raw);
            let changed = stamp(raw.modified * 1000);
            let anki_cards = cards_of.get(id).cloned().unwrap_or_default();
            let cards = faces
                .into_iter()
                .enumerate()
                .map(|(index, face)| {
                    let schedule = if choice.keep_progress { anki_cards.get(index).and_then(|card| schedule_of(card, package.start_day, clock.day)) } else { None };
                    portable::Card { uid: format!("{note_uid}:{}", face.key), changed: changed.clone(), key: face.key, front: face.front, back: face.back, archived: false, schedule }
                })
                .collect();
            let mut value = serde_json::to_value(&note).map_err(|error| ApiError::new(error.to_string()))?;
            if let Value::Object(fields) = &mut value {
                fields.remove("topic");
            }
            let _ = &raw.tags;
            snapshot.notes.push(portable::Note { uid: note_uid, changed, topic, note: value, source: None, created: clock.day, cards });
        }
        Ok(snapshot)
    }
}

/// A UUID-shaped ID that's always the same for the same text, so importing a file again
/// finds what came in the first time. (Image references only allow hex digits and dashes.)
pub(crate) fn stable_uid(text: &str) -> String {
    let fnv = |seed: u64| text.bytes().fold(seed, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3));
    let (high, low) = (fnv(0xcbf2_9ce4_8422_2325), fnv(0x6c62_272e_07bb_0142));
    let hex = format!("{high:016x}{low:016x}");
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// An Anki review card's place in its schedule, as the app's (roughly: Anki and the app
/// schedule a little differently). New and learning cards start as new.
fn schedule_of(card: &RawCard, start_day: i64, today: i64) -> Option<CardSchedule> {
    // Anki with FSRS on keeps the memory state in the card's data.
    let data: Value = serde_json::from_str(&card.data).unwrap_or(Value::Null);
    if matches!(card.kind, 1 | 3) && card.reps > 0 {
        // Still learning: it carries on today.
        let stability = data.get("s").and_then(Value::as_f64).unwrap_or(1.0);
        let difficulty = data.get("d").and_then(Value::as_f64).unwrap_or(5.0);
        return Some(CardSchedule { stability: Some(stability), difficulty: Some(difficulty), due: today, last_review: Some(today), reps: card.reps, lapses: card.lapses });
    }
    if card.kind != 2 || card.interval <= 0 {
        return None;
    }
    let due = start_day + card.due;
    let stability = data.get("s").and_then(Value::as_f64).unwrap_or(card.interval as f64);
    let difficulty = data.get("d").and_then(Value::as_f64).unwrap_or_else(|| (1.0 + (3500.0 - card.factor as f64) / 2200.0 * 9.0).clamp(1.0, 10.0));
    Some(CardSchedule { stability: Some(stability), difficulty: Some(difficulty), due: due.max(today - 3650), last_review: Some(due - card.interval), reps: card.reps, lapses: card.lapses })
}

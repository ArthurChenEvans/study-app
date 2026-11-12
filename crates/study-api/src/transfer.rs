//! Export and import (R112 to R118): a `.studyapp` file is a zip of `manifest.json`, one
//! JSON Lines file per kind of record (`data/topics.jsonl`) and the images as files
//! (`images/<id>`). Readable with any zip tool, so your data never depends on this app.
//!
//! An import is checked and previewed before anything changes: what's new, what's here
//! already, and what's changed in both places, and how to settle those. Files from other
//! people are untrusted, so sizes are capped and every record is checked as it's read.

use std::io::{Cursor, Read, Write};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use study_core::question::TopicId;
use study_store::portable::{Plan, Scope, Snapshot};
use zip::write::SimpleFileOptions;

use crate::backup::Reason;
use crate::practice::Clock;
use crate::{ApiError, ApiResult, App};

pub const FORMAT: &str = "studyapp";
pub const VERSION: u32 = 1;
/// Largest file the app will read, and the most its contents may unpack to.
pub const MAX_FILE: usize = 200 * 1024 * 1024;
pub const MAX_UNPACKED: u64 = if cfg!(test) { 64 * 1024 * 1024 } else { 1024 * 1024 * 1024 };
const MAX_ENTRIES: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequest {
    /// `None` exports everything. Otherwise these topics, to share.
    pub topics: Option<Vec<TopicId>>,
    pub history: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Count {
    pub what: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportSummary {
    /// The file name it'll be saved as.
    pub name: String,
    pub counts: Vec<Count>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub app_version: String,
    /// The day it was exported, as a day number.
    pub exported: i64,
    pub device: String,
    pub everything: bool,
    pub history: bool,
    pub counts: Vec<Count>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreviewRow {
    pub what: String,
    pub new: usize,
    pub same: usize,
    pub changed: usize,
}

/// A record that's here and in the file, changed in at least one of them (R116).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Clash {
    pub uid: String,
    pub what: String,
    pub label: String,
    /// The day each side last changed it.
    pub mine_day: i64,
    pub theirs_day: i64,
    pub theirs_newer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClashRule {
    KeepMine,
    TakeTheirs,
    Newer,
    EachOne,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportPreview {
    pub file_name: String,
    pub everything: bool,
    pub history: bool,
    pub exported: i64,
    /// Exported from this computer.
    pub from_here: bool,
    pub rows: Vec<PreviewRow>,
    pub clashes: Vec<Clash>,
    /// What the dialog starts on: the newer one for a full export, mine for shared topics.
    pub suggested: ClashRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportChoice {
    pub rule: ClashRule,
    /// With `EachOne`: the clashes to take the file's version of.
    pub theirs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportResult {
    pub added: Vec<Count>,
    pub replaced: usize,
    /// The backup made first, to undo the import with.
    pub backup: Option<crate::backup::BackupInfo>,
}

/// An export, saved to a folder (desktop) or handed to the browser as base64 (web).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportFile {
    pub name: String,
    pub path: Option<String>,
    pub data: Option<String>,
}

/// A name in `folder` that isn't taken: `Networks.studyapp`, then `Networks (2).studyapp`.
pub fn free_path(folder: &std::path::Path, name: &str) -> std::path::PathBuf {
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, "studyapp"));
    (1..)
        .map(|copy| folder.join(if copy == 1 { name.to_string() } else { format!("{stem} ({copy}).{extension}") }))
        .find(|path| !path.exists())
        .expect("a free name")
}

/// An import that's been read and previewed, waiting to be confirmed.
pub struct PendingImport {
    snapshot: Snapshot,
    clashes: Vec<Clash>,
}

// ---------- Kinds of record ----------

/// (file name, what it's called, the table its IDs are in, whether it can clash)
const KINDS: &[(&str, &str, &str, bool)] = &[
    ("modules", "Modules", "modules", true),
    ("topics", "Topics", "topics", true),
    ("sources", "Sources", "sources", true),
    ("images", "Images", "images", false),
    ("questions", "Questions", "questions", true),
    ("notes", "Cards", "notes", true),
    ("reviews", "Card reviews", "card_reviews", false),
    ("checklists", "Checklists", "recall_checklists", true),
    ("checks", "Recall checks", "recall_checks", false),
    ("attempts", "Quizzes", "attempts", false),
    ("projects", "Projects", "projects", true),
    ("sessions", "Sessions", "project_sessions", false),
    ("issues", "Issues", "issues", true),
    ("drills", "Drills", "drills", true),
    ("drill_runs", "Drill runs", "drill_runs", false),
    ("struggles", "Struggles", "struggles", true),
    ("dates", "Dates", "dates", true),
    ("settings", "Settings", "settings", true),
    ("suggestions_done", "Dismissed suggestions", "card_suggestions_done", false),
];

/// One record as the preview sees it: its ID, stamp and a label.
struct Entry {
    uid: String,
    changed: String,
    label: String,
}

fn short(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    if line.chars().count() > 80 { format!("{}…", line.chars().take(79).collect::<String>()) } else { line.to_string() }
}

fn entries(snapshot: &Snapshot, kind: &str) -> Vec<Entry> {
    let entry = |uid: &str, changed: &str, label: String| Entry { uid: uid.into(), changed: changed.into(), label };
    match kind {
        "modules" => snapshot.modules.iter().map(|record| entry(&record.uid, &record.changed, record.name.clone())).collect(),
        "topics" => snapshot.topics.iter().map(|record| entry(&record.uid, &record.changed, record.name.clone())).collect(),
        "sources" => snapshot.sources.iter().map(|record| entry(&record.uid, &record.changed, short(&record.title))).collect(),
        "images" => snapshot.images.iter().map(|record| entry(&record.uid, &record.changed, String::new())).collect(),
        "questions" => snapshot.questions.iter().map(|record| entry(&record.uid, &record.changed, short(&record.text))).collect(),
        "notes" => snapshot
            .notes
            .iter()
            .map(|record| {
                let text = ["front", "text"].iter().filter_map(|field| record.note.get(field).and_then(|value| value.as_str())).find(|text| !text.is_empty()).unwrap_or_default();
                entry(&record.uid, &record.changed, short(text))
            })
            .collect(),
        "reviews" => snapshot.reviews.iter().map(|record| entry(&record.uid, &record.changed, String::new())).collect(),
        "checklists" => snapshot.checklists.iter().map(|record| entry(&record.uid, &record.changed, record.subject_name.clone())).collect(),
        "checks" => snapshot.checks.iter().map(|record| entry(&record.uid, &record.changed, String::new())).collect(),
        "attempts" => snapshot.attempts.iter().map(|record| entry(&record.uid, &record.changed, String::new())).collect(),
        "projects" => snapshot.projects.iter().map(|record| entry(&record.uid, &record.changed, record.name.clone())).collect(),
        "sessions" => snapshot.sessions.iter().map(|record| entry(&record.uid, &record.changed, String::new())).collect(),
        "issues" => snapshot.issues.iter().map(|record| entry(&record.uid, &record.changed, record.name.clone())).collect(),
        "drills" => snapshot.drills.iter().map(|record| entry(&record.uid, &record.changed, short(&record.task))).collect(),
        "drill_runs" => snapshot.drill_runs.iter().map(|record| entry(&record.uid, &record.changed, String::new())).collect(),
        "struggles" => snapshot.struggles.iter().map(|record| entry(&record.uid, &record.changed, short(&record.what))).collect(),
        "dates" => snapshot.dates.iter().map(|record| entry(&record.uid, &record.changed, record.name.clone())).collect(),
        "settings" => snapshot.settings.iter().map(|record| entry(&format!("settings:{}", record.key), &record.changed, setting_name(&record.key))).collect(),
        "suggestions_done" => snapshot.suggestions_done.iter().map(|record| entry(&format!("suggestion:{}", record.key), &record.changed, String::new())).collect(),
        _ => vec![],
    }
}

fn setting_name(key: &str) -> String {
    match key {
        "quiz_schedule" => "Quiz schedule".into(),
        "recall_schedule" => "Recall schedule".into(),
        "card_settings" => "Card settings".into(),
        other => other.replace('_', " "),
    }
}

/// The day a stamp was made, from its milliseconds.
fn stamp_day(stamp: &str) -> i64 {
    stamp.split('-').next().and_then(|ms| ms.parse::<i64>().ok()).map_or(0, |ms| ms / 86_400_000)
}

fn counts(snapshot: &Snapshot) -> Vec<Count> {
    let mut counts: Vec<Count> = KINDS
        .iter()
        .map(|(kind, what, _, _)| Count { what: what.to_string(), count: entries(snapshot, kind).len() })
        .filter(|count| count.count > 0 && count.what != "Dismissed suggestions")
        .collect();
    let cards: usize = snapshot.notes.iter().map(|note| note.cards.len()).sum();
    if let Some(row) = counts.iter_mut().find(|count| count.what == "Cards") {
        row.count = cards;
    }
    counts
}

// ---------- The file ----------

fn bad_file(detail: &str) -> ApiError {
    ApiError::new(format!("that file couldn't be imported: {detail}"))
}

fn write_lines<T: Serialize>(zip: &mut zip::ZipWriter<Cursor<Vec<u8>>>, name: &str, records: &[T]) -> ApiResult<()> {
    if records.is_empty() {
        return Ok(());
    }
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(format!("data/{name}.jsonl"), options).map_err(|error| ApiError::new(error.to_string()))?;
    for record in records {
        let line = serde_json::to_string(record).map_err(|error| ApiError::new(error.to_string()))?;
        zip.write_all(line.as_bytes()).and_then(|_| zip.write_all(b"\n")).map_err(|error| ApiError::new(error.to_string()))?;
    }
    Ok(())
}

pub fn pack(snapshot: &Snapshot, manifest: &Manifest) -> ApiResult<Vec<u8>> {
    let failed = |error: zip::result::ZipError| ApiError::new(format!("couldn't write the export: {error}"));
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options).map_err(failed)?;
    zip.write_all(serde_json::to_string_pretty(manifest).map_err(|error| ApiError::new(error.to_string()))?.as_bytes()).map_err(|error| ApiError::new(error.to_string()))?;
    write_lines(&mut zip, "modules", &snapshot.modules)?;
    write_lines(&mut zip, "topics", &snapshot.topics)?;
    write_lines(&mut zip, "sources", &snapshot.sources)?;
    write_lines(&mut zip, "images", &snapshot.images)?;
    write_lines(&mut zip, "questions", &snapshot.questions)?;
    write_lines(&mut zip, "notes", &snapshot.notes)?;
    write_lines(&mut zip, "reviews", &snapshot.reviews)?;
    write_lines(&mut zip, "checklists", &snapshot.checklists)?;
    write_lines(&mut zip, "checks", &snapshot.checks)?;
    write_lines(&mut zip, "attempts", &snapshot.attempts)?;
    write_lines(&mut zip, "projects", &snapshot.projects)?;
    write_lines(&mut zip, "sessions", &snapshot.sessions)?;
    write_lines(&mut zip, "issues", &snapshot.issues)?;
    write_lines(&mut zip, "drills", &snapshot.drills)?;
    write_lines(&mut zip, "drill_runs", &snapshot.drill_runs)?;
    write_lines(&mut zip, "struggles", &snapshot.struggles)?;
    write_lines(&mut zip, "dates", &snapshot.dates)?;
    write_lines(&mut zip, "settings", &snapshot.settings)?;
    write_lines(&mut zip, "suggestions_done", &snapshot.suggestions_done)?;
    // Images are already compressed, so they're stored as they are.
    for image in &snapshot.images {
        zip.start_file(format!("images/{}", image.uid), SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).map_err(failed)?;
        zip.write_all(&image.bytes).map_err(|error| ApiError::new(error.to_string()))?;
    }
    Ok(zip.finish().map_err(failed)?.into_inner())
}

/// Reads one entry, refusing more than `limit` bytes whatever its header claims.
fn read_entry<R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>, name: &str, limit: &mut u64) -> ApiResult<Option<Vec<u8>>> {
    let entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(_) => return Err(bad_file("part of it is damaged")),
    };
    let mut bytes = Vec::new();
    entry.take(*limit + 1).read_to_end(&mut bytes).map_err(|_| bad_file("part of it is damaged"))?;
    if bytes.len() as u64 > *limit {
        return Err(bad_file("it unpacks to more than the app will read"));
    }
    *limit -= bytes.len() as u64;
    Ok(Some(bytes))
}

fn read_lines<T: DeserializeOwned, R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>, name: &str, limit: &mut u64) -> ApiResult<Vec<T>> {
    let Some(bytes) = read_entry(archive, &format!("data/{name}.jsonl"), limit)? else { return Ok(vec![]) };
    let text = String::from_utf8(bytes).map_err(|_| bad_file(&format!("{name} isn't text")))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(number, line)| serde_json::from_str(line).map_err(|error| bad_file(&format!("line {} of {name} doesn't make sense ({error})", number + 1))))
        .collect()
}

pub fn unpack(bytes: &[u8]) -> ApiResult<(Manifest, Snapshot)> {
    if bytes.len() > MAX_FILE {
        return Err(bad_file("it's bigger than 200 MB"));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| bad_file("it isn't a .studyapp file"))?;
    if archive.len() > MAX_ENTRIES {
        return Err(bad_file("it has too many parts"));
    }
    let mut limit = MAX_UNPACKED;
    let manifest = read_entry(&mut archive, "manifest.json", &mut limit)?.ok_or_else(|| bad_file("it isn't a .studyapp file"))?;
    let manifest: Manifest = serde_json::from_slice(&manifest).map_err(|_| bad_file("it isn't a .studyapp file"))?;
    if manifest.format != FORMAT {
        return Err(bad_file("it isn't a .studyapp file"));
    }
    if manifest.version > VERSION {
        return Err(bad_file("it was made by a newer version of the app, update this one first"));
    }
    let mut snapshot = Snapshot {
        modules: read_lines(&mut archive, "modules", &mut limit)?,
        topics: read_lines(&mut archive, "topics", &mut limit)?,
        sources: read_lines(&mut archive, "sources", &mut limit)?,
        images: read_lines(&mut archive, "images", &mut limit)?,
        questions: read_lines(&mut archive, "questions", &mut limit)?,
        notes: read_lines(&mut archive, "notes", &mut limit)?,
        reviews: read_lines(&mut archive, "reviews", &mut limit)?,
        checklists: read_lines(&mut archive, "checklists", &mut limit)?,
        checks: read_lines(&mut archive, "checks", &mut limit)?,
        attempts: read_lines(&mut archive, "attempts", &mut limit)?,
        projects: read_lines(&mut archive, "projects", &mut limit)?,
        sessions: read_lines(&mut archive, "sessions", &mut limit)?,
        issues: read_lines(&mut archive, "issues", &mut limit)?,
        drills: read_lines(&mut archive, "drills", &mut limit)?,
        drill_runs: read_lines(&mut archive, "drill_runs", &mut limit)?,
        struggles: read_lines(&mut archive, "struggles", &mut limit)?,
        dates: read_lines(&mut archive, "dates", &mut limit)?,
        settings: read_lines(&mut archive, "settings", &mut limit)?,
        suggestions_done: read_lines(&mut archive, "suggestions_done", &mut limit)?,
    };
    for image in &mut snapshot.images {
        if !image.uid.chars().all(|letter| letter.is_ascii_hexdigit() || letter == '-') {
            return Err(bad_file("an image has a broken name"));
        }
        let bytes = read_entry(&mut archive, &format!("images/{}", image.uid), &mut limit)?.ok_or_else(|| bad_file("an image is missing"))?;
        crate::check_image(&bytes).map_err(|_| bad_file("one of its images isn't a picture the app can show"))?;
        image.bytes = bytes;
    }
    check(&snapshot)?;
    Ok((manifest, snapshot))
}

/// Every record has an ID, and every reference inside the file is to something the file
/// has, or that's allowed to be missing (R118).
fn check(snapshot: &Snapshot) -> ApiResult<()> {
    let mut seen = std::collections::HashSet::new();
    for (kind, _, _, _) in KINDS {
        for entry in entries(snapshot, kind) {
            if entry.uid.is_empty() || entry.uid.len() > 200 || !seen.insert(entry.uid.clone()) {
                return Err(bad_file(&format!("two {kind} have the same ID, or one has none")));
            }
        }
    }
    let topics: std::collections::HashSet<&str> = snapshot.topics.iter().map(|topic| topic.uid.as_str()).collect();
    if snapshot.questions.iter().any(|question| !topics.contains(question.topic.as_str())) || snapshot.notes.iter().any(|note| !topics.contains(note.topic.as_str())) {
        return Err(bad_file("a question or card belongs to a topic that isn't in it"));
    }
    if snapshot.questions.iter().any(|question| question.text.len() > 1_000_000) {
        return Err(bad_file("a question is far too long"));
    }
    Ok(())
}

fn file_name(app: &App, request: &ExportRequest, today: i64) -> ApiResult<String> {
    let date = crate::backup::date_label(today);
    let name = match &request.topics {
        None => format!("Study App {date}"),
        Some(picked) => {
            let topics = app.topics()?;
            let chosen: Vec<_> = topics.iter().filter(|topic| picked.contains(&topic.id)).collect();
            let modules = app.modules()?;
            let whole_module = chosen.first().and_then(|first| first.module).filter(|module| {
                chosen.iter().all(|topic| topic.module == Some(*module)) && topics.iter().filter(|topic| topic.module == Some(*module)).count() == chosen.len()
            });
            match (whole_module, chosen.as_slice()) {
                (Some(module), _) => modules.iter().find(|row| row.id == module).map(|row| row.name.clone()).unwrap_or_else(|| "Topics".into()),
                (None, [only]) => only.name.clone(),
                _ => format!("{} topics {date}", chosen.len()),
            }
        }
    };
    let safe: String = name.chars().map(|letter| if letter.is_alphanumeric() || " -_,()'".contains(letter) { letter } else { '-' }).collect();
    Ok(format!("{}.studyapp", safe.trim()))
}

impl App {
    fn scope(&self, request: &ExportRequest) -> ApiResult<Scope> {
        if let Some(topics) = &request.topics
            && topics.is_empty()
        {
            return Err(ApiError::new("pick at least one topic to share"));
        }
        Ok(Scope { topics: request.topics.as_ref().map(|topics| topics.iter().map(|topic| topic.0).collect()), history: request.history })
    }

    /// What an export would hold, and its file name (R112, R113).
    pub fn export_summary(&self, request: &ExportRequest, today: i64) -> ApiResult<ExportSummary> {
        let snapshot = self.store.snapshot(&self.scope(request)?)?;
        Ok(ExportSummary { name: file_name(self, request, today)?, counts: counts(&snapshot) })
    }

    /// The `.studyapp` file, and its name (R112, R113).
    pub fn export(&self, request: &ExportRequest, today: i64) -> ApiResult<(String, Vec<u8>)> {
        let snapshot = self.store.snapshot(&self.scope(request)?)?;
        let manifest = Manifest {
            format: FORMAT.into(),
            version: VERSION,
            app_version: env!("CARGO_PKG_VERSION").into(),
            exported: today,
            device: self.store.device_id(),
            everything: request.topics.is_none(),
            history: request.history,
            counts: counts(&snapshot),
        };
        Ok((file_name(self, request, today)?, pack(&snapshot, &manifest)?))
    }

    /// Reads a file and says what importing it would do, changing nothing yet (R114, R115).
    pub fn import_preview(&mut self, bytes: &[u8], file_name: &str) -> ApiResult<ImportPreview> {
        self.pending_import = None;
        let (manifest, snapshot) = unpack(bytes)?;
        let here = self.store.stamps_here()?;
        let mut rows = vec![];
        let mut clashes = vec![];
        for (kind, what, _, can_clash) in KINDS {
            let mut row = PreviewRow { what: what.to_string(), new: 0, same: 0, changed: 0 };
            for entry in entries(&snapshot, kind) {
                match here.get(&entry.uid) {
                    None => row.new += 1,
                    Some(mine) if *mine == entry.changed || !can_clash => row.same += 1,
                    Some(mine) => {
                        row.changed += 1;
                        clashes.push(Clash { uid: entry.uid, what: what.trim_end_matches('s').to_string(), label: entry.label, mine_day: stamp_day(mine), theirs_day: stamp_day(&entry.changed), theirs_newer: entry.changed > *mine });
                    }
                }
            }
            if *kind == "notes" {
                // Cards are counted one by one, though they change with their note.
                let clashing: std::collections::HashSet<&str> = clashes.iter().map(|clash| clash.uid.as_str()).collect();
                row = PreviewRow { what: "Cards".into(), new: 0, same: 0, changed: 0 };
                for note in &snapshot.notes {
                    for card in &note.cards {
                        match (here.contains_key(&card.uid), clashing.contains(note.uid.as_str())) {
                            (false, _) => row.new += 1,
                            (true, false) => row.same += 1,
                            (true, true) => row.changed += 1,
                        }
                    }
                }
            }
            if row.new + row.same + row.changed > 0 && *kind != "suggestions_done" && *kind != "images" {
                rows.push(row);
            }
        }
        let preview = ImportPreview {
            file_name: file_name.to_string(),
            everything: manifest.everything,
            history: manifest.history,
            exported: manifest.exported,
            from_here: manifest.device == self.store.device_id(),
            rows,
            clashes: clashes.clone(),
            suggested: if manifest.everything { ClashRule::Newer } else { ClashRule::KeepMine },
        };
        self.pending_import = Some(PendingImport { snapshot, clashes });
        Ok(preview)
    }

    pub fn cancel_import(&mut self) {
        self.pending_import = None;
    }

    /// Imports the previewed file: a backup first, then everything at once (R116, R117).
    pub fn import_apply(&mut self, choice: ImportChoice, clock: Clock) -> ApiResult<ImportResult> {
        let pending = self.pending_import.take().ok_or_else(|| ApiError::new("pick the file to import again"))?;
        let theirs = pending
            .clashes
            .iter()
            .filter(|clash| match choice.rule {
                ClashRule::KeepMine => false,
                ClashRule::TakeTheirs => true,
                ClashRule::Newer => clash.theirs_newer,
                ClashRule::EachOne => choice.theirs.contains(&clash.uid),
            })
            .map(|clash| clash.uid.clone())
            .collect();
        let backup = match self.backups.as_ref() {
            Some(_) => Some(self.make_backup(clock, Reason::BeforeImport)?),
            None => None,
        };
        let plan = Plan { theirs, today: clock.day, history_edits: false };
        let added = self.kept_only_if_readable(clock.day, "that file has something in it this version of the app can't read, so nothing was imported", |_| true, |app| Ok(app.store.apply(&pending.snapshot, &plan)?))?;
        self.forget_running_quiz()?;
        self.card_session = None;
        Ok(ImportResult { added: added.counts.into_iter().map(|(what, count)| Count { what, count }).collect(), replaced: added.replaced, backup })
    }
}


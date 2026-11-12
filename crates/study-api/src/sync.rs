//! Sync through a folder (R119 to R128): the folder side. The database side, which works
//! out what changed and merges other computers' changes, is `study_store::sync`.
//!
//! The folder holds, for each computer, `devices/<device>.json` (its name, how far its
//! changes go, when it was last seen) and `changes/<device>/<YYYY-MM>.jsonl` (its change
//! lines, only ever appended to). Each file has one writer, so the app keeping the folder
//! in step never has two versions of one file to choose between.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use study_store::sync::{Conflict, Cursor, DELETED, Line};

use crate::backup::Reason;
use crate::practice::Clock;
use crate::{ApiError, ApiResult, App};

/// The version of the folder's files. A computer writing a newer one is skipped (R127).
const FORMAT: u32 = 1;
/// Lines read in one go, so a first sync of a big profile doesn't hold the app up for long.
const LINES_PER_SYNC: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSetup {
    pub folder: String,
    pub name: String,
}

/// Kept beside the backups, not in the database, so restoring a backup leaves it alone.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Config {
    folder: Option<String>,
    name: Option<String>,
    on: bool,
    paused: Option<String>,
    /// Other computers' names, for when the folder can't be read.
    #[serde(default)]
    names: BTreeMap<String, String>,
    last_synced: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct Sync {
    home: Option<PathBuf>,
    default_name: Option<String>,
    config: Config,
    problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceFile {
    format: u32,
    device: String,
    name: String,
    seq: u64,
    seen_at: i64,
    #[serde(default)]
    app: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncDevice {
    pub id: String,
    pub name: String,
    pub this_one: bool,
    /// Unix seconds.
    pub last_seen: Option<i64>,
    /// For this computer, changes not yet in the folder. For the others, changes they've
    /// made that haven't arrived here yet.
    pub waiting: u64,
    pub newer_app: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncStatus {
    pub on: bool,
    pub paused: Option<String>,
    pub folder: Option<String>,
    pub device: String,
    pub name: String,
    pub devices: Vec<SyncDevice>,
    /// Unix seconds.
    pub last_synced: Option<i64>,
    pub problem: Option<String>,
    /// Changes made here that aren't in the folder yet.
    pub waiting: u64,
    /// Records waiting for something they refer to (R126).
    pub held: usize,
    /// Things to check, counted by record.
    pub to_check: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncResult {
    pub sent: usize,
    pub received: usize,
    /// Waited because a quiz or review is in progress (R121).
    pub deferred: bool,
    pub status: SyncStatus,
}

/// A thing to check (R123, R124), in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToCheck {
    pub n: i64,
    /// The record, so several fields of one record can be shown together.
    pub record: String,
    pub what: String,
    pub label: String,
    pub context: Option<String>,
    pub field: String,
    /// A delete that was undone because something changed on another computer.
    pub deleted: bool,
    pub kept: Option<String>,
    pub other: Option<String>,
    pub kept_by: String,
    /// Unix seconds.
    pub kept_when: i64,
    pub other_by: String,
    pub other_when: i64,
}

fn io(context: &str, error: std::io::Error) -> ApiError {
    ApiError::new(format!("{context}: {error}"))
}

fn is_device_id(name: &str) -> bool {
    name.len() == 8 && name.chars().all(|letter| letter.is_ascii_hexdigit())
}

fn is_change_file(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.len() == 13 && name.ends_with(".jsonl") && bytes[4] == b'-' && name[..4].chars().all(|digit| digit.is_ascii_digit()) && name[5..7].chars().all(|digit| digit.is_ascii_digit())
}

/// Seconds from a stamp, `0001730733120000-0000-1a2b3c4d`.
fn when(stamp: &str) -> i64 {
    stamp.split('-').next().and_then(|ms| ms.parse::<i64>().ok()).map_or(0, |ms| ms / 1000)
}

/// Writes a file whole or not at all, so a sync app never copies half of it.
fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path)
}

fn read_device(folder: &Path, device: &str) -> Option<DeviceFile> {
    std::fs::read_to_string(folder.join("devices").join(format!("{device}.json"))).ok().and_then(|json| serde_json::from_str(&json).ok())
}

fn default_name() -> String {
    let host = std::fs::read_to_string("/etc/hostname").ok().map(|name| name.trim().to_string()).filter(|name| !name.is_empty());
    host.unwrap_or_else(|| "This computer".into())
}

impl Sync {
    fn path(&self) -> Option<PathBuf> {
        self.home.as_ref().map(|home| home.join("sync.json"))
    }

    fn save(&self) -> ApiResult<()> {
        let Some(path) = self.path() else { return Ok(()) };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| io("couldn't save the sync settings", error))?;
        }
        let json = serde_json::to_string_pretty(&self.config).map_err(|error| ApiError::new(error.to_string()))?;
        write_whole(&path, json.as_bytes()).map_err(|error| io("couldn't save the sync settings", error))
    }

    fn folder(&self) -> Option<PathBuf> {
        self.config.folder.as_ref().map(PathBuf::from)
    }
}

/// Lines read from the folder, where to read from next time, and what couldn't be read.
struct FromFolder {
    lines: Vec<(String, Line)>,
    cursors: Vec<Cursor>,
    unreadable: BTreeMap<String, usize>,
    newer: Vec<String>,
}

impl App {
    /// Where the sync settings are kept. The shells set this when the app starts.
    pub fn set_sync_home(&mut self, dir: impl Into<PathBuf>) {
        let home = dir.into();
        let config = std::fs::read_to_string(home.join("sync.json")).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
        self.sync = Sync { home: Some(home), config, problem: None, default_name: self.sync.default_name.take() };
    }

    /// The name offered when setting up sync, before this computer has one.
    pub fn set_sync_default_name(&mut self, name: &str) {
        self.sync.default_name = Some(name.to_string());
    }

    fn sync_name(&self) -> String {
        self.sync.config.name.clone().or_else(|| self.sync.default_name.clone()).unwrap_or_else(default_name)
    }

    fn name_of(&self, device: &str) -> String {
        if device == self.store.device_id() || device.is_empty() {
            return self.sync_name();
        }
        self.sync.config.names.get(device).cloned().unwrap_or_else(|| format!("Computer {device}"))
    }

    /// This computer's change lines, from the outbox into its change file (R119).
    fn push(&mut self, folder: &Path, clock: Clock) -> ApiResult<usize> {
        let me = self.store.device_id();
        let pushed = read_device(folder, &me).map_or(0, |device| device.seq);
        let lines = self.store.sync_outbox(pushed)?;
        let last = self.store.sync_last_seq()?;
        let dir = folder.join("changes").join(&me);
        std::fs::create_dir_all(&dir).map_err(|error| io("couldn't write to the sync folder", error))?;
        let file_name = format!("{}.jsonl", &crate::backup::date_label(clock.day)[..7]);
        let path = dir.join(&file_name);
        if !lines.is_empty() {
            let mut text = lines.join("\n");
            text.push('\n');
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|error| io("couldn't write to the sync folder", error))?;
            file.write_all(text.as_bytes()).and_then(|()| file.sync_all()).map_err(|error| io("couldn't write to the sync folder", error))?;
            // This computer doesn't need to read its own lines back.
            let length = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            self.store.set_sync_cursor(&Cursor { device: me.clone(), file: file_name, offset: length })?;
        }
        let seen_before = read_device(folder, &me);
        let stale = seen_before.as_ref().is_none_or(|device| device.seq != last || device.name != self.sync_name() || clock.unix - device.seen_at > 300);
        if stale {
            let device = DeviceFile { format: FORMAT, device: me.clone(), name: self.sync_name(), seq: last, seen_at: clock.unix, app: env!("CARGO_PKG_VERSION").into() };
            std::fs::create_dir_all(folder.join("devices")).map_err(|error| io("couldn't write to the sync folder", error))?;
            let json = serde_json::to_string_pretty(&device).map_err(|error| ApiError::new(error.to_string()))?;
            write_whole(&folder.join("devices").join(format!("{me}.json")), json.as_bytes()).map_err(|error| io("couldn't write to the sync folder", error))?;
        }
        self.store.set_sync_meta("pushed", &last.to_string())?;
        Ok(lines.len())
    }

    /// Every computer's new lines, this one's too after a restore (R127).
    fn read_folder(&mut self, folder: &Path) -> ApiResult<FromFolder> {
        let cursors = self.store.sync_cursors()?;
        let mut read = FromFolder { lines: vec![], cursors: vec![], unreadable: BTreeMap::new(), newer: vec![] };
        let Ok(entries) = std::fs::read_dir(folder.join("changes")) else { return Ok(read) };
        let mut devices: Vec<String> = entries.filter_map(|entry| entry.ok()?.file_name().into_string().ok()).filter(|name| is_device_id(name)).collect();
        devices.sort();
        for device in devices {
            if let Some(file) = read_device(folder, &device) {
                if device != self.store.device_id() {
                    self.sync.config.names.insert(device.clone(), file.name.clone());
                }
                if file.format > FORMAT {
                    read.newer.push(device);
                    continue;
                }
            }
            let (done, from_file, from_offset) = cursors.get(&device).cloned().unwrap_or((0, None, 0));
            let dir = folder.join("changes").join(&device);
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            let mut files: Vec<String> = entries.filter_map(|entry| entry.ok()?.file_name().into_string().ok()).filter(|name| is_change_file(name)).collect();
            files.sort();
            for file_name in files.into_iter().filter(|name| from_file.as_ref().is_none_or(|from| name >= from)) {
                if read.lines.len() >= LINES_PER_SYNC {
                    break;
                }
                let mut start = if from_file.as_deref() == Some(file_name.as_str()) { from_offset } else { 0 };
                let path = dir.join(&file_name);
                let Ok(mut file) = std::fs::File::open(&path) else { continue };
                let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
                if length < start {
                    // Replaced by a shorter file: read it all again, lines already merged are skipped.
                    start = 0;
                }
                let mut bytes = vec![];
                if file.seek(SeekFrom::Start(start)).and_then(|_| file.read_to_end(&mut bytes)).is_err() {
                    continue;
                }
                // Only whole lines: the rest is still being written or copied (R127).
                let Some(end) = bytes.iter().rposition(|byte| *byte == b'\n') else { continue };
                let mut consumed = 0;
                for raw in bytes[..=end].split(|byte| *byte == b'\n') {
                    consumed += raw.len() + 1;
                    if consumed > end + 1 {
                        break;
                    }
                    if raw.iter().all(u8::is_ascii_whitespace) {
                        continue;
                    }
                    match serde_json::from_slice::<Line>(raw) {
                        Ok(line) if line.seq > done => read.lines.push((device.clone(), line)),
                        Ok(_) => {}
                        Err(_) => *read.unreadable.entry(device.clone()).or_default() += 1,
                    }
                    if read.lines.len() >= LINES_PER_SYNC {
                        break;
                    }
                }
                read.cursors.push(Cursor { device: device.clone(), file: file_name, offset: start + consumed.min(end + 1) as u64 });
            }
        }
        Ok(read)
    }

    /// Syncs now (R121). While a quiz or review is in progress the 30-second tick (`auto`)
    /// waits, and "Sync now" says to finish first, since a merge could change what's asked.
    pub fn sync_now(&mut self, auto: bool, clock: Clock) -> ApiResult<SyncResult> {
        let quiet = |app: &Self, deferred: bool| -> ApiResult<SyncResult> { Ok(SyncResult { sent: 0, received: 0, deferred, status: app.sync_status(clock)? }) };
        if !self.sync.config.on || self.sync.config.paused.is_some() {
            return quiet(self, false);
        }
        if self.active.is_some() || self.card_session.is_some() {
            if !auto {
                return Err(ApiError::new("finish the quiz or review first, then sync"));
            }
            return quiet(self, true);
        }
        let Some(folder) = self.sync.folder() else { return quiet(self, false) };
        let here = folder.is_dir();
        if here && let Some(device) = read_device(&folder, &self.store.device_id()) {
            self.store.sync_seq_floor(device.seq)?;
        }
        self.store.sync_collect()?;
        if !here {
            self.sync.problem = Some(format!("Can't find the sync folder at {}. Changes here are kept and go out once it's back.", folder.display()));
            return quiet(self, false);
        }
        let mut sent = self.push(&folder, clock)?;
        let read = self.read_folder(&folder)?;
        // Another computer's lines are only kept if everything can still be read afterwards.
        let merged = self.kept_only_if_readable(clock.day, "a change from another computer can't be read by this version of the app, so nothing was taken in. Your data here is as it was", |app| {
            let merged = app.store.sync_merge(&read.lines, &read.cursors, clock.day)?;
            if !merged.replay.is_empty() {
                app.store.sync_replay(&merged.replay)?;
            }
            // What the merge changed (a card worked out again, say) goes straight out.
            app.store.sync_collect()?;
            Ok(merged)
        })?;
        sent += self.push(&folder, clock)?;
        let mut problems = vec![];
        for device in &read.newer {
            problems.push(format!("{} has a newer version of the app, so its changes are skipped. Update this one to sync with it.", self.name_of(device)));
        }
        for (device, count) in &read.unreadable {
            problems.push(format!("{count} {} from {} couldn't be read and {} skipped.", if *count == 1 { "change" } else { "changes" }, self.name_of(device), if *count == 1 { "was" } else { "were" }));
        }
        self.sync.problem = (!problems.is_empty()).then(|| problems.join(" "));
        self.sync.config.last_synced = Some(clock.unix);
        self.sync.save()?;
        Ok(SyncResult { sent, received: merged.changes, deferred: false, status: self.sync_status(clock)? })
    }

    pub fn sync_status(&self, clock: Clock) -> ApiResult<SyncStatus> {
        let me = self.store.device_id();
        let folder = self.sync.folder();
        let last = self.store.sync_last_seq()?;
        let pushed: u64 = folder.as_deref().and_then(|folder| read_device(folder, &me)).map(|device| device.seq).or_else(|| self.store.sync_meta("pushed").ok().flatten().and_then(|seq| seq.parse().ok())).unwrap_or(0);
        let waiting = if self.sync.config.on { last.saturating_sub(pushed) } else { 0 };
        let applied = self.store.sync_applied()?;
        let mut devices = vec![SyncDevice { id: me.clone(), name: self.sync_name(), this_one: true, last_seen: Some(clock.unix), waiting, newer_app: false }];
        if let Some(folder) = folder.as_deref()
            && let Ok(entries) = std::fs::read_dir(folder.join("devices"))
        {
            let mut others: Vec<SyncDevice> = entries
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .filter_map(|name| name.strip_suffix(".json").map(String::from))
                .filter(|device| is_device_id(device) && *device != me)
                .filter_map(|device| read_device(folder, &device))
                .map(|file| SyncDevice { waiting: file.seq.saturating_sub(applied.get(&file.device).copied().unwrap_or(0)), id: file.device, name: file.name, this_one: false, last_seen: Some(file.seen_at), newer_app: file.format > FORMAT })
                .collect();
            others.sort_by(|left, right| right.last_seen.cmp(&left.last_seen).then(left.name.cmp(&right.name)));
            devices.extend(others);
        } else {
            devices.extend(self.sync.config.names.iter().map(|(id, name)| SyncDevice { id: id.clone(), name: name.clone(), this_one: false, last_seen: None, waiting: 0, newer_app: false }));
        }
        let conflicts = self.store.sync_conflicts()?;
        let records: std::collections::HashSet<(String, String)> = conflicts.iter().map(|conflict| (conflict.kind.clone(), conflict.uid.clone())).collect();
        Ok(SyncStatus {
            on: self.sync.config.on,
            paused: self.sync.config.paused.clone(),
            folder: self.sync.config.folder.clone(),
            device: me,
            name: self.sync_name(),
            devices,
            last_synced: self.sync.config.last_synced,
            problem: if self.sync.config.on { self.sync.problem.clone() } else { None },
            waiting,
            held: self.store.sync_waiting()?,
            to_check: records.len(),
        })
    }

    /// Turns sync on, or moves it to another folder (R120). Turning it on makes a backup
    /// first.
    pub fn sync_setup(&mut self, setup: SyncSetup, clock: Clock) -> ApiResult<SyncResult> {
        let folder = setup.folder.trim();
        let name = setup.name.trim();
        if !Path::new(folder).is_absolute() {
            return Err(ApiError::new("give the folder's full path, starting with /"));
        }
        if !Path::new(folder).is_dir() {
            return Err(ApiError::new(format!("there's no folder at {folder}")));
        }
        if name.is_empty() {
            return Err(ApiError::new("give this computer a name, so the others know which it is"));
        }
        if name.chars().count() > 40 {
            return Err(ApiError::new("keep the name to 40 letters or fewer"));
        }
        if !self.sync.config.on && self.backups.is_some() {
            self.make_backup(clock, Reason::BeforeSync)?;
        }
        self.sync.config.folder = Some(folder.to_string());
        self.sync.config.name = Some(name.to_string());
        self.sync.config.on = true;
        self.sync.config.paused = None;
        self.sync.problem = None;
        self.sync.save()?;
        self.sync_now(false, clock)
    }

    pub fn sync_rename(&mut self, name: &str) -> ApiResult<()> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err(ApiError::new("give this computer a name of 40 letters or fewer"));
        }
        self.sync.config.name = Some(name.to_string());
        self.sync.save()?;
        if let Some(folder) = self.sync.folder().filter(|folder| folder.is_dir()) {
            let me = self.store.device_id();
            if let Some(mut device) = read_device(&folder, &me) {
                device.name = name.to_string();
                let json = serde_json::to_string_pretty(&device).map_err(|error| ApiError::new(error.to_string()))?;
                write_whole(&folder.join("devices").join(format!("{me}.json")), json.as_bytes()).map_err(|error| io("couldn't write to the sync folder", error))?;
            }
        }
        Ok(())
    }

    pub fn sync_off(&mut self) -> ApiResult<SyncStatus> {
        self.sync.config.on = false;
        self.sync.config.paused = None;
        self.sync.problem = None;
        self.sync.save()?;
        self.sync_status(Clock { day: 0, minute: 0, unix: 0 })
    }

    /// Turns sync back on after a restore paused it (R128).
    pub fn sync_resume(&mut self, clock: Clock) -> ApiResult<SyncResult> {
        self.sync.config.paused = None;
        self.sync.save()?;
        self.sync_now(false, clock)
    }

    pub(crate) fn pause_sync_after_restore(&mut self) {
        if self.sync.config.on {
            self.sync.config.paused = Some("Paused after restoring a backup".into());
            let _ = self.sync.save();
        }
    }

    /// Things to check, in words (R123, R124).
    pub fn sync_to_check(&self) -> ApiResult<Vec<ToCheck>> {
        let conflicts = self.store.sync_conflicts()?;
        conflicts.iter().map(|conflict| self.describe(conflict)).collect()
    }

    /// Keeps the version in use (`use_other` false), or puts the other one back here and on
    /// the other computers (true). For a delete that was undone, `use_other` deletes it.
    pub fn sync_settle(&mut self, n: i64, use_other: bool, clock: Clock) -> ApiResult<Vec<ToCheck>> {
        let conflict = self.store.sync_conflicts()?.into_iter().find(|conflict| conflict.n == n).ok_or_else(|| ApiError::new("that's already been settled"))?;
        if !use_other {
            self.store.sync_dismiss(n)?;
        } else if conflict.kind == "notes" && conflict.field.starts_with("note.") {
            // A note's cards are made from it, so it goes through the same save as an edit.
            let id = self.store.local_id("notes", &conflict.uid)?.ok_or_else(|| ApiError::new("that card isn't here any more"))?;
            let (note, suggestion) = self.store.note(study_store::cards::NoteId(id))?;
            let mut value = serde_json::to_value(&note).map_err(|error| ApiError::new(error.to_string()))?;
            value[&conflict.field["note.".len()..]] = conflict.other.clone().unwrap_or(Value::Null);
            let note = serde_json::from_value(value).map_err(|_| ApiError::new("that version can't be put back"))?;
            self.save_note(Some(study_store::cards::NoteId(id)), crate::cards::NoteInput { note, suggestion }, clock.day)?;
            self.store.sync_dismiss(n)?;
        } else {
            self.store.sync_take_other(n, clock.day)?;
        }
        if self.sync.config.on && self.sync.config.paused.is_none() {
            self.sync_now(false, clock)?;
        }
        self.sync_to_check()
    }

    fn describe(&self, conflict: &Conflict) -> ApiResult<ToCheck> {
        let record = self.store.sync_record(&conflict.kind, &conflict.uid)?.unwrap_or_default();
        let (what, label) = self.label_of(&conflict.kind, &record);
        let shown = |value: &Option<Value>| value.as_ref().map(|value| self.shown(&conflict.kind, &conflict.field, value));
        Ok(ToCheck {
            n: conflict.n,
            record: format!("{}:{}", conflict.kind, conflict.uid),
            what: what.into(),
            label,
            context: self.context_of(&conflict.kind, &record),
            field: conflict.field.clone(),
            deleted: conflict.field == DELETED,
            kept: shown(&conflict.kept),
            other: shown(&conflict.other),
            kept_by: self.name_of(&conflict.kept_by),
            kept_when: when(&conflict.kept_at),
            other_by: self.name_of(&conflict.other_by),
            other_when: when(&conflict.other_at),
        })
    }

    fn label_of(&self, kind: &str, record: &serde_json::Map<String, Value>) -> (&'static str, String) {
        let text = |field: &str| record.get(field).and_then(Value::as_str).unwrap_or_default().to_string();
        match kind {
            "modules" => ("Module", text("name")),
            "topics" => ("Topic", text("name")),
            "sources" => ("Source", text("title")),
            "images" => ("Image", String::new()),
            "questions" => ("Question", text("text")),
            "notes" => ("Card", {
                let front = text("note.front");
                if front.is_empty() { text("note.text") } else { front }
            }),
            "cards" => ("Card", text("front")),
            "reviews" => ("Card review", String::new()),
            "checklists" => ("Checklist", text("subject_name")),
            "checks" => ("Recall check", String::new()),
            "attempts" => ("Quiz", String::new()),
            "projects" => ("Project", text("name")),
            "sessions" => ("Project session", String::new()),
            "issues" => ("Issue", text("name")),
            "drills" => ("Drill", text("task")),
            "drill_runs" => ("Drill run", String::new()),
            "struggles" => ("Struggle", text("what")),
            "dates" => ("Date", text("name")),
            "settings" => ("Setting", record.get("value").map(|_| String::new()).unwrap_or_default()),
            _ => ("Suggestion", String::new()),
        }
    }

    /// The name of what a record ID points at, from the synced records.
    fn name_by_uid(&self, kind: &str, uid: &str) -> Option<String> {
        let record = self.store.sync_record(kind, uid).ok()??;
        let (_, label) = self.label_of(kind, &record);
        (!label.is_empty()).then_some(label)
    }

    fn context_of(&self, kind: &str, record: &serde_json::Map<String, Value>) -> Option<String> {
        let uid = |field: &str| record.get(field).and_then(Value::as_str).map(String::from);
        let topic_and_module = |topic: &str| -> Option<String> {
            let topic_record = self.store.sync_record("topics", topic).ok()??;
            let name = topic_record.get("name").and_then(Value::as_str)?.to_string();
            let module = topic_record.get("module").and_then(Value::as_str).and_then(|module| self.name_by_uid("modules", module));
            Some(module.map_or(name.clone(), |module| format!("{name} · {module}")))
        };
        match kind {
            "questions" | "notes" | "drills" => topic_and_module(&uid("topic")?),
            "cards" => {
                let note = self.store.sync_record("notes", &uid("note")?).ok()??;
                topic_and_module(note.get("topic").and_then(Value::as_str)?)
            }
            "topics" => self.name_by_uid("modules", &uid("module")?),
            _ => None,
        }
    }

    /// A field's value in words.
    fn shown(&self, kind: &str, field: &str, value: &Value) -> String {
        let reference = match field {
            "module" => Some("modules"),
            "topic" => Some("topics"),
            "note" => Some("notes"),
            "project" => Some("projects"),
            "issue" => Some("issues"),
            "checklist" => Some("checklists"),
            _ => None,
        };
        match value {
            Value::Null => String::new(),
            Value::Bool(yes) => (if *yes { "Yes" } else { "No" }).into(),
            Value::String(text) => match reference {
                Some(target) if kind != "settings" => self.name_by_uid(target, text).unwrap_or_else(|| text.clone()),
                _ => text.clone(),
            },
            Value::Array(items) => items
                .iter()
                .map(|item| match item {
                    Value::String(text) if kind == "dates" && field == "topics" => self.name_by_uid("topics", text).unwrap_or_else(|| text.clone()),
                    Value::String(text) => text.clone(),
                    Value::Object(fields) if fields.contains_key("correct") => {
                        let text = fields.get("text").and_then(Value::as_str).unwrap_or_default();
                        if fields.get("correct").and_then(Value::as_bool).unwrap_or(false) { format!("{text} (right answer)") } else { text.to_string() }
                    }
                    Value::Object(fields) if fields.contains_key("source") => {
                        let source = fields.get("source").and_then(Value::as_str).and_then(|source| self.name_by_uid("sources", source)).unwrap_or_default();
                        let location = fields.get("location").and_then(Value::as_str).unwrap_or_default();
                        if location.is_empty() { source } else { format!("{source}, {location}") }
                    }
                    Value::Object(fields) => fields.get("text").and_then(Value::as_str).map_or_else(|| item.to_string(), String::from),
                    other => other.to_string(),
                })
                .collect::<Vec<_>>()
                .join(if items.iter().all(Value::is_string) { ", " } else { "\n" }),
            other => other.to_string(),
        }
    }
}

//! Backups (R101 to R108): a full copy of the database each day, by hand, and before a
//! restore or an app update, thinned out over time, optionally copied to a second folder.
//!
//! Files are named so they read well in a file manager and sort by date:
//! `study 2026-11-04 09-12 daily.sqlite`. Settings live in `settings.json` beside them,
//! so restoring a backup never changes how backups are kept (R107).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};
use study_store::backup::{Counts, counts_of, schema_version_of};

use crate::practice::Clock;
use crate::{ApiError, ApiResult, App};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    Daily,
    ByHand,
    BeforeRestore,
    BeforeUpdate,
    BeforeImport,
    BeforeSync,
}

impl Reason {
    fn slug(self) -> &'static str {
        match self {
            Reason::Daily => "daily",
            Reason::ByHand => "by-hand",
            Reason::BeforeRestore => "before-restore",
            Reason::BeforeUpdate => "before-update",
            Reason::BeforeImport => "before-import",
            Reason::BeforeSync => "before-sync",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        [Reason::Daily, Reason::ByHand, Reason::BeforeRestore, Reason::BeforeUpdate, Reason::BeforeImport, Reason::BeforeSync].into_iter().find(|reason| reason.slug() == slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackupInfo {
    /// The file name, which is also how a backup is picked for a restore.
    pub name: String,
    pub day: i64,
    /// Minutes after midnight.
    pub minute: u32,
    pub reason: Reason,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSettings {
    pub keep_days: u32,
    pub keep_weeks: u32,
    pub keep_months: u32,
    /// A second folder every backup is also copied to (R106).
    pub copy_to: Option<String>,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self { keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackupStatus {
    pub latest: Option<BackupInfo>,
    pub count: usize,
    pub bytes: u64,
    pub folder: String,
    pub settings: BackupSettings,
    /// Why the second folder isn't up to date, if it isn't (R106).
    pub copy_problem: Option<String>,
    /// Whether there's anything to back up yet (R102).
    pub has_data: bool,
}

/// One thing a restore would undo, like 183 card reviews (R105).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Undone {
    pub count: i64,
    /// What it is, for a count of one ("card review") and more ("card reviews").
    pub one: String,
    pub many: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RestorePreview {
    pub backup: BackupInfo,
    pub undone: Vec<Undone>,
    /// The backup is from an older version and will be upgraded.
    pub older: bool,
}

// ---------- Dates in file names ----------

fn epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date")
}

fn date_of(day: i64) -> NaiveDate {
    epoch() + Days::new(day.max(0) as u64)
}

/// "2026-11-04", for file names.
pub(crate) fn date_label(day: i64) -> String {
    date_of(day).format("%Y-%m-%d").to_string()
}

fn file_name(day: i64, minute: u32, reason: Reason, copy: usize) -> String {
    let suffix = if copy > 1 { format!(" {copy}") } else { String::new() };
    format!("study {} {:02}-{:02} {}{suffix}.sqlite", date_of(day).format("%Y-%m-%d"), minute / 60, minute % 60, reason.slug())
}

fn parse(name: &str) -> Option<(i64, u32, Reason)> {
    let stem = name.strip_prefix("study ")?.strip_suffix(".sqlite")?;
    let mut parts = stem.split(' ');
    let date = NaiveDate::parse_from_str(parts.next()?, "%Y-%m-%d").ok()?;
    let (hours, minutes) = parts.next()?.split_once('-')?;
    let reason = Reason::from_slug(parts.next()?)?;
    if let Some(copy) = parts.next() {
        copy.parse::<u32>().ok()?;
    }
    let minute = hours.parse::<u32>().ok()? * 60 + minutes.parse::<u32>().ok()?;
    Some(((date - epoch()).num_days(), minute, reason))
}

// ---------- Which backups to keep (R103) ----------

/// Which day, week or month a day falls in.
type PeriodOf = fn(i64) -> i64;

/// Names to keep: everything from today, and the newest backup of each of the most
/// recent `keep_days` days, `keep_weeks` weeks (Monday to Sunday) and `keep_months` months
/// that have a backup.
pub fn keep(backups: &[BackupInfo], settings: &BackupSettings, today: i64) -> HashSet<String> {
    // A stable sort, so backups from the same minute stay in the order they're given.
    let mut newest_first: Vec<&BackupInfo> = backups.iter().collect();
    newest_first.sort_by_key(|backup| std::cmp::Reverse((backup.day, backup.minute)));
    let mut kept: HashSet<String> = newest_first.iter().filter(|backup| backup.day >= today).map(|backup| backup.name.clone()).collect();
    let periods: [(u32, PeriodOf); 3] = [
        (settings.keep_days, |day| day),
        // Day 0 was a Thursday, so +3 makes weeks start on Monday.
        (settings.keep_weeks, |day| (day + 3).div_euclid(7)),
        (settings.keep_months, |day| {
            let date = date_of(day);
            date.year() as i64 * 12 + date.month0() as i64
        }),
    ];
    for (count, period) in periods {
        let mut seen = HashSet::new();
        for backup in &newest_first {
            if seen.len() >= count as usize && !seen.contains(&period(backup.day)) {
                break;
            }
            if seen.insert(period(backup.day)) {
                kept.insert(backup.name.clone());
            }
        }
    }
    kept
}

// ---------- A folder of backups ----------

#[derive(Debug, Clone)]
pub struct Backups {
    dir: PathBuf,
}

fn io(context: &str, error: std::io::Error) -> ApiError {
    ApiError::new(format!("{context}: {error}"))
}

fn list_in(dir: &Path) -> Vec<BackupInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut backups: Vec<(BackupInfo, std::time::SystemTime)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (day, minute, reason) = parse(&name)?;
            let meta = entry.metadata().ok()?;
            let made = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            Some((BackupInfo { name, day, minute, reason, bytes: meta.len() }, made))
        })
        .collect();
    // Newest first. Two made in the same minute are told apart by when the file was written.
    backups.sort_by(|(a, a_made), (b, b_made)| (b.day, b.minute, b_made).cmp(&(a.day, a.minute, a_made)));
    backups.into_iter().map(|(backup, _)| backup).collect()
}

fn prune_in(dir: &Path, settings: &BackupSettings, today: i64) {
    let backups = list_in(dir);
    let kept = keep(&backups, settings, today);
    for backup in backups.iter().filter(|backup| !kept.contains(&backup.name)) {
        let _ = std::fs::remove_file(dir.join(&backup.name));
    }
}

/// A file copied in whole or not at all: written under a temporary name, then renamed.
fn copy_whole(from: &Path, to: &Path) -> std::io::Result<()> {
    let partial = to.with_extension("partial");
    std::fs::copy(from, &partial)?;
    std::fs::rename(&partial, to)
}

impl Backups {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn list(&self) -> Vec<BackupInfo> {
        list_in(&self.dir)
    }

    pub fn settings(&self) -> BackupSettings {
        std::fs::read_to_string(self.dir.join("settings.json")).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
    }

    fn save_settings(&self, settings: &BackupSettings) -> ApiResult<()> {
        std::fs::create_dir_all(&self.dir).map_err(|error| io("couldn't make the backups folder", error))?;
        let json = serde_json::to_string_pretty(settings).map_err(|error| ApiError::new(error.to_string()))?;
        std::fs::write(self.dir.join("settings.json"), json).map_err(|error| io("couldn't save the backup settings", error))
    }

    fn free_name(&self, clock: Clock, reason: Reason) -> String {
        (1..).map(|copy| file_name(clock.day, clock.minute, reason, copy)).find(|name| !self.dir.join(name).exists()).expect("a free name")
    }

    /// Writes a new backup with `write`, which must create the file it's given, then
    /// copies it to the second folder and thins out both.
    fn add(&self, clock: Clock, reason: Reason, write: impl FnOnce(&Path) -> ApiResult<()>) -> ApiResult<BackupInfo> {
        std::fs::create_dir_all(&self.dir).map_err(|error| io("couldn't make the backups folder", error))?;
        let name = self.free_name(clock, reason);
        let path = self.dir.join(&name);
        let partial = path.with_extension("partial");
        let _ = std::fs::remove_file(&partial);
        if let Err(error) = write(&partial) {
            let _ = std::fs::remove_file(&partial);
            return Err(error);
        }
        std::fs::rename(&partial, &path).map_err(|error| io("couldn't finish the backup", error))?;
        let settings = self.settings();
        prune_in(&self.dir, &settings, clock.day);
        if let Some(other) = settings.copy_to.as_deref().map(Path::new) {
            // A second folder that can't be reached never stops the backup itself (R106).
            if other.is_dir() && copy_whole(&path, &other.join(&name)).is_ok() {
                prune_in(other, &settings, clock.day);
            }
        }
        let bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        Ok(BackupInfo { name, day: clock.day, minute: clock.minute, reason, bytes })
    }

    fn find(&self, name: &str) -> ApiResult<BackupInfo> {
        self.list().into_iter().find(|backup| backup.name == name).ok_or_else(|| ApiError::new("that backup isn't there any more"))
    }

    fn copy_problem(&self, settings: &BackupSettings, latest: Option<&BackupInfo>) -> Option<String> {
        let other = settings.copy_to.as_deref()?;
        if !Path::new(other).is_dir() {
            return Some(format!("Can't reach {other}, so backups are only being kept here"));
        }
        let latest = latest?;
        (!Path::new(other).join(&latest.name).is_file()).then(|| format!("The latest backup couldn't be copied to {other}"))
    }
}

/// Before an app update changes an existing database, copy it as it is (R102). Run by the
/// shells before opening the store. Returns the backup made, if one was needed.
pub fn before_update(database: &Path, dir: &Path, clock: Clock) -> ApiResult<Option<BackupInfo>> {
    let Ok(version) = schema_version_of(database) else { return Ok(None) };
    if version == 0 || version >= study_store::LATEST_SCHEMA {
        return Ok(None);
    }
    let backups = Backups::new(dir);
    backups
        .add(clock, Reason::BeforeUpdate, |partial| std::fs::copy(database, partial).map(|_| ()).map_err(|error| io("couldn't back up before updating", error)))
        .map(Some)
}

fn undone(now: Counts, then: Counts) -> Vec<Undone> {
    let pairs = [
        ("card review", "card reviews", now.card_reviews - then.card_reviews),
        ("quiz", "quizzes", now.quizzes - then.quizzes),
        ("recall check", "recall checks", now.recall_checks - then.recall_checks),
        ("drill run", "drill runs", now.drill_runs - then.drill_runs),
        ("struggle logged", "struggles logged", now.struggles - then.struggles),
        ("question added", "questions added", now.questions - then.questions),
        ("card added", "cards added", now.notes - then.notes),
        ("topic added", "topics added", now.topics - then.topics),
    ];
    pairs.into_iter().filter(|(_, _, count)| *count > 0).map(|(one, many, count)| Undone { count, one: one.into(), many: many.into() }).collect()
}

impl App {
    /// Where backups go. The shells set this when the app starts.
    pub fn set_backups(&mut self, dir: impl Into<PathBuf>) {
        self.backups = Some(Backups::new(dir));
    }

    fn backups_folder(&self) -> ApiResult<&Backups> {
        self.backups.as_ref().ok_or_else(|| ApiError::new("backups aren't set up in this version of the app"))
    }

    pub fn backups(&self) -> ApiResult<Vec<BackupInfo>> {
        Ok(self.backups_folder()?.list())
    }

    pub fn backup_status(&self) -> ApiResult<BackupStatus> {
        let backups = self.backups_folder()?;
        let list = backups.list();
        let settings = backups.settings();
        Ok(BackupStatus {
            copy_problem: backups.copy_problem(&settings, list.first()),
            latest: list.first().cloned(),
            count: list.len(),
            bytes: list.iter().map(|backup| backup.bytes).sum(),
            folder: backups.dir().display().to_string(),
            settings,
            has_data: self.store.has_data()?,
        })
    }

    pub(crate) fn make_backup(&self, clock: Clock, reason: Reason) -> ApiResult<BackupInfo> {
        let backups = self.backups_folder()?;
        backups.add(clock, reason, |partial| Ok(self.store.backup_into(partial)?))
    }

    /// "Back up now" (R104).
    pub fn back_up_now(&self, clock: Clock) -> ApiResult<BackupInfo> {
        self.make_backup(clock, Reason::ByHand)
    }

    /// The day's backup, once there's something to back up and nothing from today yet (R102).
    pub fn daily_backup(&self, clock: Clock) -> ApiResult<Option<BackupInfo>> {
        let backups = self.backups_folder()?;
        if !self.store.has_data()? || backups.list().iter().any(|backup| backup.day == clock.day) {
            return Ok(None);
        }
        self.make_backup(clock, Reason::Daily).map(Some)
    }

    /// What restoring `name` would undo (R105).
    pub fn restore_preview(&self, name: &str) -> ApiResult<RestorePreview> {
        let backups = self.backups_folder()?;
        let backup = backups.find(name)?;
        let path = backups.dir().join(name);
        let then = counts_of(&path)?;
        Ok(RestorePreview { undone: undone(self.store.counts()?, then), older: schema_version_of(&path)? < self.store.schema_version()?, backup })
    }

    /// Backs up the data as it is, then puts `name` back (R105). Returns the backup just
    /// made, so it can be undone.
    pub fn restore_backup(&mut self, name: &str, clock: Clock) -> ApiResult<BackupInfo> {
        let path = {
            let backups = self.backups_folder()?;
            backups.find(name)?;
            backups.dir().join(name)
        };
        schema_version_of(&path)?;
        let safety = self.make_backup(clock, Reason::BeforeRestore)?;
        self.store.restore_from(&path)?;
        self.forget_running_quiz()?;
        self.card_session = None;
        self.pause_sync_after_restore();
        Ok(safety)
    }

    /// How many to keep, and the second folder (R103, R106). A new second folder gets the
    /// latest backup straight away.
    pub fn set_backup_settings(&self, settings: BackupSettings, clock: Clock) -> ApiResult<BackupStatus> {
        if settings.keep_days == 0 {
            return Err(ApiError::new("keep at least 1 day of backups"));
        }
        if settings.keep_days > 365 || settings.keep_weeks > 520 || settings.keep_months > 240 {
            return Err(ApiError::new("that's more backups than the app keeps, try a smaller number"));
        }
        let copy_to = settings.copy_to.as_deref().map(str::trim).filter(|path| !path.is_empty()).map(String::from);
        if let Some(path) = copy_to.as_deref() {
            let folder = Path::new(path);
            if !folder.is_absolute() {
                return Err(ApiError::new("give the folder's full path, starting with /"));
            }
            if !folder.is_dir() {
                return Err(ApiError::new(format!("there's no folder at {path}")));
            }
            let backups = self.backups_folder()?;
            if folder == backups.dir() {
                return Err(ApiError::new("that's where backups already go, pick another folder or drive"));
            }
        }
        let settings = BackupSettings { copy_to, ..settings };
        let backups = self.backups_folder()?;
        backups.save_settings(&settings)?;
        prune_in(backups.dir(), &settings, clock.day);
        if let (Some(other), Some(latest)) = (settings.copy_to.as_deref().map(Path::new), backups.list().first()) {
            let target = other.join(&latest.name);
            if !target.exists() {
                copy_whole(&backups.dir().join(&latest.name), &target).map_err(|error| io(&format!("couldn't copy the latest backup to {}", other.display()), error))?;
            }
            prune_in(other, &settings, clock.day);
        }
        self.backup_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use study_store::Store;

    const TODAY: i64 = 20_761; // Wed 4 Nov 2026

    fn at(day: i64, minute: u32) -> Clock {
        Clock { day, minute, unix: day * 86_400 + minute as i64 * 60 }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("study-api-backup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app(name: &str) -> (App, PathBuf) {
        let dir = scratch(name);
        let mut app = App::with_seed(Store::in_memory().unwrap(), 1);
        app.set_backups(dir.join("backups"));
        (app, dir)
    }

    fn info(day: i64, minute: u32) -> BackupInfo {
        BackupInfo { name: file_name(day, minute, Reason::Daily, 1), day, minute, reason: Reason::Daily, bytes: 1 }
    }

    #[test]
    fn file_names_read_as_dates_and_parse_back() {
        let name = file_name(TODAY, 9 * 60 + 12, Reason::ByHand, 1);
        assert_eq!(name, "study 2026-11-04 09-12 by-hand.sqlite");
        assert_eq!(parse(&name), Some((TODAY, 552, Reason::ByHand)));
        assert_eq!(parse(&file_name(TODAY, 0, Reason::BeforeRestore, 2)), Some((TODAY, 0, Reason::BeforeRestore)));
        for junk in ["settings.json", "study 2026-11-04 09-12 daily.partial", "study nope 09-12 daily.sqlite", "study 2026-11-04 09-12 weekly.sqlite"] {
            assert_eq!(parse(junk), None, "{junk}");
        }
    }

    // backup.feature: Old backups are thinned out (R103)
    #[test]
    fn keeps_the_newest_of_each_recent_day_week_and_month() {
        let mut backups: Vec<BackupInfo> = (0..400).map(|ago| info(TODAY - ago, 9 * 60)).collect();
        backups.push(info(TODAY - 1, 8 * 60)); // an earlier one the same day as another
        backups.push(info(TODAY, 7 * 60)); // made today, always kept
        let kept = keep(&backups, &BackupSettings::default(), TODAY);
        let days: HashSet<i64> = backups.iter().filter(|backup| kept.contains(&backup.name)).map(|backup| backup.day).collect();
        for ago in 0..7 {
            assert!(days.contains(&(TODAY - ago)), "day {ago} ago");
        }
        assert!(!kept.contains(&file_name(TODAY - 1, 8 * 60, Reason::Daily, 1)), "only the newest of a day");
        assert!(kept.contains(&file_name(TODAY, 7 * 60, Reason::Daily, 1)), "anything from today stays");
        // Weeks run Monday to Sunday. The 7 days already cover this week and last, so the
        // other two weeks kept are the ones ending Sun 25 Oct and Sun 18 Oct.
        assert!(days.contains(&(TODAY - 10)) && days.contains(&(TODAY - 17)), "Sun 25 Oct and Sun 18 Oct");
        assert!(!days.contains(&(TODAY - 24)), "a fifth week back isn't kept for its week");
        // Months: the last day of each of the previous 12 months that have a backup.
        assert!(days.contains(&(TODAY - 4)), "31 Oct");
        let last_kept = *days.iter().min().unwrap();
        assert_eq!(date_of(last_kept).format("%Y-%m-%d").to_string(), "2025-12-31", "the 12th month back is Dec 2025");
        assert_eq!(kept.len(), 7 + 2 + 10 + 1, "{days:?}"); // 7 days, 2 more weeks, 10 more months, and the earlier one from today
    }

    #[test]
    fn fewer_backups_than_the_limits_are_all_kept() {
        let backups: Vec<BackupInfo> = [0, 3, 40].into_iter().map(|ago| info(TODAY - ago, 600)).collect();
        assert_eq!(keep(&backups, &BackupSettings::default(), TODAY).len(), 3);
        assert!(keep(&[], &BackupSettings::default(), TODAY).is_empty());
    }

    // backup.feature: The first backup of the day / An empty database isn't backed up (R102)
    #[test]
    fn the_daily_backup_waits_for_data_then_runs_once_a_day() {
        let (mut app, _) = app("daily");
        assert_eq!(app.daily_backup(at(TODAY, 600)).unwrap(), None, "nothing to back up yet");
        assert!(!app.backup_status().unwrap().has_data);
        app.create_topic("Caches", None).unwrap();
        let made = app.daily_backup(at(TODAY, 600)).unwrap().expect("a daily backup");
        assert_eq!((made.reason, made.day, made.minute), (Reason::Daily, TODAY, 600));
        assert!(made.bytes > 0);
        assert_eq!(app.daily_backup(at(TODAY, 900)).unwrap(), None, "one a day");
        assert_eq!(app.daily_backup(at(TODAY + 1, 30)).unwrap().map(|backup| backup.day), Some(TODAY + 1));
        let status = app.backup_status().unwrap();
        assert_eq!((status.count, status.latest.map(|backup| backup.day)), (2, Some(TODAY + 1)));
    }

    // backup.feature: Backing up by hand (R104)
    #[test]
    fn backing_up_by_hand_counts_as_todays_and_two_in_a_minute_both_survive() {
        let (app, _) = app("by-hand");
        let first = app.back_up_now(at(TODAY, 600)).unwrap();
        let second = app.back_up_now(at(TODAY, 600)).unwrap();
        assert_ne!(first.name, second.name);
        let daily_then_by_hand = {
            let mut other = App::with_seed(Store::in_memory().unwrap(), 1);
            other.set_backups(scratch("same-minute").join("backups"));
            other.create_topic("Caches", None).unwrap();
            other.daily_backup(at(TODAY, 600)).unwrap();
            other.back_up_now(at(TODAY, 600)).unwrap();
            other.backups().unwrap().iter().map(|backup| backup.reason).collect::<Vec<_>>()
        };
        assert_eq!(daily_then_by_hand, [Reason::ByHand, Reason::Daily], "newest first, even in the same minute");
        assert_eq!(app.backups().unwrap()[0].name, second.name);
        assert_eq!(app.backups().unwrap().iter().map(|backup| backup.reason).collect::<Vec<_>>(), [Reason::ByHand, Reason::ByHand]);
        assert_eq!(app.daily_backup(at(TODAY, 700)).unwrap(), None, "a backup by hand today is enough");
        let folder = app.backup_status().unwrap().folder;
        assert!(std::fs::read_dir(&folder).unwrap().flatten().all(|entry| !entry.file_name().to_string_lossy().ends_with(".partial")), "nothing half-written left behind");
    }

    // backup.feature: Restoring shows what will be undone / keeps a way back (R105)
    #[test]
    fn restoring_says_what_it_undoes_backs_up_first_and_puts_the_old_data_back() {
        let (mut app, _) = app("restore");
        app.create_topic("Caches", None).unwrap();
        let old = app.back_up_now(at(TODAY - 2, 600)).unwrap();
        app.create_topic("Pipelining", None).unwrap();
        app.create_topic("Branch prediction", None).unwrap();

        let preview = app.restore_preview(&old.name).unwrap();
        assert_eq!(preview.undone, [Undone { count: 2, one: "topic added".into(), many: "topics added".into() }]);
        assert!(!preview.older);

        let safety = app.restore_backup(&old.name, at(TODAY, 700)).unwrap();
        assert_eq!(safety.reason, Reason::BeforeRestore);
        assert_eq!(app.topics().unwrap().iter().map(|topic| topic.name.as_str()).collect::<Vec<_>>(), ["Caches"]);
        // And the restore can itself be undone
        assert_eq!(app.restore_preview(&safety.name).unwrap().undone, []);
        app.restore_backup(&safety.name, at(TODAY, 710)).unwrap();
        assert_eq!(app.topics().unwrap().len(), 3);
    }

    #[test]
    fn only_listed_backups_can_be_restored() {
        let (mut app, dir) = app("names");
        app.create_topic("Caches", None).unwrap();
        app.back_up_now(at(TODAY, 600)).unwrap();
        let elsewhere = dir.join("study 2026-11-04 10-00 daily.sqlite");
        std::fs::write(&elsewhere, b"x").unwrap();
        for name in ["../study 2026-11-04 10-00 daily.sqlite", "/etc/passwd", "settings.json", "study 2026-11-04 10-00 daily.sqlite"] {
            assert!(app.restore_preview(name).is_err(), "{name}");
            assert!(app.restore_backup(name, at(TODAY, 700)).is_err(), "{name}");
        }
        assert_eq!(app.backups().unwrap().len(), 1, "a refused restore makes no safety backup");
    }

    // backup.feature: An app update backs up first (R102)
    #[test]
    fn an_older_database_is_copied_before_it_is_upgraded() {
        let dir = scratch("update");
        let database = dir.join("study.sqlite");
        {
            let connection = study_store::backup::open_at_version(&database, 4).unwrap();
            connection.execute("INSERT INTO topics (name) VALUES ('From long ago')", []).unwrap();
        }
        let made = before_update(&database, &dir.join("backups"), at(TODAY, 600)).unwrap().expect("a backup");
        assert_eq!(made.reason, Reason::BeforeUpdate);
        assert_eq!(schema_version_of(&dir.join("backups").join(&made.name)).unwrap(), 4, "copied as it was");
        Store::open(&database).unwrap();
        assert_eq!(before_update(&database, &dir.join("backups"), at(TODAY, 601)).unwrap(), None, "already up to date");
        assert_eq!(before_update(&dir.join("new.sqlite"), &dir.join("backups"), at(TODAY, 602)).unwrap(), None, "a brand new database");
    }

    // backup.feature: Copying backups to a second folder (R106)
    #[test]
    fn backups_are_copied_to_a_second_folder_and_a_missing_one_is_reported() {
        let (mut app, dir) = app("copy");
        app.create_topic("Caches", None).unwrap();
        app.back_up_now(at(TODAY, 600)).unwrap();
        let usb = dir.join("usb");
        let settings = |copy_to: Option<&Path>| BackupSettings { copy_to: copy_to.map(|path| path.display().to_string()), ..BackupSettings::default() };
        assert!(app.set_backup_settings(settings(Some(&usb)), at(TODAY, 601)).is_err(), "the folder must exist");
        assert!(app.set_backup_settings(BackupSettings { copy_to: Some("relative/path".into()), ..BackupSettings::default() }, at(TODAY, 601)).is_err());
        std::fs::create_dir_all(&usb).unwrap();
        let status = app.set_backup_settings(settings(Some(&usb)), at(TODAY, 601)).unwrap();
        assert_eq!(status.copy_problem, None);
        assert_eq!(list_in(&usb).len(), 1, "the latest is copied straight away");
        let made = app.daily_backup(at(TODAY + 1, 600)).unwrap().unwrap();
        assert!(usb.join(&made.name).is_file());

        std::fs::remove_dir_all(&usb).unwrap();
        let unplugged = app.daily_backup(at(TODAY + 2, 600)).unwrap().expect("still backs up here");
        assert!(app.backups().unwrap().iter().any(|backup| backup.name == unplugged.name));
        assert!(app.backup_status().unwrap().copy_problem.unwrap().starts_with("Can't reach"));
        app.set_backup_settings(settings(None), at(TODAY + 2, 601)).unwrap();
        assert_eq!(app.backup_status().unwrap().copy_problem, None);
    }

    // backup.feature: Restoring doesn't change backup settings (R107)
    #[test]
    fn settings_live_beside_the_backups_and_survive_a_restore() {
        let (mut app, _) = app("settings");
        app.create_topic("Caches", None).unwrap();
        let old = app.back_up_now(at(TODAY - 1, 600)).unwrap();
        let wanted = BackupSettings { keep_days: 14, ..BackupSettings::default() };
        app.set_backup_settings(wanted.clone(), at(TODAY, 600)).unwrap();
        assert!(app.set_backup_settings(BackupSettings { keep_days: 0, ..BackupSettings::default() }, at(TODAY, 600)).is_err());
        app.restore_backup(&old.name, at(TODAY, 601)).unwrap();
        assert_eq!(app.backup_status().unwrap().settings, wanted);
    }
}

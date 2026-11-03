//! Storage for quiz schedules per topic, projects, sessions, struggles, issues and drills
//! (R87 to R96), and the per-day counts the calendar shows (R98).

use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use study_core::practice::{DrillRating, StruggleKind, issue_key, issue_name};
use study_core::question::TopicId;
use study_core::recall::{Position, Schedule};

use crate::recall::ChecklistId;
use crate::{AttemptId, Result, Store, StoreError};

const QUIZ_SCHEDULE_KEY: &str = "quiz_schedule";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StruggleId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IssueId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DrillId(pub i64);

/// Where a topic is on its quiz ladder, and when it's next due (R96).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicQuiz {
    pub topic: TopicId,
    pub position: Position,
    pub due: i64,
}

/// What one finished quiz did to one topic's schedule, kept so an overrule can redo it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizMove {
    pub attempt: AttemptId,
    pub topic: TopicId,
    pub day: i64,
    /// Where it was before, None if this quiz started its schedule.
    pub from: Option<(Position, i64)>,
    pub score: u8,
    pub passed: bool,
    pub to: Position,
    pub next_due: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: ProjectId,
    pub name: String,
    pub kind: String,
    pub created: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: SessionId,
    pub project: ProjectId,
    pub day: i64,
    /// Unix seconds.
    pub started: i64,
    pub ended: Option<i64>,
}

/// What a reviewed struggle became (R90).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StruggleOutcome {
    Card,
    Checklist(ChecklistId),
    Drill(DrillId),
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StruggleRecord {
    pub id: StruggleId,
    pub project: ProjectId,
    pub session: Option<SessionId>,
    pub day: i64,
    /// Minutes after midnight, local time.
    pub minute: u32,
    pub what: String,
    pub why: String,
    pub kind: StruggleKind,
    pub topic: Option<TopicId>,
    pub issue: Option<IssueId>,
    /// None until it's reviewed.
    pub outcome: Option<StruggleOutcome>,
}

/// A new struggle, before it has an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStruggle<'a> {
    pub project: ProjectId,
    pub session: SessionId,
    pub day: i64,
    pub minute: u32,
    pub what: &'a str,
    pub why: &'a str,
    pub kind: StruggleKind,
    pub topic: Option<TopicId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRecord {
    pub id: IssueId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrillRecord {
    pub id: DrillId,
    pub issue: Option<IssueId>,
    pub topic: Option<TopicId>,
    pub task: String,
    pub hints: String,
    pub created: i64,
    /// None once it's ticked off.
    pub due: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrillRun {
    pub drill: DrillId,
    pub day: i64,
    pub rating: DrillRating,
    pub seconds: u32,
    pub note: String,
}

/// What was done on one day, for the calendar's past days (R98).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoneCounts {
    pub cards: usize,
    pub checks: usize,
    pub quizzes: usize,
    pub drills: usize,
    pub struggles: usize,
}

fn kind_of(text: String) -> rusqlite::Result<StruggleKind> {
    StruggleKind::parse(&text).ok_or_else(|| rusqlite::Error::InvalidColumnType(0, text, rusqlite::types::Type::Text))
}

fn rating_of(text: String) -> rusqlite::Result<DrillRating> {
    DrillRating::parse(&text).ok_or_else(|| rusqlite::Error::InvalidColumnType(0, text, rusqlite::types::Type::Text))
}

const STRUGGLE_COLUMNS: &str = "id, project_id, session_id, day, minute, what, why, kind, topic_id, issue_id, outcome";
const DRILL_COLUMNS: &str = "id, issue_id, topic_id, task, hints, created, due";

fn struggle_row(row: &rusqlite::Row) -> rusqlite::Result<(StruggleRecord, Option<String>)> {
    Ok((
        StruggleRecord {
            id: StruggleId(row.get(0)?),
            project: ProjectId(row.get(1)?),
            session: row.get::<_, Option<i64>>(2)?.map(SessionId),
            day: row.get(3)?,
            minute: row.get(4)?,
            what: row.get(5)?,
            why: row.get(6)?,
            kind: kind_of(row.get(7)?)?,
            topic: row.get::<_, Option<i64>>(8)?.map(TopicId),
            issue: row.get::<_, Option<i64>>(9)?.map(IssueId),
            outcome: None,
        },
        row.get(10)?,
    ))
}

fn drill_row(row: &rusqlite::Row) -> rusqlite::Result<DrillRecord> {
    Ok(DrillRecord {
        id: DrillId(row.get(0)?),
        issue: row.get::<_, Option<i64>>(1)?.map(IssueId),
        topic: row.get::<_, Option<i64>>(2)?.map(TopicId),
        task: row.get(3)?,
        hints: row.get(4)?,
        created: row.get(5)?,
        due: row.get(6)?,
    })
}

impl Store {
    // ---------- Quiz schedules (R96) ----------

    pub fn quiz_schedule(&self) -> Result<Schedule> {
        let saved: Option<String> = self
            .connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [QUIZ_SCHEDULE_KEY], |row| row.get(0))
            .optional()?;
        Ok(match saved {
            Some(json) => serde_json::from_str(&json)?,
            None => Schedule::default(),
        })
    }

    pub fn set_quiz_schedule(&mut self, schedule: &Schedule) -> Result<()> {
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![QUIZ_SCHEDULE_KEY, serde_json::to_string(schedule)?],
        )?;
        Ok(())
    }

    pub fn reset_quiz_schedule(&mut self) -> Result<()> {
        self.connection.execute("DELETE FROM settings WHERE key = ?1", [QUIZ_SCHEDULE_KEY])?;
        Ok(())
    }

    pub fn topic_quizzes(&self) -> Result<Vec<TopicQuiz>> {
        let mut statement = self.connection.prepare("SELECT topic_id, step, rep, due FROM topic_schedule ORDER BY due, topic_id")?;
        let rows = statement.query_map([], |row| {
            Ok(TopicQuiz {
                topic: TopicId(row.get(0)?),
                position: Position { step: row.get::<_, i64>(1)? as usize, rep: row.get(2)? },
                due: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn topic_quiz(&self, topic: TopicId) -> Result<Option<TopicQuiz>> {
        Ok(self.topic_quizzes()?.into_iter().find(|quiz| quiz.topic == topic))
    }

    /// Records a quiz's move and puts the topic where it says.
    pub fn save_quiz_move(&mut self, quiz_move: &QuizMove) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO quiz_moves (attempt_id, topic_id, day, from_step, from_rep, from_due, score, passed, to_step, to_rep, next_due)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                quiz_move.attempt.0,
                quiz_move.topic.0,
                quiz_move.day,
                quiz_move.from.map(|(position, _)| position.step as i64),
                quiz_move.from.map(|(position, _)| position.rep),
                quiz_move.from.map(|(_, due)| due),
                quiz_move.score,
                quiz_move.passed,
                quiz_move.to.step as i64,
                quiz_move.to.rep,
                quiz_move.next_due
            ],
        )?;
        transaction.execute(
            "INSERT INTO topic_schedule (topic_id, step, rep, due) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(topic_id) DO UPDATE SET step = excluded.step, rep = excluded.rep, due = excluded.due",
            params![quiz_move.topic.0, quiz_move.to.step as i64, quiz_move.to.rep, quiz_move.next_due],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn quiz_moves(&self, attempt: AttemptId) -> Result<Vec<QuizMove>> {
        let mut statement = self.connection.prepare(
            "SELECT attempt_id, topic_id, day, from_step, from_rep, from_due, score, passed, to_step, to_rep, next_due
             FROM quiz_moves WHERE attempt_id = ?1 ORDER BY topic_id",
        )?;
        let rows = statement.query_map([attempt.0], |row| {
            let from_step: Option<i64> = row.get(3)?;
            let from_rep: Option<u32> = row.get(4)?;
            let from_due: Option<i64> = row.get(5)?;
            Ok(QuizMove {
                attempt: AttemptId(row.get(0)?),
                topic: TopicId(row.get(1)?),
                day: row.get(2)?,
                from: match (from_step, from_rep, from_due) {
                    (Some(step), Some(rep), Some(due)) => Some((Position { step: step as usize, rep }, due)),
                    _ => None,
                },
                score: row.get(6)?,
                passed: row.get(7)?,
                to: Position { step: row.get::<_, i64>(8)? as usize, rep: row.get(9)? },
                next_due: row.get(10)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The attempt whose move set the topic's current schedule, if any.
    pub fn latest_quiz_move(&self, topic: TopicId) -> Result<Option<AttemptId>> {
        Ok(self
            .connection
            .query_row("SELECT MAX(attempt_id) FROM quiz_moves WHERE topic_id = ?1", [topic.0], |row| row.get::<_, Option<i64>>(0))?
            .map(AttemptId))
    }

    // ---------- Projects and sessions (R87, R88) ----------

    pub fn create_project(&mut self, name: &str, kind: &str, day: i64) -> Result<ProjectId> {
        self.connection.execute("INSERT INTO projects (name, kind, created) VALUES (?1, ?2, ?3)", params![name, kind, day])?;
        Ok(ProjectId(self.connection.last_insert_rowid()))
    }

    pub fn projects(&self) -> Result<Vec<ProjectRecord>> {
        let mut statement = self.connection.prepare("SELECT id, name, kind, created FROM projects ORDER BY name COLLATE NOCASE")?;
        let rows = statement.query_map([], |row| {
            Ok(ProjectRecord { id: ProjectId(row.get(0)?), name: row.get(1)?, kind: row.get(2)?, created: row.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn project(&self, id: ProjectId) -> Result<ProjectRecord> {
        self.projects()?.into_iter().find(|project| project.id == id).ok_or(StoreError::NotFound("project"))
    }

    pub fn delete_project(&mut self, id: ProjectId) -> Result<()> {
        if self.connection.execute("DELETE FROM projects WHERE id = ?1", [id.0])? == 0 {
            return Err(StoreError::NotFound("project"));
        }
        Ok(())
    }

    pub fn sessions(&self, project: ProjectId) -> Result<Vec<SessionRecord>> {
        let mut statement = self.connection.prepare("SELECT id, project_id, day, started, ended FROM project_sessions WHERE project_id = ?1 ORDER BY started")?;
        let rows = statement.query_map([project.0], |row| {
            Ok(SessionRecord { id: SessionId(row.get(0)?), project: ProjectId(row.get(1)?), day: row.get(2)?, started: row.get(3)?, ended: row.get(4)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn session(&self, id: SessionId) -> Result<SessionRecord> {
        self.connection
            .query_row("SELECT id, project_id, day, started, ended FROM project_sessions WHERE id = ?1", [id.0], |row| {
                Ok(SessionRecord { id: SessionId(row.get(0)?), project: ProjectId(row.get(1)?), day: row.get(2)?, started: row.get(3)?, ended: row.get(4)? })
            })
            .optional()?
            .ok_or(StoreError::NotFound("session"))
    }

    pub fn running_session(&self, project: ProjectId) -> Result<Option<SessionRecord>> {
        Ok(self.sessions(project)?.into_iter().find(|session| session.ended.is_none()))
    }

    pub fn start_session(&mut self, project: ProjectId, day: i64, started: i64) -> Result<SessionId> {
        self.project(project)?;
        if self.running_session(project)?.is_some() {
            return Err(StoreError::Invalid("a session is already running on this project"));
        }
        self.connection.execute("INSERT INTO project_sessions (project_id, day, started) VALUES (?1, ?2, ?3)", params![project.0, day, started])?;
        Ok(SessionId(self.connection.last_insert_rowid()))
    }

    pub fn end_session(&mut self, id: SessionId, ended: i64) -> Result<()> {
        if self.connection.execute("UPDATE project_sessions SET ended = ?2 WHERE id = ?1 AND ended IS NULL", params![id.0, ended])? == 0 {
            return Err(StoreError::NotFound("running session"));
        }
        Ok(())
    }

    // ---------- Struggles (R89, R90) ----------

    pub fn add_struggle(&mut self, new: &NewStruggle) -> Result<StruggleId> {
        self.connection.execute(
            "INSERT INTO struggles (project_id, session_id, day, minute, what, why, kind, topic_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![new.project.0, new.session.0, new.day, new.minute, new.what, new.why, new.kind.as_str(), new.topic.map(|topic| topic.0)],
        )?;
        Ok(StruggleId(self.connection.last_insert_rowid()))
    }

    fn struggles_where(&self, filter: &str, values: impl rusqlite::Params) -> Result<Vec<StruggleRecord>> {
        let sql = format!("SELECT {STRUGGLE_COLUMNS} FROM struggles WHERE {filter} ORDER BY day DESC, minute DESC, id DESC");
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(values, struggle_row)?;
        rows.map(|row| {
            let (mut record, outcome) = row?;
            record.outcome = outcome.map(|json| serde_json::from_str(&json)).transpose()?;
            Ok(record)
        })
        .collect()
    }

    /// A project's struggles, newest first.
    pub fn struggles(&self, project: ProjectId) -> Result<Vec<StruggleRecord>> {
        self.struggles_where("project_id = ?1", [project.0])
    }

    pub fn all_struggles(&self) -> Result<Vec<StruggleRecord>> {
        self.struggles_where("1 = 1", [])
    }

    pub fn struggles_on(&self, day: i64) -> Result<Vec<StruggleRecord>> {
        self.struggles_where("day = ?1", [day])
    }

    pub fn struggle(&self, id: StruggleId) -> Result<StruggleRecord> {
        self.struggles_where("id = ?1", [id.0])?.pop().ok_or(StoreError::NotFound("struggle"))
    }

    pub fn review_struggle(&mut self, id: StruggleId, issue: Option<IssueId>, outcome: StruggleOutcome) -> Result<()> {
        let json = serde_json::to_string(&outcome)?;
        if self.connection.execute("UPDATE struggles SET issue_id = ?2, outcome = ?3 WHERE id = ?1", params![id.0, issue.map(|issue| issue.0), json])? == 0 {
            return Err(StoreError::NotFound("struggle"));
        }
        Ok(())
    }

    pub fn delete_struggle(&mut self, id: StruggleId) -> Result<()> {
        if self.connection.execute("DELETE FROM struggles WHERE id = ?1", [id.0])? == 0 {
            return Err(StoreError::NotFound("struggle"));
        }
        Ok(())
    }

    // ---------- Issues (R91, R94) ----------

    /// The issue with this name ignoring case and spacing, made if it's new.
    pub fn issue_named(&mut self, name: &str) -> Result<IssueId> {
        let name = issue_name(name);
        if name.is_empty() {
            return Err(StoreError::Invalid("name the issue, so you'd recognise it next time"));
        }
        self.connection.execute("INSERT OR IGNORE INTO issues (name, key) VALUES (?1, ?2)", params![name, issue_key(&name)])?;
        Ok(IssueId(self.connection.query_row("SELECT id FROM issues WHERE key = ?1", [issue_key(&name)], |row| row.get(0))?))
    }

    pub fn issues(&self) -> Result<Vec<IssueRecord>> {
        let mut statement = self.connection.prepare("SELECT id, name FROM issues ORDER BY name COLLATE NOCASE")?;
        let rows = statement.query_map([], |row| Ok(IssueRecord { id: IssueId(row.get(0)?), name: row.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---------- Drills (R92, R93) ----------

    pub fn add_drill(&mut self, issue: Option<IssueId>, topic: Option<TopicId>, task: &str, hints: &str, created: i64, due: i64) -> Result<DrillId> {
        self.connection.execute(
            "INSERT INTO drills (issue_id, topic_id, task, hints, created, due) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![issue.map(|issue| issue.0), topic.map(|topic| topic.0), task, hints, created, due],
        )?;
        Ok(DrillId(self.connection.last_insert_rowid()))
    }

    pub fn drills(&self) -> Result<Vec<DrillRecord>> {
        let sql = format!("SELECT {DRILL_COLUMNS} FROM drills ORDER BY COALESCE(due, 1e12), id");
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map([], drill_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn drill(&self, id: DrillId) -> Result<DrillRecord> {
        self.drills()?.into_iter().find(|drill| drill.id == id).ok_or(StoreError::NotFound("drill"))
    }

    /// The issue's most recent drill (R91).
    pub fn latest_drill(&self, issue: IssueId) -> Result<Option<DrillRecord>> {
        Ok(self.drills()?.into_iter().filter(|drill| drill.issue == Some(issue)).max_by_key(|drill| drill.id.0))
    }

    /// None ticks it off.
    pub fn set_drill_due(&mut self, id: DrillId, due: Option<i64>) -> Result<()> {
        if self.connection.execute("UPDATE drills SET due = ?2 WHERE id = ?1", params![id.0, due])? == 0 {
            return Err(StoreError::NotFound("drill"));
        }
        Ok(())
    }

    /// Records how a drill went and sets when it's next due (R93).
    pub fn record_drill_run(&mut self, run: &DrillRun) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO drill_runs (drill_id, day, rating, seconds, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run.drill.0, run.day, run.rating.as_str(), run.seconds, run.note],
        )?;
        if transaction.execute("UPDATE drills SET due = ?2 WHERE id = ?1", params![run.drill.0, run.rating.next_due(run.day)])? == 0 {
            return Err(StoreError::NotFound("drill"));
        }
        transaction.commit()?;
        Ok(())
    }

    fn runs_where(&self, filter: &str, values: impl rusqlite::Params) -> Result<Vec<DrillRun>> {
        let sql = format!("SELECT drill_id, day, rating, seconds, note FROM drill_runs WHERE {filter} ORDER BY day, id");
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(values, |row| {
            Ok(DrillRun { drill: DrillId(row.get(0)?), day: row.get(1)?, rating: rating_of(row.get(2)?)?, seconds: row.get(3)?, note: row.get(4)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn drill_runs(&self, drill: DrillId) -> Result<Vec<DrillRun>> {
        self.runs_where("drill_id = ?1", [drill.0])
    }

    pub fn drill_runs_on(&self, day: i64) -> Result<Vec<DrillRun>> {
        self.runs_where("day = ?1", [day])
    }

    // ---------- The calendar (R98) ----------

    /// What was done on each day from `from` to `to` inclusive. Days with nothing are left out.
    pub fn done_counts(&self, from: i64, to: i64) -> Result<BTreeMap<i64, DoneCounts>> {
        let mut counts: BTreeMap<i64, DoneCounts> = BTreeMap::new();
        type Field = fn(&mut DoneCounts) -> &mut usize;
        let tables: [(&str, Field); 5] = [
            ("card_reviews", |c| &mut c.cards),
            ("recall_checks", |c| &mut c.checks),
            ("attempts", |c| &mut c.quizzes),
            ("drill_runs", |c| &mut c.drills),
            ("struggles", |c| &mut c.struggles),
        ];
        for (table, field) in tables {
            let sql = format!("SELECT day, COUNT(*) FROM {table} WHERE day BETWEEN ?1 AND ?2 GROUP BY day");
            let mut statement = self.connection.prepare(&sql)?;
            let rows = statement.query_map([from, to], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as usize)))?;
            for row in rows {
                let (day, count) = row?;
                *field(counts.entry(day).or_default()) = count;
            }
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: i64 = 20_760;

    fn store_with_project() -> (Store, ProjectId) {
        let mut store = Store::in_memory().unwrap();
        let project = store.create_project("Study app", "Coding", TODAY).unwrap();
        (store, project)
    }

    // projects.feature: Work on a project in sessions
    #[test]
    fn one_session_at_a_time() {
        let (mut store, project) = store_with_project();
        let session = store.start_session(project, TODAY, 1_000).unwrap();
        assert!(matches!(store.start_session(project, TODAY, 1_010), Err(StoreError::Invalid(_))));
        assert_eq!(store.running_session(project).unwrap().map(|s| s.id), Some(session));
        store.end_session(session, 1_600).unwrap();
        assert_eq!(store.running_session(project).unwrap(), None);
        assert_eq!(store.session(session).unwrap().ended, Some(1_600));
        assert!(store.end_session(session, 1_700).is_err(), "can't end it twice");
        assert!(store.start_session(project, TODAY, 2_000).is_ok(), "a new one can start after");
        assert!(store.start_session(ProjectId(99), TODAY, 2_000).is_err());
    }

    // projects.feature: Log a struggle during a session, A recall miss becomes a card suggestion
    #[test]
    fn struggles_are_kept_newest_first_and_remember_their_review() {
        let (mut store, project) = store_with_project();
        let session = store.start_session(project, TODAY - 1, 1_000).unwrap();
        let new = |day, minute, what: &'static str| NewStruggle { project, session, day, minute, what, why: "because", kind: StruggleKind::Apply, topic: None };
        let older = store.add_struggle(&new(TODAY - 1, 600, "first")).unwrap();
        let newer = store.add_struggle(&new(TODAY, 30, "second")).unwrap();
        assert_eq!(store.struggles(project).unwrap().iter().map(|s| s.id).collect::<Vec<_>>(), vec![newer, older]);
        assert_eq!(store.struggle(older).unwrap().outcome, None);

        let issue = store.issue_named("Off-by-one in loops").unwrap();
        store.review_struggle(older, Some(issue), StruggleOutcome::Card).unwrap();
        let reviewed = store.struggle(older).unwrap();
        assert_eq!((reviewed.issue, reviewed.outcome), (Some(issue), Some(StruggleOutcome::Card)));
        assert_eq!(store.struggles_on(TODAY).unwrap().len(), 1);
        store.delete_struggle(newer).unwrap();
        assert_eq!(store.struggles(project).unwrap().len(), 1);

        store.delete_project(project).unwrap();
        assert!(store.all_struggles().unwrap().is_empty(), "struggles go with their project");
    }

    // projects.feature: A known issue offers its drill again
    #[test]
    fn issues_are_shared_and_matched_ignoring_case() {
        let mut store = Store::in_memory().unwrap();
        let first = store.issue_named("Off-by-one in loops").unwrap();
        assert_eq!(store.issue_named("  off-by-one IN   loops ").unwrap(), first);
        assert_ne!(store.issue_named("Mirror seams").unwrap(), first);
        assert_eq!(store.issues().unwrap().iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["Mirror seams", "Off-by-one in loops"]);
        assert!(store.issue_named("   ").is_err());
    }

    // projects.feature: Rating a drill
    #[test]
    fn drill_runs_set_the_next_day_or_tick_it_off() {
        let mut store = Store::in_memory().unwrap();
        let issue = store.issue_named("Mirror seams").unwrap();
        let drill = store.add_drill(Some(issue), None, "Mirror a mug", "", TODAY - 1, TODAY).unwrap();
        let run = |rating| DrillRun { drill, day: TODAY, rating, seconds: 300, note: "note".into() };
        store.record_drill_run(&run(DrillRating::Shaky)).unwrap();
        assert_eq!(store.drill(drill).unwrap().due, Some(TODAY + 1));
        store.record_drill_run(&DrillRun { day: TODAY + 1, ..run(DrillRating::Solid) }).unwrap();
        assert_eq!(store.drill(drill).unwrap().due, None);
        assert_eq!(store.drill_runs(drill).unwrap().iter().map(|r| r.rating).collect::<Vec<_>>(), vec![DrillRating::Shaky, DrillRating::Solid]);
        let newer = store.add_drill(Some(issue), None, "Mirror a cup", "", TODAY, TODAY + 2).unwrap();
        assert_eq!(store.latest_drill(issue).unwrap().map(|d| d.id), Some(newer));
        store.set_drill_due(newer, Some(TODAY + 5)).unwrap();
        assert_eq!(store.drill(newer).unwrap().due, Some(TODAY + 5));
    }

    // calendar.feature: A topic's quiz schedule moves with its score
    #[test]
    fn quiz_moves_set_the_topic_schedule_and_are_kept() {
        let mut store = Store::in_memory().unwrap();
        let topic = store.create_topic("CPU", None).unwrap();
        assert_eq!(store.topic_quiz(topic).unwrap(), None);
        let first = QuizMove { attempt: AttemptId(1), topic, day: TODAY, from: None, score: 80, passed: true, to: Position { step: 0, rep: 1 }, next_due: TODAY + 1 };
        store.save_quiz_move(&first).unwrap();
        let second = QuizMove { attempt: AttemptId(2), day: TODAY + 1, from: Some((Position { step: 0, rep: 1 }, TODAY + 1)), to: Position { step: 1, rep: 1 }, next_due: TODAY + 8, ..first };
        store.save_quiz_move(&second).unwrap();
        assert_eq!(store.topic_quiz(topic).unwrap(), Some(TopicQuiz { topic, position: Position { step: 1, rep: 1 }, due: TODAY + 8 }));
        assert_eq!(store.quiz_moves(AttemptId(2)).unwrap(), vec![second]);
        assert_eq!(store.latest_quiz_move(topic).unwrap(), Some(AttemptId(2)));
        assert_eq!(store.quiz_schedule().unwrap(), Schedule::default());
    }

    // calendar.feature: See what's when
    #[test]
    fn done_counts_per_day() {
        let (mut store, project) = store_with_project();
        let session = store.start_session(project, TODAY, 0).unwrap();
        for day in [TODAY - 2, TODAY - 2, TODAY] {
            store.add_struggle(&NewStruggle { project, session, day, minute: 0, what: "x", why: "y", kind: StruggleKind::Unsure, topic: None }).unwrap();
        }
        let drill = store.add_drill(None, None, "task", "", TODAY, TODAY).unwrap();
        store.record_drill_run(&DrillRun { drill, day: TODAY - 2, rating: DrillRating::Shaky, seconds: 1, note: String::new() }).unwrap();
        let counts = store.done_counts(TODAY - 3, TODAY - 1).unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[&(TODAY - 2)], DoneCounts { struggles: 2, drills: 1, ..DoneCounts::default() });
    }
}

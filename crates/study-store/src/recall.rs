//! Free recall storage (R52 to R60): checklists, their points, checks and the schedule.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use study_core::question::{QuestionId, TopicId};
use study_core::recall::{Position, Schedule};

use crate::cards::NoteId;
use crate::{ModuleId, Result, Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChecklistId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subject {
    Module(ModuleId),
    Topic(TopicId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub text: String,
    /// Set when the point was added from a question (R54).
    pub question: Option<QuestionId>,
    /// Set when the point was added from a flashcard note (R54).
    #[serde(default)]
    pub note: Option<NoteId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistRow {
    pub id: ChecklistId,
    /// None once the module or topic has been deleted. The name is kept.
    pub subject: Option<Subject>,
    pub subject_name: String,
    pub module: bool,
    pub created: i64,
    /// None until the first check.
    pub position: Option<Position>,
    pub due: i64,
    pub points: usize,
    pub last_score: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRecord {
    pub day: i64,
    /// What was typed, or None when the dump was done outside the app (R55).
    pub dump: Option<String>,
    /// Every point as it was at the time, and whether it was recalled.
    pub points: Vec<(String, bool)>,
    pub score: u8,
    pub passed: bool,
    pub from_label: Option<String>,
    pub to_label: String,
}

const SCHEDULE_KEY: &str = "recall_schedule";

impl Store {
    pub fn create_checklist(&mut self, subject: Subject, today: i64) -> Result<ChecklistId> {
        let (module, topic, name, kind): (Option<i64>, Option<i64>, String, &str) = match subject {
            Subject::Module(id) => {
                let name = self
                    .connection
                    .query_row("SELECT name FROM modules WHERE id = ?1", [id.0], |row| {
                        row.get(0)
                    })
                    .optional()?
                    .ok_or(StoreError::NotFound("module"))?;
                (Some(id.0), None, name, "Module")
            }
            Subject::Topic(id) => {
                let name = self
                    .connection
                    .query_row("SELECT name FROM topics WHERE id = ?1", [id.0], |row| {
                        row.get(0)
                    })
                    .optional()?
                    .ok_or(StoreError::NotFound("topic"))?;
                (None, Some(id.0), name, "Topic")
            }
        };
        self.connection.execute(
            "INSERT INTO recall_checklists (module_id, topic_id, subject_name, subject_kind, created, due) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![module, topic, name, kind, today],
        )?;
        Ok(ChecklistId(self.connection.last_insert_rowid()))
    }

    /// Replaces the points, in order (R53).
    pub fn set_points(&mut self, id: ChecklistId, points: &[Point]) -> Result<()> {
        let transaction = self.connection.transaction()?;
        if transaction
            .query_row(
                "SELECT 1 FROM recall_checklists WHERE id = ?1",
                [id.0],
                |_| Ok(()),
            )
            .optional()?
            .is_none()
        {
            return Err(StoreError::NotFound("checklist"));
        }
        transaction.execute("DELETE FROM recall_points WHERE checklist_id = ?1", [id.0])?;
        for (position, point) in points.iter().enumerate() {
            transaction.execute(
                "INSERT INTO recall_points (checklist_id, position, text, question_id, note_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id.0, position as i64, point.text, point.question.map(|q| q.0), point.note.map(|n| n.0)],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn points(&self, id: ChecklistId) -> Result<Vec<Point>> {
        let mut statement = self.connection.prepare(
            "SELECT text, question_id, note_id FROM recall_points WHERE checklist_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([id.0], |row| {
            Ok(Point {
                text: row.get(0)?,
                question: row.get::<_, Option<i64>>(1)?.map(QuestionId),
                note: row.get::<_, Option<i64>>(2)?.map(NoteId),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every checklist, soonest due first (R58).
    pub fn checklists(&self) -> Result<Vec<ChecklistRow>> {
        self.checklist_rows("1 = 1", params![])
    }

    pub fn checklist(&self, id: ChecklistId) -> Result<ChecklistRow> {
        self.checklist_rows("c.id = ?1", params![id.0])?
            .pop()
            .ok_or(StoreError::NotFound("checklist"))
    }

    fn checklist_rows(
        &self,
        filter: &str,
        values: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<ChecklistRow>> {
        let sql = format!(
            "SELECT c.id, c.module_id, c.topic_id, COALESCE(m.name, t.name, c.subject_name), c.subject_kind, c.created, c.step, c.rep, c.due,
                    (SELECT COUNT(*) FROM recall_points p WHERE p.checklist_id = c.id),
                    (SELECT score FROM recall_checks k WHERE k.checklist_id = c.id ORDER BY k.day DESC, k.id DESC LIMIT 1)
             FROM recall_checklists c
             LEFT JOIN modules m ON m.id = c.module_id
             LEFT JOIN topics t ON t.id = c.topic_id
             WHERE {filter} ORDER BY c.due, COALESCE(m.name, t.name, c.subject_name)"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(values, |row| {
            let module: Option<i64> = row.get(1)?;
            let topic: Option<i64> = row.get(2)?;
            let step: Option<i64> = row.get(6)?;
            let rep: Option<i64> = row.get(7)?;
            Ok(ChecklistRow {
                id: ChecklistId(row.get(0)?),
                subject: module
                    .map(|id| Subject::Module(ModuleId(id)))
                    .or(topic.map(|id| Subject::Topic(TopicId(id)))),
                subject_name: row.get(3)?,
                module: row.get::<_, String>(4)? == "Module",
                created: row.get(5)?,
                position: step.zip(rep).map(|(step, rep)| Position {
                    step: step as usize,
                    rep: rep as u32,
                }),
                due: row.get(8)?,
                points: row.get::<_, i64>(9)? as usize,
                last_score: row.get::<_, Option<i64>>(10)?.map(|score| score as u8),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Saves a finished check and moves the checklist to its new step and date.
    pub fn save_check(
        &mut self,
        id: ChecklistId,
        check: &CheckRecord,
        to: Position,
        due: i64,
    ) -> Result<()> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE recall_checklists SET step = ?2, rep = ?3, due = ?4 WHERE id = ?1",
            params![id.0, to.step as i64, to.rep as i64, due],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("checklist"));
        }
        transaction.execute(
            "INSERT INTO recall_checks (checklist_id, day, dump, points, score, passed, from_label, to_label) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id.0, check.day, check.dump, serde_json::to_string(&check.points)?, check.score as i64, check.passed, check.from_label, check.to_label],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Checks of one checklist, newest first (R59).
    pub fn checks(&self, id: ChecklistId) -> Result<Vec<CheckRecord>> {
        let mut statement = self.connection.prepare("SELECT day, dump, points, score, passed, from_label, to_label FROM recall_checks WHERE checklist_id = ?1 ORDER BY day DESC, id DESC")?;
        let rows = statement.query_map([id.0], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut checks = vec![];
        for row in rows {
            let (day, dump, points, score, passed, from_label, to_label) = row?;
            checks.push(CheckRecord {
                day,
                dump,
                points: serde_json::from_str(&points)?,
                score: score as u8,
                passed,
                from_label,
                to_label,
            });
        }
        Ok(checks)
    }

    pub fn delete_checklist(&mut self, id: ChecklistId) -> Result<()> {
        if self
            .connection
            .execute("DELETE FROM recall_checklists WHERE id = ?1", [id.0])?
            == 0
        {
            return Err(StoreError::NotFound("checklist"));
        }
        Ok(())
    }

    /// The saved schedule, or the default ladder (R60).
    pub fn schedule(&self) -> Result<Schedule> {
        let saved: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [SCHEDULE_KEY],
                |row| row.get(0),
            )
            .optional()?;
        Ok(match saved {
            Some(json) => serde_json::from_str(&json)?,
            None => Schedule::default(),
        })
    }

    pub fn set_schedule(&mut self, schedule: &Schedule) -> Result<()> {
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SCHEDULE_KEY, serde_json::to_string(schedule)?],
        )?;
        Ok(())
    }

    pub fn reset_schedule(&mut self) -> Result<()> {
        self.connection
            .execute("DELETE FROM settings WHERE key = ?1", [SCHEDULE_KEY])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: i64 = 20_760;

    fn point(text: &str) -> Point {
        Point {
            text: text.into(),
            question: None,
            note: None,
        }
    }

    fn check(day: i64, ticks: &[(&str, bool)], score: u8) -> CheckRecord {
        CheckRecord {
            day,
            dump: None,
            points: ticks.iter().map(|(t, r)| (t.to_string(), *r)).collect(),
            score,
            passed: score >= 70,
            from_label: None,
            to_label: "Day".into(),
        }
    }

    // free-recall.feature: Make a checklist about a module or a topic
    #[test]
    fn checklists_about_a_module_or_a_topic_are_due_the_day_they_are_made() {
        let mut store = Store::in_memory().unwrap();
        let module = store.create_module("Computer Architecture").unwrap();
        let topic = store.create_topic("CPU", Some(module)).unwrap();
        let a = store
            .create_checklist(Subject::Module(module), TODAY)
            .unwrap();
        let b = store
            .create_checklist(Subject::Topic(topic), TODAY)
            .unwrap();
        let rows = store.checklists().unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| (r.id, r.subject_name.as_str(), r.module, r.due, r.position))
                .collect::<Vec<_>>(),
            vec![
                (b, "CPU", false, TODAY, None),
                (a, "Computer Architecture", true, TODAY, None)
            ]
        );
        assert!(matches!(
            store.create_checklist(Subject::Topic(TopicId(99)), TODAY),
            Err(StoreError::NotFound(_))
        ));
    }

    // free-recall.feature: Add points from flashcards
    #[test]
    fn a_point_remembers_its_card_until_the_card_is_deleted() {
        use study_core::cards::{Note, NoteKind};
        let mut store = Store::in_memory().unwrap();
        let topic = store.create_topic("Memory", None).unwrap();
        let note = Note { topic, kind: NoteKind::Basic, front: "What caches translations?".into(), back: "The TLB".into(), cues: vec![], text: String::new(), extra: String::new() };
        let faces = note.faces().unwrap();
        let note_id = store.save_note(None, &note, &faces, None, TODAY).unwrap();
        let id = store.create_checklist(Subject::Topic(topic), TODAY).unwrap();
        let from_card = Point { text: "What caches translations? → The TLB".into(), question: None, note: Some(note_id) };
        store.set_points(id, &[from_card.clone(), point("Thrashing")]).unwrap();
        assert_eq!(store.points(id).unwrap(), vec![from_card, point("Thrashing")]);

        store.delete_note(note_id).unwrap();
        let points = store.points(id).unwrap();
        assert_eq!(points[0].text, "What caches translations? → The TLB", "the point stays");
        assert_eq!(points[0].note, None, "but no longer links to the card");
    }

    // free-recall.feature: Write, reorder and remove points
    #[test]
    fn points_are_kept_in_order() {
        let mut store = Store::in_memory().unwrap();
        let topic = store.create_topic("CPU", None).unwrap();
        let id = store
            .create_checklist(Subject::Topic(topic), TODAY)
            .unwrap();
        store
            .set_points(
                id,
                &[
                    point("Pipeline stages"),
                    point("Cache levels"),
                    point("Amdahl's law"),
                ],
            )
            .unwrap();
        store
            .set_points(id, &[point("Amdahl's law"), point("Pipeline stages")])
            .unwrap();
        let texts: Vec<_> = store
            .points(id)
            .unwrap()
            .into_iter()
            .map(|p| p.text)
            .collect();
        assert_eq!(texts, vec!["Amdahl's law", "Pipeline stages"]);
        assert_eq!(store.checklist(id).unwrap().points, 2);
    }

    // free-recall.feature: The step moves up..., History of a checklist, Due checklists come first
    #[test]
    fn checks_move_the_checklist_and_are_kept_newest_first() {
        let mut store = Store::in_memory().unwrap();
        let topic = store.create_topic("CPU", None).unwrap();
        let later = store
            .create_checklist(Subject::Topic(topic), TODAY)
            .unwrap();
        let soon = store
            .create_checklist(Subject::Topic(topic), TODAY)
            .unwrap();
        store
            .save_check(
                later,
                &check(TODAY, &[("A", true), ("B", false)], 50),
                Position { step: 0, rep: 1 },
                TODAY + 30,
            )
            .unwrap();
        store
            .save_check(
                later,
                &check(TODAY + 30, &[("A", true), ("B", true)], 100),
                Position { step: 1, rep: 1 },
                TODAY + 37,
            )
            .unwrap();
        store
            .save_check(
                soon,
                &check(TODAY, &[("A", true)], 100),
                Position { step: 0, rep: 1 },
                TODAY + 1,
            )
            .unwrap();
        let rows = store.checklists().unwrap();
        assert_eq!(
            rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![soon, later],
            "soonest due first"
        );
        assert_eq!(
            (rows[1].position, rows[1].last_score),
            (Some(Position { step: 1, rep: 1 }), Some(100))
        );
        let checks = store.checks(later).unwrap();
        assert_eq!(
            checks.iter().map(|c| c.score).collect::<Vec<_>>(),
            vec![100, 50]
        );
        assert_eq!(
            checks[1].points,
            vec![("A".to_string(), true), ("B".to_string(), false)]
        );
    }

    #[test]
    fn a_checklist_and_its_history_outlive_its_topic() {
        let mut store = Store::in_memory().unwrap();
        let topic = store.create_topic("CPU", None).unwrap();
        let id = store
            .create_checklist(Subject::Topic(topic), TODAY)
            .unwrap();
        store
            .save_check(
                id,
                &check(TODAY, &[("A", true)], 100),
                Position { step: 0, rep: 1 },
                TODAY + 1,
            )
            .unwrap();
        store.delete_topic(topic).unwrap();
        let row = store.checklist(id).unwrap();
        assert_eq!((row.subject, row.subject_name.as_str()), (None, "CPU"));
        assert_eq!(store.checks(id).unwrap().len(), 1);
        store.delete_checklist(id).unwrap();
        assert!(store.checklists().unwrap().is_empty());
    }

    // free-recall.feature: Edit and reset the schedule
    #[test]
    fn schedule_defaults_saves_and_resets() {
        let mut store = Store::in_memory().unwrap();
        assert_eq!(store.schedule().unwrap(), Schedule::default());
        let edited = Schedule {
            pass_mark: 80,
            ..Schedule::default()
        };
        store.set_schedule(&edited).unwrap();
        store.set_schedule(&edited).unwrap();
        assert_eq!(store.schedule().unwrap().pass_mark, 80);
        store.reset_schedule().unwrap();
        assert_eq!(store.schedule().unwrap(), Schedule::default());
    }
}

//! Storage for dates on the calendar (R130 to R132), and the "last done" days that
//! tightening is worked out from (R133).

use std::collections::HashMap;

use rusqlite::{Transaction, params};
use serde::{Deserialize, Serialize};

use study_core::question::TopicId;

use crate::practice::DrillId;
use crate::{ModuleId, Result, Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DateId(pub i64);

/// What a date covers (R131).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Covers {
    Nothing,
    /// Every topic in the module, including ones added later.
    Module(ModuleId),
    Topics(Vec<TopicId>),
}

/// A date as it's saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateInput {
    pub name: String,
    pub day: i64,
    pub kind: String,
    pub covers: Covers,
    /// Only kept when the date covers something (R132).
    pub tighten: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRecord {
    pub id: DateId,
    pub name: String,
    pub day: i64,
    pub kind: String,
    pub covers: Covers,
    pub tighten: bool,
}

impl DateRecord {
    /// The topics it covers now, given every topic and its module.
    pub fn topics(&self, topics: &[(TopicId, Option<ModuleId>)]) -> Vec<TopicId> {
        match &self.covers {
            Covers::Nothing => vec![],
            Covers::Module(module) => topics.iter().filter(|(_, of)| *of == Some(*module)).map(|(topic, _)| *topic).collect(),
            Covers::Topics(picked) => picked.clone(),
        }
    }
}

fn check(input: &DateInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(StoreError::Invalid("a date needs a name"));
    }
    Ok(())
}

fn write_covers(tx: &Transaction, id: i64, input: &DateInput) -> Result<()> {
    let module = match input.covers {
        Covers::Module(module) => Some(module.0),
        _ => None,
    };
    let picked: &[TopicId] = match &input.covers {
        Covers::Topics(topics) => topics,
        _ => &[],
    };
    let tighten = input.tighten && match &input.covers {
        Covers::Nothing => false,
        Covers::Module(_) => true,
        Covers::Topics(topics) => !topics.is_empty(),
    };
    tx.execute(
        "UPDATE dates SET name = ?2, day = ?3, kind = ?4, module_id = ?5, tighten = ?6 WHERE id = ?1 AND (name IS NOT ?2 OR day IS NOT ?3 OR kind IS NOT ?4 OR module_id IS NOT ?5 OR tighten IS NOT ?6)",
        params![id, input.name.trim(), input.day, input.kind.trim(), module, tighten],
    )?;
    let here: Vec<i64> = tx.prepare("SELECT topic_id FROM date_topics WHERE date_id = ?1")?.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    // Only touch rows that change, so saving the same topics doesn't count as an edit.
    for topic in &here {
        if !picked.iter().any(|picked| picked.0 == *topic) {
            tx.execute("DELETE FROM date_topics WHERE date_id = ?1 AND topic_id = ?2", params![id, topic])?;
        }
    }
    for topic in picked {
        if !here.contains(&topic.0) {
            tx.execute("INSERT INTO date_topics (date_id, topic_id) VALUES (?1, ?2)", params![id, topic.0])?;
        }
    }
    Ok(())
}

impl Store {
    pub fn dates(&self) -> Result<Vec<DateRecord>> {
        let mut statement = self.connection.prepare("SELECT id, name, day, kind, module_id, tighten FROM dates ORDER BY day, id")?;
        let rows: Vec<(i64, String, i64, String, Option<i64>, bool)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut picked: HashMap<i64, Vec<TopicId>> = HashMap::new();
        let mut statement = self.connection.prepare("SELECT date_id, topic_id FROM date_topics ORDER BY date_id, topic_id")?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))? {
            let (date, topic) = row?;
            picked.entry(date).or_default().push(TopicId(topic));
        }
        Ok(rows
            .into_iter()
            .map(|(id, name, day, kind, module, tighten)| {
                let covers = match (module, picked.remove(&id)) {
                    (Some(module), _) => Covers::Module(ModuleId(module)),
                    (None, Some(topics)) => Covers::Topics(topics),
                    (None, None) => Covers::Nothing,
                };
                // A date whose module or topics are all gone covers nothing, so it can't tighten.
                let tighten = tighten && covers != Covers::Nothing;
                DateRecord { id: DateId(id), name, day, kind, covers, tighten }
            })
            .collect())
    }

    pub fn date(&self, id: DateId) -> Result<DateRecord> {
        self.dates()?.into_iter().find(|date| date.id == id).ok_or(StoreError::NotFound("date"))
    }

    pub fn add_date(&mut self, input: &DateInput) -> Result<DateId> {
        check(input)?;
        let tx = self.connection.transaction()?;
        tx.execute("INSERT INTO dates (name, day) VALUES (?1, ?2)", params![input.name.trim(), input.day])?;
        let id = tx.last_insert_rowid();
        write_covers(&tx, id, input)?;
        tx.commit()?;
        Ok(DateId(id))
    }

    pub fn update_date(&mut self, id: DateId, input: &DateInput) -> Result<()> {
        check(input)?;
        let tx = self.connection.transaction()?;
        let found: i64 = tx.query_row("SELECT COUNT(*) FROM dates WHERE id = ?1", [id.0], |row| row.get(0))?;
        if found == 0 {
            return Err(StoreError::NotFound("date"));
        }
        write_covers(&tx, id.0, input)?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_date(&mut self, id: DateId) -> Result<()> {
        if self.connection.execute("DELETE FROM dates WHERE id = ?1", [id.0])? == 0 {
            return Err(StoreError::NotFound("date"));
        }
        Ok(())
    }

    /// The day each topic was last quizzed in a quiz that moved its schedule.
    pub fn last_quiz_days(&self) -> Result<HashMap<TopicId, i64>> {
        let mut statement = self.connection.prepare("SELECT topic_id, MAX(day) FROM quiz_moves GROUP BY topic_id")?;
        let rows = statement.query_map([], |row| Ok((TopicId(row.get(0)?), row.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The day each drill was last done, or made if it hasn't been done yet.
    pub fn last_drill_days(&self) -> Result<HashMap<DrillId, i64>> {
        let mut statement = self.connection.prepare("SELECT d.id, coalesce((SELECT MAX(r.day) FROM drill_runs r WHERE r.drill_id = d.id), d.created) FROM drills d")?;
        let rows = statement.query_map([], |row| Ok((DrillId(row.get(0)?), row.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_topics() -> (Store, ModuleId, TopicId, TopicId) {
        let mut store = Store::in_memory().unwrap();
        let module = store.create_module("Networks").unwrap();
        let osi = store.create_topic("OSI model", Some(module)).unwrap();
        let tcp = store.create_topic("TCP/IP", Some(module)).unwrap();
        (store, module, osi, tcp)
    }

    fn exam(covers: Covers) -> DateInput {
        DateInput { name: "Networks exam".into(), day: 23, kind: "Exam".into(), covers, tighten: true }
    }

    // dates.feature: Add a date, Edit and delete a date
    #[test]
    fn dates_are_added_edited_and_deleted() {
        let (mut store, ..) = store_with_topics();
        let id = store.add_date(&DateInput { name: " Study app write-up ".into(), day: 13, kind: "Deadline".into(), covers: Covers::Nothing, tighten: false }).unwrap();
        let date = store.date(id).unwrap();
        assert_eq!((date.name.as_str(), date.day, date.kind.as_str(), &date.covers), ("Study app write-up", 13, "Deadline", &Covers::Nothing));
        store.update_date(id, &DateInput { name: "Study app write-up".into(), day: 16, kind: "Deadline".into(), covers: Covers::Nothing, tighten: false }).unwrap();
        assert_eq!(store.date(id).unwrap().day, 16);
        store.delete_date(id).unwrap();
        assert!(store.dates().unwrap().is_empty());
        assert!(matches!(store.delete_date(id), Err(StoreError::NotFound(_))));
    }

    #[test]
    fn a_date_needs_a_name() {
        let (mut store, ..) = store_with_topics();
        let mut input = exam(Covers::Nothing);
        input.name = "  ".into();
        assert!(matches!(store.add_date(&input), Err(StoreError::Invalid(_))));
    }

    // dates.feature: Tightening is on by default for exams (it can't be ticked without covers)
    #[test]
    fn covering_nothing_never_tightens() {
        let (mut store, ..) = store_with_topics();
        let id = store.add_date(&exam(Covers::Nothing)).unwrap();
        assert!(!store.date(id).unwrap().tighten);
        let id = store.add_date(&exam(Covers::Topics(vec![]))).unwrap();
        assert_eq!(store.date(id).unwrap().covers, Covers::Nothing);
        assert!(!store.date(id).unwrap().tighten);
    }

    // dates.feature: A date covering a module covers topics added later
    #[test]
    fn a_module_covers_topics_added_later() {
        let (mut store, module, osi, tcp) = store_with_topics();
        let id = store.add_date(&exam(Covers::Module(module))).unwrap();
        let routing = store.create_topic("Routing", Some(module)).unwrap();
        let topics: Vec<(TopicId, Option<ModuleId>)> = store.topics().unwrap().into_iter().map(|t| (t.id, t.module)).collect();
        let mut covered = store.date(id).unwrap().topics(&topics);
        covered.sort();
        assert_eq!(covered, vec![osi, tcp, routing]);
    }

    // dates.feature: A deleted topic drops out
    #[test]
    fn deleted_topics_and_modules_drop_out() {
        let (mut store, module, osi, tcp) = store_with_topics();
        let picked = store.add_date(&exam(Covers::Topics(vec![osi, tcp]))).unwrap();
        let whole = store.add_date(&exam(Covers::Module(module))).unwrap();
        store.delete_topic(tcp).unwrap();
        assert_eq!(store.date(picked).unwrap().covers, Covers::Topics(vec![osi]));
        store.delete_topic(osi).unwrap();
        assert_eq!(store.date(picked).unwrap().covers, Covers::Nothing);
        assert!(!store.date(picked).unwrap().tighten);
        store.delete_module(module).unwrap();
        assert_eq!(store.date(whole).unwrap().covers, Covers::Nothing);
    }

    // R110: changing what a date covers moves its stamp, saving the same doesn't
    #[test]
    fn changing_the_topics_moves_the_stamp() {
        let (mut store, _, osi, tcp) = store_with_topics();
        let id = store.add_date(&exam(Covers::Topics(vec![osi]))).unwrap();
        let stamp = |store: &Store| store.connection.query_row("SELECT changed FROM dates WHERE id = ?1", [id.0], |row| row.get::<_, String>(0)).unwrap();
        let first = stamp(&store);
        store.update_date(id, &exam(Covers::Topics(vec![osi]))).unwrap();
        assert_eq!(stamp(&store), first);
        store.update_date(id, &exam(Covers::Topics(vec![osi, tcp]))).unwrap();
        assert!(stamp(&store) > first);
    }

    #[test]
    fn last_done_days() {
        let (mut store, ..) = store_with_topics();
        let drill = store.add_drill(None, None, "Loop without off-by-one", "", 5, 6).unwrap();
        assert_eq!(store.last_drill_days().unwrap()[&drill], 5);
        assert!(store.last_quiz_days().unwrap().is_empty());
    }
}

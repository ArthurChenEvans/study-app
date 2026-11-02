//! SQLite storage for the Study App.
//!
//! History is stored as snapshots, separate from the live questions, so deleting or
//! editing a topic or question never changes past attempts (R28, R30).
//!
//! Not implemented because they're open questions in specs/README.md:
//! deleting a module (Q9), deleting or editing a cited source (Q10, Q11),
//! resuming an incomplete attempt (Q13).

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use study_core::attempt::{Attempt, Status};
use study_core::bibliography::{SourceFields, apa};
use study_core::progress::AttemptRecord;
use study_core::question::{
    AnswerKind, AnswerOption, Citation, Question, QuestionDraft, QuestionId, SourceId, TopicId,
};
use study_core::results::{Reasons, Summary, reason_labels, summarise};

mod schema;
pub mod search;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AttemptId(pub i64);

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Invalid(&'static str),
    #[error(
        "the database was made by a newer version of the app (schema {found}, this app knows up to {known})"
    )]
    NewerSchema { found: i64, known: i64 },
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleRow {
    pub id: ModuleId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRow {
    pub id: TopicId,
    pub name: String,
    pub module: Option<ModuleId>,
    pub tags: Vec<String>,
    pub question_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRow {
    pub id: SourceId,
    pub fields: SourceFields,
    pub apa: String,
    /// Topics with at least one question citing this source (R13).
    pub used_by: Vec<TopicId>,
    pub cited_by_questions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRow {
    pub id: AttemptId,
    pub day: i64,
    pub status: Status,
    pub retry_of: Option<AttemptId>,
    pub topics: Vec<TopicId>,
    pub summary: Summary,
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", true)?;
        schema::migrate(&connection)?;
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    // ---------- Modules and topics (R1, R2, R3, R30) ----------

    pub fn create_module(&mut self, name: &str) -> Result<ModuleId> {
        self.connection
            .execute("INSERT INTO modules (name) VALUES (?1)", [name.trim()])?;
        Ok(ModuleId(self.connection.last_insert_rowid()))
    }

    pub fn modules(&self) -> Result<Vec<ModuleRow>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, name FROM modules ORDER BY name")?;
        let rows = statement.query_map([], |row| {
            Ok(ModuleRow {
                id: ModuleId(row.get(0)?),
                name: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// R43: topics in the module move to "No module". Nothing else is deleted.
    pub fn delete_module(&mut self, module: ModuleId) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("UPDATE topics SET module_id = NULL WHERE module_id = ?1", [module.0])?;
        if tx.execute("DELETE FROM modules WHERE id = ?1", [module.0])? == 0 {
            return Err(StoreError::NotFound("module"));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn create_topic(&mut self, name: &str, module: Option<ModuleId>) -> Result<TopicId> {
        self.connection.execute(
            "INSERT INTO topics (name, module_id) VALUES (?1, ?2)",
            params![name.trim(), module.map(|m| m.0)],
        )?;
        Ok(TopicId(self.connection.last_insert_rowid()))
    }

    pub fn move_topic(&mut self, topic: TopicId, module: Option<ModuleId>) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE topics SET module_id = ?2 WHERE id = ?1",
            params![topic.0, module.map(|m| m.0)],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("topic"));
        }
        Ok(())
    }

    pub fn set_topic_tags(&mut self, topic: TopicId, tags: &[&str]) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM topic_tags WHERE topic_id = ?1", [topic.0])?;
        for tag in tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
        {
            transaction.execute(
                "INSERT OR IGNORE INTO topic_tags (topic_id, tag) VALUES (?1, ?2)",
                params![topic.0, tag],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn topics(&self) -> Result<Vec<TopicRow>> {
        let mut statement = self.connection.prepare(
            "SELECT t.id, t.name, t.module_id, (SELECT COUNT(*) FROM questions q WHERE q.topic_id = t.id) FROM topics t ORDER BY t.name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TopicRow {
                id: TopicId(row.get(0)?),
                name: row.get(1)?,
                module: row.get::<_, Option<i64>>(2)?.map(ModuleId),
                tags: vec![],
                question_count: row.get::<_, i64>(3)? as usize,
            })
        })?;
        let mut topics: Vec<TopicRow> = rows.collect::<rusqlite::Result<_>>()?;
        let mut tags = self
            .connection
            .prepare("SELECT tag FROM topic_tags WHERE topic_id = ?1 ORDER BY tag")?;
        for topic in &mut topics {
            topic.tags = tags
                .query_map([topic.id.0], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
        }
        Ok(topics)
    }

    pub fn delete_topic(&mut self, topic: TopicId) -> Result<()> {
        let changed = self
            .connection
            .execute("DELETE FROM topics WHERE id = ?1", [topic.0])?;
        if changed == 0 {
            return Err(StoreError::NotFound("topic"));
        }
        Ok(())
    }

    // ---------- Questions (R4 to R10, R14) ----------

    /// The question has already passed the core's rules, so it's stored as is.
    pub fn save_question(&mut self, question: &Question) -> Result<QuestionId> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO questions (topic_id, kind, text, explanation) VALUES (?1, ?2, ?3, ?4)",
            params![
                question.topic().0,
                kind_name(question.kind()),
                question.text(),
                question.explanation()
            ],
        )?;
        let id = QuestionId(transaction.last_insert_rowid());
        write_question_parts(&transaction, id, question)?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn update_question(&mut self, id: QuestionId, question: &Question) -> Result<()> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE questions SET topic_id = ?2, kind = ?3, text = ?4, explanation = ?5, edited_at = datetime('now') WHERE id = ?1",
            params![id.0, question.topic().0, kind_name(question.kind()), question.text(), question.explanation()],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("question"));
        }
        for table in ["options", "question_tags", "citations"] {
            transaction.execute(
                &format!("DELETE FROM {table} WHERE question_id = ?1"),
                [id.0],
            )?;
        }
        write_question_parts(&transaction, id, question)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn question(&self, id: QuestionId) -> Result<Question> {
        let head: Option<(i64, String, String, String)> = self
            .connection
            .query_row(
                "SELECT topic_id, kind, text, explanation FROM questions WHERE id = ?1",
                [id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let (topic, kind, text, explanation) = head.ok_or(StoreError::NotFound("question"))?;
        let options = self
            .connection
            .prepare("SELECT text, correct FROM options WHERE question_id = ?1 ORDER BY position")?
            .query_map([id.0], |row| {
                Ok(AnswerOption {
                    text: row.get(0)?,
                    correct: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let tags = self
            .connection
            .prepare("SELECT tag FROM question_tags WHERE question_id = ?1 ORDER BY rowid")?
            .query_map([id.0], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let citations = self
            .connection
            .prepare("SELECT source_id, location FROM citations WHERE question_id = ?1 ORDER BY position")?
            .query_map([id.0], |row| Ok(Citation { source: SourceId(row.get(0)?), location: row.get(1)? }))?
            .collect::<rusqlite::Result<_>>()?;
        let kind = if kind == "multiple" {
            AnswerKind::Multiple
        } else {
            AnswerKind::Single
        };
        Question::new(QuestionDraft {
            topic: TopicId(topic),
            text,
            kind,
            options,
            explanation,
            tags,
            citations,
        })
        .map_err(|_| StoreError::NotFound("valid question"))
    }

    pub fn questions_in(&self, topics: &[TopicId]) -> Result<Vec<(QuestionId, Question)>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM questions WHERE topic_id = ?1 ORDER BY id")?;
        let mut found = vec![];
        for topic in topics {
            let ids: Vec<i64> = statement
                .query_map([topic.0], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for id in ids {
                found.push((QuestionId(id), self.question(QuestionId(id))?));
            }
        }
        Ok(found)
    }

    pub fn delete_question(&mut self, id: QuestionId) -> Result<()> {
        let changed = self
            .connection
            .execute("DELETE FROM questions WHERE id = ?1", [id.0])?;
        if changed == 0 {
            return Err(StoreError::NotFound("question"));
        }
        Ok(())
    }

    // ---------- Bibliography (R13) ----------

    pub fn add_source(&mut self, fields: &SourceFields) -> Result<SourceId> {
        check_source(fields)?;
        self.connection.execute(
            "INSERT INTO sources (authors, year, title, edition, description, publisher, url) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![fields.authors, fields.year, fields.title, fields.edition, fields.description, fields.publisher, fields.url],
        )?;
        Ok(SourceId(self.connection.last_insert_rowid()))
    }

    /// R45: questions show the new details. History keeps its own snapshot.
    pub fn update_source(&mut self, id: SourceId, fields: &SourceFields) -> Result<()> {
        check_source(fields)?;
        let changed = self.connection.execute(
            "UPDATE sources SET authors = ?2, year = ?3, title = ?4, edition = ?5, description = ?6, publisher = ?7, url = ?8 WHERE id = ?1",
            params![id.0, fields.authors, fields.year, fields.title, fields.edition, fields.description, fields.publisher, fields.url],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("source"));
        }
        Ok(())
    }

    /// R44: removes the source and its citations, returning how many questions cited it.
    pub fn delete_source(&mut self, id: SourceId) -> Result<usize> {
        let tx = self.connection.transaction()?;
        let cited: i64 = tx.query_row(
            "SELECT COUNT(DISTINCT question_id) FROM citations WHERE source_id = ?1",
            [id.0],
            |row| row.get(0),
        )?;
        tx.execute("DELETE FROM citations WHERE source_id = ?1", [id.0])?;
        if tx.execute("DELETE FROM sources WHERE id = ?1", [id.0])? == 0 {
            return Err(StoreError::NotFound("source"));
        }
        tx.commit()?;
        Ok(cited as usize)
    }

    pub fn sources(&self) -> Result<Vec<SourceRow>> {
        let mut statement = self.connection.prepare("SELECT id, authors, year, title, edition, description, publisher, url FROM sources ORDER BY authors, year")?;
        let rows = statement.query_map([], |row| {
            Ok((
                SourceId(row.get(0)?),
                SourceFields {
                    authors: row.get(1)?,
                    year: row.get(2)?,
                    title: row.get(3)?,
                    edition: row.get(4)?,
                    description: row.get(5)?,
                    publisher: row.get(6)?,
                    url: row.get(7)?,
                },
            ))
        })?;
        let mut used = self.connection.prepare("SELECT DISTINCT q.topic_id FROM citations c JOIN questions q ON q.id = c.question_id WHERE c.source_id = ?1 ORDER BY q.topic_id")?;
        let mut sources = vec![];
        for row in rows {
            let (id, fields) = row?;
            let used_by = used
                .query_map([id.0], |row| Ok(TopicId(row.get(0)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let cited: i64 = self.connection.query_row(
                "SELECT COUNT(DISTINCT question_id) FROM citations WHERE source_id = ?1",
                [id.0],
                |row| row.get(0),
            )?;
            sources.push(SourceRow {
                id,
                apa: apa(&fields),
                fields,
                used_by,
                cited_by_questions: cited as usize,
            });
        }
        Ok(sources)
    }

    // ---------- Attempts and history (R24 to R29) ----------

    pub fn save_attempt(
        &mut self,
        attempt: &Attempt,
        day: i64,
        reasons: &Reasons,
    ) -> Result<AttemptId> {
        self.connection.execute(
            "INSERT INTO attempts (day, status, retry_of, attempt, reasons) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![day, serde_json::to_string(&attempt.status())?, attempt.retry_of().map(|id| id as i64), serde_json::to_string(attempt)?, serde_json::to_string(reasons)?],
        )?;
        Ok(AttemptId(self.connection.last_insert_rowid()))
    }

    pub fn save_reasons(&mut self, id: AttemptId, reasons: &Reasons) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE attempts SET reasons = ?2 WHERE id = ?1",
            params![id.0, serde_json::to_string(reasons)?],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("attempt"));
        }
        Ok(())
    }

    pub fn attempt(&self, id: AttemptId) -> Result<(Attempt, Reasons)> {
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT attempt, reasons FROM attempts WHERE id = ?1",
                [id.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (attempt, reasons) = row.ok_or(StoreError::NotFound("attempt"))?;
        Ok((
            serde_json::from_str(&attempt)?,
            serde_json::from_str(&reasons)?,
        ))
    }

    /// Newest first.
    pub fn history(&self) -> Result<Vec<HistoryRow>> {
        self.saved_attempts("ORDER BY id DESC")?
            .into_iter()
            .map(|(id, day, attempt, _)| {
                Ok(HistoryRow {
                    id,
                    day,
                    status: attempt.status(),
                    retry_of: attempt.retry_of().map(|id| AttemptId(id as i64)),
                    topics: attempt.settings().topics.clone(),
                    summary: summarise(&attempt),
                })
            })
            .collect()
    }

    /// Everything the overview needs (R33).
    pub fn attempt_records(&self) -> Result<Vec<AttemptRecord>> {
        Ok(self
            .saved_attempts("ORDER BY id")?
            .into_iter()
            .map(|(id, day, attempt, reasons)| AttemptRecord {
                day,
                sequence: id.0 as u64,
                retry: attempt.retry_of().is_some(),
                summary: summarise(&attempt),
                reason_labels: reason_labels(&attempt, &reasons),
            })
            .collect())
    }

    fn saved_attempts(&self, order: &str) -> Result<Vec<(AttemptId, i64, Attempt, Reasons)>> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT id, day, attempt, reasons FROM attempts {order}"
        ))?;
        let rows = statement.query_map([], |row| {
            Ok((
                AttemptId(row.get(0)?),
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = vec![];
        for row in rows {
            let (id, day, attempt, reasons) = row?;
            out.push((
                id,
                day,
                serde_json::from_str(&attempt)?,
                serde_json::from_str(&reasons)?,
            ));
        }
        Ok(out)
    }
}

fn kind_name(kind: AnswerKind) -> &'static str {
    match kind {
        AnswerKind::Single => "single",
        AnswerKind::Multiple => "multiple",
    }
}

fn write_question_parts(
    transaction: &rusqlite::Transaction,
    id: QuestionId,
    question: &Question,
) -> Result<()> {
    for (position, option) in question.options().iter().enumerate() {
        transaction.execute(
            "INSERT INTO options (question_id, position, text, correct) VALUES (?1, ?2, ?3, ?4)",
            params![id.0, position as i64, option.text, option.correct],
        )?;
    }
    for tag in question.tags() {
        transaction.execute(
            "INSERT OR IGNORE INTO question_tags (question_id, tag) VALUES (?1, ?2)",
            params![id.0, tag],
        )?;
    }
    for (position, citation) in question.citations().iter().enumerate() {
        transaction.execute("INSERT INTO citations (question_id, position, source_id, location) VALUES (?1, ?2, ?3, ?4)", params![id.0, position as i64, citation.source.0, citation.location])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// R46: a source needs a title. Every other field is optional.
fn check_source(fields: &SourceFields) -> Result<()> {
    if fields.title.trim().is_empty() {
        return Err(StoreError::Invalid("a source needs a title"));
    }
    Ok(())
}

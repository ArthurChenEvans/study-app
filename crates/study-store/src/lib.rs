//! SQLite storage for the Study App.
//!
//! History is stored as snapshots, separate from the live questions, so deleting or
//! editing a topic or question never changes past attempts (R28, R30).
//!

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use study_core::attempt::{Attempt, Status};
use study_core::bibliography::{SourceFields, apa};
use study_core::image::{ImageFormat, ImageId};
use study_core::progress::AttemptRecord;
use study_core::question::{
    AnswerKind, AnswerOption, Citation, Question, QuestionDraft, QuestionId, SourceId, TopicId,
};
use study_core::results::{Reasons, Summary, reason_labels, summarise};

pub mod backup;
pub mod cards;
pub mod dates;
pub mod ids;
pub mod portable;
pub mod practice;
pub mod recall;
mod schema;
pub mod search;
pub mod sync;
pub mod tree_queries;
#[cfg(feature = "trace")]
pub mod trace;

/// Default's ID is the same on every computer, so two computers that each made one sync
/// them as one module rather than two (R217).
pub const DEFAULT_MODULE_UID: &str = "01000000-0000-7000-8000-00000000def0";

/// A module's node in [`Store::tree`]: modules even, topics odd.
pub fn module_node(module: ModuleId) -> study_core::tree::NodeId {
    // An ID no row can have (a hostile or damaged one) maps to a node no tree has.
    if module.0 <= 0 || module.0 > i64::MAX / 4 { u64::MAX - 4 } else { (module.0 as u64) * 2 }
}

/// A topic's node in [`Store::tree`].
pub fn topic_node(topic: TopicId) -> study_core::tree::NodeId {
    if topic.0 <= 0 || topic.0 > i64::MAX / 4 { u64::MAX - 5 } else { (topic.0 as u64) * 2 + 1 }
}

pub fn node_module(node: study_core::tree::NodeId) -> ModuleId {
    ModuleId((node / 2) as i64)
}

pub fn node_topic(node: study_core::tree::NodeId) -> TopicId {
    TopicId((node / 2) as i64)
}

fn load_module(tree: &mut study_core::tree::Tree, module: &ModuleRow) {
    let node = study_core::tree::Node {
        id: module_node(module.id),
        kind: study_core::tree::NodeKind::Module,
        name: module.name.clone(),
        parent: module.parent.map(module_node),
        is_default: module.is_default,
    };
    tree.force(node);
}

/// The newest database schema this version of the app knows.
pub const LATEST_SCHEMA: i64 = schema::LATEST;

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
    /// A rule of the module tree (R217, R218), with the message to show as it is.
    #[error(transparent)]
    Tree(#[from] study_core::tree::TreeError),
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
    /// `None` for a root module (R217).
    #[serde(default)]
    pub parent: Option<ModuleId>,
    /// The module that takes topics made without one (R217).
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRow {
    pub id: TopicId,
    pub name: String,
    pub module: Option<ModuleId>,
    pub tags: Vec<String>,
    pub question_count: usize,
    /// How many of those are short answer (R85).
    pub short_count: usize,
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
    clock: ids::SharedClock,
}

impl Store {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// R211: the whole database as bytes, for the phone to keep in the browser between visits.
    pub fn to_bytes(&mut self) -> Result<Vec<u8>> {
        Ok(self.connection.serialize(rusqlite::MAIN_DB)?.to_vec())
    }

    /// A database kept by [`Store::to_bytes`], opened in memory. It's upgraded like any other.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut connection = Connection::open_in_memory()?;
        connection.deserialize_read_exact(rusqlite::MAIN_DB, bytes, bytes.len(), false)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        // The ID and stamp functions come first: migration 8 and every trigger use them.
        let clock = ids::register(&connection)?;
        // Migrations run with foreign keys off so a table can be rebuilt without its
        // children cascading away. Then every link is checked before they go back on.
        connection.pragma_update(None, "foreign_keys", false)?;
        schema::migrate(&connection)?;
        ids::catch_up(&connection, &clock)?;
        let broken: i64 = connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| row.get(0))?;
        if broken > 0 {
            return Err(StoreError::NotFound("consistent links between tables after upgrading"));
        }
        connection.pragma_update(None, "foreign_keys", true)?;
        Ok(Self { connection, clock })
    }

    /// This computer's ID, which ends every change stamp made here (R110).
    /// A record's ID that means the same on any computer, from its row number here.
    pub fn uid_of(&self, table: &str, id: i64) -> Result<Option<String>> {
        if !crate::ids::STAMPED.contains(&table) {
            return Err(StoreError::Invalid("not a table with IDs"));
        }
        Ok(self.connection.query_row(&format!("SELECT uid FROM {table} WHERE id = ?1"), [id], |row| row.get(0)).optional()?.flatten())
    }

    pub fn device_id(&self) -> String {
        self.clock.lock().map(|clock| clock.device().to_string()).unwrap_or_default()
    }

    /// What SQLite's own checks find wrong with the database: damaged pages or indexes
    /// (`integrity_check`) and rows pointing at rows that don't exist (`foreign_key_check`).
    /// Empty means sound.
    pub fn health(&self) -> Result<Vec<String>> {
        let mut problems: Vec<String> = self
            .connection
            .prepare("PRAGMA integrity_check")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
        problems.retain(|line| line != "ok");
        let mut statement = self.connection.prepare("SELECT \"table\", rowid, parent FROM pragma_foreign_key_check")?;
        let broken = statement.query_map([], |row| Ok(format!("{} row {} points at a missing row in {}", row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0), row.get::<_, String>(2)?)))?;
        problems.extend(broken.collect::<rusqlite::Result<Vec<_>>>()?);
        Ok(problems)
    }

    /// Starts changes that can still be taken back: everything written until `end_trial`
    /// is kept or undone together. Imports and sync merges run inside one, so their result
    /// can be read back and checked before it's kept. (Those use savepoints, which nest
    /// inside this one, where a plain transaction couldn't.)
    pub fn begin_trial(&mut self) -> Result<()> {
        Ok(self.connection.execute_batch("SAVEPOINT trial")?)
    }

    /// Keeps or undoes everything since `begin_trial`.
    pub fn end_trial(&mut self, keep: bool) -> Result<()> {
        Ok(self.connection.execute_batch(if keep { "RELEASE trial" } else { "ROLLBACK TO trial; RELEASE trial" })?)
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    // ---------- Modules and topics (R1, R2, R3, R30) ----------

    pub fn create_module(&mut self, name: &str) -> Result<ModuleId> {
        self.create_submodule(name, None)
    }

    /// Makes a module under `parent`, or at the root, checked against the tree rules (R217).
    pub fn create_submodule(&mut self, name: &str, parent: Option<ModuleId>) -> Result<ModuleId> {
        let mut tree = self.tree()?;
        tree.insert_module(u64::MAX, name, parent.map(module_node))?;
        self.connection.execute("INSERT INTO modules (name, parent_id) VALUES (?1, ?2)", params![name.trim(), parent.map(|module| module.0)])?;
        Ok(ModuleId(self.connection.last_insert_rowid()))
    }

    pub fn modules(&self) -> Result<Vec<ModuleRow>> {
        let mut statement = self.connection.prepare("SELECT id, name, parent_id, is_default FROM modules ORDER BY name")?;
        let rows = statement.query_map([], |row| {
            Ok(ModuleRow { id: ModuleId(row.get(0)?), name: row.get(1)?, parent: row.get::<_, Option<i64>>(2)?.map(ModuleId), is_default: row.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Modules, topics and related links as the core's tree. Module ids are even and topic
    /// ids odd (see [`module_node`] and [`topic_node`]), since the two tables count separately.
    pub fn tree(&self) -> Result<study_core::tree::Tree> {
        let mut tree = study_core::tree::Tree::new();
        let modules = self.modules()?;
        // Parents first: each pass places the modules whose parent is already in.
        let mut waiting: Vec<&ModuleRow> = modules.iter().collect();
        while !waiting.is_empty() {
            let before = waiting.len();
            waiting.retain(|module| {
                let ready = module.parent.is_none_or(|parent| tree.get(module_node(parent)).is_some());
                if ready {
                    load_module(&mut tree, module);
                }
                !ready
            });
            if waiting.len() == before {
                // A loop or a missing parent (only a damaged file makes one): those go to the root.
                for module in waiting.drain(..) {
                    load_module(&mut tree, &ModuleRow { parent: None, ..module.clone() });
                }
            }
        }
        let mut statement = self.connection.prepare("SELECT id, name, module_id FROM topics")?;
        let topics = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<i64>>(2)?)))?;
        for topic in topics {
            let (id, name, module) = topic?;
            let parent = module.map(|module| module_node(ModuleId(module))).filter(|node| tree.get(*node).is_some());
            let node = study_core::tree::Node { id: topic_node(TopicId(id)), kind: study_core::tree::NodeKind::Topic, name, parent, is_default: false };
            tree.force(node);
        }
        let mut statement = self.connection.prepare("SELECT first, second FROM related_topics")?;
        for link in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))? {
            let (first, second) = link?;
            tree.insert_link(topic_node(TopicId(first)), topic_node(TopicId(second)));
        }
        Ok(tree)
    }

    /// The Default module, if any topic has needed it yet (R217).
    pub fn default_module(&self) -> Result<Option<ModuleId>> {
        Ok(self.connection.query_row("SELECT id FROM modules WHERE is_default = 1 ORDER BY id LIMIT 1", [], |row| row.get(0)).optional()?.map(ModuleId))
    }

    fn ensure_default(&mut self) -> Result<ModuleId> {
        if let Some(default) = self.default_module()? {
            return Ok(default);
        }
        self.connection.execute("INSERT INTO modules (name, is_default, uid) VALUES (?1, 1, ?2)", [study_core::tree::DEFAULT_MODULE_NAME, DEFAULT_MODULE_UID])?;
        Ok(ModuleId(self.connection.last_insert_rowid()))
    }

    /// Moves a module under another, or to the root, all or nothing (R217).
    pub fn move_module(&mut self, module: ModuleId, parent: Option<ModuleId>) -> Result<()> {
        let mut tree = self.tree()?;
        if tree.get(module_node(module)).is_none() {
            return Err(StoreError::NotFound("module"));
        }
        tree.move_node(module_node(module), parent.map(module_node))?;
        self.connection.execute("UPDATE modules SET parent_id = ?2 WHERE id = ?1 AND parent_id IS NOT ?2", params![module.0, parent.map(|parent| parent.0)])?;
        Ok(())
    }

    /// Names that would clash if a module's contents moved up a level (R217).
    pub fn module_clashes(&self, module: ModuleId) -> Result<Vec<String>> {
        Ok(self.tree()?.clashes_moving_up(module_node(module))?)
    }

    /// Deletes a module (R217). Everything takes its submodules and topics with it; moving
    /// contents up puts them in its parent, a root's topics going to Default.
    pub fn delete_module_with(&mut self, module: ModuleId, mode: study_core::tree::DeleteMode) -> Result<()> {
        let mut tree = self.tree()?;
        if tree.get(module_node(module)).is_none() {
            return Err(StoreError::NotFound("module"));
        }
        let parent = tree.get(module_node(module)).and_then(|node| node.parent).map(node_module);
        let preview = tree.delete_preview(module_node(module))?;
        tree.delete(module_node(module), mode)?;
        let tx = self.connection.transaction()?;
        match mode {
            study_core::tree::DeleteMode::Everything => {
                for topic in &preview.topics {
                    tx.execute("DELETE FROM topics WHERE id = ?1", [node_topic(*topic).0])?;
                }
                // Deepest first, so no module is left pointing at a deleted parent.
                let mut modules: Vec<i64> = preview.modules.iter().map(|node| node_module(*node).0).collect();
                modules.reverse();
                for id in modules {
                    tx.execute("DELETE FROM modules WHERE id = ?1", [id])?;
                }
            }
            study_core::tree::DeleteMode::MoveContentsUp => {
                tx.execute("UPDATE modules SET parent_id = ?2 WHERE parent_id = ?1", params![module.0, parent.map(|parent| parent.0)])?;
                let topics_go_to = match parent {
                    Some(parent) => parent.0,
                    None => {
                        let has_topics: bool = tx.query_row("SELECT EXISTS (SELECT 1 FROM topics WHERE module_id = ?1)", [module.0], |row| row.get(0))?;
                        let existing: Option<i64> = tx.query_row("SELECT id FROM modules WHERE is_default = 1 AND id != ?1 LIMIT 1", [module.0], |row| row.get(0)).optional()?;
                        match (existing, has_topics) {
                            (Some(default), _) => default,
                            (None, true) => {
                                tx.execute("INSERT INTO modules (name, is_default, uid) VALUES (?1, 1, ?2)", [study_core::tree::DEFAULT_MODULE_NAME, DEFAULT_MODULE_UID])?;
                                tx.last_insert_rowid()
                            }
                            (None, false) => 0,
                        }
                    }
                };
                tx.execute("UPDATE topics SET module_id = ?2 WHERE module_id = ?1", params![module.0, topics_go_to])?;
            }
        }
        tx.execute("UPDATE recall_checklists SET module_id = NULL WHERE module_id = ?1", [module.0])?;
        tx.execute("UPDATE dates SET module_id = NULL WHERE module_id = ?1", [module.0])?;
        if tx.execute("DELETE FROM modules WHERE id = ?1", [module.0])? == 0 {
            return Err(StoreError::NotFound("module"));
        }
        tx.commit()?;
        Ok(())
    }

    /// R43 then R217: deleting a module keeps its contents, moving them up a level.
    pub fn delete_module(&mut self, module: ModuleId) -> Result<()> {
        self.delete_module_with(module, study_core::tree::DeleteMode::MoveContentsUp)
    }

    pub fn rename_module(&mut self, module: ModuleId, name: &str) -> Result<()> {
        let mut tree = self.tree()?;
        if tree.get(module_node(module)).is_none() {
            return Err(StoreError::NotFound("module"));
        }
        tree.rename(module_node(module), name)?;
        self.connection.execute("UPDATE modules SET name = ?2 WHERE id = ?1 AND name IS NOT ?2", params![module.0, name.trim()])?;
        Ok(())
    }

    pub fn rename_topic(&mut self, topic: TopicId, name: &str) -> Result<()> {
        let mut tree = self.tree()?;
        if tree.get(topic_node(topic)).is_none() {
            return Err(StoreError::NotFound("topic"));
        }
        tree.rename(topic_node(topic), name)?;
        self.connection.execute("UPDATE topics SET name = ?2 WHERE id = ?1 AND name IS NOT ?2", params![topic.0, name.trim()])?;
        Ok(())
    }

    /// Makes a topic. With no module it goes to Default, made the first time (R217).
    pub fn create_topic(&mut self, name: &str, module: Option<ModuleId>) -> Result<TopicId> {
        let mut tree = self.tree()?;
        if name.trim().is_empty() {
            return Err(study_core::tree::TreeError::EmptyName.into());
        }
        let module = match module {
            Some(module) => module,
            None => {
                if let Some(default) = self.default_module()? {
                    default
                } else {
                    // Checked against an empty Default before making it, so a refused topic leaves none behind.
                    let check = u64::MAX - 1;
                    tree.insert_default(check);
                    tree.insert_topic(u64::MAX, name, check)?;
                    let default = self.ensure_default()?;
                    self.connection.execute("INSERT INTO topics (name, module_id) VALUES (?1, ?2)", params![name.trim(), default.0])?;
                    return Ok(TopicId(self.connection.last_insert_rowid()));
                }
            }
        };
        if tree.get(module_node(module)).is_none() {
            return Err(StoreError::NotFound("module"));
        }
        tree.insert_topic(u64::MAX, name, module_node(module))?;
        self.connection.execute("INSERT INTO topics (name, module_id) VALUES (?1, ?2)", params![name.trim(), module.0])?;
        Ok(TopicId(self.connection.last_insert_rowid()))
    }

    /// Moves a topic to another module, Default if none (R217). Its questions, cards,
    /// sources and history stay with it, since they point at the topic.
    pub fn move_topic(&mut self, topic: TopicId, module: Option<ModuleId>) -> Result<()> {
        let mut tree = self.tree()?;
        if tree.get(topic_node(topic)).is_none() {
            return Err(StoreError::NotFound("topic"));
        }
        let module = match module {
            Some(module) => module,
            None => self.ensure_default()?,
        };
        let mut tree_now = if tree.get(module_node(module)).is_none() { self.tree()? } else { std::mem::take(&mut tree) };
        tree_now.move_node(topic_node(topic), Some(module_node(module)))?;
        self.connection.execute("UPDATE topics SET module_id = ?2 WHERE id = ?1 AND module_id IS NOT ?2", params![topic.0, module.0])?;
        Ok(())
    }

    /// Links two topics as related (R218). Linking twice changes nothing.
    pub fn link_related(&mut self, first: TopicId, second: TopicId) -> Result<()> {
        let mut tree = self.tree()?;
        for topic in [first, second] {
            if tree.get(topic_node(topic)).is_none() {
                return Err(StoreError::NotFound("topic"));
            }
        }
        tree.link_related(topic_node(first), topic_node(second))?;
        let (low, high) = if first.0 < second.0 { (first.0, second.0) } else { (second.0, first.0) };
        self.connection.execute("INSERT OR IGNORE INTO related_topics (first, second) VALUES (?1, ?2)", params![low, high])?;
        Ok(())
    }

    pub fn unlink_related(&mut self, first: TopicId, second: TopicId) -> Result<()> {
        let (low, high) = if first.0 < second.0 { (first.0, second.0) } else { (second.0, first.0) };
        self.connection.execute("DELETE FROM related_topics WHERE first = ?1 AND second = ?2", params![low, high])?;
        Ok(())
    }

    /// Topics directly related to `topic`, in id order (R218).
    pub fn related_topics(&self, topic: TopicId) -> Result<Vec<TopicId>> {
        let mut statement = self.connection.prepare("SELECT second FROM related_topics WHERE first = ?1 UNION SELECT first FROM related_topics WHERE second = ?1 ORDER BY 1")?;
        let rows = statement.query_map([topic.0], |row| row.get(0))?;
        Ok(rows.map(|row| row.map(TopicId)).collect::<rusqlite::Result<_>>()?)
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
            "SELECT t.id, t.name, t.module_id, (SELECT COUNT(*) FROM questions q WHERE q.topic_id = t.id), (SELECT COUNT(*) FROM questions q WHERE q.topic_id = t.id AND q.kind = 'short') FROM topics t ORDER BY t.name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TopicRow {
                id: TopicId(row.get(0)?),
                name: row.get(1)?,
                module: row.get::<_, Option<i64>>(2)?.map(ModuleId),
                tags: vec![],
                question_count: row.get::<_, i64>(3)? as usize,
                short_count: row.get::<_, i64>(4)? as usize,
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
            "INSERT INTO questions (topic_id, kind, text, explanation, short) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                question.topic().0,
                kind_name(question.kind()),
                question.text(),
                question.explanation(),
                question.short().map(serde_json::to_string).transpose()?
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
            "UPDATE questions SET topic_id = ?2, kind = ?3, text = ?4, explanation = ?5, short = ?6, edited_at = datetime('now') WHERE id = ?1",
            params![id.0, question.topic().0, kind_name(question.kind()), question.text(), question.explanation(), question.short().map(serde_json::to_string).transpose()?],
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
        let head: Option<(i64, String, String, String, Option<String>)> = self
            .connection
            .query_row(
                "SELECT topic_id, kind, text, explanation, short FROM questions WHERE id = ?1",
                [id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;
        let (topic, kind, text, explanation, short) = head.ok_or(StoreError::NotFound("question"))?;
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
        let kind = match kind.as_str() {
            "multiple" => AnswerKind::Multiple,
            "short" => AnswerKind::Short,
            _ => AnswerKind::Single,
        };
        let short = short.map(|json| serde_json::from_str(&json)).transpose()?;
        Question::new(QuestionDraft {
            topic: TopicId(topic),
            text,
            kind,
            options,
            explanation,
            tags,
            citations,
            short,
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

    // ---------- Images (R7) ----------

    /// Stores an image already checked by the core. Pasting the same image twice reuses it.
    pub fn save_image(&mut self, format: ImageFormat, bytes: &[u8]) -> Result<ImageId> {
        let existing: Option<i64> = self
            .connection
            .query_row("SELECT id FROM images WHERE bytes = ?1", [bytes], |row| row.get(0))
            .optional()?;
        if let Some(id) = existing {
            return Ok(ImageId(id));
        }
        self.connection.execute(
            "INSERT INTO images (format, bytes) VALUES (?1, ?2)",
            params![serde_json::to_string(&format)?, bytes],
        )?;
        Ok(ImageId(self.connection.last_insert_rowid()))
    }

    pub fn image(&self, id: ImageId) -> Result<(ImageFormat, Vec<u8>)> {
        let row: Option<(String, Vec<u8>)> = self
            .connection
            .query_row("SELECT format, bytes FROM images WHERE id = ?1", [id.0], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        let (format, bytes) = row.ok_or(StoreError::NotFound("image"))?;
        Ok((serde_json::from_str(&format)?, bytes))
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

    /// The quiz being taken right now, as the app last saved it. Kept in `sync_meta`, which
    /// belongs to this computer alone: it's never exported or synced.
    pub fn running_quiz(&self) -> Result<Option<String>> {
        Ok(self.connection.query_row("SELECT value FROM sync_meta WHERE key = 'running_quiz'", [], |row| row.get(0)).optional()?)
    }

    /// Saves the quiz being taken, or forgets it with `None`.
    pub fn set_running_quiz(&mut self, attempt: Option<&Attempt>) -> Result<()> {
        match attempt {
            Some(attempt) => self.connection.execute(
                "INSERT INTO sync_meta (key, value) VALUES ('running_quiz', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [serde_json::to_string(attempt)?],
            )?,
            None => self.connection.execute("DELETE FROM sync_meta WHERE key = 'running_quiz'", [])?,
        };
        Ok(())
    }

    /// Saves the running quiz to history and forgets it as the running one, together, so a
    /// crash can't leave it both finished and waiting to be resumed.
    pub fn save_finished_quiz(&mut self, attempt: &Attempt, day: i64) -> Result<AttemptId> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO attempts (day, status, retry_of, attempt, reasons) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![day, serde_json::to_string(&attempt.status())?, attempt.retry_of().map(|id| id as i64), serde_json::to_string(attempt)?, serde_json::to_string(&Reasons::default())?],
        )?;
        let id = AttemptId(transaction.last_insert_rowid());
        transaction.execute("DELETE FROM sync_meta WHERE key = 'running_quiz'", [])?;
        transaction.commit()?;
        Ok(id)
    }

    /// Replaces a saved attempt, after a short answer on its results is overruled (R84).
    pub fn update_attempt(&mut self, id: AttemptId, attempt: &Attempt) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE attempts SET status = ?2, attempt = ?3 WHERE id = ?1",
            params![id.0, serde_json::to_string(&attempt.status())?, serde_json::to_string(attempt)?],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("attempt"));
        }
        Ok(())
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
        AnswerKind::Short => "short",
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
#[cfg(test)]
mod upgrade_tests;
#[cfg(test)]
mod tree_tests;

/// R46: a source needs a title. Every other field is optional.
fn check_source(fields: &SourceFields) -> Result<()> {
    if fields.title.trim().is_empty() {
        return Err(StoreError::Invalid("a source needs a title"));
    }
    Ok(())
}

// Plain synced settings: a key and a JSON value in the settings table, which sync carries to
// every device (card settings use the same table).
impl Store {
    pub fn setting_value(&self, key: &str) -> Result<Option<String>> {
        Ok(self.connection.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0)).optional()?)
    }

    pub fn set_setting_value(&mut self, key: &str, value: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }
}

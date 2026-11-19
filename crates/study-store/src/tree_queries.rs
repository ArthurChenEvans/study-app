//! Read-only queries for the tree screen, the graph and query search (R217 to R220).

use std::collections::HashMap;

use study_core::query::{Item, ItemKind};
use study_core::question::{SourceId, TopicId};

use crate::{ModuleId, Result, Store, module_node, topic_node};

/// One searchable record with what the results list shows for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRow {
    pub item: Item,
    pub title: String,
}

/// Cards and due cards in each topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardCounts {
    pub cards: usize,
    pub due: usize,
}

impl Store {
    /// Cards, and cards due by `today`, per topic. Archived cards don't count.
    pub fn topic_card_counts(&self, today: i64) -> Result<HashMap<TopicId, CardCounts>> {
        let mut statement = self.connection.prepare(
            "SELECT n.topic_id, COUNT(*), SUM(c.due <= ?1) FROM cards c JOIN notes n ON n.id = c.note_id WHERE c.archived = 0 GROUP BY n.topic_id",
        )?;
        let rows = statement.query_map([today], |row| Ok((TopicId(row.get(0)?), CardCounts { cards: row.get::<_, i64>(1)? as usize, due: row.get::<_, i64>(2)? as usize })))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The sources each topic's questions cite, in id order.
    pub fn topic_sources(&self) -> Result<HashMap<TopicId, Vec<SourceId>>> {
        let mut statement = self.connection.prepare("SELECT DISTINCT q.topic_id, c.source_id FROM citations c JOIN questions q ON q.id = c.question_id ORDER BY q.topic_id, c.source_id")?;
        let mut found: HashMap<TopicId, Vec<SourceId>> = HashMap::new();
        for row in statement.query_map([], |row| Ok((TopicId(row.get(0)?), SourceId(row.get(1)?))))? {
            let (topic, source) = row?;
            found.entry(topic).or_default().push(source);
        }
        Ok(found)
    }

    /// How many cards and recall checklists the given topics and modules hold, for a delete
    /// preview: (cards, checklists).
    pub fn held_by(&self, topics: &[TopicId], modules: &[ModuleId]) -> Result<(usize, usize)> {
        let mut cards = 0;
        let mut checklists = 0;
        for topic in topics {
            cards += self.connection.query_row("SELECT COUNT(*) FROM cards c JOIN notes n ON n.id = c.note_id WHERE n.topic_id = ?1", [topic.0], |row| row.get::<_, i64>(0))? as usize;
            checklists += self.connection.query_row("SELECT COUNT(*) FROM recall_checklists WHERE topic_id = ?1", [topic.0], |row| row.get::<_, i64>(0))? as usize;
        }
        for module in modules {
            checklists += self.connection.query_row("SELECT COUNT(*) FROM recall_checklists WHERE module_id = ?1", [module.0], |row| row.get::<_, i64>(0))? as usize;
        }
        Ok((cards, checklists))
    }

    /// Everything query search looks through, flattened with where each record sits (R220):
    /// modules, topics, questions (with options and explanation), cards, recall checklist
    /// points and practice log entries.
    pub fn query_rows(&self) -> Result<Vec<QueryRow>> {
        let tree = self.tree()?;
        let above = |node: u64| -> Vec<String> {
            let mut path = tree.path(node);
            path.pop();
            path
        };
        let topic_place = |topic: i64| -> (Vec<String>, Option<(u64, String)>) {
            let node = topic_node(TopicId(topic));
            match tree.get(node) {
                Some(found) => (above(node), Some((topic as u64, found.name.clone()))),
                None => (vec![], None),
            }
        };
        let mut rows = Vec::new();
        for module in self.modules()? {
            rows.push(QueryRow { item: Item { id: module.id.0 as u64, kind: ItemKind::Module, text: module.name.clone(), modules: above(module_node(module.id)), topic: None }, title: module.name });
        }
        let mut statement = self.connection.prepare("SELECT id, name FROM topics")?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))? {
            let (id, name) = row?;
            rows.push(QueryRow { item: Item { id: id as u64, kind: ItemKind::Topic, text: name.clone(), modules: above(topic_node(TopicId(id))), topic: None }, title: name });
        }
        let mut statement = self.connection.prepare(
            "SELECT q.id, q.topic_id, q.text, q.explanation, coalesce((SELECT group_concat(o.text, ' ') FROM options o WHERE o.question_id = q.id), '') FROM questions q",
        )?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)))? {
            let (id, topic, text, explanation, options) = row?;
            let (modules, topic) = topic_place(topic);
            rows.push(QueryRow { item: Item { id: id as u64, kind: ItemKind::Question, text: format!("{text} {options} {explanation}"), modules, topic }, title: text });
        }
        let mut statement = self.connection.prepare("SELECT c.id, n.topic_id, c.front, c.back FROM cards c JOIN notes n ON n.id = c.note_id WHERE c.archived = 0")?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))? {
            let (id, topic, front, back) = row?;
            let (modules, topic) = topic_place(topic);
            rows.push(QueryRow { item: Item { id: id as u64, kind: ItemKind::Flashcard, text: format!("{front} {back}"), modules, topic }, title: front });
        }
        let mut statement = self.connection.prepare(
            "SELECT c.id * 1000 + p.position, c.topic_id, c.module_id, p.text FROM recall_points p JOIN recall_checklists c ON c.id = p.checklist_id",
        )?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, Option<i64>>(2)?, row.get::<_, String>(3)?)))? {
            let (id, topic, module, text) = row?;
            let (modules, topic) = match (topic, module) {
                (Some(topic), _) => topic_place(topic),
                (None, Some(module)) => (tree.path(module_node(ModuleId(module))), None),
                (None, None) => (vec![], None),
            };
            rows.push(QueryRow { item: Item { id: id as u64, kind: ItemKind::RecallPoint, text: text.clone(), modules, topic }, title: text });
        }
        let mut statement = self.connection.prepare("SELECT id, topic_id, what, why FROM struggles")?;
        for row in statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))? {
            let (id, topic, what, why) = row?;
            let (modules, topic) = topic.map_or((vec![], None), topic_place);
            rows.push(QueryRow { item: Item { id: id as u64, kind: ItemKind::PracticeEntry, text: format!("{what} {why}"), modules, topic }, title: what });
        }
        Ok(rows)
    }
}

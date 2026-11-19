//! Nested modules, Default, related topics, the graph and query search, as commands
//! (R217 to R220). The rules live in `study_core::tree` and `study_core::query`; this layer
//! loads and saves them and joins in counts, sources and due numbers for the screens.
//!
//! Specs: `specs/features/nested-modules.feature`, `specs/features/search-query.feature`.
//! Tests: `src/tree_tests.rs`, plus the sync and Anki tests in `sync_tests.rs` and
//! `anki_tests.rs`. Test IDs: `specs/tests/nested-modules-and-search.md`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use study_core::query::{Expr, ItemKind, Part, Results, ScopeKind, UnknownScope};
use study_core::question::{SourceId, TopicId};
use study_core::tree::{DeleteMode, NodeKind};
use study_store::{ModuleId, module_node, node_module, node_topic, topic_node};

use crate::{ApiResult, App, required};

/// A module or a topic. Their ids are separate, so a node says which it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id")]
pub enum NodeRef {
    Module(ModuleId),
    Topic(TopicId),
}

/// One row of the tree screen, the graph and the list fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeNode {
    pub node: NodeRef,
    pub name: String,
    pub parent: Option<ModuleId>,
    /// Names from the root down, for tooltips (T-PATH-01).
    pub path: Vec<String>,
    /// Shortened in the middle when long (T-PATH-02).
    pub breadcrumb: String,
    pub is_default: bool,
    /// Rolled up the tree for modules (T-USE-05).
    pub due: u32,
    pub question_count: usize,
    pub card_count: usize,
    /// Sources cited by a topic's questions, for the side panel and graph source nodes.
    pub sources: Vec<SourceId>,
    /// Directly related topics (R218).
    pub related: Vec<TopicId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeView {
    /// Roots first, then each module's children in name order, modules before topics.
    pub nodes: Vec<TreeNode>,
}

/// What deleting a module would remove. Spec: "Deleting a module names everything it removes".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeletePreviewView {
    pub modules: Vec<String>,
    pub topics: Vec<String>,
    pub questions: usize,
    pub cards: usize,
    pub checklists: usize,
    /// Names that would clash if the contents moved up (T-DEL-04).
    pub clashes: Vec<String>,
}

/// How to delete a module, as the dialog sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteChoice {
    Everything,
    MoveContentsUp,
}

/// A query search for the search screen, list filters and the graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuerySearch {
    /// The pills to draw, empty when the query is broken.
    pub parts: Vec<Part>,
    /// Set when the query is broken (T-PAR-E1 to T-PAR-E8), with the byte range to outline.
    pub error: Option<QueryProblem>,
    pub results: Results,
    /// Full paths for every node in the results, so namesakes are told apart (T-EVAL-14).
    pub paths: Vec<(NodeRef, String)>,
    pub unknown: Vec<UnknownScope>,
    /// For each scope in the query, the full paths of the modules or topics it names, for the
    /// pill's tooltip (spec: "Scope pills show their kind").
    pub scope_paths: Vec<(String, Vec<String>)>,
    /// Read-out for screen readers (T-PILL-24).
    pub spoken: String,
    /// What the results list shows for every id in `results`, children included.
    pub items: Vec<ResultItem>,
}

/// One result row: its title and where it sits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResultItem {
    pub kind: ItemKind,
    pub id: u64,
    pub title: String,
    pub path: String,
    /// The topic it sits under, for opening it.
    pub topic: Option<TopicId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryProblem {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl App {
    /// Makes a module, under `parent` or at the root. Spec: "Create a submodule".
    pub fn create_submodule(&mut self, name: &str, parent: Option<ModuleId>) -> ApiResult<ModuleId> {
        Ok(self.store.create_submodule(required(name, "give the module a name")?, parent)?)
    }

    /// Moves a module under another, or to the root. Spec: "Moving" scenarios.
    pub fn move_module(&mut self, module: ModuleId, parent: Option<ModuleId>) -> ApiResult<()> {
        Ok(self.store.move_module(module, parent)?)
    }

    /// Spec: "Deleting a module names everything it removes". Test: T-DEL-01.
    pub fn module_delete_preview(&self, module: ModuleId) -> ApiResult<DeletePreviewView> {
        let tree = self.store.tree()?;
        let preview = tree.delete_preview(module_node(module))?;
        let topics: Vec<TopicId> = preview.topics.iter().map(|node| node_topic(*node)).collect();
        let mut modules: Vec<ModuleId> = preview.modules.iter().map(|node| node_module(*node)).collect();
        let names = |nodes: &[u64]| -> Vec<String> { nodes.iter().filter_map(|node| tree.get(*node)).map(|node| node.name.clone()).collect() };
        let counts: HashMap<TopicId, usize> = self.store.topics()?.into_iter().map(|topic| (topic.id, topic.question_count)).collect();
        modules.push(module);
        let (cards, checklists) = self.store.held_by(&topics, &modules)?;
        Ok(DeletePreviewView {
            modules: names(&preview.modules),
            topics: names(&preview.topics),
            questions: topics.iter().map(|topic| counts.get(topic).copied().unwrap_or(0)).sum(),
            cards,
            checklists,
            clashes: self.store.module_clashes(module)?,
        })
    }

    /// Deletes a module, keeping its contents when asked. Tests: T-DEL-02 to T-DEL-05.
    pub fn delete_module_with(&mut self, module: ModuleId, choice: DeleteChoice) -> ApiResult<()> {
        let mode = match choice {
            DeleteChoice::Everything => DeleteMode::Everything,
            DeleteChoice::MoveContentsUp => DeleteMode::MoveContentsUp,
        };
        Ok(self.store.delete_module_with(module, mode)?)
    }

    /// The Default module, once it exists. Spec: "Default is created only when first needed".
    pub fn default_module(&self) -> ApiResult<Option<ModuleId>> {
        Ok(self.store.default_module()?)
    }

    /// The whole tree with counts, for the tree screen and the graph.
    pub fn module_tree(&self, today: i64) -> ApiResult<TreeView> {
        let tree = self.store.tree()?;
        let questions: HashMap<TopicId, usize> = self.store.topics()?.into_iter().map(|topic| (topic.id, topic.question_count)).collect();
        let cards = self.store.topic_card_counts(today)?;
        let sources = self.store.topic_sources()?;
        let per_topic = |topic: TopicId| (questions.get(&topic).copied().unwrap_or(0), cards.get(&topic).copied().unwrap_or_default());
        let mut nodes = Vec::new();
        let mut stack: Vec<u64> = tree.roots();
        stack.reverse();
        while let Some(id) = stack.pop() {
            let Some(node) = tree.get(id) else { continue };
            let mut children = tree.children(id);
            children.reverse();
            stack.extend(children);
            let (question_count, counts, reference, node_sources, related) = match node.kind {
                NodeKind::Topic => {
                    let topic = node_topic(id);
                    let (questions, counts) = per_topic(topic);
                    let related = tree.related(id).into_iter().map(node_topic).collect();
                    (questions, counts, NodeRef::Topic(topic), sources.get(&topic).cloned().unwrap_or_default(), related)
                }
                NodeKind::Module => {
                    let mut total = (0, study_store::tree_queries::CardCounts::default());
                    for topic in tree.topics_under(id, &[]) {
                        let (questions, counts) = per_topic(node_topic(topic));
                        total.0 += questions;
                        total.1.cards += counts.cards;
                        total.1.due += counts.due;
                    }
                    (total.0, total.1, NodeRef::Module(node_module(id)), vec![], vec![])
                }
            };
            nodes.push(TreeNode {
                node: reference,
                name: node.name.clone(),
                parent: node.parent.map(node_module),
                path: tree.path(id),
                breadcrumb: tree.breadcrumb(id),
                is_default: node.is_default,
                due: counts.due as u32,
                question_count,
                card_count: counts.cards,
                sources: node_sources,
                related,
            });
        }
        Ok(TreeView { nodes })
    }

    /// Every topic under a module, minus unticked submodules, for quiz setup, recall
    /// checklists and calendar dates. Tests: T-USE-01 to T-USE-04.
    pub fn topics_under(&self, module: ModuleId, excluded: &[ModuleId]) -> ApiResult<Vec<TopicId>> {
        let tree = self.store.tree()?;
        let excluded: Vec<u64> = excluded.iter().map(|module| module_node(*module)).collect();
        Ok(tree.topics_under(module_node(module), &excluded).into_iter().map(node_topic).collect())
    }

    /// Spec: "Link related topics". Tests: T-REL-01 to T-REL-04.
    pub fn link_related(&mut self, first: TopicId, second: TopicId) -> ApiResult<()> {
        Ok(self.store.link_related(first, second)?)
    }

    pub fn unlink_related(&mut self, first: TopicId, second: TopicId) -> ApiResult<()> {
        Ok(self.store.unlink_related(first, second)?)
    }

    pub fn related_topics(&self, topic: TopicId) -> ApiResult<Vec<TopicId>> {
        Ok(self.store.related_topics(topic)?)
    }

    /// Runs a query over modules, topics, questions, cards, checklist points and practice log
    /// entries. Spec: `search-query.feature`.
    pub fn query_search(&self, input: &str, today: i64) -> ApiResult<QuerySearch> {
        let _ = today;
        let query = match study_core::query::parse(input) {
            Ok(query) => query,
            Err(problem) => {
                return Ok(QuerySearch {
                    parts: vec![],
                    error: Some(QueryProblem { message: problem.message, start: problem.start, end: problem.end }),
                    results: Results::default(),
                    paths: vec![],
                    unknown: vec![],
                    scope_paths: vec![],
                    spoken: String::new(),
                    items: vec![],
                });
            }
        };
        let tree = self.store.tree()?;
        let rows = self.store.query_rows()?;
        let items: Vec<study_core::query::Item> = rows.iter().map(|row| row.item.clone()).collect();
        let results = study_core::query::search(&items, &query);
        let modules: Vec<String> = tree.nodes().filter(|node| node.kind == NodeKind::Module).map(|node| node.name.clone()).collect();
        let topics: Vec<String> = tree.nodes().filter(|node| node.kind == NodeKind::Topic).map(|node| node.name.clone()).collect();

        let mut paths = Vec::new();
        for (kind, ids) in &results.groups {
            for id in ids {
                match kind {
                    ItemKind::Module => paths.push((NodeRef::Module(ModuleId(*id as i64)), tree.breadcrumb(module_node(ModuleId(*id as i64))))),
                    ItemKind::Topic => paths.push((NodeRef::Topic(TopicId(*id as i64)), tree.breadcrumb(topic_node(TopicId(*id as i64))))),
                    _ => {}
                }
            }
        }
        // The topics the other results sit under, so namesakes are told apart (T-EVAL-14).
        for row in &rows {
            if let Some((topic, _)) = &row.item.topic {
                let reference = NodeRef::Topic(TopicId(*topic as i64));
                let matched = results.groups.get(&row.item.kind).is_some_and(|ids| ids.contains(&row.item.id));
                if matched && !paths.iter().any(|(node, _)| *node == reference) {
                    paths.push((reference, tree.breadcrumb(topic_node(TopicId(*topic as i64)))));
                }
            }
        }

        let mut scope_paths = Vec::new();
        if let Some(expr) = &query.expr {
            let mut found = Vec::new();
            scopes(expr, &mut found);
            for (kind, name) in found {
                let wanted = match kind {
                    ScopeKind::Module => NodeKind::Module,
                    ScopeKind::Topic => NodeKind::Topic,
                };
                let matching = tree.nodes().filter(|node| node.kind == wanted && study_core::tree::same_name(&node.name, &name)).map(|node| tree.breadcrumb(node.id)).collect();
                scope_paths.push((name, matching));
            }
        }

        let by_key: BTreeMap<(ItemKind, u64), &study_store::tree_queries::QueryRow> = rows.iter().map(|row| ((row.item.kind, row.item.id), row)).collect();
        let mut shown = Vec::new();
        let mut keys: Vec<(ItemKind, u64)> = results.groups.iter().flat_map(|(kind, ids)| ids.iter().map(move |id| (*kind, *id))).collect();
        for children in results.children.values() {
            for child in children {
                if let Some(row) = rows.iter().find(|row| row.item.id == *child && row.item.topic.is_some()) {
                    keys.push((row.item.kind, row.item.id));
                }
            }
        }
        keys.sort();
        keys.dedup();
        for key in keys {
            if let Some(row) = by_key.get(&key) {
                let mut path = row.item.modules.clone();
                if let Some((_, topic)) = &row.item.topic {
                    path.push(topic.clone());
                }
                if matches!(key.0, ItemKind::Module | ItemKind::Topic) {
                    path.push(row.title.clone());
                }
                shown.push(ResultItem { kind: key.0, id: key.1, title: row.title.clone(), path: study_core::tree::shorten(&path), topic: row.item.topic.as_ref().map(|(id, _)| TopicId(*id as i64)) });
            }
        }

        Ok(QuerySearch {
            parts: study_core::query::display(&query),
            error: None,
            unknown: study_core::query::unknown_scopes(&query, &modules, &topics),
            spoken: study_core::query::spoken(&query),
            results,
            paths,
            scope_paths,
            items: shown,
        })
    }
}

fn scopes(expr: &Expr, found: &mut Vec<(ScopeKind, String)>) {
    match expr {
        Expr::Scope { kind, name } => found.push((*kind, name.clone())),
        Expr::Not(inner) | Expr::Group(inner) => scopes(inner, found),
        Expr::And { left, right, .. } | Expr::Or(left, right) | Expr::Xor(left, right) => {
            scopes(left, found);
            scopes(right, found);
        }
        _ => {}
    }
}

/// Finds or makes the modules along an `A::B` path for an import (R217): each level matches a
/// module here with that name under the level above, else a new one goes into the snapshot.
pub(crate) struct ModulePlaces {
    /// Lower-case path to the module's id in the snapshot (here or new).
    known: HashMap<String, String>,
    stamp: String,
    prefix: &'static str,
}

impl ModulePlaces {
    pub(crate) fn read(app: &App, stamp: String, prefix: &'static str) -> ApiResult<Self> {
        let tree = app.store.tree()?;
        let mut known = HashMap::new();
        for node in tree.nodes().filter(|node| node.kind == NodeKind::Module) {
            if let Some(uid) = app.store.uid_of("modules", node_module(node.id).0)? {
                known.insert(tree.path(node.id).join("::").to_lowercase(), uid);
            }
        }
        Ok(Self { known, stamp, prefix })
    }

    /// A topic here with this name in the module `module` (an id from [`ModulePlaces::place`]),
    /// so an import adds to it instead of making a namesake beside it (T-IMP-03).
    pub(crate) fn existing_topic(&self, app: &App, module: &str, name: &str) -> ApiResult<Option<String>> {
        let Some(local) = app.store.local_id("modules", module)? else { return Ok(None) };
        for topic in app.store.topics()? {
            if topic.module == Some(ModuleId(local)) && study_core::tree::same_name(&topic.name, name) {
                return Ok(app.store.uid_of("topics", topic.id.0)?);
            }
        }
        Ok(None)
    }

    /// The id of the deepest module on `path`, adding any missing levels to `snapshot`.
    pub(crate) fn place(&mut self, _app: &App, path: &str, snapshot: &mut study_store::portable::Snapshot) -> ApiResult<String> {
        let parts: Vec<&str> = path.split("::").map(str::trim).filter(|part| !part.is_empty()).take(study_core::tree::MAX_MODULE_DEPTH).collect();
        let mut parent: Option<String> = None;
        let mut so_far = Vec::new();
        for part in parts {
            so_far.push(part.to_lowercase());
            let key = so_far.join("::");
            let uid = match self.known.get(&key) {
                Some(uid) => uid.clone(),
                None => {
                    let uid = crate::anki::stable_uid(&format!("{}:{key}", self.prefix));
                    snapshot.modules.push(study_store::portable::Module { uid: uid.clone(), changed: self.stamp.clone(), name: part.to_string(), parent: parent.clone(), is_default: false });
                    self.known.insert(key, uid.clone());
                    uid
                }
            };
            parent = Some(uid);
        }
        parent.ok_or_else(|| crate::ApiError::new("give the module a name"))
    }
}

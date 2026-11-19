//! Nested modules, Default, related topics, the graph and query search, as commands
//! (R217 to R220). The rules live in `study_core::tree` and `study_core::query`; this layer
//! loads and saves them and joins in counts, sources and due numbers for the screens.
//!
//! Specs: `specs/features/nested-modules.feature`, `specs/features/search-query.feature`.
//! Tests: `src/tree_tests.rs`, plus the sync and Anki tests in `sync_tests.rs` and
//! `anki_tests.rs`. Test IDs: `specs/tests/nested-modules-and-search.md`.

use serde::{Deserialize, Serialize};

use study_core::query::{Part, Results, UnknownScope};
use study_core::question::{SourceId, TopicId};
use study_store::ModuleId;

use crate::{ApiResult, App};

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
        let _ = (name, parent);
        todo!("R217: create a nested module")
    }

    /// Moves a module under another, or to the root. Spec: "Moving" scenarios.
    pub fn move_module(&mut self, module: ModuleId, parent: Option<ModuleId>) -> ApiResult<()> {
        let _ = (module, parent);
        todo!("R217: move a module")
    }

    /// Spec: "Deleting a module names everything it removes". Test: T-DEL-01.
    pub fn module_delete_preview(&self, module: ModuleId) -> ApiResult<DeletePreviewView> {
        let _ = module;
        todo!("R217: delete preview")
    }

    /// Deletes a module, keeping its contents when asked. Tests: T-DEL-02 to T-DEL-05.
    pub fn delete_module_with(&mut self, module: ModuleId, choice: DeleteChoice) -> ApiResult<()> {
        let _ = (module, choice);
        todo!("R217: delete a module")
    }

    /// The Default module, once it exists. Spec: "Default is created only when first needed".
    pub fn default_module(&self) -> ApiResult<Option<ModuleId>> {
        todo!("R217: Default module")
    }

    /// The whole tree with counts, for the tree screen and the graph.
    pub fn module_tree(&self, today: i64) -> ApiResult<TreeView> {
        let _ = today;
        todo!("R217: tree view")
    }

    /// Every topic under a module, minus unticked submodules, for quiz setup, recall
    /// checklists and calendar dates. Tests: T-USE-01 to T-USE-04.
    pub fn topics_under(&self, module: ModuleId, excluded: &[ModuleId]) -> ApiResult<Vec<TopicId>> {
        let _ = (module, excluded);
        todo!("R217: topics under a module")
    }

    /// Spec: "Link related topics". Tests: T-REL-01 to T-REL-04.
    pub fn link_related(&mut self, first: TopicId, second: TopicId) -> ApiResult<()> {
        let _ = (first, second);
        todo!("R218: link related topics")
    }

    pub fn unlink_related(&mut self, first: TopicId, second: TopicId) -> ApiResult<()> {
        let _ = (first, second);
        todo!("R218: unlink related topics")
    }

    pub fn related_topics(&self, topic: TopicId) -> ApiResult<Vec<TopicId>> {
        let _ = topic;
        todo!("R218: related topics")
    }

    /// Runs a query over modules, topics, questions, cards, checklist points and practice log
    /// entries. Spec: `search-query.feature`.
    pub fn query_search(&self, input: &str, today: i64) -> ApiResult<QuerySearch> {
        let _ = (input, today);
        todo!("R220: query search")
    }
}

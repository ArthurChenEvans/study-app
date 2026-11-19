//! The module tree (R217) and related topics (R218).
//!
//! Modules nest up to [`MAX_MODULE_DEPTH`] levels. A topic belongs to exactly one module and
//! never holds modules. Two topics with the same name under different parents are separate
//! topics, each with its own questions and sources, so a node is known by its id, never by
//! its name. A topic made without a module goes to the root module [`DEFAULT_MODULE_NAME`],
//! which is created the first time it is needed.
//!
//! Specs: `specs/features/nested-modules.feature`.
//! Tests: `specs/tests/nested-modules-and-search.md`, sections 1 to 6.

use std::collections::{BTreeMap, BTreeSet};

/// Deepest a module can sit, counting a root module as level 1. Topics don't count.
///
/// Spec: "Maximum nesting depth is 4 module levels". Tests: T-TREE-09, T-TREE-10, T-MOVE-05.
pub const MAX_MODULE_DEPTH: usize = 4;

/// The root module that takes topics made without a module.
///
/// Spec: "Default module" scenarios. Tests: T-DEF-01 to T-DEF-08.
pub const DEFAULT_MODULE_NAME: &str = "Default";

/// The separator shown between levels of a path.
pub const PATH_SEPARATOR: &str = " › ";

pub type NodeId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Module,
    Topic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    /// As the user typed it, trimmed of outer spaces.
    pub name: String,
    /// `None` only for root modules. A topic always has a parent.
    pub parent: Option<NodeId>,
    /// True only for the Default module.
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    /// Spec: "Duplicate name under the same parent is refused". Tests: T-TREE-05, T-TREE-06, T-MOVE-02.
    #[error("{parent} already has a {kind} called {name}")]
    DuplicateName { parent: String, kind: &'static str, name: String },
    /// Spec: "Maximum nesting depth is 4 module levels". Tests: T-TREE-10, T-MOVE-05.
    #[error("Modules can go {limit} levels deep")]
    TooDeep { limit: usize },
    /// Spec: "A topic can't contain modules". Test: T-TREE-08.
    #[error("A topic can't hold modules or other topics")]
    ParentIsTopic,
    /// Spec: "A module can't be moved inside itself". Tests: T-MOVE-03, T-MOVE-04.
    #[error("A module can't go inside one of its own submodules")]
    Cycle,
    /// Test: T-TREE-11.
    #[error("Give it a name")]
    EmptyName,
    /// Default can't be renamed, nested or given submodules. Tests: T-DEF-04 to T-DEF-06.
    #[error("Default can't be renamed, moved or given submodules")]
    ReservedModule,
    /// Spec: "A user module named Default is refused". Test: T-DEF-07.
    #[error("Default is reserved for topics without a module")]
    ReservedName,
    /// Spec: "A topic can't be related to itself". Test: T-REL-03.
    #[error("A topic can't be related to itself")]
    SelfLink,
    /// Only topics are related, never modules. Test: T-REL-08.
    #[error("Only topics can be related")]
    NotATopic,
    /// "Move contents up" would put two same-named things side by side.
    /// Spec: "Deleting a module can keep its contents". Test: T-DEL-04.
    #[error("Rename these first: {}", names.join(", "))]
    Clashes { names: Vec<String> },
    #[error("That module or topic no longer exists")]
    NotFound(NodeId),
}

/// How a module is deleted. Spec: "Deleting a module can keep its contents".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    /// The module and everything under it. Test: T-DEL-02.
    Everything,
    /// Its children move up one level first. A root's own topics go to Default.
    /// Tests: T-DEL-03 to T-DEL-05.
    MoveContentsUp,
}

/// What a delete would remove, for the confirmation. Test: T-DEL-01.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeletePreview {
    /// Every module under the one being deleted (not the module itself).
    pub modules: Vec<NodeId>,
    /// Every topic under it, at any depth.
    pub topics: Vec<NodeId>,
}

/// Modules, topics and related links. Pure data: the store saves it, this checks the rules.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    nodes: BTreeMap<NodeId, Node>,
    /// Each link once, smaller id first.
    related: BTreeSet<(NodeId, NodeId)>,
    next_id: NodeId,
}

/// Whether two names count as the same: case and outer spaces ignored, inner spaces kept.
///
/// Tests: T-TREE-06, T-TREE-07, T-DEF-07.
pub fn same_name(first: &str, second: &str) -> bool {
    let _ = (first, second);
    todo!("R217: compare names ignoring case and outer spaces")
}

impl Tree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Makes a module, at the root when `parent` is `None`.
    ///
    /// Refuses an empty name, a duplicate under the same parent, the reserved name Default,
    /// a topic or Default as the parent, and anything deeper than [`MAX_MODULE_DEPTH`].
    /// Tests: T-TREE-01, T-TREE-05, T-TREE-08 to T-TREE-11, T-DEF-06, T-DEF-07.
    pub fn add_module(&mut self, name: &str, parent: Option<NodeId>) -> Result<NodeId, TreeError> {
        let _ = (name, parent, &mut self.next_id);
        todo!("R217: add a module")
    }

    /// Makes a topic. `None` puts it in Default, creating Default if needed. The warning
    /// before that is the screen's job, this is called once the user says Yes.
    ///
    /// Tests: T-TREE-02, T-TREE-05 to T-TREE-08, T-DEF-01, T-DEF-02.
    pub fn add_topic(&mut self, name: &str, module: Option<NodeId>) -> Result<NodeId, TreeError> {
        let _ = (name, module);
        todo!("R217: add a topic")
    }

    /// The Default module, if it has been created. Tests: T-DEF-02, T-DEF-03.
    pub fn default_module(&self) -> Option<NodeId> {
        todo!("R217: find Default")
    }

    /// Root modules in name order, Default among them. Test: T-DEF-03.
    pub fn roots(&self) -> Vec<NodeId> {
        todo!("R217: list root modules")
    }

    /// Direct children in name order, modules first.
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        let _ = id;
        todo!("R217: list children")
    }

    /// Renames a node, with the same checks as creating it. Test: T-DEF-04.
    pub fn rename(&mut self, id: NodeId, name: &str) -> Result<(), TreeError> {
        let _ = (id, name);
        todo!("R217: rename")
    }

    /// Moves a node under `parent` (`None` makes a module a root). All or nothing: on any
    /// error the tree is unchanged.
    ///
    /// Spec: "Moving" scenarios. Tests: T-DEF-05, T-DEF-08, T-MOVE-02 to T-MOVE-06.
    pub fn move_node(&mut self, id: NodeId, parent: Option<NodeId>) -> Result<(), TreeError> {
        let _ = (id, parent);
        todo!("R217: move a node")
    }

    /// What deleting `id` would remove. Spec: "Deleting a module names everything it removes".
    /// Test: T-DEL-01.
    pub fn delete_preview(&self, id: NodeId) -> Result<DeletePreview, TreeError> {
        let _ = id;
        todo!("R217: preview a delete")
    }

    /// Deletes a node and returns every id removed (for deletion markers, R111). Related
    /// links to removed topics go too.
    ///
    /// Tests: T-DEL-02 to T-DEL-06, T-REL-07.
    pub fn delete(&mut self, id: NodeId, mode: DeleteMode) -> Result<Vec<NodeId>, TreeError> {
        let _ = (id, mode);
        todo!("R217: delete a node")
    }

    /// The level a module sits at, a root being 1. For a topic, its module's level.
    pub fn depth(&self, id: NodeId) -> usize {
        let _ = id;
        todo!("R217: depth")
    }

    /// Names from the root down to `id`. Test: T-PATH-01, T-PATH-03.
    pub fn path(&self, id: NodeId) -> Vec<String> {
        let _ = id;
        todo!("R217: path")
    }

    /// The path joined with [`PATH_SEPARATOR`]. Over 4 parts it keeps the first and the last
    /// two with "…" between.
    ///
    /// Spec: "Long paths are shortened in the middle". Tests: T-PATH-01 to T-PATH-03.
    pub fn breadcrumb(&self, id: NodeId) -> String {
        let _ = id;
        todo!("R217: breadcrumb")
    }

    /// Every topic under `id` at any depth, or `id` itself if it is a topic, minus anything
    /// under `excluded`.
    ///
    /// Spec: "Quizzing a module includes its submodules", "Excluding a submodule from a quiz",
    /// recall checklists and calendar dates covering a module. Tests: T-USE-01 to T-USE-04.
    pub fn topics_under(&self, id: NodeId, excluded: &[NodeId]) -> Vec<NodeId> {
        let _ = (id, excluded);
        todo!("R217: topics under a node")
    }

    /// A count rolled up the tree: `own` holds each topic's count, the result is the total
    /// for `id` across everything under it.
    ///
    /// Spec: "Dashboard rolls counts up the tree". Tests: T-USE-05, T-USE-06.
    pub fn rollup(&self, id: NodeId, own: &BTreeMap<NodeId, u32>) -> u32 {
        let _ = (id, own);
        todo!("R217: roll a count up")
    }

    /// Links two topics as related. Linking twice is a no-op. Never merges them.
    ///
    /// Spec: "Link related topics". Tests: T-REL-01 to T-REL-04, T-REL-08.
    pub fn link_related(&mut self, first: NodeId, second: NodeId) -> Result<(), TreeError> {
        let _ = (first, second, &mut self.related);
        todo!("R218: link related topics")
    }

    /// Topics directly related to `id`, one hop only. Tests: T-REL-01, T-REL-06.
    pub fn related(&self, id: NodeId) -> Vec<NodeId> {
        let _ = id;
        todo!("R218: related topics")
    }

    /// The topics a quiz on `topics` covers, adding directly related topics when asked.
    ///
    /// Spec: "A quiz can include related topics". Tests: T-REL-05, T-REL-06.
    pub fn quiz_topics(&self, topics: &[NodeId], include_related: bool) -> Vec<NodeId> {
        let _ = (topics, include_related);
        todo!("R218: quiz topics with related")
    }
}

/// Where an Anki deck lands: its `::` parts become modules and the last part the topic. Parts
/// past [`MAX_MODULE_DEPTH`] modules are joined into the topic name with " › ", and a deck
/// named Default lands in "Default (imported)" so it never merges into the real Default.
///
/// Spec: "Anki nested decks import as nested modules", "Anki decks deeper than the limit".
/// Tests: T-IMP-01, T-IMP-02, T-IMP-04.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnkiPlacement {
    pub modules: Vec<String>,
    pub topic: String,
    /// True when levels were joined into the topic name, so the import can say so.
    pub flattened: bool,
}

pub fn anki_placement(deck: &str) -> AnkiPlacement {
    let _ = deck;
    todo!("R217: place an Anki deck in the tree")
}

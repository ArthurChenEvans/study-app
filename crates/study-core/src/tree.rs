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
    first.trim().to_lowercase() == second.trim().to_lowercase()
}

fn kind_word(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Module => "module",
        NodeKind::Topic => "topic",
    }
}

fn ordered(first: NodeId, second: NodeId) -> (NodeId, NodeId) {
    if first < second { (first, second) } else { (second, first) }
}

impl Tree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Every node, in id order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Every related link once, smaller id first.
    pub fn links(&self) -> impl Iterator<Item = (NodeId, NodeId)> + '_ {
        self.related.iter().copied()
    }

    fn node(&self, id: NodeId) -> Result<&Node, TreeError> {
        self.nodes.get(&id).ok_or(TreeError::NotFound(id))
    }

    fn fresh_id(&mut self) -> NodeId {
        self.next_id = self.next_id.max(self.nodes.keys().next_back().map_or(0, |last| last + 1));
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// The checks shared by making, renaming and moving: a name, not reserved, a parent that
    /// can hold it, and no namesake beside it. `moving` is left out of the namesake check.
    fn check_place(&self, name: &str, kind: NodeKind, parent: Option<NodeId>, moving: Option<NodeId>) -> Result<String, TreeError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(TreeError::EmptyName);
        }
        if kind == NodeKind::Module && same_name(name, DEFAULT_MODULE_NAME) {
            return Err(TreeError::ReservedName);
        }
        let parent_name = match parent {
            Some(parent) => {
                let holder = self.node(parent)?;
                if holder.kind == NodeKind::Topic {
                    return Err(TreeError::ParentIsTopic);
                }
                if holder.is_default && kind == NodeKind::Module {
                    return Err(TreeError::ReservedModule);
                }
                holder.name.clone()
            }
            None if kind == NodeKind::Topic => return Err(TreeError::ParentIsTopic),
            None => "The top level".to_string(),
        };
        let clash = self.nodes.values().find(|other| other.parent == parent && other.kind == kind && Some(other.id) != moving && same_name(&other.name, name));
        if let Some(existing) = clash {
            return Err(TreeError::DuplicateName { parent: parent_name, kind: kind_word(kind), name: existing.name.clone() });
        }
        Ok(name.to_string())
    }

    /// Levels of modules below `id` (0 for a module with no submodules, or a topic).
    fn height(&self, id: NodeId) -> usize {
        self.nodes.values().filter(|child| child.parent == Some(id) && child.kind == NodeKind::Module).map(|child| 1 + self.height(child.id)).max().unwrap_or(0)
    }

    /// Makes a module, at the root when `parent` is `None`.
    ///
    /// Refuses an empty name, a duplicate under the same parent, the reserved name Default,
    /// a topic or Default as the parent, and anything deeper than [`MAX_MODULE_DEPTH`].
    /// Tests: T-TREE-01, T-TREE-05, T-TREE-08 to T-TREE-11, T-DEF-06, T-DEF-07.
    pub fn add_module(&mut self, name: &str, parent: Option<NodeId>) -> Result<NodeId, TreeError> {
        let id = self.fresh_id();
        self.insert_module(id, name, parent)?;
        Ok(id)
    }

    /// [`Tree::add_module`] with an id chosen by the caller (the store's row id).
    pub fn insert_module(&mut self, id: NodeId, name: &str, parent: Option<NodeId>) -> Result<(), TreeError> {
        let name = self.check_place(name, NodeKind::Module, parent, None)?;
        let depth = parent.map_or(0, |parent| self.depth(parent)) + 1;
        if depth > MAX_MODULE_DEPTH {
            return Err(TreeError::TooDeep { limit: MAX_MODULE_DEPTH });
        }
        self.nodes.insert(id, Node { id, kind: NodeKind::Module, name, parent, is_default: false });
        Ok(())
    }

    /// Puts a node in as it is, without checks: for loading saved data, which may predate a
    /// rule (a namesake made before nesting, say). The rules apply to every later change.
    pub fn force(&mut self, node: Node) {
        self.nodes.insert(node.id, node);
    }

    /// Adds the Default module with a given id (the store's row). Test: T-DEF-03.
    pub fn insert_default(&mut self, id: NodeId) {
        self.nodes.insert(id, Node { id, kind: NodeKind::Module, name: DEFAULT_MODULE_NAME.into(), parent: None, is_default: true });
    }

    /// Makes a topic. `None` puts it in Default, creating Default if needed. The warning
    /// before that is the screen's job, this is called once the user says Yes.
    ///
    /// Tests: T-TREE-02, T-TREE-05 to T-TREE-08, T-DEF-01, T-DEF-02.
    pub fn add_topic(&mut self, name: &str, module: Option<NodeId>) -> Result<NodeId, TreeError> {
        if name.trim().is_empty() {
            return Err(TreeError::EmptyName);
        }
        let module = match module {
            Some(module) => module,
            None => match self.default_module() {
                Some(default) => default,
                None => {
                    let id = self.fresh_id();
                    self.insert_default(id);
                    id
                }
            },
        };
        let id = self.fresh_id();
        self.insert_topic(id, name, module)?;
        Ok(id)
    }

    /// [`Tree::add_topic`] with an id chosen by the caller and a module that must exist.
    pub fn insert_topic(&mut self, id: NodeId, name: &str, module: NodeId) -> Result<(), TreeError> {
        let name = self.check_place(name, NodeKind::Topic, Some(module), None)?;
        self.nodes.insert(id, Node { id, kind: NodeKind::Topic, name, parent: Some(module), is_default: false });
        Ok(())
    }

    /// Adds a related link while loading, without checks beyond both ends existing.
    pub fn insert_link(&mut self, first: NodeId, second: NodeId) {
        if first != second && self.nodes.contains_key(&first) && self.nodes.contains_key(&second) {
            self.related.insert(ordered(first, second));
        }
    }

    /// The Default module, if it has been created. Tests: T-DEF-02, T-DEF-03.
    pub fn default_module(&self) -> Option<NodeId> {
        self.nodes.values().find(|node| node.is_default).map(|node| node.id)
    }

    fn sorted(&self, mut ids: Vec<NodeId>) -> Vec<NodeId> {
        ids.sort_by_key(|id| {
            let node = &self.nodes[id];
            (node.kind == NodeKind::Topic, node.is_default, node.name.to_lowercase(), node.id)
        });
        ids
    }

    /// Root modules in name order, Default last. Test: T-DEF-03.
    pub fn roots(&self) -> Vec<NodeId> {
        self.sorted(self.nodes.values().filter(|node| node.parent.is_none()).map(|node| node.id).collect())
    }

    /// Direct children in name order, modules first.
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.sorted(self.nodes.values().filter(|node| node.parent == Some(id)).map(|node| node.id).collect())
    }

    /// Renames a node, with the same checks as creating it. Test: T-DEF-04.
    pub fn rename(&mut self, id: NodeId, name: &str) -> Result<(), TreeError> {
        let node = self.node(id)?.clone();
        if node.is_default {
            return Err(TreeError::ReservedModule);
        }
        let name = self.check_place(name, node.kind, node.parent, Some(id))?;
        self.nodes.get_mut(&id).unwrap().name = name;
        Ok(())
    }

    fn is_under(&self, id: NodeId, ancestor: NodeId) -> bool {
        let mut current = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            current = self.nodes.get(&parent).and_then(|node| node.parent);
        }
        false
    }

    /// Moves a node under `parent` (`None` makes a module a root). All or nothing: on any
    /// error the tree is unchanged.
    ///
    /// Spec: "Moving" scenarios. Tests: T-DEF-05, T-DEF-08, T-MOVE-02 to T-MOVE-06.
    pub fn move_node(&mut self, id: NodeId, parent: Option<NodeId>) -> Result<(), TreeError> {
        let node = self.node(id)?.clone();
        if node.is_default {
            return Err(TreeError::ReservedModule);
        }
        if node.parent == parent {
            return Ok(());
        }
        if let Some(target) = parent {
            if target == id || self.is_under(target, id) {
                return Err(TreeError::Cycle);
            }
        }
        self.check_place(&node.name, node.kind, parent, Some(id))?;
        if node.kind == NodeKind::Module {
            let depth = parent.map_or(0, |parent| self.depth(parent)) + 1 + self.height(id);
            if depth > MAX_MODULE_DEPTH {
                return Err(TreeError::TooDeep { limit: MAX_MODULE_DEPTH });
            }
        }
        self.nodes.get_mut(&id).unwrap().parent = parent;
        Ok(())
    }

    fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        for child in self.children(id) {
            found.push(child);
            found.extend(self.subtree(child));
        }
        found
    }

    /// What deleting `id` would remove. Spec: "Deleting a module names everything it removes".
    /// Test: T-DEL-01.
    pub fn delete_preview(&self, id: NodeId) -> Result<DeletePreview, TreeError> {
        self.node(id)?;
        let mut preview = DeletePreview::default();
        for below in self.subtree(id) {
            match self.nodes[&below].kind {
                NodeKind::Module => preview.modules.push(below),
                NodeKind::Topic => preview.topics.push(below),
            }
        }
        Ok(preview)
    }

    /// Names that would clash if `id`'s children moved up into its parent. Test: T-DEL-04.
    pub fn clashes_moving_up(&self, id: NodeId) -> Result<Vec<String>, TreeError> {
        let node = self.node(id)?;
        let mut names = Vec::new();
        for child in self.children(id) {
            let child = &self.nodes[&child];
            let target = match (child.kind, node.parent) {
                (_, Some(parent)) => Some(parent),
                (NodeKind::Module, None) => None,
                (NodeKind::Topic, None) => match self.default_module() {
                    Some(default) if default != id => Some(default),
                    _ => continue,
                },
            };
            let clash = self.nodes.values().any(|other| other.id != id && other.parent == target && other.kind == child.kind && same_name(&other.name, &child.name));
            if clash {
                names.push(child.name.clone());
            }
        }
        Ok(names)
    }

    /// Deletes a node and returns every id removed (for deletion markers, R111). Related
    /// links to removed topics go too.
    ///
    /// Tests: T-DEL-02 to T-DEL-06, T-REL-07.
    pub fn delete(&mut self, id: NodeId, mode: DeleteMode) -> Result<Vec<NodeId>, TreeError> {
        let node = self.node(id)?.clone();
        let removed = match (mode, node.kind) {
            (DeleteMode::MoveContentsUp, NodeKind::Module) => {
                let names = self.clashes_moving_up(id)?;
                if !names.is_empty() {
                    return Err(TreeError::Clashes { names });
                }
                for child in self.children(id) {
                    let target = match (self.nodes[&child].kind, node.parent) {
                        (_, Some(parent)) => Some(parent),
                        (NodeKind::Module, None) => None,
                        (NodeKind::Topic, None) => Some(match self.default_module().filter(|default| *default != id) {
                            Some(default) => default,
                            None => {
                                let fresh = self.fresh_id();
                                self.insert_default(fresh);
                                fresh
                            }
                        }),
                    };
                    self.nodes.get_mut(&child).unwrap().parent = target;
                }
                vec![id]
            }
            _ => {
                let mut all = vec![id];
                all.extend(self.subtree(id));
                all
            }
        };
        for gone in &removed {
            self.nodes.remove(gone);
        }
        self.related.retain(|(first, second)| !removed.contains(first) && !removed.contains(second));
        Ok(removed)
    }

    /// The level a module sits at, a root being 1. For a topic, its module's level.
    pub fn depth(&self, id: NodeId) -> usize {
        let Some(node) = self.nodes.get(&id) else { return 0 };
        let own = usize::from(node.kind == NodeKind::Module);
        own + node.parent.map_or(0, |parent| self.depth(parent))
    }

    /// Names from the root down to `id`. Test: T-PATH-01, T-PATH-03.
    pub fn path(&self, id: NodeId) -> Vec<String> {
        let mut names = Vec::new();
        let mut current = Some(id);
        while let Some(at) = current {
            let Some(node) = self.nodes.get(&at) else { break };
            names.push(node.name.clone());
            current = node.parent;
        }
        names.reverse();
        names
    }

    /// The path joined with [`PATH_SEPARATOR`]. Over 4 parts it keeps the first and the last
    /// two with "…" between.
    ///
    /// Spec: "Long paths are shortened in the middle". Tests: T-PATH-01 to T-PATH-03.
    pub fn breadcrumb(&self, id: NodeId) -> String {
        shorten(&self.path(id))
    }

    /// Every topic under `id` at any depth, or `id` itself if it is a topic, minus anything
    /// under `excluded`.
    ///
    /// Spec: "Quizzing a module includes its submodules", "Excluding a submodule from a quiz",
    /// recall checklists and calendar dates covering a module. Tests: T-USE-01 to T-USE-04.
    pub fn topics_under(&self, id: NodeId, excluded: &[NodeId]) -> Vec<NodeId> {
        let Some(node) = self.nodes.get(&id) else { return vec![] };
        if excluded.contains(&id) {
            return vec![];
        }
        if node.kind == NodeKind::Topic {
            return vec![id];
        }
        self.children(id).into_iter().flat_map(|child| self.topics_under(child, excluded)).collect()
    }

    /// A count rolled up the tree: `own` holds each topic's count, the result is the total
    /// for `id` across everything under it.
    ///
    /// Spec: "Dashboard rolls counts up the tree". Tests: T-USE-05, T-USE-06.
    pub fn rollup(&self, id: NodeId, own: &BTreeMap<NodeId, u32>) -> u32 {
        self.topics_under(id, &[]).iter().map(|topic| own.get(topic).copied().unwrap_or(0)).sum()
    }

    /// Links two topics as related. Linking twice is a no-op. Never merges them.
    ///
    /// Spec: "Link related topics". Tests: T-REL-01 to T-REL-04, T-REL-08.
    pub fn link_related(&mut self, first: NodeId, second: NodeId) -> Result<(), TreeError> {
        if first == second {
            return Err(TreeError::SelfLink);
        }
        if self.node(first)?.kind != NodeKind::Topic || self.node(second)?.kind != NodeKind::Topic {
            return Err(TreeError::NotATopic);
        }
        self.related.insert(ordered(first, second));
        Ok(())
    }

    /// Removes a link, if there is one.
    pub fn unlink_related(&mut self, first: NodeId, second: NodeId) {
        self.related.remove(&ordered(first, second));
    }

    /// Topics directly related to `id`, one hop only. Tests: T-REL-01, T-REL-06.
    pub fn related(&self, id: NodeId) -> Vec<NodeId> {
        self.related.iter().filter_map(|&(first, second)| if first == id { Some(second) } else if second == id { Some(first) } else { None }).collect()
    }

    /// The topics a quiz on `topics` covers, adding directly related topics when asked.
    ///
    /// Spec: "A quiz can include related topics". Tests: T-REL-05, T-REL-06.
    pub fn quiz_topics(&self, topics: &[NodeId], include_related: bool) -> Vec<NodeId> {
        let mut all: Vec<NodeId> = topics.to_vec();
        if include_related {
            for topic in topics {
                all.extend(self.related(*topic));
            }
        }
        let mut seen = BTreeSet::new();
        all.retain(|id| seen.insert(*id));
        all
    }
}

/// Joins a path for showing, shortened in the middle past 4 parts. Test: T-PATH-02.
pub fn shorten(path: &[String]) -> String {
    if path.len() > 4 {
        let mut parts = vec![path[0].clone(), "…".to_string()];
        parts.extend(path[path.len() - 2..].iter().cloned());
        parts.join(PATH_SEPARATOR)
    } else {
        path.join(PATH_SEPARATOR)
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
    let mut parts: Vec<String> = deck.split("::").map(str::trim).filter(|part| !part.is_empty()).map(|part| if same_name(part, DEFAULT_MODULE_NAME) { format!("{DEFAULT_MODULE_NAME} (imported)") } else { part.to_string() }).collect();
    if parts.is_empty() {
        parts.push(deck.trim().to_string());
    }
    let topic = parts.pop().unwrap();
    if parts.len() > MAX_MODULE_DEPTH {
        let extra: Vec<String> = parts.drain(MAX_MODULE_DEPTH..).collect();
        let topic = extra.into_iter().chain(std::iter::once(topic)).collect::<Vec<_>>().join(PATH_SEPARATOR);
        return AnkiPlacement { modules: parts, topic, flattened: true };
    }
    AnkiPlacement { modules: parts, topic, flattened: false }
}

//! Module tree, Default module, moving, deleting, paths, rollups and related topics (R217, R218).
//!
//! Spec: `specs/features/nested-modules.feature`.
//! Test IDs: `specs/tests/nested-modules-and-search.md`, sections 1 to 6. Tests marked
//! (store) or (component) there live in those layers instead.

use std::collections::BTreeMap;
use study_core::tree::{DEFAULT_MODULE_NAME, DeleteMode, MAX_MODULE_DEPTH, NodeId, NodeKind, Tree, TreeError, same_name};

/// The Background: Computer Architecture with N2T, CS:APP and UCM CySec CA Module.
struct Fixture {
    tree: Tree,
    ca: NodeId,
    n2t: NodeId,
    csapp: NodeId,
    ucm: NodeId,
}

fn background() -> Fixture {
    let mut tree = Tree::new();
    let ca = tree.add_module("Computer Architecture", None).unwrap();
    let n2t = tree.add_module("N2T", Some(ca)).unwrap();
    let csapp = tree.add_module("CS:APP", Some(ca)).unwrap();
    let ucm = tree.add_module("UCM CySec CA Module", Some(ca)).unwrap();
    Fixture { tree, ca, n2t, csapp, ucm }
}

/// A chain of `levels` modules, returning the bottom one.
fn chain(tree: &mut Tree, levels: usize) -> NodeId {
    let mut parent = None;
    for level in 1..=levels {
        parent = Some(tree.add_module(&format!("Level {level}"), parent).unwrap());
    }
    parent.unwrap()
}

// 1. Module tree rules

/// T-TREE-01. Spec: "Create a submodule".
#[test]
fn t_tree_01_submodule_is_created_under_its_parent() {
    let mut fx = background();
    let extras = fx.tree.add_module("Extras", Some(fx.ca)).unwrap();
    assert_eq!(fx.tree.get(extras).unwrap().parent, Some(fx.ca));
    assert!(fx.tree.children(fx.ca).contains(&extras));
}

/// T-TREE-02. Spec: "Same-named topics under different parents are separate".
#[test]
fn t_tree_02_same_name_under_different_parents_makes_two_topics() {
    let mut fx = background();
    let first = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let second = fx.tree.add_topic("CPU", Some(fx.csapp)).unwrap();
    assert_ne!(first, second);
    assert_eq!(fx.tree.get(first).unwrap().kind, NodeKind::Topic);
    assert_eq!(fx.tree.get(second).unwrap().parent, Some(fx.csapp));
}

/// T-TREE-05. Spec: "Duplicate name under the same parent is refused".
#[test]
fn t_tree_05_duplicate_name_under_the_same_parent_is_refused() {
    let mut fx = background();
    fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let error = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap_err();
    assert!(matches!(error, TreeError::DuplicateName { .. }));
    assert_eq!(error.to_string(), "N2T already has a topic called CPU");
    assert_eq!(fx.tree.children(fx.n2t).len(), 1, "nothing is written");
}

/// T-TREE-06. Spec: "Names compare without case or outer spaces".
#[test]
fn t_tree_06_duplicate_check_ignores_case_and_outer_spaces() {
    let mut fx = background();
    fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    assert!(matches!(fx.tree.add_topic(" cpu ", Some(fx.n2t)), Err(TreeError::DuplicateName { .. })));
    assert!(same_name("CPU", " cpu "));
}

/// T-TREE-07.
#[test]
fn t_tree_07_inner_spaces_still_count() {
    let mut fx = background();
    fx.tree.add_topic("CPU Logic", Some(fx.n2t)).unwrap();
    assert!(fx.tree.add_topic("CPULogic", Some(fx.n2t)).is_ok());
    assert!(!same_name("CPU Logic", "CPULogic"));
}

/// T-TREE-08. Spec: "A topic can't contain modules".
#[test]
fn t_tree_08_a_topic_cannot_hold_a_module_or_topic() {
    let mut fx = background();
    let cpu = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    assert_eq!(fx.tree.add_module("Inside", Some(cpu)), Err(TreeError::ParentIsTopic));
    assert_eq!(fx.tree.add_topic("Inside", Some(cpu)), Err(TreeError::ParentIsTopic));
}

/// T-TREE-09. Spec: "Maximum nesting depth is 4 module levels", depth 3.
#[test]
fn t_tree_09_a_fourth_level_is_allowed() {
    let mut tree = Tree::new();
    let bottom = chain(&mut tree, 3);
    let fourth = tree.add_module("Fourth", Some(bottom)).unwrap();
    assert_eq!(tree.depth(fourth), MAX_MODULE_DEPTH);
}

/// T-TREE-10. Spec: "Maximum nesting depth is 4 module levels", depth 4.
#[test]
fn t_tree_10_a_fifth_level_is_refused() {
    let mut tree = Tree::new();
    let bottom = chain(&mut tree, 4);
    let error = tree.add_module("Fifth", Some(bottom)).unwrap_err();
    assert_eq!(error, TreeError::TooDeep { limit: 4 });
    assert_eq!(error.to_string(), "Modules can go 4 levels deep");
}

/// T-TREE-10, topics don't count as a level.
#[test]
fn t_tree_10_topics_can_sit_in_the_deepest_module() {
    let mut tree = Tree::new();
    let bottom = chain(&mut tree, 4);
    assert!(tree.add_topic("CPU", Some(bottom)).is_ok());
}

/// T-TREE-11.
#[test]
fn t_tree_11_empty_names_are_refused() {
    let mut fx = background();
    assert_eq!(fx.tree.add_module("   ", None), Err(TreeError::EmptyName));
    assert_eq!(fx.tree.add_topic("", Some(fx.n2t)), Err(TreeError::EmptyName));
}

// 2. Default module

/// T-DEF-01. Spec: "Confirming puts the topic in Default".
#[test]
fn t_def_01_a_topic_without_a_module_goes_to_default() {
    let mut fx = background();
    let misc = fx.tree.add_topic("Misc", None).unwrap();
    let default = fx.tree.default_module().unwrap();
    assert_eq!(fx.tree.get(misc).unwrap().parent, Some(default));
    assert_eq!(fx.tree.get(default).unwrap().name, DEFAULT_MODULE_NAME);
}

/// T-DEF-02. Spec: "Default is created only when first needed".
#[test]
fn t_def_02_default_is_created_lazily() {
    let mut fx = background();
    assert_eq!(fx.tree.default_module(), None);
    fx.tree.add_topic("Misc", None).unwrap();
    fx.tree.add_topic("Other", None).unwrap();
    let defaults = fx.tree.roots().into_iter().filter(|id| fx.tree.get(*id).unwrap().is_default).count();
    assert_eq!(defaults, 1, "one Default, made once");
}

/// T-DEF-03. Spec: "Default is a root module, not a parent of other roots".
#[test]
fn t_def_03_default_is_a_root_beside_the_others() {
    let mut fx = background();
    fx.tree.add_topic("Misc", None).unwrap();
    let default = fx.tree.default_module().unwrap();
    assert_eq!(fx.tree.get(default).unwrap().parent, None);
    let roots = fx.tree.roots();
    assert!(roots.contains(&default) && roots.contains(&fx.ca));
    assert!(!fx.tree.children(default).contains(&fx.ca));
}

/// T-DEF-04. Spec: "Default can't be renamed or nested".
#[test]
fn t_def_04_default_cannot_be_renamed() {
    let mut fx = background();
    fx.tree.add_topic("Misc", None).unwrap();
    let default = fx.tree.default_module().unwrap();
    assert_eq!(fx.tree.rename(default, "Inbox"), Err(TreeError::ReservedModule));
}

/// T-DEF-05. Spec: "Default can't be renamed or nested".
#[test]
fn t_def_05_default_cannot_be_moved_under_another_module() {
    let mut fx = background();
    fx.tree.add_topic("Misc", None).unwrap();
    let default = fx.tree.default_module().unwrap();
    assert_eq!(fx.tree.move_node(default, Some(fx.ca)), Err(TreeError::ReservedModule));
}

/// T-DEF-06. Spec: "Submodules can't be created in Default".
#[test]
fn t_def_06_default_takes_no_submodules() {
    let mut fx = background();
    fx.tree.add_topic("Misc", None).unwrap();
    let default = fx.tree.default_module().unwrap();
    assert_eq!(fx.tree.add_module("Sub", Some(default)), Err(TreeError::ReservedModule));
    assert_eq!(fx.tree.move_node(fx.n2t, Some(default)), Err(TreeError::ReservedModule));
}

/// T-DEF-07. Spec: "A user module named Default is refused".
#[test]
fn t_def_07_a_module_named_default_is_refused_in_any_case() {
    let mut fx = background();
    for name in ["Default", "default", " DEFAULT "] {
        let error = fx.tree.add_module(name, None).unwrap_err();
        assert_eq!(error, TreeError::ReservedName);
        assert_eq!(error.to_string(), "Default is reserved for topics without a module");
    }
    assert_eq!(fx.tree.rename(fx.n2t, "default"), Err(TreeError::ReservedName));
}

/// T-DEF-08. Spec: "Topics can be moved out of Default".
#[test]
fn t_def_08_topics_move_out_of_default() {
    let mut fx = background();
    let misc = fx.tree.add_topic("Misc", None).unwrap();
    fx.tree.move_node(misc, Some(fx.n2t)).unwrap();
    assert_eq!(fx.tree.get(misc).unwrap().parent, Some(fx.n2t));
}

// 3. Moving and deleting

/// T-MOVE-02. Spec: "Moving into a parent that has the same name is blocked".
#[test]
fn t_move_02_moving_onto_a_same_named_topic_is_refused() {
    let mut fx = background();
    let cpu = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    fx.tree.add_topic("CPU", Some(fx.ucm)).unwrap();
    assert!(matches!(fx.tree.move_node(cpu, Some(fx.ucm)), Err(TreeError::DuplicateName { .. })));
    assert_eq!(fx.tree.get(cpu).unwrap().parent, Some(fx.n2t));
}

/// T-MOVE-03. Spec: "A module can't be moved inside itself".
#[test]
fn t_move_03_a_module_cannot_move_into_itself() {
    let mut fx = background();
    assert_eq!(fx.tree.move_node(fx.ca, Some(fx.ca)), Err(TreeError::Cycle));
}

/// T-MOVE-04. Spec: "A module can't be moved inside itself".
#[test]
fn t_move_04_a_module_cannot_move_into_its_descendant() {
    let mut fx = background();
    let error = fx.tree.move_node(fx.ca, Some(fx.n2t)).unwrap_err();
    assert_eq!(error, TreeError::Cycle);
    assert_eq!(error.to_string(), "A module can't go inside one of its own submodules");
}

/// T-MOVE-05. Spec: "A move past the depth limit is blocked". The deepest descendant decides.
#[test]
fn t_move_05_a_move_that_pushes_a_descendant_too_deep_is_refused() {
    let mut tree = Tree::new();
    let target = chain(&mut tree, 2);
    let mover = tree.add_module("Mover", None).unwrap();
    let child = tree.add_module("Child", Some(mover)).unwrap();
    tree.add_module("Grandchild", Some(child)).unwrap();
    assert_eq!(tree.move_node(mover, Some(target)), Err(TreeError::TooDeep { limit: 4 }));
}

/// T-MOVE-06. A refused move changes nothing.
#[test]
fn t_move_06_a_refused_move_leaves_the_tree_unchanged() {
    let mut fx = background();
    let before: Vec<_> = fx.tree.children(fx.ca);
    let _ = fx.tree.move_node(fx.ca, Some(fx.csapp));
    assert_eq!(fx.tree.children(fx.ca), before);
    assert_eq!(fx.tree.get(fx.ca).unwrap().parent, None);
}

/// T-DEL-01. Spec: "Deleting a module names everything it removes".
#[test]
fn t_del_01_the_preview_lists_the_whole_subtree() {
    let mut fx = background();
    let part = fx.tree.add_module("Part 1", Some(fx.n2t)).unwrap();
    let cpu = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let alu = fx.tree.add_topic("ALU", Some(part)).unwrap();
    let preview = fx.tree.delete_preview(fx.n2t).unwrap();
    assert_eq!(preview.modules, vec![part]);
    let mut topics = preview.topics.clone();
    topics.sort();
    let mut expected = vec![cpu, alu];
    expected.sort();
    assert_eq!(topics, expected);
}

/// T-DEL-02. Delete everything removes the subtree and reports every id.
#[test]
fn t_del_02_deleting_everything_removes_the_subtree() {
    let mut fx = background();
    let cpu = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let removed = fx.tree.delete(fx.n2t, DeleteMode::Everything).unwrap();
    assert!(removed.contains(&fx.n2t) && removed.contains(&cpu));
    assert!(fx.tree.get(cpu).is_none());
    assert!(!fx.tree.children(fx.ca).contains(&fx.n2t));
}

/// T-DEL-03. Spec: "Deleting a module can keep its contents".
#[test]
fn t_del_03_move_contents_up_reparents_the_children() {
    let mut fx = background();
    let cpu = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let part = fx.tree.add_module("Part 1", Some(fx.n2t)).unwrap();
    let removed = fx.tree.delete(fx.n2t, DeleteMode::MoveContentsUp).unwrap();
    assert_eq!(removed, vec![fx.n2t]);
    assert_eq!(fx.tree.get(cpu).unwrap().parent, Some(fx.ca));
    assert_eq!(fx.tree.get(part).unwrap().parent, Some(fx.ca));
}

/// T-DEL-04. Clashes are reported and nothing is deleted.
#[test]
fn t_del_04_move_contents_up_reports_clashes_first() {
    let mut fx = background();
    fx.tree.add_topic("Motherboard", Some(fx.n2t)).unwrap();
    fx.tree.add_topic("Motherboard", Some(fx.ca)).unwrap();
    let error = fx.tree.delete(fx.n2t, DeleteMode::MoveContentsUp).unwrap_err();
    assert_eq!(error, TreeError::Clashes { names: vec!["Motherboard".into()] });
    assert!(fx.tree.get(fx.n2t).is_some());
}

/// T-DEL-05. Spec: "Deleting a root module and moving its contents up".
#[test]
fn t_del_05_move_contents_up_on_a_root() {
    let mut fx = background();
    let overview = fx.tree.add_topic("Overview", Some(fx.ca)).unwrap();
    fx.tree.delete(fx.ca, DeleteMode::MoveContentsUp).unwrap();
    assert_eq!(fx.tree.get(fx.n2t).unwrap().parent, None);
    let default = fx.tree.default_module().unwrap();
    assert_eq!(fx.tree.get(overview).unwrap().parent, Some(default));
}

/// T-DEL-06. Spec: "Deleting a topic doesn't touch same-named topics".
#[test]
fn t_del_06_deleting_a_topic_leaves_its_namesakes() {
    let mut fx = background();
    let first = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let second = fx.tree.add_topic("CPU", Some(fx.csapp)).unwrap();
    fx.tree.delete(first, DeleteMode::Everything).unwrap();
    assert!(fx.tree.get(second).is_some());
}

// 4. Paths and breadcrumbs

/// T-PATH-01. Spec: "Results show the full path".
#[test]
fn t_path_01_full_path_from_root() {
    let mut fx = background();
    let cpu = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    assert_eq!(fx.tree.path(cpu), vec!["Computer Architecture", "N2T", "CPU"]);
    assert_eq!(fx.tree.breadcrumb(cpu), "Computer Architecture › N2T › CPU");
}

/// T-PATH-02. Spec: "Long paths are shortened in the middle".
#[test]
fn t_path_02_long_paths_keep_the_first_and_last_two() {
    let mut tree = Tree::new();
    let bottom = chain(&mut tree, 4);
    let cpu = tree.add_topic("CPU", Some(bottom)).unwrap();
    assert_eq!(tree.path(cpu).len(), 5);
    assert_eq!(tree.breadcrumb(cpu), "Level 1 › … › Level 4 › CPU");
}

/// T-PATH-03.
#[test]
fn t_path_03_default_topics_show_default() {
    let mut tree = Tree::new();
    let misc = tree.add_topic("Misc", None).unwrap();
    assert_eq!(tree.breadcrumb(misc), "Default › Misc");
}

// 5. Tree used elsewhere

/// T-USE-01. Spec: "Quizzing a module includes its submodules" (also checklists and dates,
/// T-USE-03 and T-USE-04, which use the same call).
#[test]
fn t_use_01_a_module_covers_every_topic_below_it() {
    let mut fx = background();
    let a = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.tree.add_topic("CPU", Some(fx.csapp)).unwrap();
    let c = fx.tree.add_topic("Buses", Some(fx.ucm)).unwrap();
    let mut topics = fx.tree.topics_under(fx.ca, &[]);
    topics.sort();
    assert_eq!(topics, vec![a, b, c]);
    assert_eq!(fx.tree.topics_under(a, &[]), vec![a], "a topic covers itself");
}

/// T-USE-02. Spec: "Excluding a submodule from a quiz".
#[test]
fn t_use_02_an_unticked_submodule_is_left_out() {
    let mut fx = background();
    let a = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    fx.tree.add_topic("CPU", Some(fx.csapp)).unwrap();
    assert_eq!(fx.tree.topics_under(fx.ca, &[fx.csapp]), vec![a]);
}

/// T-USE-05 and T-USE-06. Spec: "Dashboard rolls counts up the tree".
#[test]
fn t_use_05_counts_roll_up_without_double_counting() {
    let mut fx = background();
    let a = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.tree.add_topic("CPU", Some(fx.csapp)).unwrap();
    let c = fx.tree.add_topic("Overview", Some(fx.ca)).unwrap();
    let due = BTreeMap::from([(a, 6), (b, 3), (c, 1)]);
    assert_eq!(fx.tree.rollup(fx.n2t, &due), 6);
    assert_eq!(fx.tree.rollup(fx.ca, &due), 10);
    assert_eq!(fx.tree.rollup(fx.ucm, &due), 0);
}

// 6. Related topics

fn two_cpus() -> (Fixture, NodeId, NodeId) {
    let mut fx = background();
    let a = fx.tree.add_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.tree.add_topic("CPU", Some(fx.csapp)).unwrap();
    (fx, a, b)
}

/// T-REL-01 and T-REL-02. Spec: "Link related topics".
#[test]
fn t_rel_01_links_are_symmetric_and_never_merge() {
    let (mut fx, a, b) = two_cpus();
    fx.tree.link_related(a, b).unwrap();
    assert_eq!(fx.tree.related(a), vec![b]);
    assert_eq!(fx.tree.related(b), vec![a]);
    assert_eq!(fx.tree.get(a).unwrap().parent, Some(fx.n2t));
    assert_eq!(fx.tree.get(b).unwrap().parent, Some(fx.csapp));
}

/// T-REL-03. Spec: "A topic can't be related to itself".
#[test]
fn t_rel_03_self_links_are_refused() {
    let (mut fx, a, _) = two_cpus();
    assert_eq!(fx.tree.link_related(a, a), Err(TreeError::SelfLink));
}

/// T-REL-04.
#[test]
fn t_rel_04_linking_twice_keeps_one_link() {
    let (mut fx, a, b) = two_cpus();
    fx.tree.link_related(a, b).unwrap();
    fx.tree.link_related(b, a).unwrap();
    assert_eq!(fx.tree.related(a), vec![b]);
}

/// T-REL-05 and T-REL-06. Spec: "A quiz can include related topics", one hop only.
#[test]
fn t_rel_05_quizzes_can_add_related_topics_one_hop() {
    let (mut fx, a, b) = two_cpus();
    let c = fx.tree.add_topic("CPU", Some(fx.ucm)).unwrap();
    fx.tree.link_related(a, b).unwrap();
    fx.tree.link_related(b, c).unwrap();
    assert_eq!(fx.tree.quiz_topics(&[a], false), vec![a]);
    let mut with = fx.tree.quiz_topics(&[a], true);
    with.sort();
    assert_eq!(with, vec![a, b]);
}

/// T-REL-07.
#[test]
fn t_rel_07_deleting_a_topic_removes_its_links() {
    let (mut fx, a, b) = two_cpus();
    fx.tree.link_related(a, b).unwrap();
    fx.tree.delete(b, DeleteMode::Everything).unwrap();
    assert!(fx.tree.related(a).is_empty());
}

/// T-REL-08.
#[test]
fn t_rel_08_only_topics_can_be_related() {
    let (mut fx, a, _) = two_cpus();
    assert_eq!(fx.tree.link_related(a, fx.n2t), Err(TreeError::NotATopic));
}

// 13. Anki placement (the store half, T-IMP-03, is in study-api)

use study_core::tree::{AnkiPlacement, anki_placement};

/// T-IMP-01. Spec: "Anki nested decks import as nested modules".
#[test]
fn t_imp_01_deck_parts_become_modules_and_a_topic() {
    assert_eq!(
        anki_placement("Computer Architecture::N2T::CPU"),
        AnkiPlacement { modules: vec!["Computer Architecture".into(), "N2T".into()], topic: "CPU".into(), flattened: false }
    );
    assert_eq!(anki_placement("Misc"), AnkiPlacement { modules: vec![], topic: "Misc".into(), flattened: false });
}

/// T-IMP-02. Spec: "Anki decks deeper than the limit".
#[test]
fn t_imp_02_levels_past_four_join_the_topic_name() {
    let placed = anki_placement("A::B::C::D::E::F");
    assert_eq!(placed.modules, vec!["A", "B", "C", "D"]);
    assert_eq!(placed.topic, "E › F");
    assert!(placed.flattened);
}

/// T-IMP-04
#[test]
fn t_imp_04_a_deck_named_default_is_renamed() {
    assert_eq!(anki_placement("Default").topic, "Default (imported)");
    assert_eq!(anki_placement("Default::Loose").modules, vec!["Default (imported)"]);
}

//! nested-modules.feature and search-query.feature through the command layer (R217 to R220):
//! what the rules in study-core can't see on their own, such as questions, sources, cards and
//! quizzes moving with their topic, export and import, and query search over real records.
//!
//! Test IDs: `specs/tests/nested-modules-and-search.md` (the store rows of sections 1, 3, 5,
//! 9, 12 and 13). Sync and Anki rows are in `sync_tests.rs` and `anki_tests.rs`.

use super::*;
use crate::cards::NoteInput;
use crate::practice::Clock;
use crate::transfer::{ClashRule, ExportRequest, ImportChoice};
use crate::tree::{DeleteChoice, NodeRef};
use study_core::bibliography::SourceFields;
use study_core::cards::{Note, NoteKind};
use study_core::query::{ItemKind, Part};

const TODAY: i64 = 20_771;

fn at(day: i64) -> Clock {
    Clock { day, minute: 600, unix: day * 86_400 + 36_000 }
}

fn app() -> App {
    let mut app = App::with_seed(Store::in_memory().unwrap(), 11);
    app.set_backups(std::env::temp_dir().join(format!("study-tree-{}-{}", std::process::id(), rand::random::<u32>())));
    app
}

struct Background {
    app: App,
    ca: ModuleId,
    n2t: ModuleId,
    csapp: ModuleId,
    ucm: ModuleId,
}

/// The Background of nested-modules.feature.
fn background() -> Background {
    let mut app = app();
    let ca = app.create_submodule("Computer Architecture", None).unwrap();
    let n2t = app.create_submodule("N2T", Some(ca)).unwrap();
    let csapp = app.create_submodule("CS:APP", Some(ca)).unwrap();
    let ucm = app.create_submodule("UCM CySec CA Module", Some(ca)).unwrap();
    Background { app, ca, n2t, csapp, ucm }
}

fn question(topic: TopicId, text: &str, citations: Vec<CitationInput>) -> QuestionInput {
    QuestionInput {
        topic,
        text: text.into(),
        multiple: false,
        options: ["A", "B", "C"].iter().enumerate().map(|(i, text)| OptionInput { text: (*text).into(), correct: i == 0 }).collect(),
        explanation: String::new(),
        tags: vec![],
        citations,
        short: None,
    }
}

fn source(app: &mut App, title: &str) -> SourceId {
    app.add_source(SourceFields { authors: "Author, A.".into(), title: title.into(), ..Default::default() }).unwrap()
}

fn card(app: &mut App, topic: TopicId, front: &str) {
    let note = Note { topic, kind: NoteKind::Basic, front: front.into(), back: "back".into(), cues: vec![], text: String::new(), extra: String::new(), tags: vec![] };
    app.save_note(None, NoteInput { note, suggestion: None }, TODAY).unwrap();
}

fn node<'a>(view: &'a crate::tree::TreeView, wanted: NodeRef) -> &'a crate::tree::TreeNode {
    view.nodes.iter().find(|node| node.node == wanted).unwrap_or_else(|| panic!("no {wanted:?} in the tree"))
}

// 1. Same-named topics share nothing

/// T-TREE-03. Spec: "Same-named topics under different parents are separate".
#[test]
fn t_tree_03_a_question_in_one_cpu_is_not_in_the_other() {
    let mut fx = background();
    let first = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let second = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    fx.app.save_question(question(first, "What does the PC hold?", vec![])).unwrap();
    assert_eq!(fx.app.questions_in(first).unwrap().len(), 1);
    assert!(fx.app.questions_in(second).unwrap().is_empty());
}

/// T-TREE-04. Spec: "Same-named topics under different parents keep their own sources".
#[test]
fn t_tree_04_each_cpu_lists_only_its_own_source() {
    let mut fx = background();
    let first = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let second = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    let ecs = source(&mut fx.app, "The Elements of Computing Systems");
    let csapp = source(&mut fx.app, "Computer Systems: A Programmer's Perspective");
    fx.app.save_question(question(first, "Hack CPU?", vec![CitationInput { source: ecs, location: "p. 94".into() }])).unwrap();
    fx.app.save_question(question(second, "Y86 pipeline?", vec![CitationInput { source: csapp, location: "p. 412".into() }])).unwrap();
    let tree = fx.app.module_tree(TODAY).unwrap();
    assert_eq!(node(&tree, NodeRef::Topic(first)).sources, vec![ecs]);
    assert_eq!(node(&tree, NodeRef::Topic(second)).sources, vec![csapp]);
}

/// T-TREE-05 at the command layer: the message the user sees.
#[test]
fn t_tree_05_the_duplicate_message_names_the_module() {
    let mut fx = background();
    fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let error = fx.app.create_topic(" cpu ", Some(fx.n2t)).unwrap_err();
    assert_eq!(error.to_string(), "N2T already has a topic called CPU");
}

// 2. Default at the command layer

/// T-DEF-01, T-DEF-02. Spec: "Confirming puts the topic in Default".
#[test]
fn t_def_01_a_topic_with_no_module_lands_in_default() {
    let mut fx = background();
    assert_eq!(fx.app.default_module().unwrap(), None);
    let misc = fx.app.create_topic("Misc", None).unwrap();
    let default = fx.app.default_module().unwrap().expect("Default made on first use");
    let row = fx.app.topics().unwrap().into_iter().find(|topic| topic.id == misc).unwrap();
    assert_eq!(row.module, Some(default));
    let tree = fx.app.module_tree(TODAY).unwrap();
    assert!(node(&tree, NodeRef::Module(default)).is_default);
    assert_eq!(node(&tree, NodeRef::Topic(misc)).breadcrumb, "Default › Misc");
}

// 3. Moving and deleting with real records

/// T-MOVE-01. Spec: "Move a topic to another module".
#[test]
fn t_move_01_a_topics_questions_cards_and_sources_move_with_it() {
    let mut fx = background();
    let cpu = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let ecs = source(&mut fx.app, "The Elements of Computing Systems");
    fx.app.save_question(question(cpu, "Hack CPU?", vec![CitationInput { source: ecs, location: "p. 94".into() }])).unwrap();
    card(&mut fx.app, cpu, "ALU inputs?");
    fx.app.move_topic(cpu, Some(fx.ucm)).unwrap();
    assert_eq!(fx.app.questions_in(cpu).unwrap().len(), 1);
    assert_eq!(fx.app.topic_cards(cpu, TODAY).unwrap().cards.len(), 1);
    let tree = fx.app.module_tree(TODAY).unwrap();
    let moved = node(&tree, NodeRef::Topic(cpu));
    assert_eq!(moved.parent, Some(fx.ucm));
    assert_eq!(moved.sources, vec![ecs]);
}

/// T-MOVE-03 at the command layer.
#[test]
fn t_move_03_the_cycle_message_is_plain() {
    let mut fx = background();
    let error = fx.app.move_module(fx.ca, Some(fx.n2t)).unwrap_err();
    assert_eq!(error.to_string(), "A module can't go inside one of its own submodules");
}

/// T-DEL-01. Spec: "Deleting a module names everything it removes".
#[test]
fn t_del_01_the_preview_counts_questions_and_cards() {
    let mut fx = background();
    let cpu = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let alu = fx.app.create_topic("ALU", Some(fx.n2t)).unwrap();
    fx.app.save_question(question(cpu, "One?", vec![])).unwrap();
    fx.app.save_question(question(alu, "Two?", vec![])).unwrap();
    card(&mut fx.app, cpu, "Three?");
    let preview = fx.app.module_delete_preview(fx.n2t).unwrap();
    let mut topics = preview.topics.clone();
    topics.sort();
    assert_eq!(topics, vec!["ALU", "CPU"]);
    assert_eq!((preview.questions, preview.cards), (2, 1));
    assert!(preview.clashes.is_empty());
}

/// T-DEL-02. Deleting everything takes the questions with it and leaves past attempts (R30).
#[test]
fn t_del_02_deleting_everything_removes_topics_and_questions() {
    let mut fx = background();
    let cpu = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    fx.app.save_question(question(cpu, "One?", vec![])).unwrap();
    fx.app.delete_module_with(fx.n2t, DeleteChoice::Everything).unwrap();
    assert!(fx.app.topics().unwrap().iter().all(|topic| topic.id != cpu));
    assert!(fx.app.modules().unwrap().iter().all(|module| module.id != fx.n2t));
}

/// T-DEL-03 and T-DEL-04. Spec: "Deleting a module can keep its contents".
#[test]
fn t_del_03_moving_contents_up_keeps_questions_and_reports_clashes() {
    let mut fx = background();
    let cpu = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    fx.app.save_question(question(cpu, "One?", vec![])).unwrap();
    fx.app.create_topic("Motherboard", Some(fx.n2t)).unwrap();
    fx.app.create_topic("Motherboard", Some(fx.ca)).unwrap();
    assert_eq!(fx.app.module_delete_preview(fx.n2t).unwrap().clashes, vec!["Motherboard"]);
    assert!(fx.app.delete_module_with(fx.n2t, DeleteChoice::MoveContentsUp).is_err(), "refused while names clash");
    let clash = fx.app.topics().unwrap().into_iter().find(|topic| topic.name == "Motherboard" && topic.module == Some(fx.n2t)).unwrap().id;
    fx.app.rename_topic(clash, "Hack motherboard").unwrap();
    fx.app.delete_module_with(fx.n2t, DeleteChoice::MoveContentsUp).unwrap();
    let row = fx.app.topics().unwrap().into_iter().find(|topic| topic.id == cpu).unwrap();
    assert_eq!(row.module, Some(fx.ca));
    assert_eq!(fx.app.questions_in(cpu).unwrap().len(), 1);
}

// 5. The tree used elsewhere

/// T-USE-01, T-USE-02. Spec: "Quizzing a module includes its submodules", "Excluding a submodule".
#[test]
fn t_use_01_a_quiz_on_a_module_covers_its_submodules() {
    let mut fx = background();
    let a = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    let c = fx.app.create_topic("Buses", Some(fx.ucm)).unwrap();
    for topic in [a, b, c] {
        fx.app.save_question(question(topic, "Q?", vec![])).unwrap();
    }
    let mut all = fx.app.topics_under(fx.ca, &[]).unwrap();
    all.sort();
    assert_eq!(all, { let mut v = vec![a, b, c]; v.sort(); v });
    let without = fx.app.topics_under(fx.ca, &[fx.csapp]).unwrap();
    assert!(!without.contains(&b));
    let setup = QuizSetupInput { topics: without, count: None, even: false, shuffle_questions: false, shuffle_options: false, exam: true, total_minutes: None, per_question_seconds: None, easy: false, types: study_core::assembly::QuestionTypes::Both };
    assert_eq!(fx.app.start_quiz(setup).unwrap().total, 2);
}

/// T-USE-05. Spec: "Dashboard rolls counts up the tree".
#[test]
fn t_use_05_question_counts_roll_up_to_modules() {
    let mut fx = background();
    let a = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    fx.app.save_question(question(a, "One?", vec![])).unwrap();
    fx.app.save_question(question(a, "Two?", vec![])).unwrap();
    fx.app.save_question(question(b, "Three?", vec![])).unwrap();
    let tree = fx.app.module_tree(TODAY).unwrap();
    assert_eq!(node(&tree, NodeRef::Module(fx.n2t)).question_count, 2);
    assert_eq!(node(&tree, NodeRef::Module(fx.ca)).question_count, 3);
}

// 6. Related topics saved

/// T-REL-01, T-REL-07. Links are saved, and go when a topic does.
#[test]
fn t_rel_01_links_are_saved_and_removed_with_their_topic() {
    let mut fx = background();
    let a = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    fx.app.link_related(a, b).unwrap();
    assert_eq!(fx.app.related_topics(b).unwrap(), vec![a]);
    assert_eq!(node(&fx.app.module_tree(TODAY).unwrap(), NodeRef::Topic(a)).related, vec![b]);
    fx.app.delete_topic(b).unwrap();
    assert!(fx.app.related_topics(a).unwrap().is_empty());
}

// 9. Query search over real records

fn searched() -> (Background, TopicId, TopicId, TopicId) {
    let mut fx = background();
    let cpu = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let other = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    let alu = fx.app.create_topic("ALU", Some(fx.n2t)).unwrap();
    fx.app.save_question(question(cpu, "What does the program counter hold?", vec![])).unwrap();
    fx.app.save_question(question(other, "Which CPU stage reads memory?", vec![])).unwrap();
    fx.app.save_question(question(alu, "The ALU zero flag is used by the CPU to jump", vec![])).unwrap();
    card(&mut fx.app, cpu, "Fetch, decode, execute");
    (fx, cpu, other, alu)
}

/// T-EVAL-09, T-EVAL-10 over saved records.
#[test]
fn t_eval_09_scopes_work_over_saved_records() {
    let (fx, _, _, _) = searched();
    let wide = fx.app.query_search("module:N2T CPU", TODAY).unwrap();
    assert_eq!(wide.results.count(ItemKind::Question), 1, "the ALU question mentioning CPU, not CS:APP's");
    let narrow = fx.app.query_search("module:N2T topic:CPU", TODAY).unwrap();
    assert_eq!(narrow.results.count(ItemKind::Question), 1);
    assert_eq!(narrow.results.count(ItemKind::Flashcard), 1);
}

/// T-EVAL-08 over saved records.
#[test]
fn t_eval_08_a_matching_topic_brings_its_children() {
    let (fx, cpu, _, _) = searched();
    let found = fx.app.query_search("CPU", TODAY).unwrap();
    assert_eq!(found.results.children.get(&(cpu.0 as u64)).map(Vec::len), Some(2), "its question and its card");
}

/// T-EVAL-14. Namesakes come with their full paths.
#[test]
fn t_eval_14_results_carry_full_paths() {
    let (fx, cpu, other, _) = searched();
    let found = fx.app.query_search("topic:CPU", TODAY).unwrap();
    let path = |topic: TopicId| found.paths.iter().find(|(node, _)| *node == NodeRef::Topic(topic)).map(|(_, path)| path.clone());
    assert_eq!(path(cpu).as_deref(), Some("Computer Architecture › N2T › CPU"));
    assert_eq!(path(other).as_deref(), Some("Computer Architecture › CS:APP › CPU"));
}

/// T-PAR-E1 and T-PILL-09 through the command: pills or an error, never both.
#[test]
fn t_pill_09_the_command_returns_pills_or_an_error() {
    let (fx, _, _, _) = searched();
    let good = fx.app.query_search("CPU $ ALU", TODAY).unwrap();
    assert!(good.error.is_none());
    assert!(matches!(good.parts.as_slice(), [Part::Group(_)]));
    assert_eq!(good.spoken, "CPU or ALU");
    let broken = fx.app.query_search("CPU $", TODAY).unwrap();
    assert_eq!(broken.error.as_ref().map(|problem| problem.message.as_str()), Some("$ needs something on its right"));
    assert!(broken.parts.is_empty());
}

/// T-EVAL-13 through the command.
#[test]
fn t_eval_13_unknown_scopes_get_a_suggestion() {
    let (fx, _, _, _) = searched();
    let found = fx.app.query_search("module:n2", TODAY).unwrap();
    assert_eq!(found.unknown.first().and_then(|scope| scope.suggestion.as_deref()), Some("N2T"));
}

// 12. Graph data

/// T-GRAPH-01. Every module and topic is a node, joined to its parent.
#[test]
fn t_graph_01_the_tree_has_a_node_per_module_and_topic() {
    let (fx, _, _, _) = searched();
    let tree = fx.app.module_tree(TODAY).unwrap();
    assert_eq!(tree.nodes.len(), 4 + 3);
    let parents = tree.nodes.iter().filter(|node| node.parent.is_some()).count();
    assert_eq!(parents, 3 + 3, "everything but the root has a parent");
}

// 13. Export and import keep the tree

/// T-EXP-01. Spec: "Export keeps the tree".
#[test]
fn t_exp_01_export_and_import_rebuild_the_tree_and_links() {
    let mut fx = background();
    let a = fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let b = fx.app.create_topic("CPU", Some(fx.csapp)).unwrap();
    let ecs = source(&mut fx.app, "The Elements of Computing Systems");
    fx.app.save_question(question(a, "Hack CPU?", vec![CitationInput { source: ecs, location: "p. 94".into() }])).unwrap();
    fx.app.link_related(a, b).unwrap();
    let (_, bytes) = fx.app.export(&ExportRequest { topics: None, history: false, ..Default::default() }, TODAY).unwrap();

    let mut other = app();
    other.import_preview(&bytes, "file.studyapp").unwrap();
    other.import_apply(ImportChoice { rule: ClashRule::TakeTheirs, theirs: vec![] }, at(TODAY)).unwrap();
    let tree = other.module_tree(TODAY).unwrap();
    let mut paths: Vec<String> = tree.nodes.iter().map(|node| node.breadcrumb.clone()).collect();
    paths.sort();
    assert!(paths.contains(&"Computer Architecture › N2T › CPU".to_string()));
    assert!(paths.contains(&"Computer Architecture › CS:APP › CPU".to_string()));
    let cpu = tree.nodes.iter().find(|node| node.breadcrumb == "Computer Architecture › N2T › CPU").unwrap();
    assert_eq!(cpu.related.len(), 1);
    assert_eq!(cpu.sources.len(), 1);
}

/// T-EXP-02. Importing into a database that already has rows doesn't clash ids.
#[test]
fn t_exp_02_imported_ids_are_remapped() {
    let mut fx = background();
    fx.app.create_topic("CPU", Some(fx.n2t)).unwrap();
    let (_, bytes) = fx.app.export(&ExportRequest { topics: None, history: false, ..Default::default() }, TODAY).unwrap();
    let mut other = app();
    let mine = other.create_submodule("Mine", None).unwrap();
    other.create_topic("Mine topic", Some(mine)).unwrap();
    other.import_preview(&bytes, "file.studyapp").unwrap();
    other.import_apply(ImportChoice { rule: ClashRule::EachOne, theirs: vec![] }, at(TODAY)).unwrap();
    let tree = other.module_tree(TODAY).unwrap();
    assert!(tree.nodes.iter().any(|node| node.breadcrumb == "Mine › Mine topic"));
    assert!(tree.nodes.iter().any(|node| node.breadcrumb == "Computer Architecture › N2T › CPU"));
}

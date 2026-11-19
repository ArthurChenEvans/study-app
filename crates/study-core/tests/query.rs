//! Search query language: tokeniser, parser, pill display model and evaluator (R220).
//!
//! Spec: `specs/features/search-query.feature`.
//! Test IDs: `specs/tests/nested-modules-and-search.md`, sections 7, 8, 9 and 11.

use study_core::query::{
    Brackets, Item, ItemKind, Outline, Part, PillKind, ScopeKind, Spelling, TokenKind, display, matches, parse, plain_english, search,
    spoken, suggest_scopes, tokenise, unknown_scopes,
};

fn kinds(input: &str) -> Vec<TokenKind> {
    tokenise(input).into_iter().map(|token| token.kind).collect()
}

fn term(text: &str) -> TokenKind {
    TokenKind::Term(text.into())
}

fn tree(input: &str) -> String {
    parse(input).unwrap().expr.unwrap().bracketed()
}

fn error(input: &str) -> String {
    parse(input).unwrap_err().message
}

fn shown(input: &str) -> String {
    plain_english(&display(&parse(input).unwrap()))
}

// 7. Tokeniser

/// T-TOK-01
#[test]
fn t_tok_01_words_split_on_spaces() {
    assert_eq!(kinds("CPU ALU"), vec![term("CPU"), term("ALU")]);
}

/// T-TOK-02. Spec: "Quotes keep words together".
#[test]
fn t_tok_02_quotes_make_one_phrase() {
    assert_eq!(kinds("\"CPU Logic\""), vec![TokenKind::Phrase { text: "CPU Logic".into(), closed: true }]);
}

/// T-TOK-03. Spec: "An unclosed quote on Enter".
#[test]
fn t_tok_03_an_unclosed_quote_runs_to_the_end() {
    assert_eq!(kinds("\"CPU Logic"), vec![TokenKind::Phrase { text: "CPU Logic".into(), closed: false }]);
}

/// T-TOK-04
#[test]
fn t_tok_04_symbols_are_operators() {
    use Spelling::Symbol;
    assert_eq!(kinds("& $ ^ !"), vec![TokenKind::And(Symbol), TokenKind::Or(Symbol), TokenKind::Xor(Symbol), TokenKind::Not(Symbol)]);
}

/// T-TOK-05. Spec: "Symbols and capital keywords both work".
#[test]
fn t_tok_05_capital_words_are_operators() {
    use Spelling::Word;
    assert_eq!(kinds("AND OR XOR NOT"), vec![TokenKind::And(Word), TokenKind::Or(Word), TokenKind::Xor(Word), TokenKind::Not(Word)]);
}

/// T-TOK-06. Spec: "Lower-case keywords are plain words".
#[test]
fn t_tok_06_lower_case_keywords_are_words() {
    assert_eq!(kinds("and or"), vec![term("and"), term("or")]);
}

/// T-TOK-07. Spec: "Searching for an operator word itself".
#[test]
fn t_tok_07_a_quoted_keyword_is_a_phrase() {
    assert_eq!(kinds("\"AND\""), vec![TokenKind::Phrase { text: "AND".into(), closed: true }]);
}

/// T-TOK-08
#[test]
fn t_tok_08_not_needs_no_space() {
    assert_eq!(kinds("!CPU"), vec![TokenKind::Not(Spelling::Symbol), term("CPU")]);
}

/// T-TOK-09
#[test]
fn t_tok_09_module_scope() {
    assert_eq!(kinds("module:N2T"), vec![TokenKind::Scope { kind: ScopeKind::Module, value: "N2T".into() }]);
    assert_eq!(kinds("topic:CPU"), vec![TokenKind::Scope { kind: ScopeKind::Topic, value: "CPU".into() }]);
}

/// T-TOK-10. Spec: "Scope names with spaces need quotes".
#[test]
fn t_tok_10_quoted_scope_value() {
    assert_eq!(kinds("module:\"UCM CySec CA Module\""), vec![TokenKind::Scope { kind: ScopeKind::Module, value: "UCM CySec CA Module".into() }]);
}

/// T-TOK-11
#[test]
fn t_tok_11_only_the_first_colon_splits() {
    assert_eq!(kinds("module:CS:APP"), vec![TokenKind::Scope { kind: ScopeKind::Module, value: "CS:APP".into() }]);
}

/// T-TOK-12
#[test]
fn t_tok_12_brackets() {
    assert_eq!(kinds("( )"), vec![TokenKind::LParen, TokenKind::RParen]);
    assert_eq!(kinds("(CPU)"), vec![TokenKind::LParen, term("CPU"), TokenKind::RParen]);
}

/// T-TOK-13
#[test]
fn t_tok_13_a_symbol_ends_a_word() {
    assert_eq!(kinds("box$"), vec![term("box"), TokenKind::Or(Spelling::Symbol)]);
}

/// T-TOK-14
#[test]
fn t_tok_14_extra_whitespace_is_ignored() {
    assert_eq!(kinds("  CPU \t  ALU  "), vec![term("CPU"), term("ALU")]);
}

/// T-TOK-15
#[test]
fn t_tok_15_empty_input_is_an_empty_query() {
    assert!(kinds("").is_empty());
    let query = parse("   ").unwrap();
    assert_eq!(query.expr, None);
}

/// Token positions point at the source text, for outlining the problem pill.
#[test]
fn token_positions_cover_their_text() {
    let tokens = tokenise("CPU $ ALU");
    assert_eq!((tokens[0].start, tokens[0].end), (0, 3));
    assert_eq!((tokens[1].start, tokens[1].end), (4, 5));
    assert_eq!((tokens[2].start, tokens[2].end), (6, 9));
}

// 8. Parser and grouping

/// T-PAR-01. Spec: "Unquoted words are separate pills joined by AND".
#[test]
fn t_par_01_a_space_is_and() {
    assert_eq!(tree("CPU ALU"), "(CPU AND ALU)");
}

/// T-PAR-02. Spec: "An operator joins only its immediate neighbours".
#[test]
fn t_par_02_an_operator_binds_tighter_than_a_space() {
    assert_eq!(tree("N2T CPU $ ALU"), "(N2T AND (CPU OR ALU))");
}

/// T-PAR-03. Spec: "A chain of explicit operators reads left to right".
#[test]
fn t_par_03_explicit_chains_read_left_to_right() {
    assert_eq!(tree("CPU & ALU $ RAM"), "((CPU AND ALU) OR RAM)");
}

/// T-PAR-04. Spec: "Brackets group terms".
#[test]
fn t_par_04_brackets_group() {
    assert_eq!(tree("CPU & (ALU $ RAM)"), "(CPU AND (ALU OR RAM))");
}

/// T-PAR-05. Spec: "NOT works on a group".
#[test]
fn t_par_05_not_on_a_group() {
    assert_eq!(tree("!(ALU $ RAM)"), "NOT (ALU OR RAM)");
}

/// T-PAR-06
#[test]
fn t_par_06_double_not() {
    assert_eq!(tree("!!CPU"), "NOT NOT CPU");
}

/// T-PAR-07
#[test]
fn t_par_07_xor_chains_left_to_right() {
    assert_eq!(tree("CPU ^ ALU ^ RAM"), "((CPU XOR ALU) XOR RAM)");
}

/// T-PAR-08
#[test]
fn t_par_08_spaces_around_an_explicit_group() {
    assert_eq!(tree("a b $ c d"), "((a AND (b OR c)) AND d)");
}

/// T-PAR-09. Spec: "Operators work across scopes".
#[test]
fn t_par_09_operators_across_scopes() {
    assert_eq!(tree("module:N2T $ module:CS:APP topic:CPU"), "((module:N2T OR module:CS:APP) AND topic:CPU)");
}

/// T-PAR-10. Spec: "NOT works on a scope".
#[test]
fn t_par_10_not_on_a_scope() {
    assert_eq!(tree("CPU !module:CS:APP"), "(CPU AND NOT module:CS:APP)");
}

/// T-PAR-11. Spec: "Nested groups nest visually".
#[test]
fn t_par_11_nested_brackets() {
    assert_eq!(tree("CPU & (ALU $ (RAM ^ ROM))"), "(CPU AND (ALU OR (RAM XOR ROM)))");
}

/// Spec: "Symbols and capital keywords both work".
#[test]
fn keywords_and_symbols_parse_the_same() {
    for (symbols, words) in [("CPU & ALU", "CPU AND ALU"), ("CPU $ ALU", "CPU OR ALU"), ("CPU ^ ALU", "CPU XOR ALU"), ("!CPU", "NOT CPU")] {
        assert_eq!(tree(symbols), tree(words), "{symbols} vs {words}");
    }
}

/// Phrases keep their quotes in the bracketed form.
#[test]
fn phrases_show_quoted() {
    assert_eq!(tree("\"CPU Logic\" ALU"), "(\"CPU Logic\" AND ALU)");
}

/// T-PAR-E1. Spec: "Broken queries explain themselves".
#[test]
fn t_par_e1_missing_right_side() {
    assert_eq!(error("CPU $"), "$ needs something on its right");
}

/// T-PAR-E2
#[test]
fn t_par_e2_missing_left_side() {
    assert_eq!(error("& ALU"), "& needs something on its left");
}

/// T-PAR-E3
#[test]
fn t_par_e3_two_operators_in_a_row() {
    let problem = parse("CPU $ & ALU").unwrap_err();
    assert_eq!(problem.message, "Two operators in a row, remove one");
    assert_eq!((problem.start, problem.end), (6, 7), "points at the second operator");
}

/// T-PAR-E4
#[test]
fn t_par_e4_missing_closing_bracket() {
    assert_eq!(error("(CPU $ ALU"), "Missing a closing bracket");
}

/// T-PAR-E5
#[test]
fn t_par_e5_extra_closing_bracket() {
    assert_eq!(error("CPU)"), "Extra closing bracket");
}

/// T-PAR-E6
#[test]
fn t_par_e6_not_with_nothing() {
    assert_eq!(error("!"), "! needs something to apply to");
}

/// T-PAR-E7
#[test]
fn t_par_e7_empty_brackets() {
    assert_eq!(error("()"), "Empty brackets");
}

/// T-PAR-E8. Messages use the user's own spelling.
#[test]
fn t_par_e8_errors_use_what_was_typed() {
    assert_eq!(error("CPU OR"), "OR needs something on its right");
    assert_eq!(error("NOT"), "NOT needs something to apply to");
}

/// An unclosed quote is closed for you, not an error.
#[test]
fn an_unclosed_quote_is_not_an_error() {
    assert_eq!(tree("\"CPU Logic"), "\"CPU Logic\"");
}

// 11. Pill display model

/// T-PILL-09. Spec: "Operator symbols become words".
#[test]
fn t_pill_09_symbols_show_as_words() {
    assert_eq!(shown("CPU & ALU"), "CPU AND ALU");
    assert_eq!(shown("CPU $ ALU"), "CPU OR ALU");
    assert_eq!(shown("CPU ^ ALU"), "CPU XOR ALU");
    assert_eq!(shown("!CPU"), "NOT CPU");
}

/// T-PILL-09. Spec: "Typed keywords look the same as symbols".
#[test]
fn t_pill_09_keywords_look_like_symbols() {
    assert_eq!(display(&parse("CPU OR ALU").unwrap()), display(&parse("CPU $ ALU").unwrap()));
}

/// T-PILL-10. Spec: "The space between terms shows as AND".
#[test]
fn t_pill_10_a_space_shows_as_and() {
    assert_eq!(shown("CPU ALU"), "CPU AND ALU");
}

fn only_group(input: &str) -> study_core::query::Group {
    match display(&parse(input).unwrap()).as_slice() {
        [Part::Group(group)] => group.clone(),
        other => panic!("expected one group, got {other:?}"),
    }
}

/// T-PILL-11 and T-PILL-13. Spec: "An operator merges with the pills it joins", "OR groups have a solid outline".
#[test]
fn t_pill_11_or_merges_into_one_solid_group() {
    let group = only_group("CPU $ ALU");
    assert_eq!(group.outline, Outline::Solid);
    assert_eq!(group.parts.len(), 3);
    assert_eq!(group.parts[1], Part::Op("OR"));
}

/// T-PILL-12. Spec: "AND groups are plain".
#[test]
fn t_pill_12_and_has_no_outline() {
    assert_eq!(only_group("CPU & ALU").outline, Outline::None);
}

/// T-PILL-13. Spec: "XOR groups have a dashed outline".
#[test]
fn t_pill_13_xor_is_dashed() {
    assert_eq!(only_group("CPU ^ ALU").outline, Outline::Dashed);
}

/// T-PILL-14. Spec: "NOT is greyed and struck through".
#[test]
fn t_pill_14_not_greys_a_pill() {
    let parts = display(&parse("!CPU").unwrap());
    assert!(parts.iter().any(|part| matches!(part, Part::Pill(pill) if pill.negated && pill.text == "CPU")));
}

/// T-PILL-14. Spec: "NOT on a group strikes through the whole group".
#[test]
fn t_pill_14_not_greys_a_whole_group() {
    let parts = display(&parse("!(ALU $ RAM)").unwrap());
    assert!(parts.iter().any(|part| matches!(part, Part::Group(group) if group.negated && group.brackets == Brackets::Typed)));
    assert_eq!(shown("!(ALU $ RAM)"), "NOT (ALU OR RAM)");
}

/// T-PILL-15. Spec: "Typed brackets are shown".
#[test]
fn t_pill_15_typed_brackets_are_solid() {
    assert_eq!(shown("CPU & (ALU $ RAM)"), "CPU AND (ALU OR RAM)");
    let parts = display(&parse("CPU & (ALU $ RAM)").unwrap());
    let inner = find_groups(&parts).into_iter().find(|group| group.outline == Outline::Solid).unwrap();
    assert_eq!(inner.brackets, Brackets::Typed);
}

/// T-PILL-15. Spec: "Grouping the app works out is shown as faint brackets".
#[test]
fn t_pill_15_worked_out_grouping_is_faint() {
    assert_eq!(shown("N2T CPU $ ALU"), "N2T AND (CPU OR ALU)");
    let parts = display(&parse("N2T CPU $ ALU").unwrap());
    let inner = find_groups(&parts).into_iter().find(|group| group.outline == Outline::Solid).unwrap();
    assert_eq!(inner.brackets, Brackets::Faint);
}

/// T-PILL-15. Spec: "Left-to-right chains show their grouping".
#[test]
fn t_pill_15_left_to_right_chains_show_faint_brackets() {
    assert_eq!(shown("CPU & ALU $ RAM"), "(CPU AND ALU) OR RAM");
}

/// T-PILL-16. Spec: "Nested groups nest visually".
#[test]
fn t_pill_16_nested_groups_nest() {
    assert_eq!(shown("CPU & (ALU $ (RAM ^ ROM))"), "CPU AND (ALU OR (RAM XOR ROM))");
    let parts = display(&parse("CPU & (ALU $ (RAM ^ ROM))").unwrap());
    let or = find_groups(&parts).into_iter().find(|group| group.outline == Outline::Solid).unwrap();
    assert!(or.parts.iter().any(|part| matches!(part, Part::Group(inner) if inner.outline == Outline::Dashed)), "XOR sits inside OR");
}

/// T-PILL-17. Spec: "A single term gets no brackets".
#[test]
fn t_pill_17_a_single_term_has_no_group() {
    let parts = display(&parse("CPU").unwrap());
    assert!(matches!(parts.as_slice(), [Part::Pill(pill)] if pill.text == "CPU" && pill.kind == PillKind::Term));
}

/// T-PILL-19. Spec: "Searching for an operator word itself".
#[test]
fn t_pill_19_a_quoted_keyword_stays_a_pill() {
    let parts = display(&parse("\"AND\"").unwrap());
    assert!(matches!(parts.as_slice(), [Part::Pill(pill)] if pill.kind == PillKind::Phrase && pill.text == "AND"));
}

/// Scope pills carry their kind for the icon. Spec: "Scope pills show their kind".
#[test]
fn scope_pills_carry_their_kind() {
    let parts = display(&parse("module:N2T").unwrap());
    assert!(matches!(parts.as_slice(), [Part::Pill(pill)] if pill.kind == PillKind::Scope(ScopeKind::Module) && pill.text == "N2T"));
}

/// T-PILL-22. Spec: "The query can be edited as text".
#[test]
fn t_pill_22_edit_as_text_gives_back_the_input() {
    for input in ["CPU & (ALU $ (RAM ^ ROM))", "CPU AND (ALU OR RAM)", "module:N2T CPU $ ALU !draft"] {
        assert_eq!(parse(input).unwrap().input, input);
    }
}

/// T-PILL-23. Spec: "Scope suggestions while typing".
#[test]
fn t_pill_23_scope_suggestions_start_with_the_prefix() {
    let names = vec!["N2T".to_string(), "CS:APP".into(), "Networks".into(), "Computer Architecture".into()];
    assert_eq!(suggest_scopes(&names, "n"), vec!["N2T".to_string(), "Networks".into()]);
}

/// T-PILL-24. Spec: "Pills read correctly to screen readers".
#[test]
fn t_pill_24_screen_reader_text() {
    assert_eq!(spoken(&parse("CPU $ !ALU").unwrap()), "CPU or not ALU");
}

fn find_groups(parts: &[Part]) -> Vec<study_core::query::Group> {
    let mut found = Vec::new();
    for part in parts {
        if let Part::Group(group) = part {
            found.push(group.clone());
            found.extend(find_groups(&group.parts));
        }
    }
    found
}

// 9. Evaluator

fn item(id: u64, kind: ItemKind, text: &str, modules: &[&str], topic: Option<(u64, &str)>) -> Item {
    Item { id, kind, text: text.into(), modules: modules.iter().map(|name| name.to_string()).collect(), topic: topic.map(|(id, name)| (id, name.to_string())) }
}

/// The Background: N2T › CPU with a question, a card and a checklist point, CS:APP › CPU with a
/// question, and N2T › ALU with a question mentioning CPU.
fn library() -> Vec<Item> {
    let ca = "Computer Architecture";
    vec![
        item(1, ItemKind::Module, "Computer Architecture", &[], None),
        item(2, ItemKind::Module, "N2T", &[ca], None),
        item(3, ItemKind::Module, "CS:APP", &[ca], None),
        item(10, ItemKind::Topic, "CPU", &[ca, "N2T"], None),
        item(11, ItemKind::Topic, "CPU", &[ca, "CS:APP"], None),
        item(12, ItemKind::Topic, "ALU", &[ca, "N2T"], None),
        item(20, ItemKind::Question, "What does the program counter hold?", &[ca, "N2T"], Some((10, "CPU"))),
        item(21, ItemKind::Flashcard, "Fetch, decode, execute", &[ca, "N2T"], Some((10, "CPU"))),
        item(22, ItemKind::RecallPoint, "How the CPU fetches", &[ca, "N2T"], Some((10, "CPU"))),
        item(23, ItemKind::Question, "Which CPU stage reads memory?", &[ca, "CS:APP"], Some((11, "CPU"))),
        item(24, ItemKind::Question, "The ALU zero flag is used by the CPU to jump", &[ca, "N2T"], Some((12, "ALU"))),
        item(25, ItemKind::PracticeEntry, "Built the cpu in HDL", &[ca, "N2T"], Some((10, "CPU"))),
    ]
}

fn ids(input: &str) -> Vec<u64> {
    let results = search(&library(), &parse(input).unwrap());
    let mut all: Vec<u64> = results.groups.values().flatten().copied().collect();
    all.sort();
    all
}

fn hit(input: &str, text: &str) -> bool {
    let expr = parse(input).unwrap().expr.unwrap();
    matches(&expr, &item(1, ItemKind::Question, text, &[], None))
}

/// T-EVAL-01. Spec: "Matching is by contains, ignoring case".
#[test]
fn t_eval_01_contains_ignoring_case() {
    for text in ["CPU", "CPUs", "the cpu cycle"] {
        assert!(hit("cpu", text), "{text}");
    }
    assert!(!hit("cpu", "ALU"));
}

/// T-EVAL-02. Spec: "Quotes keep words together".
#[test]
fn t_eval_02_phrases_are_exact_and_contiguous() {
    assert!(hit("\"CPU Logic\"", "CPU Logic gates"));
    assert!(!hit("\"CPU Logic\"", "Logic of a CPU"));
}

/// T-EVAL-03 to T-EVAL-06: truth tables. Spec: "XOR means one or the other, not both".
#[test]
fn t_eval_03_to_06_truth_tables() {
    let cases = [("A only", true, false), ("B only", false, true), ("A and B", true, true), ("neither", false, false)];
    for (text, a, b) in cases {
        let text = text.replace("A", "cpu").replace("B", "alu");
        assert_eq!(hit("cpu & alu", &text), a && b, "AND on {text}");
        assert_eq!(hit("cpu $ alu", &text), a || b, "OR on {text}");
        assert_eq!(hit("cpu ^ alu", &text), a != b, "XOR on {text}");
        assert_eq!(hit("!cpu", &text), !a, "NOT on {text}");
    }
}

/// T-EVAL-07. Spec: "A plain word searches everything".
#[test]
fn t_eval_07_a_plain_word_searches_every_kind() {
    let results = search(&library(), &parse("cpu").unwrap());
    for kind in [ItemKind::Topic, ItemKind::Question, ItemKind::RecallPoint, ItemKind::PracticeEntry] {
        assert!(results.count(kind) > 0, "{kind:?}");
    }
}

/// T-EVAL-08. Spec: "A matching topic shows its children".
#[test]
fn t_eval_08_a_matching_topic_lists_its_children() {
    let results = search(&library(), &parse("CPU").unwrap());
    let mut children = results.children.get(&10).cloned().unwrap_or_default();
    children.sort();
    assert_eq!(children, vec![20, 21, 22, 25], "everything under N2T › CPU, matching or not");
}

/// T-EVAL-09. Spec: "module: narrows to anything inside a module".
#[test]
fn t_eval_09_module_scope_finds_mentions_in_other_topics() {
    let found = ids("module:N2T CPU");
    assert!(found.contains(&24), "the ALU question mentioning CPU");
    assert!(!found.contains(&23), "nothing from CS:APP");
}

/// T-EVAL-10. Spec: "topic: narrows to things under a topic".
#[test]
fn t_eval_10_topic_scope_keeps_to_that_topic() {
    assert_eq!(ids("module:N2T topic:CPU"), vec![20, 21, 22, 25]);
}

/// T-EVAL-11. Spec: "module: includes submodules".
#[test]
fn t_eval_11_module_scope_includes_submodules() {
    let found = ids("module:\"Computer Architecture\" CPU");
    assert!(found.contains(&22) && found.contains(&23));
}

/// T-EVAL-12. Spec: "topic: on its own covers every topic with that name".
#[test]
fn t_eval_12_topic_scope_covers_namesakes() {
    let found = ids("topic:CPU");
    assert!(found.contains(&20) && found.contains(&23));
}

/// T-EVAL-13. Spec: "Scope names match exactly, ignoring case".
#[test]
fn t_eval_13_scope_names_match_whole_ignoring_case() {
    assert_eq!(ids("module:n2t topic:cpu"), vec![20, 21, 22, 25]);
    assert!(ids("module:n2").is_empty());
    let modules = vec!["Computer Architecture".to_string(), "N2T".into(), "CS:APP".into()];
    let unknown = unknown_scopes(&parse("module:n2").unwrap(), &modules, &[]);
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].suggestion.as_deref(), Some("N2T"));
}

/// T-EVAL-14. Spec: "Two modules with the same name".
#[test]
fn t_eval_14_same_named_modules_are_both_searched() {
    let mut items = library();
    items.push(item(30, ItemKind::Question, "Hobby CPU build", &["Hobby", "N2T"], Some((31, "Builds"))));
    let results = search(&items, &parse("module:N2T CPU").unwrap());
    let all: Vec<u64> = results.groups.values().flatten().copied().collect();
    assert!(all.contains(&30) && all.contains(&24));
}

/// T-EVAL-15. Spec: "Results are grouped by kind".
#[test]
fn t_eval_15_groups_count_their_items() {
    let results = search(&library(), &parse("cpu").unwrap());
    for (kind, list) in &results.groups {
        assert_eq!(results.count(*kind), list.len());
    }
}

/// T-EVAL-16
#[test]
fn t_eval_16_default_topics_are_searchable() {
    let items = vec![item(40, ItemKind::Question, "Loose question", &["Default"], Some((41, "Misc")))];
    let results = search(&items, &parse("module:Default").unwrap());
    assert_eq!(results.count(ItemKind::Question), 1);
}

/// T-TOK-15. An empty query matches nothing.
#[test]
fn an_empty_query_matches_nothing() {
    assert!(ids("").is_empty());
}

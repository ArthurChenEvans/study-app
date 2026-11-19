//! The search query language (R220), used by the main search, list filters and graph search.
//!
//! A query is words, `"quoted phrases"` and `module:` / `topic:` scopes, joined by operators
//! written as symbols or capital words:
//!
//! | Symbol | Word  | Meaning              |
//! |--------|-------|----------------------|
//! | `&`    | `AND` | both                 |
//! | `$`    | `OR`  | either               |
//! | `^`    | `XOR` | one but not both     |
//! | `!`    | `NOT` | leave out (prefix)   |
//!
//! A space between two terms is an AND. An explicit operator joins only its immediate
//! neighbours, so it binds tighter than the space: `N2T CPU $ ALU` is N2T AND (CPU OR ALU).
//! A chain of explicit operators reads left to right, and brackets group anything.
//!
//! The parsed query also gives the display model the search box draws as pills (symbols shown
//! as words, grouping always visible), so those rules are tested here and not in the UI.
//!
//! Specs: `specs/features/search-query.feature`.
//! Tests: `specs/tests/nested-modules-and-search.md`, sections 7 to 11.

use std::collections::BTreeMap;

use serde::Serialize;

// ---------------------------------------------------------------------------------------------
// Tokens (tests T-TOK-01 to T-TOK-15)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ScopeKind {
    /// `module:`, anything inside a module or its submodules.
    Module,
    /// `topic:`, only things under a topic with that name.
    Topic,
}

/// How an operator was typed, so error messages and "Edit as text" use the user's own form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spelling {
    Symbol,
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Term(String),
    /// `closed` is false when the closing quote was missing (T-TOK-03).
    Phrase { text: String, closed: bool },
    /// Only the first colon splits, so `module:CS:APP` names "CS:APP" (T-TOK-11).
    Scope { kind: ScopeKind, value: String },
    And(Spelling),
    Or(Spelling),
    Xor(Spelling),
    Not(Spelling),
    LParen,
    RParen,
}

/// A token and the byte range it came from, for pointing at the problem pill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

/// Splits a query into tokens. Never fails: problems are the parser's to report.
///
/// Capital `AND OR XOR NOT` are operators, lower case are plain words (T-TOK-06), and a quoted
/// `"AND"` is always a phrase (T-TOK-07). The symbols `& $ ^ ! ( )` end a word, so `box$` is
/// `box` then OR (T-TOK-13).
pub fn tokenise(input: &str) -> Vec<Token> {
    let _ = input;
    todo!("R220: tokenise a query")
}

// ---------------------------------------------------------------------------------------------
// Parsed query (tests T-PAR-01 to T-PAR-11, T-PAR-E1 to T-PAR-E8)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Term(String),
    Phrase(String),
    Scope { kind: ScopeKind, name: String },
    Not(Box<Expr>),
    /// `implicit` is true when the AND came from a space.
    And { left: Box<Expr>, right: Box<Expr>, implicit: bool },
    Or(Box<Expr>, Box<Expr>),
    Xor(Box<Expr>, Box<Expr>),
    /// Brackets the user typed. Grouping that only comes from the rules has no `Group`.
    Group(Box<Expr>),
}

impl Expr {
    /// Fully bracketed form used to compare trees in tests: `(N2T AND (CPU OR ALU))`,
    /// `NOT (ALU OR RAM)`, `module:CS:APP`, `"CPU Logic"`. Typed groups add nothing.
    pub fn bracketed(&self) -> String {
        todo!("R220: canonical bracketed form")
    }

    /// A query string, with symbols and every group bracketed, that parses back to the same
    /// tree (T-PROP-01).
    pub fn to_query(&self) -> String {
        todo!("R220: expression back to query text")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Exactly what was typed, for "Edit as text" (T-PILL-22).
    pub input: String,
    /// `None` for an empty query, which matches nothing and shows no error (T-TOK-15).
    pub expr: Option<Expr>,
}

/// A broken query, with the byte range of the token to outline.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[error("{message}")]
pub struct QueryError {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

/// Parses a query. Spec: "Operators" scenarios.
///
/// Messages (T-PAR-E1 to T-PAR-E8), with `X` spelled as the user typed it:
/// "X needs something on its right", "X needs something on its left",
/// "Two operators in a row, remove one", "Missing a closing bracket",
/// "Extra closing bracket", "X needs something to apply to", "Empty brackets".
pub fn parse(input: &str) -> Result<Query, QueryError> {
    let _ = input;
    todo!("R220: parse a query")
}

// ---------------------------------------------------------------------------------------------
// Pill display model (tests T-PILL-09 to T-PILL-24, the core half)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PillKind {
    Term,
    Phrase,
    Scope(ScopeKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pill {
    pub text: String,
    pub kind: PillKind,
    /// Greyed and struck through (T-PILL-14).
    pub negated: bool,
}

/// Outline around a group. Spec: "AND groups are plain", "OR groups have a solid outline",
/// "XOR groups have a dashed outline".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Outline {
    None,
    Solid,
    Dashed,
}

/// Brackets drawn around a group. Spec: "Typed brackets are shown", "Grouping the app works
/// out is shown as faint brackets".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Brackets {
    None,
    Typed,
    Faint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    pub outline: Outline,
    pub brackets: Brackets,
    /// The whole group greyed and struck through (T-PILL-14).
    pub negated: bool,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Part {
    Pill(Pill),
    /// An operator as its word: "AND", "OR", "XOR" or "NOT", whatever was typed (T-PILL-09).
    Op(&'static str),
    Group(Group),
}

/// What the search box draws for a query. A single term has no brackets or outline
/// (T-PILL-17), and a group nested in the query is nested here (T-PILL-16).
pub fn display(query: &Query) -> Vec<Part> {
    let _ = query;
    todo!("R220: build the pill display model")
}

/// The display read as one line, brackets included whether typed or faint:
/// `CPU AND (ALU OR (RAM XOR ROM))`. Spec: "Operator symbols become words".
pub fn plain_english(parts: &[Part]) -> String {
    let _ = parts;
    todo!("R220: display as plain English")
}

/// What a screen reader says for the query, in lower-case words: "CPU or not ALU" (T-PILL-24).
pub fn spoken(query: &Query) -> String {
    let _ = query;
    todo!("R220: screen reader text")
}

/// Scope names starting with `prefix`, ignoring case, in name order, for autocomplete.
/// Spec: "Scope suggestions while typing". Test: T-PILL-23.
pub fn suggest_scopes(names: &[String], prefix: &str) -> Vec<String> {
    let _ = (names, prefix);
    todo!("R220: scope suggestions")
}

// ---------------------------------------------------------------------------------------------
// Evaluation (tests T-EVAL-01 to T-EVAL-16)
// ---------------------------------------------------------------------------------------------

/// Result groups, in the order the screen shows them. Spec: "Results are grouped by kind".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ItemKind {
    Module,
    Topic,
    Question,
    Flashcard,
    RecallPoint,
    PracticeEntry,
}

/// One searchable thing, flattened by the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: u64,
    pub kind: ItemKind,
    /// Everything a plain term looks in (a name, a question with its options, a card's sides).
    pub text: String,
    /// Names of the modules it sits in, root first. A module lists its own ancestors only.
    pub modules: Vec<String>,
    /// The topic it sits under, for anything inside a topic. `None` for modules and topics.
    pub topic: Option<(u64, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Results {
    /// Matching item ids per kind, in the order given.
    pub groups: BTreeMap<ItemKind, Vec<u64>>,
    /// For each matching topic, the ids of everything under it (T-EVAL-08).
    pub children: BTreeMap<u64, Vec<u64>>,
}

impl Results {
    pub fn count(&self, kind: ItemKind) -> usize {
        self.groups.get(&kind).map_or(0, Vec::len)
    }
}

/// Whether one item matches. Terms and phrases look in `text`, ignoring case; a phrase must be
/// contiguous (T-EVAL-02). `module:X` matches anything with a module named X among its
/// `modules` (so submodules count, T-EVAL-11). `topic:X` matches anything under a topic named X.
/// Scope names match whole, ignoring case (T-EVAL-13).
pub fn matches(expr: &Expr, item: &Item) -> bool {
    let _ = (expr, item);
    todo!("R220: match one item")
}

/// Runs a query over every item. An empty query matches nothing.
pub fn search(items: &[Item], query: &Query) -> Results {
    let _ = (items, query);
    todo!("R220: search")
}

/// A scope that names nothing, with the closest name to offer. Spec: "Scope names match
/// exactly, ignoring case" (`module:n2` suggests "N2T").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnknownScope {
    pub kind: ScopeKind,
    pub value: String,
    pub suggestion: Option<String>,
}

/// Every scope in the query that matches no module or topic name. Test: T-EVAL-13.
pub fn unknown_scopes(query: &Query, modules: &[String], topics: &[String]) -> Vec<UnknownScope> {
    let _ = (query, modules, topics);
    todo!("R220: unknown scopes")
}

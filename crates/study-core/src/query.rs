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
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let end_of = |index: usize| chars.get(index).map_or(input.len(), |(at, _)| *at);
    let special = |c: char| matches!(c, '&' | '$' | '^' | '!' | '(' | ')' | '"');
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let (start, c) = chars[index];
        if c.is_whitespace() {
            index += 1;
            continue;
        }
        let single = match c {
            '&' => Some(TokenKind::And(Spelling::Symbol)),
            '$' => Some(TokenKind::Or(Spelling::Symbol)),
            '^' => Some(TokenKind::Xor(Spelling::Symbol)),
            '!' => Some(TokenKind::Not(Spelling::Symbol)),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            _ => None,
        };
        if let Some(kind) = single {
            index += 1;
            tokens.push(Token { kind, start, end: end_of(index) });
            continue;
        }
        if c == '"' {
            let (text, closed, next) = quoted(&chars, index + 1);
            index = next;
            tokens.push(Token { kind: TokenKind::Phrase { text, closed }, start, end: end_of(index) });
            continue;
        }
        let mut word = String::new();
        while index < chars.len() && !chars[index].1.is_whitespace() && !special(chars[index].1) {
            word.push(chars[index].1);
            index += 1;
        }
        let lower = word.to_lowercase();
        let scope = [("module:", ScopeKind::Module), ("topic:", ScopeKind::Topic)].into_iter().find(|(prefix, _)| lower.starts_with(prefix));
        let kind = match (word.as_str(), scope) {
            ("AND", _) => TokenKind::And(Spelling::Word),
            ("OR", _) => TokenKind::Or(Spelling::Word),
            ("XOR", _) => TokenKind::Xor(Spelling::Word),
            ("NOT", _) => TokenKind::Not(Spelling::Word),
            (_, Some((prefix, kind))) => {
                let mut value = word[prefix.len()..].to_string();
                if value.is_empty() && index < chars.len() && chars[index].1 == '"' {
                    let (text, _, next) = quoted(&chars, index + 1);
                    value = text;
                    index = next;
                }
                if value.trim().is_empty() { TokenKind::Term(word) } else { TokenKind::Scope { kind, value: value.trim().to_string() } }
            }
            _ => TokenKind::Term(word),
        };
        tokens.push(Token { kind, start, end: end_of(index) });
    }
    tokens
}

/// Reads up to the closing quote: the text, whether it was closed, and the next index.
fn quoted(chars: &[(usize, char)], from: usize) -> (String, bool, usize) {
    let mut text = String::new();
    let mut index = from;
    while index < chars.len() {
        if chars[index].1 == '"' {
            return (text.trim().to_string(), true, index + 1);
        }
        text.push(chars[index].1);
        index += 1;
    }
    (text.trim().to_string(), false, index)
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
        match self {
            Expr::Term(text) => text.clone(),
            Expr::Phrase(text) => format!("\"{text}\""),
            Expr::Scope { kind, name } => scope_text(*kind, name),
            Expr::Not(inner) => format!("NOT {}", inner.bracketed()),
            Expr::And { left, right, .. } => format!("({} AND {})", left.bracketed(), right.bracketed()),
            Expr::Or(left, right) => format!("({} OR {})", left.bracketed(), right.bracketed()),
            Expr::Xor(left, right) => format!("({} XOR {})", left.bracketed(), right.bracketed()),
            Expr::Group(inner) => inner.bracketed(),
        }
}

    /// A query string, with symbols and every group bracketed, that parses back to the same
    /// tree (T-PROP-01).
    pub fn to_query(&self) -> String {
        match self {
            Expr::Term(text) => text.clone(),
            Expr::Phrase(text) => format!("\"{text}\""),
            Expr::Scope { kind, name } => scope_text(*kind, name),
            Expr::Not(inner) => format!("!{}", inner.to_query()),
            Expr::And { left, right, .. } => format!("({} & {})", left.to_query(), right.to_query()),
            Expr::Or(left, right) => format!("({} $ {})", left.to_query(), right.to_query()),
            Expr::Xor(left, right) => format!("({} ^ {})", left.to_query(), right.to_query()),
            Expr::Group(inner) => inner.to_query(),
        }
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
    let tokens = tokenise(input);
    let mut parser = Parser { input, tokens: &tokens, at: 0 };
    let expr = if tokens.is_empty() { None } else { Some(parser.sequence()?) };
    if let Some(token) = tokens.get(parser.at) {
        return Err(parser.problem(token, "Extra closing bracket".into()));
    }
    Ok(Query { input: input.to_string(), expr })
}

fn scope_text(kind: ScopeKind, name: &str) -> String {
    let prefix = match kind {
        ScopeKind::Module => "module:",
        ScopeKind::Topic => "topic:",
    };
    if name.chars().any(char::is_whitespace) { format!("{prefix}\"{name}\"") } else { format!("{prefix}{name}") }
}

#[derive(Clone, Copy)]
enum Binary {
    And,
    Or,
    Xor,
}

struct Parser<'a> {
    input: &'a str,
    tokens: &'a [Token],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn spelled(&self, token: &Token) -> String {
        self.input[token.start..token.end].to_string()
    }

    fn problem(&self, token: &Token, message: String) -> QueryError {
        QueryError { message, start: token.start, end: token.end }
    }

    fn binary(token: &Token) -> Option<Binary> {
        match token.kind {
            TokenKind::And(_) => Some(Binary::And),
            TokenKind::Or(_) => Some(Binary::Or),
            TokenKind::Xor(_) => Some(Binary::Xor),
            _ => None,
        }
    }

    /// Terms side by side, each joined to the last by an implicit AND.
    fn sequence(&mut self) -> Result<Expr, QueryError> {
        let mut left = self.explicit()?;
        while let Some(token) = self.peek() {
            if matches!(token.kind, TokenKind::RParen) {
                break;
            }
            let right = self.explicit()?;
            left = Expr::And { left: Box::new(left), right: Box::new(right), implicit: true };
        }
        Ok(left)
    }

    /// Unary terms joined by explicit operators, read left to right.
    fn explicit(&mut self) -> Result<Expr, QueryError> {
        let mut left = self.unary(None)?;
        while let Some(token) = self.peek() {
            let Some(operator) = Self::binary(token) else { break };
            let token = token.clone();
            self.at += 1;
            match self.peek() {
                None => return Err(self.problem(&token, format!("{} needs something on its right", self.spelled(&token)))),
                Some(next) if matches!(next.kind, TokenKind::RParen) => return Err(self.problem(&token, format!("{} needs something on its right", self.spelled(&token)))),
                _ => {}
            }
            let right = self.unary(Some(&token))?;
            left = match operator {
                Binary::And => Expr::And { left: Box::new(left), right: Box::new(right), implicit: false },
                Binary::Or => Expr::Or(Box::new(left), Box::new(right)),
                Binary::Xor => Expr::Xor(Box::new(left), Box::new(right)),
            };
        }
        Ok(left)
    }

    fn unary(&mut self, after: Option<&Token>) -> Result<Expr, QueryError> {
        let token = self.peek().cloned().expect("callers check for a token");
        if Self::binary(&token).is_some() {
            return Err(match after {
                Some(_) => self.problem(&token, "Two operators in a row, remove one".into()),
                None => self.problem(&token, format!("{} needs something on its left", self.spelled(&token))),
            });
        }
        self.at += 1;
        match &token.kind {
            TokenKind::Not(_) => match self.peek() {
                Some(next) if !matches!(next.kind, TokenKind::RParen) && Self::binary(next).is_none() => Ok(Expr::Not(Box::new(self.unary(None)?))),
                _ => Err(self.problem(&token, format!("{} needs something to apply to", self.spelled(&token)))),
            },
            TokenKind::Term(text) => Ok(Expr::Term(text.clone())),
            TokenKind::Phrase { text, .. } => Ok(Expr::Phrase(text.clone())),
            TokenKind::Scope { kind, value } => Ok(Expr::Scope { kind: *kind, name: value.clone() }),
            TokenKind::LParen => {
                match self.peek() {
                    None => return Err(self.problem(&token, "Missing a closing bracket".into())),
                    Some(next) if matches!(next.kind, TokenKind::RParen) => {
                        let end = next.end;
                        return Err(QueryError { message: "Empty brackets".into(), start: token.start, end });
                    }
                    _ => {}
                }
                let inner = self.sequence()?;
                match self.peek() {
                    Some(close) if matches!(close.kind, TokenKind::RParen) => {
                        self.at += 1;
                        Ok(Expr::Group(Box::new(inner)))
                    }
                    _ => Err(self.problem(&token, "Missing a closing bracket".into())),
                }
            }
            TokenKind::RParen => Err(self.problem(&token, "Extra closing bracket".into())),
            TokenKind::And(_) | TokenKind::Or(_) | TokenKind::Xor(_) => unreachable!("checked above"),
        }
    }
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
    match &query.expr {
        None => vec![],
        Some(expr) => match expr {
            Expr::And { .. } | Expr::Or(..) | Expr::Xor(..) => vec![Part::Group(group_of(expr, Brackets::None, false))],
            other => parts_of(other, false),
        },
    }
}

fn outline_of(expr: &Expr) -> (Outline, &'static str) {
    match expr {
        Expr::And { .. } => (Outline::None, "AND"),
        Expr::Or(..) => (Outline::Solid, "OR"),
        Expr::Xor(..) => (Outline::Dashed, "XOR"),
        _ => (Outline::None, ""),
    }
}

fn sides(expr: &Expr) -> Option<(&Expr, &Expr)> {
    match expr {
        Expr::And { left, right, .. } | Expr::Or(left, right) | Expr::Xor(left, right) => Some((left, right)),
        _ => None,
    }
}

fn same_operator(first: &Expr, second: &Expr) -> bool {
    std::mem::discriminant(first) == std::mem::discriminant(second)
}

/// A binary expression as one group. A side with the same operator (and no typed brackets)
/// joins the group, so `a & b & c` stays flat.
fn group_of(expr: &Expr, brackets: Brackets, negated: bool) -> Group {
    let (outline, word) = outline_of(expr);
    let mut parts = Vec::new();
    fn walk(expr: &Expr, top: &Expr, word: &'static str, parts: &mut Vec<Part>) {
        let (left, right) = sides(expr).expect("a binary expression");
        for (index, side) in [left, right].into_iter().enumerate() {
            if index == 1 {
                parts.push(Part::Op(word));
            }
            if same_operator(side, top) {
                walk(side, top, word, parts);
            } else {
                parts.extend(child_parts(side));
            }
        }
    }
    walk(expr, expr, word, &mut parts);
    Group { outline, brackets, negated, parts }
}

/// A side of a group: a binary side gets brackets, typed or worked out.
fn child_parts(expr: &Expr) -> Vec<Part> {
    match expr {
        Expr::Group(inner) if sides(inner).is_some() => vec![Part::Group(group_of(inner, Brackets::Typed, false))],
        Expr::Group(inner) => child_parts(inner),
        Expr::And { .. } | Expr::Or(..) | Expr::Xor(..) => vec![Part::Group(group_of(expr, Brackets::Faint, false))],
        other => parts_of(other, false),
    }
}

fn parts_of(expr: &Expr, negated: bool) -> Vec<Part> {
    let pill = |text: &str, kind: PillKind| vec![Part::Pill(Pill { text: text.to_string(), kind, negated })];
    match expr {
        Expr::Term(text) => pill(text, PillKind::Term),
        Expr::Phrase(text) => pill(text, PillKind::Phrase),
        Expr::Scope { kind, name } => pill(name, PillKind::Scope(*kind)),
        Expr::Not(inner) => {
            let mut parts = vec![Part::Op("NOT")];
            match inner.as_ref() {
                Expr::Group(grouped) if sides(grouped).is_some() => parts.push(Part::Group(group_of(grouped, Brackets::Typed, true))),
                Expr::And { .. } | Expr::Or(..) | Expr::Xor(..) => parts.push(Part::Group(group_of(inner, Brackets::Faint, true))),
                other => parts.extend(parts_of(other, true)),
            }
            parts
        }
        Expr::Group(inner) if sides(inner).is_some() => vec![Part::Group(group_of(inner, Brackets::Typed, negated))],
        Expr::Group(inner) => parts_of(inner, negated),
        Expr::And { .. } | Expr::Or(..) | Expr::Xor(..) => vec![Part::Group(group_of(expr, Brackets::None, negated))],
    }
}

/// The display read as one line, brackets included whether typed or faint:
/// `CPU AND (ALU OR (RAM XOR ROM))`. Spec: "Operator symbols become words".
pub fn plain_english(parts: &[Part]) -> String {
    let words: Vec<String> = parts
        .iter()
        .map(|part| match part {
            Part::Pill(pill) => match pill.kind {
                PillKind::Term => pill.text.clone(),
                PillKind::Phrase => format!("\"{}\"", pill.text),
                PillKind::Scope(kind) => scope_text(kind, &pill.text),
            },
            Part::Op(word) => word.to_string(),
            Part::Group(group) => {
                let inner = plain_english(&group.parts);
                if group.brackets == Brackets::None { inner } else { format!("({inner})") }
            }
        })
        .collect();
    words.join(" ")
}

/// What a screen reader says for the query, in lower-case words: "CPU or not ALU" (T-PILL-24).
pub fn spoken(query: &Query) -> String {
    fn say(parts: &[Part]) -> String {
        parts
            .iter()
            .map(|part| match part {
                Part::Pill(pill) => match pill.kind {
                    PillKind::Scope(ScopeKind::Module) => format!("module {}", pill.text),
                    PillKind::Scope(ScopeKind::Topic) => format!("topic {}", pill.text),
                    _ => pill.text.clone(),
                },
                Part::Op(word) => word.to_lowercase(),
                Part::Group(group) => say(&group.parts),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    say(&display(query))
}

/// Scope names starting with `prefix`, ignoring case, in name order, for autocomplete.
/// Spec: "Scope suggestions while typing". Test: T-PILL-23.
pub fn suggest_scopes(names: &[String], prefix: &str) -> Vec<String> {
    let prefix = prefix.trim().to_lowercase();
    let mut found: Vec<String> = names.iter().filter(|name| name.to_lowercase().starts_with(&prefix)).cloned().collect();
    found.sort_by_key(|name| name.to_lowercase());
    found.dedup_by(|first, second| crate::tree::same_name(first, second));
    found
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
    let text = || item.text.to_lowercase();
    match expr {
        Expr::Term(word) => text().contains(&word.to_lowercase()),
        Expr::Phrase(phrase) => text().contains(&phrase.to_lowercase()),
        Expr::Scope { kind: ScopeKind::Module, name } => item.modules.iter().any(|module| crate::tree::same_name(module, name)),
        Expr::Scope { kind: ScopeKind::Topic, name } => item.topic.as_ref().is_some_and(|(_, topic)| crate::tree::same_name(topic, name)),
        Expr::Not(inner) => !matches(inner, item),
        Expr::And { left, right, .. } => matches(left, item) && matches(right, item),
        Expr::Or(left, right) => matches(left, item) || matches(right, item),
        Expr::Xor(left, right) => matches(left, item) != matches(right, item),
        Expr::Group(inner) => matches(inner, item),
    }
}

/// Runs a query over every item. An empty query matches nothing.
pub fn search(items: &[Item], query: &Query) -> Results {
    let mut results = Results::default();
    let Some(expr) = &query.expr else { return results };
    for item in items {
        if matches(expr, item) {
            results.groups.entry(item.kind).or_default().push(item.id);
            if item.kind == ItemKind::Topic {
                let children = items.iter().filter(|child| child.topic.as_ref().is_some_and(|(topic, _)| *topic == item.id)).map(|child| child.id).collect();
                results.children.insert(item.id, children);
            }
        }
    }
    results
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
    fn scopes<'a>(expr: &'a Expr, found: &mut Vec<(ScopeKind, &'a str)>) {
        match expr {
            Expr::Scope { kind, name } => found.push((*kind, name)),
            Expr::Not(inner) | Expr::Group(inner) => scopes(inner, found),
            Expr::And { left, right, .. } | Expr::Or(left, right) | Expr::Xor(left, right) => {
                scopes(left, found);
                scopes(right, found);
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    if let Some(expr) = &query.expr {
        scopes(expr, &mut found);
    }
    found
        .into_iter()
        .filter_map(|(kind, value)| {
            let names = match kind {
                ScopeKind::Module => modules,
                ScopeKind::Topic => topics,
            };
            if names.iter().any(|name| crate::tree::same_name(name, value)) {
                return None;
            }
            let lower = value.to_lowercase();
            let suggestion = names.iter().find(|name| name.to_lowercase().starts_with(&lower)).or_else(|| names.iter().find(|name| name.to_lowercase().contains(&lower))).cloned();
            Some(UnknownScope { kind, value: value.to_string(), suggestion })
        })
        .collect()
}

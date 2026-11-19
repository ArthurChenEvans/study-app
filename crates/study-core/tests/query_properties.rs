//! Properties of the search query language (R220) that hold for any query proptest invents.
//!
//! Test IDs: `specs/tests/nested-modules-and-search.md`, section 10 (T-PROP-01 to T-PROP-06).

use proptest::prelude::*;
use study_core::query::{Expr, Item, ItemKind, ScopeKind, matches, parse};

/// Lower-case words, so none of them is an operator keyword.
fn word() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,6}"
}

fn leaf() -> impl Strategy<Value = Expr> {
    prop_oneof![
        word().prop_map(Expr::Term),
        (word(), word()).prop_map(|(first, second)| Expr::Phrase(format!("{first} {second}"))),
        word().prop_map(|name| Expr::Scope { kind: ScopeKind::Module, name }),
        word().prop_map(|name| Expr::Scope { kind: ScopeKind::Topic, name }),
    ]
}

/// Any expression without typed groups (those compare the same as their contents).
fn expr() -> impl Strategy<Value = Expr> {
    leaf().prop_recursive(4, 24, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|expr| Expr::Not(Box::new(expr))),
            (inner.clone(), inner.clone()).prop_map(|(left, right)| Expr::And { left: Box::new(left), right: Box::new(right), implicit: false }),
            (inner.clone(), inner.clone()).prop_map(|(left, right)| Expr::Or(Box::new(left), Box::new(right))),
            (inner.clone(), inner).prop_map(|(left, right)| Expr::Xor(Box::new(left), Box::new(right))),
        ]
    })
}

fn item_with(text: String, module: String, topic: String) -> Item {
    Item { id: 1, kind: ItemKind::Question, text, modules: vec![module], topic: Some((2, topic)) }
}

fn any_item() -> impl Strategy<Value = Item> {
    ("[a-z0-9 ]{0,30}", word(), word()).prop_map(|(text, module, topic)| item_with(text, module, topic))
}

proptest! {
    /// T-PROP-01. Writing a tree out and parsing it back gives the same tree.
    #[test]
    fn t_prop_01_to_query_round_trips(tree in expr()) {
        let back = parse(&tree.to_query()).unwrap().expr.unwrap();
        prop_assert_eq!(back.bracketed(), tree.bracketed());
    }

    /// T-PROP-02. The symbol and keyword spellings give identical trees.
    #[test]
    fn t_prop_02_symbols_and_keywords_agree(tree in expr()) {
        let symbols = tree.to_query();
        let words = symbols.replace(" & ", " AND ").replace(" $ ", " OR ").replace(" ^ ", " XOR ").replace('!', "NOT ");
        prop_assert_eq!(parse(&words).unwrap().expr.unwrap().bracketed(), tree.bracketed());
    }

    /// T-PROP-03. NOT NOT x matches exactly what x matches.
    #[test]
    fn t_prop_03_double_negation(tree in expr(), item in any_item()) {
        let doubled = Expr::Not(Box::new(Expr::Not(Box::new(tree.clone()))));
        prop_assert_eq!(matches(&doubled, &item), matches(&tree, &item));
    }

    /// T-PROP-04. x XOR y is (x OR y) AND NOT (x AND y).
    #[test]
    fn t_prop_04_xor_definition(left in expr(), right in expr(), item in any_item()) {
        let xor = Expr::Xor(Box::new(left.clone()), Box::new(right.clone()));
        let both = Expr::And { left: Box::new(left.clone()), right: Box::new(right.clone()), implicit: false };
        let either = Expr::Or(Box::new(left), Box::new(right));
        let spelled_out = Expr::And { left: Box::new(either), right: Box::new(Expr::Not(Box::new(both))), implicit: false };
        prop_assert_eq!(matches(&xor, &item), matches(&spelled_out, &item));
    }

    /// T-PROP-05. Typing the brackets the display shows faint changes nothing.
    #[test]
    fn t_prop_05_typing_the_faint_brackets_changes_nothing(first in word(), second in word(), third in word()) {
        let implied = parse(&format!("{first} {second} $ {third}")).unwrap().expr.unwrap();
        let typed = parse(&format!("{first} ({second} $ {third})")).unwrap().expr.unwrap();
        prop_assert_eq!(implied.bracketed(), typed.bracketed());
    }

    /// T-PROP-06. Any input gives a query or an error, never a panic.
    #[test]
    fn t_prop_06_junk_never_panics(input in r#"[a-zA-Z0-9 &$^!()":]{0,40}"#) {
        let _ = parse(&input);
    }
}

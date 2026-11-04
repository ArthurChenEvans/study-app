//! Short answer questions (R79 to R82): what an answer is checked against, and marking.

use serde::{Deserialize, Serialize};

/// How a short answer is checked, plus what's shown after answering.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ShortAnswer {
    pub check: Check,
    /// Markdown shown after answering. Blank shows the first accepted answer.
    pub model_answer: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// Right when the answer matches any of these (R80).
    Accepted { answers: Vec<String>, typos: bool },
    /// Right when the answer contains enough of these, in any order (R81).
    /// `needed: None` means all of them.
    Keywords { keywords: Vec<Keyword>, needed: Option<usize> },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Keyword {
    pub word: String,
    /// Other ways of writing it.
    pub also: Vec<String>,
}

/// The app's verdict on one answer.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Marking {
    pub right: bool,
    pub detail: Detail,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Detail {
    /// Matched an accepted answer. `typo` is the first word written differently.
    Matched { answer: String, typo: Option<String> },
    /// Matched none of the accepted answers.
    NoMatch,
    /// Keywords found and missed, in the order they were written in the editor.
    Keywords { hit: Vec<String>, missed: Vec<String>, needed: usize },
}

/// Markdown on one line: code fences dropped and the lines joined, for a checklist point.
pub fn one_line(markdown: &str) -> String {
    markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("```"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lowercase words, with punctuation as spaces.
fn words(text: &str) -> Vec<String> {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(String::from)
        .collect()
}

/// Words with a leading "the", "a" or "an" dropped, when there's more after it.
fn answer_words(text: &str) -> Vec<String> {
    let mut found = words(text);
    if found.len() > 1 && ["the", "a", "an"].contains(&found[0].as_str()) {
        found.remove(0);
    }
    found
}

/// Typos allowed in a word: one for 4 to 7 letters, two from 8, none in numbers or short words.
fn allowance(expected: &str) -> usize {
    if expected.chars().any(|c| c.is_numeric()) {
        return 0;
    }
    match expected.chars().count() {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

/// Edits between two words, counting swapping two neighbouring letters as one.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut table = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in table.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in table[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (table[i - 1][j] + 1).min(table[i][j - 1] + 1).min(table[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(table[i - 2][j - 2] + 1);
            }
            table[i][j] = best;
        }
    }
    table[a.len()][b.len()]
}

/// None when the words don't match, Some(None) for an exact match, Some(Some(word)) with a typo.
fn compare(typed: &[String], expected: &[String], typos: bool) -> Option<Option<String>> {
    if typed == expected {
        return Some(None);
    }
    if !typos || typed.len() != expected.len() {
        return None;
    }
    let mut first_typo = None;
    for (written, wanted) in typed.iter().zip(expected) {
        if written == wanted {
            continue;
        }
        if distance(written, wanted) > allowance(wanted) {
            return None;
        }
        first_typo.get_or_insert_with(|| written.clone());
    }
    Some(first_typo)
}

/// Marks a typed answer (R80, R81).
pub fn mark(check: &Check, typed: &str) -> Marking {
    match check {
        Check::Accepted { answers, typos } => {
            let typed = answer_words(typed);
            let mut with_typo = None;
            for answer in answers {
                match compare(&typed, &answer_words(answer), *typos) {
                    Some(None) => return Marking { right: true, detail: Detail::Matched { answer: answer.trim().into(), typo: None } },
                    Some(Some(word)) if with_typo.is_none() => with_typo = Some((answer.trim().to_string(), word)),
                    _ => {}
                }
            }
            match with_typo {
                Some((answer, word)) => Marking { right: true, detail: Detail::Matched { answer, typo: Some(word) } },
                None => Marking { right: false, detail: Detail::NoMatch },
            }
        }
        Check::Keywords { keywords, needed } => {
            let typed = words(typed);
            let mut used = vec![false; typed.len()];
            // Longest forms first, so "SYN-ACK" takes its words before "SYN" and "ACK" look.
            let mut order: Vec<(usize, Vec<Vec<String>>)> = keywords
                .iter()
                .enumerate()
                .map(|(index, keyword)| {
                    let mut forms: Vec<Vec<String>> = std::iter::once(&keyword.word).chain(&keyword.also).map(|form| words(form)).filter(|form| !form.is_empty()).collect();
                    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
                    (index, forms)
                })
                .collect();
            order.sort_by_key(|(_, forms)| std::cmp::Reverse(forms.first().map_or(0, Vec::len)));
            let mut found = vec![false; keywords.len()];
            for (index, forms) in order {
                'forms: for form in forms {
                    for start in 0..typed.len().saturating_sub(form.len() - 1) {
                        let span = start..start + form.len();
                        if typed[span.clone()] == form[..] && used[span.clone()].iter().all(|taken| !taken) {
                            used[span].iter_mut().for_each(|taken| *taken = true);
                            found[index] = true;
                            break 'forms;
                        }
                    }
                }
            }
            let (hit, missed): (Vec<_>, Vec<_>) = keywords.iter().zip(&found).partition(|(_, found)| **found);
            let needed = needed.unwrap_or(keywords.len()).clamp(1, keywords.len().max(1));
            Marking {
                right: hit.len() >= needed,
                detail: Detail::Keywords {
                    hit: hit.into_iter().map(|(keyword, _)| keyword.word.trim().to_string()).collect(),
                    missed: missed.into_iter().map(|(keyword, _)| keyword.word.trim().to_string()).collect(),
                    needed,
                },
            }
        }
    }
}

impl ShortAnswer {
    /// Drops blank answers, keywords and spellings, and trims what's left.
    pub fn tidied(mut self) -> Self {
        let keep = |list: Vec<String>| list.into_iter().map(|item| item.trim().to_string()).filter(|item| !words(item).is_empty()).collect::<Vec<_>>();
        self.check = match self.check {
            Check::Accepted { answers, typos } => Check::Accepted { answers: keep(answers), typos },
            Check::Keywords { keywords, needed } => Check::Keywords {
                keywords: keywords
                    .into_iter()
                    .filter(|keyword| !words(&keyword.word).is_empty())
                    .map(|keyword| Keyword { word: keyword.word.trim().into(), also: keep(keyword.also) })
                    .collect(),
                needed,
            },
        };
        self.model_answer = self.model_answer.trim().into();
        self
    }

    /// What's shown after answering: the model answer, or the first accepted answer.
    pub fn shown_answer(&self) -> String {
        if !self.model_answer.trim().is_empty() {
            return self.model_answer.trim().into();
        }
        match &self.check {
            Check::Accepted { answers, .. } => answers.first().cloned().unwrap_or_default(),
            Check::Keywords { keywords, .. } => keywords.iter().map(|keyword| keyword.word.clone()).collect::<Vec<_>>().join(", "),
        }
    }

    /// Accepted answers besides the one shown, for "Also accepted".
    pub fn also_accepted(&self) -> Vec<String> {
        match &self.check {
            Check::Accepted { answers, .. } => answers.iter().skip(usize::from(self.model_answer.trim().is_empty())).cloned().collect(),
            Check::Keywords { .. } => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted(answers: &[&str], typos: bool) -> Check {
        Check::Accepted { answers: answers.iter().map(|a| a.to_string()).collect(), typos }
    }

    fn handshake(needed: Option<usize>) -> Check {
        Check::Keywords {
            keywords: vec![
                Keyword { word: "SYN".into(), also: vec![] },
                Keyword { word: "SYN-ACK".into(), also: vec!["synack".into()] },
                Keyword { word: "ACK".into(), also: vec![] },
            ],
            needed,
        }
    }

    fn verdict(check: &Check, typed: &str) -> (bool, Detail) {
        let marking = mark(check, typed);
        (marking.right, marking.detail)
    }

    // short-answer.feature: Accepted answers are forgiving
    #[test]
    fn accepted_answers_ignore_case_spacing_punctuation_and_articles() {
        let check = accepted(&["Transport layer", "Layer 4"], true);
        let matched = |answer: &str, typo: Option<&str>| (true, Detail::Matched { answer: answer.into(), typo: typo.map(Into::into) });
        assert_eq!(verdict(&check, "transport layer"), matched("Transport layer", None));
        assert_eq!(verdict(&check, "  The Transport   Layer. "), matched("Transport layer", None));
        assert_eq!(verdict(&check, "an layer-4"), matched("Layer 4", None));
        assert_eq!(verdict(&check, "the transprot layer"), matched("Transport layer", Some("transprot")));
        assert_eq!(verdict(&check, "the transpot layr"), matched("Transport layer", Some("transpot")), "one typo in each word");
        assert_eq!(verdict(&check, "transfer layer"), (false, Detail::NoMatch));
        assert_eq!(verdict(&check, "network layer"), (false, Detail::NoMatch));
        assert_eq!(verdict(&check, "transport"), (false, Detail::NoMatch), "a missing word isn't a typo");
        assert_eq!(verdict(&check, ""), (false, Detail::NoMatch));
    }

    #[test]
    fn exact_matches_win_over_typos() {
        let check = accepted(&["Routers", "Router"], true);
        assert_eq!(verdict(&check, "router"), (true, Detail::Matched { answer: "Router".into(), typo: None }));
    }

    #[test]
    fn typo_allowance_depends_on_word_length_and_numbers() {
        assert_eq!(allowance("tcp"), 0);
        assert_eq!(allowance("port"), 1);
        assert_eq!(allowance("routing"), 1);
        assert_eq!(allowance("protocol"), 2);
        assert_eq!(allowance("ipv6abcd"), 0, "anything with a digit is exact");
        assert_eq!(distance("transport", "transprot"), 1, "a swap counts once");
        assert_eq!(distance("layer", "layre"), 1);
        assert_eq!(distance("abc", "abc"), 0);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "ab"), 2);
    }

    // short-answer.feature: No typos in numbers or short words
    #[test]
    fn no_typos_in_numbers_or_short_words() {
        assert!(!mark(&accepted(&["443"], true), "433").right);
        assert!(!mark(&accepted(&["TCP"], true), "TPC").right);
        assert!(!mark(&accepted(&["Transport layer"], false), "transprot layer").right, "typos switched off");
        assert!(mark(&accepted(&["Transport layer"], false), "Transport layer!").right);
    }

    // short-answer.feature: Keywords in any order, each word counted once
    #[test]
    fn keywords_in_any_order_each_word_counted_once() {
        let keywords = |hit: &[&str], missed: &[&str], needed| Detail::Keywords { hit: hit.iter().map(|s| s.to_string()).collect(), missed: missed.iter().map(|s| s.to_string()).collect(), needed };
        let all = handshake(None);
        assert_eq!(verdict(&all, "SYN, SYN-ACK, ACK"), (true, keywords(&["SYN", "SYN-ACK", "ACK"], &[], 3)));
        assert_eq!(verdict(&all, "ack then synack then syn"), (true, keywords(&["SYN", "SYN-ACK", "ACK"], &[], 3)));
        assert_eq!(verdict(&all, "SYN, then the server sends SYN-ACK"), (false, keywords(&["SYN", "SYN-ACK"], &["ACK"], 3)));
        assert_eq!(verdict(&all, "syn ack"), (false, keywords(&["SYN-ACK"], &["SYN", "ACK"], 3)), "SYN-ACK takes both words");
        assert_eq!(verdict(&all, "asynchronous acknowledgement"), (false, keywords(&[], &["SYN", "SYN-ACK", "ACK"], 3)), "whole words only");
        let two = handshake(Some(2));
        assert_eq!(verdict(&two, "SYN, then the server sends SYN-ACK"), (true, keywords(&["SYN", "SYN-ACK"], &["ACK"], 2)));
        assert_eq!(verdict(&handshake(Some(9)), "syn"), (false, keywords(&["SYN"], &["SYN-ACK", "ACK"], 3)), "needed is capped at the count");
    }

    #[test]
    fn tidying_drops_blanks_and_the_shown_answer_falls_back() {
        let tidy = ShortAnswer { check: accepted(&["  ", " Transport layer ", "!!", "Layer 4"], true), model_answer: "  ".into() }.tidied();
        assert_eq!(tidy.check, accepted(&["Transport layer", "Layer 4"], true));
        assert_eq!(tidy.shown_answer(), "Transport layer");
        assert_eq!(tidy.also_accepted(), vec!["Layer 4"]);
        let with_model = ShortAnswer { model_answer: "The **transport** layer".into(), ..tidy.clone() };
        assert_eq!(with_model.shown_answer(), "The **transport** layer");
        assert_eq!(with_model.also_accepted(), vec!["Transport layer", "Layer 4"]);
        let keywords = ShortAnswer {
            check: Check::Keywords { keywords: vec![Keyword { word: " SYN ".into(), also: vec!["".into(), " syn ".into()] }, Keyword { word: "--".into(), also: vec![] }], needed: None },
            model_answer: String::new(),
        }
        .tidied();
        assert_eq!(keywords.check, Check::Keywords { keywords: vec![Keyword { word: "SYN".into(), also: vec!["syn".into()] }], needed: None });
        assert_eq!(keywords.shown_answer(), "SYN");
    }
}

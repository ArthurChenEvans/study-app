//! Writing questions: R4 (3 to 5 options), R5 (single or multiple answer),
//! R6 (explanation), R9 (at least one correct), R10 (one topic), R14 (optional citations).

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::short::{Check, Keyword, ShortAnswer};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TopicId(pub i64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuestionId(pub i64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub i64);

pub const MIN_OPTIONS: usize = 3;
pub const MAX_OPTIONS: usize = 5;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerKind {
    Single,
    Multiple,
    /// Typed and checked against accepted answers or keywords (R78).
    Short,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AnswerOption {
    /// Markdown, may contain images, code and LaTeX (R7).
    pub text: String,
    pub correct: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub source: SourceId,
    /// Typed by hand: "p. 42", "slide 12", "line 30" (R14).
    pub location: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuestionDraft {
    pub topic: TopicId,
    pub text: String,
    pub kind: AnswerKind,
    pub options: Vec<AnswerOption>,
    pub explanation: String,
    pub tags: Vec<String>,
    pub citations: Vec<Citation>,
    /// Only for `AnswerKind::Short`.
    #[serde(default)]
    pub short: Option<ShortAnswer>,
}

/// A question that has passed every rule. Only constructible through `Question::new`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Question {
    topic: TopicId,
    text: String,
    kind: AnswerKind,
    options: Vec<AnswerOption>,
    explanation: String,
    tags: Vec<String>,
    citations: Vec<Citation>,
    #[serde(default)]
    short: Option<ShortAnswer>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum QuestionError {
    #[error("a question needs at least {MIN_OPTIONS} options, this one has {0}")]
    TooFewOptions(usize),
    #[error("a question can have at most {MAX_OPTIONS} options, this one has {0}")]
    TooManyOptions(usize),
    #[error("mark at least one correct option before saving")]
    NoCorrectOption,
    #[error("a single-answer question can only have one correct option, this one has {0}")]
    SeveralCorrectInSingle(usize),
    #[error("add at least one accepted answer, so the app can check it")]
    NoAcceptedAnswer,
    #[error("add at least one keyword, so the app can check it")]
    NoKeyword,
}

impl Question {
    pub fn new(draft: QuestionDraft) -> Result<Self, QuestionError> {
        if draft.kind == AnswerKind::Short {
            // R79: something to check against. Options don't apply.
            let short = draft.short.ok_or(QuestionError::NoAcceptedAnswer)?.tidied();
            match &short.check {
                Check::Accepted { answers, .. } if answers.is_empty() => return Err(QuestionError::NoAcceptedAnswer),
                Check::Keywords { keywords, .. } if keywords.is_empty() => return Err(QuestionError::NoKeyword),
                _ => {}
            }
            return Ok(Self {
                topic: draft.topic,
                text: draft.text,
                kind: draft.kind,
                options: vec![],
                explanation: draft.explanation,
                tags: draft.tags,
                citations: draft.citations,
                short: Some(short),
            });
        }
        let count = draft.options.len();
        if count < MIN_OPTIONS {
            return Err(QuestionError::TooFewOptions(count));
        }
        if count > MAX_OPTIONS {
            return Err(QuestionError::TooManyOptions(count));
        }
        let correct = draft.options.iter().filter(|option| option.correct).count();
        if correct == 0 {
            return Err(QuestionError::NoCorrectOption);
        }
        if draft.kind == AnswerKind::Single && correct > 1 {
            return Err(QuestionError::SeveralCorrectInSingle(correct));
        }
        Ok(Self {
            topic: draft.topic,
            text: draft.text,
            kind: draft.kind,
            options: draft.options,
            explanation: draft.explanation,
            tags: draft.tags,
            citations: draft.citations,
            short: None,
        })
    }

    pub fn topic(&self) -> TopicId {
        self.topic
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn kind(&self) -> AnswerKind {
        self.kind
    }
    pub fn options(&self) -> &[AnswerOption] {
        &self.options
    }
    pub fn explanation(&self) -> &str {
        &self.explanation
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    pub fn citations(&self) -> &[Citation] {
        &self.citations
    }
    /// How a short answer question is checked. None for multiple choice.
    pub fn short(&self) -> Option<&ShortAnswer> {
        self.short.as_ref()
    }

    /// Indices of the correct options, in option order.
    pub fn correct_indices(&self) -> Vec<usize> {
        self.options
            .iter()
            .enumerate()
            .filter(|(_, option)| option.correct)
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(kind: AnswerKind, options: &[(&str, bool)]) -> QuestionDraft {
        QuestionDraft {
            topic: TopicId(1),
            text: "What does the ALU do?".into(),
            kind,
            options: options
                .iter()
                .map(|(text, correct)| AnswerOption {
                    text: (*text).into(),
                    correct: *correct,
                })
                .collect(),
            explanation: "The ALU performs arithmetic and logic operations.".into(),
            tags: vec![],
            citations: vec![],
            short: None,
        }
    }

    fn short(check: Check) -> QuestionDraft {
        QuestionDraft { kind: AnswerKind::Short, options: vec![], short: Some(ShortAnswer { check, model_answer: String::new() }), ..draft(AnswerKind::Short, &[]) }
    }

    fn options_with_first_correct(count: usize) -> Vec<(&'static str, bool)> {
        ["A", "B", "C", "D", "E", "F"]
            .iter()
            .take(count)
            .enumerate()
            .map(|(index, text)| (*text, index == 0))
            .collect()
    }

    // questions.feature: Write a question with an explanation
    #[test]
    fn saves_a_valid_question_with_its_explanation() {
        let question = Question::new(draft(
            AnswerKind::Single,
            &[
                ("Arithmetic and logic", true),
                ("Stores data", false),
                ("Fetches instructions", false),
            ],
        ))
        .unwrap();
        assert_eq!(
            question.explanation(),
            "The ALU performs arithmetic and logic operations."
        );
        assert_eq!(question.topic(), TopicId(1));
    }

    // questions.feature: Option count limits (outline)
    #[test]
    fn option_count_limits() {
        let cases = [
            (2, Err(QuestionError::TooFewOptions(2))),
            (3, Ok(())),
            (4, Ok(())),
            (5, Ok(())),
            (6, Err(QuestionError::TooManyOptions(6))),
        ];
        for (count, expected) in cases {
            let result = Question::new(draft(
                AnswerKind::Single,
                &options_with_first_correct(count),
            ))
            .map(|_| ());
            assert_eq!(result, expected, "with {count} options");
        }
    }

    // questions.feature: No correct option
    #[test]
    fn refuses_a_question_with_no_correct_option() {
        let result = Question::new(draft(
            AnswerKind::Single,
            &[("A", false), ("B", false), ("C", false), ("D", false)],
        ));
        assert_eq!(result, Err(QuestionError::NoCorrectOption));
    }

    // questions.feature: Single-answer question
    #[test]
    fn single_answer_question_has_exactly_one_correct_option() {
        let question = Question::new(draft(
            AnswerKind::Single,
            &[("A", false), ("B", true), ("C", false), ("D", false)],
        ))
        .unwrap();
        assert_eq!(question.correct_indices(), vec![1]);
    }

    #[test]
    fn single_answer_question_cannot_be_saved_with_several_correct() {
        let result = Question::new(draft(
            AnswerKind::Single,
            &[("A", true), ("B", true), ("C", false)],
        ));
        assert_eq!(result, Err(QuestionError::SeveralCorrectInSingle(2)));
    }

    // questions.feature: Multiple-answer question
    #[test]
    fn multiple_answer_question_keeps_every_correct_option() {
        let question = Question::new(draft(
            AnswerKind::Multiple,
            &[
                ("A", true),
                ("B", false),
                ("C", true),
                ("D", true),
                ("E", false),
            ],
        ))
        .unwrap();
        assert_eq!(question.correct_indices(), vec![0, 2, 3]);
    }

    // sources.feature: Cite sources on a question / Question without a source
    #[test]
    fn citations_are_optional_and_can_be_several() {
        let mut with_two = draft(AnswerKind::Single, &options_with_first_correct(3));
        with_two.citations = vec![
            Citation {
                source: SourceId(1),
                location: "p. 42".into(),
            },
            Citation {
                source: SourceId(2),
                location: "slide 12".into(),
            },
        ];
        assert_eq!(Question::new(with_two).unwrap().citations().len(), 2);
        assert!(Question::new(draft(AnswerKind::Single, &options_with_first_correct(3))).is_ok());
    }

    // short-answer.feature: A short answer question needs something to check against
    #[test]
    fn a_short_answer_needs_an_accepted_answer_or_keyword() {
        let none = short(Check::Accepted { answers: vec![" ".into()], typos: true });
        assert_eq!(Question::new(none), Err(QuestionError::NoAcceptedAnswer));
        assert_eq!(QuestionError::NoAcceptedAnswer.to_string(), "add at least one accepted answer, so the app can check it");
        let no_keywords = short(Check::Keywords { keywords: vec![], needed: None });
        assert_eq!(Question::new(no_keywords), Err(QuestionError::NoKeyword));
        let missing = QuestionDraft { short: None, ..short(Check::Keywords { keywords: vec![], needed: None }) };
        assert_eq!(Question::new(missing), Err(QuestionError::NoAcceptedAnswer));

        let good = Question::new(short(Check::Accepted { answers: vec![" Transport layer ".into(), "".into()], typos: true })).unwrap();
        assert_eq!(good.kind(), AnswerKind::Short);
        assert!(good.options().is_empty() && good.correct_indices().is_empty());
        assert_eq!(good.short().unwrap().check, Check::Accepted { answers: vec!["Transport layer".into()], typos: true }, "tidied");
    }

    #[test]
    fn short_answers_have_no_options_and_mcqs_no_short_answer() {
        let with_options = QuestionDraft { options: vec![AnswerOption { text: "A".into(), correct: true }], ..short(Check::Accepted { answers: vec!["A".into()], typos: true }) };
        assert!(Question::new(with_options).unwrap().options().is_empty(), "options are dropped");
        let mut mcq = draft(AnswerKind::Single, &options_with_first_correct(3));
        mcq.short = Some(ShortAnswer { check: Check::Accepted { answers: vec!["A".into()], typos: true }, model_answer: String::new() });
        assert_eq!(Question::new(mcq).unwrap().short(), None);
    }

    #[test]
    fn old_saved_questions_read_without_a_short_answer() {
        let old = r#"{"topic":1,"text":"Q","kind":"Single","options":[{"text":"A","correct":true}],"explanation":"","tags":[],"citations":[]}"#;
        let question: Question = serde_json::from_str(old).unwrap();
        assert_eq!(question.short(), None);
    }

    #[test]
    fn errors_explain_how_to_fix_them() {
        assert_eq!(
            QuestionError::NoCorrectOption.to_string(),
            "mark at least one correct option before saving"
        );
        assert_eq!(
            QuestionError::TooFewOptions(2).to_string(),
            "a question needs at least 3 options, this one has 2"
        );
    }
}

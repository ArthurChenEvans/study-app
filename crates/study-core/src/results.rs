//! After a quiz: R24 (score, guessed, per-topic breakdown, review) and
//! R25 (wrong-answer reasons: preset or free text, optional but prompted).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use thiserror::Error;

use crate::attempt::{Attempt, AutoReason, Outcome};
use crate::question::TopicId;

/// Reasons the student can give (R25). Automatic reasons live in `AutoReason`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GivenReason {
    Blank,
    Misremembered,
    ConfusedWithSimilar,
    MisreadQuestion,
    CarelessSlip,
    SecondGuessed,
    NeverLearned,
    Other(String),
}

impl GivenReason {
    pub fn label(&self) -> String {
        match self {
            Self::Blank => "Blank".into(),
            Self::Misremembered => "Misremembered".into(),
            Self::ConfusedWithSimilar => "Confused with something similar".into(),
            Self::MisreadQuestion => "Misread the question".into(),
            Self::CarelessSlip => "Careless slip".into(),
            Self::SecondGuessed => "Second-guessed".into(),
            Self::NeverLearned => "Never learned it".into(),
            Self::Other(text) => text.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Tally {
    pub right_sure: usize,
    pub right_guessed: usize,
    pub wrong: usize,
    /// Wrong answers that were also flagged as guessed. Whether these count in
    /// the "N guessed" shown on screen is open question Q7, so they're kept apart.
    pub wrong_flagged: usize,
    pub unanswered: usize,
}

impl Tally {
    pub fn right(&self) -> usize {
        self.right_sure + self.right_guessed
    }
    pub fn total(&self) -> usize {
        self.right() + self.wrong + self.unanswered
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub overall: Tally,
    pub per_topic: BTreeMap<TopicId, Tally>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReasonError {
    #[error("there's no question {0} in this quiz")]
    NoSuchQuestion(usize),
    #[error("only wrong answers get a reason")]
    NotWrong,
    #[error("this reason was recorded automatically")]
    RecordedAutomatically,
    #[error("write a reason, or pick one from the list")]
    EmptyReason,
}

pub fn summarise(attempt: &Attempt) -> Summary {
    let mut overall = Tally::default();
    let mut per_topic: BTreeMap<TopicId, Tally> = BTreeMap::new();
    for asked in attempt.asked() {
        let topic = per_topic.entry(asked.snapshot.topic()).or_default();
        for tally in [&mut overall, topic] {
            match asked.outcome {
                Some(Outcome::Right) if asked.flagged_guessed => tally.right_guessed += 1,
                Some(Outcome::Right) => tally.right_sure += 1,
                Some(Outcome::Wrong(_)) => {
                    tally.wrong += 1;
                    if asked.flagged_guessed {
                        tally.wrong_flagged += 1;
                    }
                }
                None => tally.unanswered += 1,
            }
        }
    }
    Summary { overall, per_topic }
}

/// Wrong and guessed questions first, then the rest, each group in quiz order.
pub fn review_order(attempt: &Attempt) -> Vec<usize> {
    let needs_review = |index: &usize| {
        let asked = &attempt.asked()[*index];
        matches!(asked.outcome, Some(Outcome::Wrong(_))) || asked.flagged_guessed
    };
    let (first, rest): (Vec<usize>, Vec<usize>) =
        (0..attempt.asked().len()).partition(needs_review);
    first.into_iter().chain(rest).collect()
}

/// Reasons given so far, by question index.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Reasons(pub BTreeMap<usize, GivenReason>);

impl Reasons {
    pub fn give(
        &mut self,
        attempt: &Attempt,
        index: usize,
        reason: GivenReason,
    ) -> Result<(), ReasonError> {
        let asked = attempt
            .asked()
            .get(index)
            .ok_or(ReasonError::NoSuchQuestion(index))?;
        match asked.outcome {
            Some(Outcome::Wrong(None)) => {}
            Some(Outcome::Wrong(Some(_))) => return Err(ReasonError::RecordedAutomatically),
            _ => return Err(ReasonError::NotWrong),
        }
        let reason = match reason {
            GivenReason::Other(text) if text.trim().is_empty() => {
                return Err(ReasonError::EmptyReason);
            }
            GivenReason::Other(text) => GivenReason::Other(text.trim().to_string()),
            preset => preset,
        };
        self.0.insert(index, reason);
        Ok(())
    }

    /// Wrong answers still waiting for a reason. Reasons are optional, so this is a prompt, not a gate.
    pub fn to_prompt(&self, attempt: &Attempt) -> Vec<usize> {
        attempt
            .asked()
            .iter()
            .enumerate()
            .filter(|(index, asked)| {
                asked.outcome == Some(Outcome::Wrong(None)) && !self.0.contains_key(index)
            })
            .map(|(index, _)| index)
            .collect()
    }
}

/// The label recorded for each wrong answer: an automatic reason, or the one given.
pub fn reason_labels(attempt: &Attempt, reasons: &Reasons) -> Vec<String> {
    attempt
        .asked()
        .iter()
        .enumerate()
        .filter_map(|(index, asked)| match asked.outcome {
            Some(Outcome::Wrong(Some(AutoReason::Incomplete))) => {
                Some("Incomplete (auto)".to_string())
            }
            Some(Outcome::Wrong(Some(AutoReason::TimedOut))) => {
                Some("Timed out (auto)".to_string())
            }
            Some(Outcome::Wrong(None)) => reasons.0.get(&index).map(GivenReason::label),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::{Mode, QuizItem, QuizSettings};
    use crate::question::{AnswerKind, AnswerOption, Question, QuestionDraft, QuestionId};

    const CPU: TopicId = TopicId(1);
    const GPU: TopicId = TopicId(2);

    fn lookup(id: QuestionId) -> Question {
        let (topic, kind, correct): (TopicId, AnswerKind, &[usize]) = match id.0 {
            1..=3 => (CPU, AnswerKind::Single, &[0]),
            4 => (GPU, AnswerKind::Multiple, &[0, 2]),
            _ => (GPU, AnswerKind::Single, &[0]),
        };
        Question::new(QuestionDraft {
            topic,
            text: format!("question {}", id.0),
            kind,
            options: (0..4)
                .map(|index| AnswerOption {
                    text: index.to_string(),
                    correct: correct.contains(&index),
                })
                .collect(),
            explanation: "because".into(),
            tags: vec![],
            citations: vec![],
        })
        .unwrap()
    }

    /// CPU: q1 right sure, q2 right guessed, q3 wrong (flagged).
    /// GPU: q4 incomplete, q5 right sure.
    fn finished() -> Attempt {
        let items = (1..=5)
            .map(|id| QuizItem {
                question: QuestionId(id),
                option_order: vec![0, 1, 2, 3],
            })
            .collect();
        let mut attempt = Attempt::start(
            QuizSettings {
                mode: Mode::Study,
                ..QuizSettings::for_topics(vec![CPU, GPU])
            },
            items,
            lookup,
        );
        attempt.submit(&[0]).unwrap();
        attempt.toggle_flag().unwrap();
        attempt.submit(&[0]).unwrap();
        attempt.toggle_flag().unwrap();
        attempt.submit(&[1]).unwrap();
        attempt.submit(&[0]).unwrap();
        attempt.submit(&[0]).unwrap();
        attempt
    }

    // results.feature: Results summary
    #[test]
    fn summary_counts_right_guessed_and_wrong() {
        let summary = summarise(&finished());
        assert_eq!(
            summary.overall,
            Tally {
                right_sure: 2,
                right_guessed: 1,
                wrong: 2,
                wrong_flagged: 1,
                unanswered: 0
            }
        );
        assert_eq!((summary.overall.right(), summary.overall.total()), (3, 5));
    }

    #[test]
    fn summary_breaks_down_by_topic() {
        let summary = summarise(&finished());
        assert_eq!(
            summary.per_topic[&CPU],
            Tally {
                right_sure: 1,
                right_guessed: 1,
                wrong: 1,
                wrong_flagged: 1,
                unanswered: 0
            }
        );
        assert_eq!(
            summary.per_topic[&GPU],
            Tally {
                right_sure: 1,
                right_guessed: 0,
                wrong: 1,
                wrong_flagged: 0,
                unanswered: 0
            }
        );
    }

    #[test]
    fn unanswered_questions_are_counted_when_a_quiz_ends_early() {
        let items = (1..=3)
            .map(|id| QuizItem {
                question: QuestionId(id),
                option_order: vec![0, 1, 2, 3],
            })
            .collect();
        let mut attempt = Attempt::start(QuizSettings::for_topics(vec![CPU]), items, lookup);
        attempt.submit(&[0]).unwrap();
        attempt.quit();
        let summary = summarise(&attempt);
        assert_eq!(
            (
                summary.overall.right(),
                summary.overall.unanswered,
                summary.overall.total()
            ),
            (1, 2, 3)
        );
    }

    // results.feature: Review each question
    #[test]
    fn review_puts_wrong_and_guessed_first() {
        assert_eq!(review_order(&finished()), vec![1, 2, 3, 0, 4]);
    }

    // results.feature: Give a reason / Give a free text reason
    #[test]
    fn give_a_preset_or_free_text_reason() {
        let attempt = finished();
        let mut reasons = Reasons::default();
        reasons
            .give(&attempt, 2, GivenReason::ConfusedWithSimilar)
            .unwrap();
        assert_eq!(
            reason_labels(&attempt, &reasons),
            vec!["Confused with something similar", "Incomplete (auto)"]
        );
        reasons
            .give(
                &attempt,
                2,
                GivenReason::Other("mixed up L1 and L2 sizes".into()),
            )
            .unwrap();
        assert_eq!(reasons.0[&2].label(), "mixed up L1 and L2 sizes");
    }

    #[test]
    fn reasons_only_go_on_wrong_answers_without_an_automatic_one() {
        let attempt = finished();
        let mut reasons = Reasons::default();
        assert_eq!(
            reasons.give(&attempt, 0, GivenReason::Blank),
            Err(ReasonError::NotWrong)
        );
        assert_eq!(
            reasons.give(&attempt, 3, GivenReason::Blank),
            Err(ReasonError::RecordedAutomatically)
        );
        assert_eq!(
            reasons.give(&attempt, 9, GivenReason::Blank),
            Err(ReasonError::NoSuchQuestion(9))
        );
        assert_eq!(
            reasons.give(&attempt, 2, GivenReason::Other("  ".into())),
            Err(ReasonError::EmptyReason)
        );
    }

    // results.feature: Skip the reason
    #[test]
    fn reasons_are_prompted_but_optional() {
        let attempt = finished();
        let mut reasons = Reasons::default();
        assert_eq!(reasons.to_prompt(&attempt), vec![2]);
        reasons.give(&attempt, 2, GivenReason::Blank).unwrap();
        assert!(reasons.to_prompt(&attempt).is_empty());
    }
}

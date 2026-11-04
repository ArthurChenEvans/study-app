//! Applied practice (R87 to R96): struggles logged on projects, the issues they share,
//! one-off drills, and what a finished quiz does to each topic's quiz schedule.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::attempt::{Attempt, Status};
use crate::question::TopicId;
use crate::results::summarise;

/// What kind of struggle it was (R89). Not sure yet is decided at review.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StruggleKind {
    Recall,
    Apply,
    Unsure,
}

impl StruggleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recall => "Recall",
            Self::Apply => "Apply",
            Self::Unsure => "Unsure",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Recall" => Some(Self::Recall),
            "Apply" => Some(Self::Apply),
            "Unsure" => Some(Self::Unsure),
            _ => None,
        }
    }
}

/// How a drill went (R93).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillRating {
    Struggled,
    Shaky,
    Solid,
}

impl DrillRating {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Struggled => "Struggled",
            Self::Shaky => "Shaky",
            Self::Solid => "Solid",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Struggled" => Some(Self::Struggled),
            "Shaky" => Some(Self::Shaky),
            "Solid" => Some(Self::Solid),
            _ => None,
        }
    }

    /// When the drill is next due: never for Solid (ticked off), otherwise the next day.
    pub fn next_due(self, day: i64) -> Option<i64> {
        match self {
            Self::Solid => None,
            Self::Struggled | Self::Shaky => Some(day + 1),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum PracticeError {
    #[error("write what tripped you up")]
    NoWhat,
    #[error("name the issue, so you'd recognise it next time")]
    NoIssue,
    #[error("write the drill: a small task that practises this")]
    NoTask,
    #[error("give the project a name")]
    NoProjectName,
    #[error("start a session before logging a struggle")]
    NoSession,
    #[error("a session is already running on this project")]
    SessionRunning,
    #[error("that struggle has already been reviewed")]
    AlreadyReviewed,
    #[error("a drill can't be scheduled before today")]
    PastDay,
}

/// An issue's name tidied for showing: trimmed, with runs of spaces made single.
pub fn issue_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What two issue names are matched on (R91): the tidied name, ignoring case.
pub fn issue_key(name: &str) -> String {
    issue_name(name).to_lowercase()
}

/// A topic's share of a quiz: how many of its questions were right (R96).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopicScore {
    pub topic: TopicId,
    pub right: usize,
    pub total: usize,
}

/// Each topic's score in a quiz, for moving its quiz schedule. Retries and quizzes that
/// weren't finished don't move schedules, so they give nothing.
pub fn topic_scores(attempt: &Attempt) -> Vec<TopicScore> {
    if attempt.status() != Status::Finished || attempt.retry_of().is_some() {
        return vec![];
    }
    summarise(attempt)
        .per_topic
        .into_iter()
        .map(|(topic, tally)| TopicScore {
            topic,
            right: tally.right_sure + tally.right_guessed,
            total: tally.right_sure + tally.right_guessed + tally.wrong + tally.unanswered,
        })
        .filter(|score| score.total > 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::{QuizItem, QuizSettings};
    use crate::question::{AnswerKind, AnswerOption, Question, QuestionDraft, QuestionId};

    const CPU: TopicId = TopicId(1);
    const GPU: TopicId = TopicId(2);

    fn lookup(id: QuestionId) -> Question {
        Question::new(QuestionDraft {
            topic: if id.0 <= 3 { CPU } else { GPU },
            text: format!("question {}", id.0),
            kind: AnswerKind::Single,
            options: (0..3).map(|index| AnswerOption { text: index.to_string(), correct: index == 0 }).collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
            short: None,
        })
        .unwrap()
    }

    /// CPU: 2 of 3 right (one guessed). GPU: 0 of 1.
    fn quiz() -> Attempt {
        let items = (1..=4).map(|id| QuizItem { question: QuestionId(id), option_order: vec![0, 1, 2] }).collect();
        let mut attempt = Attempt::start(QuizSettings::for_topics(vec![CPU, GPU]), items, lookup);
        attempt.submit(&[0]).unwrap();
        attempt.toggle_flag().unwrap();
        attempt.submit(&[0]).unwrap();
        attempt.submit(&[1]).unwrap();
        attempt.submit(&[2]).unwrap();
        attempt
    }

    // calendar.feature: A topic's quiz schedule moves with its score
    #[test]
    fn each_topic_is_scored_on_its_own_questions() {
        assert_eq!(
            topic_scores(&quiz()),
            vec![TopicScore { topic: CPU, right: 2, total: 3 }, TopicScore { topic: GPU, right: 0, total: 1 }]
        );
    }

    // calendar.feature: Retries and quits don't move the schedule
    #[test]
    fn retries_and_quits_score_nothing() {
        let items = (1..=4).map(|id| QuizItem { question: QuestionId(id), option_order: vec![0, 1, 2] }).collect();
        let mut quit = Attempt::start(QuizSettings::for_topics(vec![CPU, GPU]), items, lookup);
        quit.submit(&[0]).unwrap();
        quit.quit();
        assert_eq!(topic_scores(&quit), vec![]);

        let mut retry = quiz().retry(1).unwrap();
        while retry.status() == Status::InProgress {
            retry.submit(&[0]).unwrap();
        }
        assert!(retry.retry_of().is_some());
        assert_eq!(topic_scores(&retry), vec![]);
    }

    // projects.feature: Rating a drill
    #[test]
    fn solid_ticks_a_drill_off_and_the_rest_come_back_tomorrow() {
        assert_eq!(DrillRating::Solid.next_due(20_760), None);
        assert_eq!(DrillRating::Shaky.next_due(20_760), Some(20_761));
        assert_eq!(DrillRating::Struggled.next_due(20_760), Some(20_761));
        for rating in [DrillRating::Struggled, DrillRating::Shaky, DrillRating::Solid] {
            assert_eq!(DrillRating::parse(rating.as_str()), Some(rating));
        }
        for kind in [StruggleKind::Recall, StruggleKind::Apply, StruggleKind::Unsure] {
            assert_eq!(StruggleKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(DrillRating::parse("Fine"), None);
    }

    // projects.feature: A known issue offers its drill again
    #[test]
    fn issues_match_ignoring_case_and_spacing() {
        assert_eq!(issue_name("  Off-by-one   in loops "), "Off-by-one in loops");
        assert_eq!(issue_key("Off-by-one in loops"), issue_key("off-by-one  IN loops"));
        assert_ne!(issue_key("Off-by-one in loops"), issue_key("Off-by-one in slices"));
    }
}

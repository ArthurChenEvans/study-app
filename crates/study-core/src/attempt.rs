//! Taking a quiz: R18 (study vs exam feedback), R19 (easy shows the answer count),
//! R20 (no going back in exam), R21 (flag as guessed), R22 (timeouts),
//! R23 (all or nothing), R25 (automatic reasons), R26 (retry), R28 (history keeps
//! the question as it was), R29 (quitting saves an incomplete attempt).

use std::collections::BTreeSet;

use thiserror::Error;

use crate::assembly::{Difficulty, Mode, QuizItem, QuizSettings};
use crate::question::{AnswerKind, Question, QuestionId};

/// Reasons the app records by itself (R25).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoReason {
    /// Multiple-answer: some correct options picked, none wrong, but not all.
    Incomplete,
    /// The per-question timer ran out.
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Right,
    Wrong(Option<AutoReason>),
}

/// One asked question, with a snapshot of the question as it was (R28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub item: QuizItem,
    pub snapshot: Question,
    pub picked: BTreeSet<usize>,
    pub flagged_guessed: bool,
    pub outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    InProgress,
    Finished,
    /// Quit halfway (R29).
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AttemptError {
    #[error("the quiz has already ended")]
    Ended,
    #[error("pick an option before submitting")]
    NothingPicked,
    #[error("option {0} doesn't exist on this question")]
    NoSuchOption(usize),
    #[error("you can't go back in exam mode")]
    NoGoingBackInExam,
    #[error("not decided yet in the specs: going back in study mode (Q6)")]
    GoingBackInStudyUndecided,
    #[error("every question was answered right and none were guessed, so there's nothing to retry")]
    NothingToRetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    settings: QuizSettings,
    asked: Vec<Asked>,
    current: usize,
    status: Status,
    retry_of: Option<usize>,
}

impl Attempt {
    /// `questions` must hold every question the items refer to.
    pub fn start(
        settings: QuizSettings,
        items: Vec<QuizItem>,
        questions: impl Fn(QuestionId) -> Question,
    ) -> Self {
        let asked = items
            .into_iter()
            .map(|item| Asked {
                snapshot: questions(item.question),
                item,
                picked: BTreeSet::new(),
                flagged_guessed: false,
                outcome: None,
            })
            .collect();
        Self {
            settings,
            asked,
            current: 0,
            status: Status::InProgress,
            retry_of: None,
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }
    pub fn settings(&self) -> &QuizSettings {
        &self.settings
    }
    pub fn asked(&self) -> &[Asked] {
        &self.asked
    }
    /// Some(id of the attempt this retries) for a retry (R26).
    pub fn retry_of(&self) -> Option<usize> {
        self.retry_of
    }

    pub fn current(&self) -> Option<&Asked> {
        if self.status != Status::InProgress {
            return None;
        }
        self.asked.get(self.current)
    }

    /// In easy mode a multiple-answer question shows how many to pick (R19).
    pub fn answers_to_pick(&self) -> Option<usize> {
        let asked = self.current()?;
        (self.settings.difficulty == Difficulty::Easy
            && asked.snapshot.kind() == AnswerKind::Multiple)
            .then(|| asked.snapshot.correct_indices().len())
    }

    pub fn toggle_flag(&mut self) -> Result<(), AttemptError> {
        let asked = self.current_mut()?;
        asked.flagged_guessed = !asked.flagged_guessed;
        Ok(())
    }

    /// Submit the picked options (original option indices). Study mode returns the
    /// outcome straight away, exam mode returns None until the end (R18).
    pub fn submit(&mut self, picked: &[usize]) -> Result<Option<Outcome>, AttemptError> {
        let mode = self.settings.mode;
        let asked = self.current_mut()?;
        if picked.is_empty() {
            return Err(AttemptError::NothingPicked);
        }
        if let Some(bad) = picked
            .iter()
            .find(|index| **index >= asked.snapshot.options().len())
        {
            return Err(AttemptError::NoSuchOption(*bad));
        }
        asked.picked = picked.iter().copied().collect();
        let outcome = score(&asked.snapshot, &asked.picked);
        asked.outcome = Some(outcome);
        self.advance();
        Ok((mode == Mode::Study).then_some(outcome))
    }

    /// Per-question timer ran out: counts as wrong and moves on (R22).
    pub fn question_timed_out(&mut self) -> Result<(), AttemptError> {
        self.current_mut()?.outcome = Some(Outcome::Wrong(Some(AutoReason::TimedOut)));
        self.advance();
        Ok(())
    }

    /// Total timer ran out: the quiz ends (R22).
    pub fn total_timed_out(&mut self) {
        if self.status == Status::InProgress {
            self.status = Status::Finished;
        }
    }

    /// Quit halfway: saved as incomplete (R29).
    pub fn quit(&mut self) {
        if self.status == Status::InProgress {
            self.status = Status::Incomplete;
        }
    }

    pub fn go_back(&mut self) -> Result<(), AttemptError> {
        match self.settings.mode {
            Mode::Exam => Err(AttemptError::NoGoingBackInExam),
            Mode::Study => Err(AttemptError::GoingBackInStudyUndecided),
        }
    }

    /// The questions to retry: wrong ones plus right ones flagged as guessed,
    /// with the same settings (R26). `own_id` is this attempt's id in history.
    pub fn retry(&self, own_id: usize) -> Result<Attempt, AttemptError> {
        let again: Vec<Asked> = self
            .asked
            .iter()
            .filter(|asked| {
                matches!(asked.outcome, Some(Outcome::Wrong(_)))
                    || (asked.outcome == Some(Outcome::Right) && asked.flagged_guessed)
            })
            .map(|asked| Asked {
                item: asked.item.clone(),
                snapshot: asked.snapshot.clone(),
                picked: BTreeSet::new(),
                flagged_guessed: false,
                outcome: None,
            })
            .collect();
        if again.is_empty() {
            return Err(AttemptError::NothingToRetry);
        }
        Ok(Attempt {
            settings: self.settings.clone(),
            asked: again,
            current: 0,
            status: Status::InProgress,
            retry_of: Some(own_id),
        })
    }

    fn current_mut(&mut self) -> Result<&mut Asked, AttemptError> {
        if self.status != Status::InProgress {
            return Err(AttemptError::Ended);
        }
        self.asked.get_mut(self.current).ok_or(AttemptError::Ended)
    }

    fn advance(&mut self) {
        self.current += 1;
        if self.current >= self.asked.len() {
            self.status = Status::Finished;
        }
    }
}

/// All or nothing (R23), with the automatic "incomplete" reason (R25).
pub fn score(question: &Question, picked: &BTreeSet<usize>) -> Outcome {
    let correct: BTreeSet<usize> = question.correct_indices().into_iter().collect();
    if *picked == correct {
        Outcome::Right
    } else if !picked.is_empty() && picked.is_subset(&correct) {
        Outcome::Wrong(Some(AutoReason::Incomplete))
    } else {
        Outcome::Wrong(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::Order;
    use crate::question::{AnswerOption, QuestionDraft, TopicId};
    use std::time::Duration;

    fn question(kind: AnswerKind, correct: &[usize], text: &str) -> Question {
        Question::new(QuestionDraft {
            topic: TopicId(1),
            text: text.into(),
            kind,
            options: (0..4)
                .map(|index| AnswerOption {
                    text: format!("option {index}"),
                    correct: correct.contains(&index),
                })
                .collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
        })
        .unwrap()
    }

    /// Question 1: single, correct = 0. Question 2: multiple, correct = A and C (0, 2). Question 3: single, correct = 1.
    fn lookup(id: QuestionId) -> Question {
        match id.0 {
            1 => question(AnswerKind::Single, &[0], "What does the ALU do?"),
            2 => question(AnswerKind::Multiple, &[0, 2], "Which describe a GPU warp?"),
            _ => question(
                AnswerKind::Single,
                &[1],
                "Which stage reads the register file?",
            ),
        }
    }

    fn items(ids: &[i64]) -> Vec<QuizItem> {
        ids.iter()
            .map(|id| QuizItem {
                question: QuestionId(*id),
                option_order: vec![0, 1, 2, 3],
            })
            .collect()
    }

    fn settings(mode: Mode) -> QuizSettings {
        QuizSettings {
            mode,
            ..QuizSettings::for_topics(vec![TopicId(1)])
        }
    }

    fn picked(indices: &[usize]) -> BTreeSet<usize> {
        indices.iter().copied().collect()
    }

    // taking-quiz.feature: All or nothing scoring (outline)
    #[test]
    fn multiple_answer_scoring_is_all_or_nothing() {
        let q = lookup(QuestionId(2));
        assert_eq!(score(&q, &picked(&[0, 2])), Outcome::Right);
        assert_eq!(
            score(&q, &picked(&[0])),
            Outcome::Wrong(Some(AutoReason::Incomplete))
        );
        assert_eq!(score(&q, &picked(&[0, 1, 2])), Outcome::Wrong(None));
        assert_eq!(score(&q, &picked(&[1])), Outcome::Wrong(None));
    }

    // taking-quiz.feature: Study mode feedback
    #[test]
    fn study_mode_tells_you_straight_away() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1, 3]), lookup);
        assert_eq!(attempt.submit(&[0]).unwrap(), Some(Outcome::Right));
        assert_eq!(attempt.current().unwrap().item.question, QuestionId(3));
    }

    // taking-quiz.feature: Exam mode feedback
    #[test]
    fn exam_mode_waits_until_the_end() {
        let mut attempt = Attempt::start(settings(Mode::Exam), items(&[1, 3]), lookup);
        assert_eq!(attempt.submit(&[0]).unwrap(), None);
        assert_eq!(attempt.submit(&[0]).unwrap(), None);
        assert_eq!(attempt.status(), Status::Finished);
        assert_eq!(attempt.asked()[1].outcome, Some(Outcome::Wrong(None)));
    }

    // taking-quiz.feature: No going back in exam mode
    #[test]
    fn no_going_back_in_exam_mode() {
        let mut attempt = Attempt::start(settings(Mode::Exam), items(&[1, 3]), lookup);
        attempt.submit(&[0]).unwrap();
        assert_eq!(attempt.go_back(), Err(AttemptError::NoGoingBackInExam));
    }

    #[test]
    fn going_back_in_study_mode_is_undecided() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1, 3]), lookup);
        attempt.submit(&[0]).unwrap();
        assert_eq!(
            attempt.go_back(),
            Err(AttemptError::GoingBackInStudyUndecided)
        );
    }

    // taking-quiz.feature: Easy and hard mode (outline)
    #[test]
    fn easy_mode_shows_how_many_to_pick_hard_mode_hides_it() {
        let easy = QuizSettings {
            difficulty: Difficulty::Easy,
            ..settings(Mode::Study)
        };
        assert_eq!(
            Attempt::start(easy.clone(), items(&[2]), lookup).answers_to_pick(),
            Some(2)
        );
        assert_eq!(
            Attempt::start(settings(Mode::Study), items(&[2]), lookup).answers_to_pick(),
            None
        );
        assert_eq!(
            Attempt::start(easy, items(&[1]), lookup).answers_to_pick(),
            None,
            "single-answer needs no count"
        );
    }

    // taking-quiz.feature: Flag a question as guessed (outline)
    #[test]
    fn flag_as_guessed_in_both_modes() {
        for mode in [Mode::Study, Mode::Exam] {
            let mut attempt = Attempt::start(settings(mode), items(&[1, 3]), lookup);
            attempt.toggle_flag().unwrap();
            attempt.submit(&[0]).unwrap();
            assert!(attempt.asked()[0].flagged_guessed, "{mode:?}");
            assert!(!attempt.asked()[1].flagged_guessed);
        }
    }

    #[test]
    fn flag_toggles_off_again() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1]), lookup);
        attempt.toggle_flag().unwrap();
        attempt.toggle_flag().unwrap();
        attempt.submit(&[0]).unwrap();
        assert!(!attempt.asked()[0].flagged_guessed);
    }

    #[test]
    fn submitting_needs_a_valid_pick() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1]), lookup);
        assert_eq!(attempt.submit(&[]), Err(AttemptError::NothingPicked));
        assert_eq!(attempt.submit(&[7]), Err(AttemptError::NoSuchOption(7)));
    }

    // taking-quiz.feature: Per-question timeout
    #[test]
    fn per_question_timeout_counts_as_wrong_and_moves_on() {
        let timed = QuizSettings {
            per_question_timer: Some(Duration::from_secs(30)),
            ..settings(Mode::Study)
        };
        let mut attempt = Attempt::start(timed, items(&[1, 3]), lookup);
        attempt.question_timed_out().unwrap();
        assert_eq!(
            attempt.asked()[0].outcome,
            Some(Outcome::Wrong(Some(AutoReason::TimedOut)))
        );
        assert_eq!(attempt.current().unwrap().item.question, QuestionId(3));
    }

    // taking-quiz.feature: Total timeout
    #[test]
    fn total_timeout_ends_the_quiz() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1, 3]), lookup);
        attempt.submit(&[0]).unwrap();
        attempt.total_timed_out();
        assert_eq!(attempt.status(), Status::Finished);
        assert!(attempt.current().is_none());
        assert_eq!(attempt.submit(&[0]), Err(AttemptError::Ended));
    }

    // taking-quiz.feature: Quit halfway
    #[test]
    fn quitting_saves_an_incomplete_attempt() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1, 3]), lookup);
        attempt.submit(&[0]).unwrap();
        attempt.quit();
        assert_eq!(attempt.status(), Status::Incomplete);
        assert_eq!(attempt.asked()[0].outcome, Some(Outcome::Right));
    }

    // history.feature: History shows the question as it was
    #[test]
    fn attempt_keeps_the_question_as_it_was() {
        let attempt = Attempt::start(settings(Mode::Study), items(&[1]), lookup);
        assert_eq!(attempt.asked()[0].snapshot.text(), "What does the ALU do?");
    }

    // retry.feature: Retry wrong and guessed / Retry uses same settings
    #[test]
    fn retry_takes_wrong_and_guessed_with_the_same_settings() {
        let exam = QuizSettings {
            option_order: Order::Shuffle,
            per_question_timer: Some(Duration::from_secs(30)),
            ..settings(Mode::Exam)
        };
        let mut attempt = Attempt::start(exam.clone(), items(&[1, 2, 3]), lookup);
        attempt.toggle_flag().unwrap();
        attempt.submit(&[0]).unwrap(); // right, guessed
        attempt.submit(&[0, 2]).unwrap(); // right, sure
        attempt.submit(&[0]).unwrap(); // wrong
        let retry = attempt.retry(4).unwrap();
        let ids: Vec<i64> = retry
            .asked()
            .iter()
            .map(|asked| asked.item.question.0)
            .collect();
        assert_eq!(ids, vec![1, 3]);
        assert_eq!(retry.settings(), &exam);
        assert_eq!(retry.retry_of(), Some(4));
    }

    // retry.feature: Nothing to retry
    #[test]
    fn nothing_to_retry_when_all_right_and_sure() {
        let mut attempt = Attempt::start(settings(Mode::Study), items(&[1]), lookup);
        attempt.submit(&[0]).unwrap();
        assert_eq!(attempt.retry(1).err(), Some(AttemptError::NothingToRetry));
    }
}

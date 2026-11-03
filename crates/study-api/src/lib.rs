//! Everything the Study App's Tauri commands do, as plain functions over `App`.
//! The Tauri crate only forwards to these, so all behaviour is tested here.
//!
//! Security rule: a `QuizView` never says which options are correct. Correct answers
//! only leave the backend in study-mode feedback after answering, and in results.

pub mod calendar;
pub mod cards;
pub mod convert;
pub mod practice;
pub mod recall;
pub mod seed;

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use study_core::assembly::QuestionTypes;
use study_core::assembly::{
    Amount, Difficulty, Mode, Order, Pick, PoolEntry, QuizSettings, assemble,
};
use study_core::short::{Check, Marking, ShortAnswer, mark};
use study_core::attempt::{Attempt, AutoReason, Outcome, Status};
use study_core::bibliography::SourceFields;
use study_core::image::{ImageId, check_image};
use study_core::progress::{Overview, overview};
use study_core::question::{
    AnswerKind, AnswerOption, Citation, Question, QuestionDraft, QuestionId, SourceId, TopicId,
};
use study_core::results::{GivenReason, Reasons, Summary, review_order, summarise};
use study_store::search::{Hit, SearchQuery};
use study_store::{AttemptId, HistoryRow, ModuleId, ModuleRow, SourceRow, Store, TopicRow};

/// What the frontend gets when something goes wrong: a message it can show as is.
#[derive(Debug, Error, Serialize)]
#[error("{message}")]
pub struct ApiError {
    pub message: String,
}

impl ApiError {
    pub fn new(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

macro_rules! from_error {
    ($($error:ty),*) => { $(impl From<$error> for ApiError { fn from(error: $error) -> Self { Self::new(error) } })* };
}
from_error!(
    study_store::StoreError,
    study_core::question::QuestionError,
    study_core::assembly::AssemblyError,
    study_core::attempt::AttemptError,
    study_core::results::ReasonError,
    study_core::image::ImageError,
    study_core::recall::RecallError,
    study_core::cards::CardError,
    study_core::cards::SchedulerError
);

pub type ApiResult<T> = Result<T, ApiError>;

// ---------- Inputs ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionInput {
    pub text: String,
    pub correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationInput {
    pub source: SourceId,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInput {
    pub topic: TopicId,
    pub text: String,
    pub multiple: bool,
    pub options: Vec<OptionInput>,
    pub explanation: String,
    pub tags: Vec<String>,
    pub citations: Vec<CitationInput>,
    /// Set for a short answer question (R78). Options are then ignored.
    #[serde(default)]
    pub short: Option<ShortAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSetupInput {
    pub topics: Vec<TopicId>,
    /// None means all questions.
    pub count: Option<usize>,
    pub even: bool,
    pub shuffle_questions: bool,
    pub shuffle_options: bool,
    pub exam: bool,
    pub total_minutes: Option<u64>,
    pub per_question_seconds: Option<u64>,
    pub easy: bool,
    /// R85: both, only MCQs or only short answers.
    #[serde(default)]
    pub types: QuestionTypes,
}

// ---------- Views ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownOption {
    /// The option's original index: send this back when answering.
    pub index: usize,
    pub text: String,
}

/// The question being asked. Deliberately has no field saying what's correct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizView {
    pub position: usize,
    pub total: usize,
    pub exam: bool,
    pub topic: TopicId,
    pub text: String,
    pub options: Vec<ShownOption>,
    pub multiple: bool,
    /// A short answer question: type the answer instead of picking (R83).
    pub short: bool,
    pub answers_to_pick: Option<usize>,
    pub flagged: bool,
    pub total_seconds: Option<u64>,
    pub per_question_seconds: Option<u64>,
    /// Results so far for the progress track: only filled in study mode.
    pub track: Vec<TrackCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackCell {
    RightSure,
    RightGuessed,
    Wrong,
    Answered,
    Current,
    Upcoming,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feedback {
    /// Which asked question this is, for overruling it (R84).
    pub index: usize,
    pub right: bool,
    pub auto_reason: Option<AutoReason>,
    pub correct: Vec<usize>,
    pub explanation: String,
    pub citations: Vec<Citation>,
    pub short: Option<ShortFeedback>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Next {
    Question(QuizView),
    Finished(AttemptId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerResult {
    /// Study mode only.
    pub feedback: Option<Feedback>,
    pub next: Next,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewStatus {
    Right,
    RightGuessed,
    Wrong,
    Unanswered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewItem {
    pub index: usize,
    pub topic: TopicId,
    pub text: String,
    pub options: Vec<String>,
    pub picked: Vec<usize>,
    pub correct: Vec<usize>,
    pub explanation: String,
    pub status: ReviewStatus,
    pub auto_reason: Option<AutoReason>,
    pub reason: Option<GivenReason>,
    /// Short answer only: what was written and how it was marked.
    pub short: Option<ShortFeedback>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultsView {
    pub attempt: AttemptId,
    pub status: Status,
    pub exam: bool,
    pub summary: Summary,
    pub review: Vec<ReviewItem>,
    pub to_prompt: Vec<usize>,
    pub retry_count: usize,
    /// Where this quiz moved each topic's quiz schedule (R97). Empty for retries and quits.
    pub moves: Vec<practice::MoveView>,
}

/// What a typed answer was and how it was marked (R83, R84).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortFeedback {
    pub typed: String,
    /// The app's own verdict. With `overruled` set, the outcome is the opposite.
    pub marking: Marking,
    /// The model answer, or the first accepted answer.
    pub shown_answer: String,
    pub also_accepted: Vec<String>,
    pub overruled: bool,
}

fn short_feedback(asked: &study_core::attempt::Asked) -> Option<ShortFeedback> {
    let short = asked.snapshot.short()?;
    Some(ShortFeedback {
        typed: asked.typed.clone()?,
        marking: asked.marking.clone()?,
        shown_answer: short.shown_answer(),
        also_accepted: short.also_accepted(),
        overruled: asked.overruled,
    })
}

/// R82: how an answer would be marked, for "Try an answer" in the editor. Nothing is saved.
pub fn try_short_answer(check: &Check, typed: &str) -> Marking {
    mark(check, typed)
}

// ---------- The app ----------

pub struct App {
    store: Store,
    rng: StdRng,
    active: Option<Attempt>,
    card_session: Option<cards::CardSession>,
}

impl App {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            rng: StdRng::from_os_rng(),
            active: None,
            card_session: None,
        }
    }

    /// Deterministic shuffles, for tests.
    pub fn with_seed(store: Store, seed: u64) -> Self {
        Self {
            store,
            rng: StdRng::seed_from_u64(seed),
            active: None,
            card_session: None,
        }
    }

    // ---------- Topics and modules ----------

    pub fn modules(&self) -> ApiResult<Vec<ModuleRow>> {
        Ok(self.store.modules()?)
    }

    pub fn create_module(&mut self, name: &str) -> ApiResult<ModuleId> {
        Ok(self
            .store
            .create_module(required(name, "give the module a name")?)?)
    }

    pub fn delete_module(&mut self, module: ModuleId) -> ApiResult<()> {
        Ok(self.store.delete_module(module)?)
    }

    pub fn topics(&self) -> ApiResult<Vec<TopicRow>> {
        Ok(self.store.topics()?)
    }

    pub fn create_topic(&mut self, name: &str, module: Option<ModuleId>) -> ApiResult<TopicId> {
        Ok(self
            .store
            .create_topic(required(name, "give the topic a name")?, module)?)
    }

    pub fn move_topic(&mut self, topic: TopicId, module: Option<ModuleId>) -> ApiResult<()> {
        Ok(self.store.move_topic(topic, module)?)
    }

    pub fn set_topic_tags(&mut self, topic: TopicId, tags: Vec<String>) -> ApiResult<()> {
        let tags: Vec<&str> = tags.iter().map(String::as_str).collect();
        Ok(self.store.set_topic_tags(topic, &tags)?)
    }

    pub fn delete_topic(&mut self, topic: TopicId) -> ApiResult<()> {
        Ok(self.store.delete_topic(topic)?)
    }

    // ---------- Questions ----------

    pub fn save_question(&mut self, input: QuestionInput) -> ApiResult<QuestionId> {
        Ok(self.store.save_question(&to_question(input)?)?)
    }

    pub fn update_question(&mut self, id: QuestionId, input: QuestionInput) -> ApiResult<()> {
        Ok(self.store.update_question(id, &to_question(input)?)?)
    }

    pub fn question(&self, id: QuestionId) -> ApiResult<QuestionInput> {
        let question = self.store.question(id)?;
        Ok(QuestionInput {
            topic: question.topic(),
            text: question.text().into(),
            multiple: question.kind() == AnswerKind::Multiple,
            options: question
                .options()
                .iter()
                .map(|option| OptionInput {
                    text: option.text.clone(),
                    correct: option.correct,
                })
                .collect(),
            explanation: question.explanation().into(),
            tags: question.tags().to_vec(),
            citations: question
                .citations()
                .iter()
                .map(|citation| CitationInput {
                    source: citation.source,
                    location: citation.location.clone(),
                })
                .collect(),
            short: question.short().cloned(),
        })
    }

    pub fn questions_in(&self, topic: TopicId) -> ApiResult<Vec<(QuestionId, String)>> {
        Ok(self
            .store
            .questions_in(&[topic])?
            .into_iter()
            .map(|(id, question)| (id, question.text().to_string()))
            .collect())
    }

    pub fn delete_question(&mut self, id: QuestionId) -> ApiResult<()> {
        Ok(self.store.delete_question(id)?)
    }

    // ---------- Bibliography ----------

    /// R7: a pasted or picked image. Returns the id the Markdown points at (image:<id>).
    pub fn add_image(&mut self, bytes: &[u8]) -> ApiResult<ImageId> {
        let format = check_image(bytes)?;
        Ok(self.store.save_image(format, bytes)?)
    }

    pub fn image(&self, id: ImageId) -> ApiResult<Vec<u8>> {
        Ok(self.store.image(id)?.1)
    }

    pub fn add_source(&mut self, fields: SourceFields) -> ApiResult<SourceId> {
        required(&fields.title, "give the source a title")?;
        Ok(self.store.add_source(&fields)?)
    }

    pub fn update_source(&mut self, id: SourceId, fields: SourceFields) -> ApiResult<()> {
        required(&fields.title, "give the source a title")?;
        Ok(self.store.update_source(id, &fields)?)
    }

    /// Returns how many questions cited the source.
    pub fn delete_source(&mut self, id: SourceId) -> ApiResult<usize> {
        Ok(self.store.delete_source(id)?)
    }

    pub fn sources(&self) -> ApiResult<Vec<SourceRow>> {
        Ok(self.store.sources()?)
    }

    // ---------- Quiz ----------

    pub fn start_quiz(&mut self, setup: QuizSetupInput) -> ApiResult<QuizView> {
        let settings = QuizSettings {
            topics: setup.topics,
            amount: setup.count.map_or(Amount::All, Amount::Count),
            pick: if setup.even { Pick::Even } else { Pick::Random },
            question_order: if setup.shuffle_questions {
                Order::Shuffle
            } else {
                Order::Keep
            },
            option_order: if setup.shuffle_options {
                Order::Shuffle
            } else {
                Order::Keep
            },
            mode: if setup.exam { Mode::Exam } else { Mode::Study },
            total_timer: setup
                .total_minutes
                .map(|minutes| std::time::Duration::from_secs(minutes * 60)),
            per_question_timer: setup
                .per_question_seconds
                .map(std::time::Duration::from_secs),
            difficulty: if setup.easy {
                Difficulty::Easy
            } else {
                Difficulty::Hard
            },
            types: setup.types,
        };
        let questions = self.store.questions_in(&settings.topics)?;
        let pool: Vec<PoolEntry> = questions
            .iter()
            .map(|(id, question)| PoolEntry { id: *id, question })
            .collect();
        let items = assemble(&pool, &settings, &mut self.rng)?;
        let lookup = |id: QuestionId| {
            questions
                .iter()
                .find(|(qid, _)| *qid == id)
                .map(|(_, q)| q.clone())
                .expect("assembled from this pool")
        };
        self.active = Some(Attempt::start(settings, items, lookup));
        self.current_question()
    }

    pub fn current_question(&self) -> ApiResult<QuizView> {
        let attempt = self
            .active
            .as_ref()
            .ok_or_else(|| ApiError::new("no quiz is running"))?;
        view_of(attempt).ok_or_else(|| ApiError::new("no quiz is running"))
    }

    pub fn toggle_flag(&mut self) -> ApiResult<QuizView> {
        self.active_mut()?.toggle_flag()?;
        self.current_question()
    }

    pub fn answer(&mut self, picked: Vec<usize>, today: i64) -> ApiResult<AnswerResult> {
        let attempt = self.active_mut()?;
        let index = current_index(attempt);
        let feedback_given = attempt.submit(&picked)?.is_some();
        let feedback = feedback_given.then(|| feedback_for(attempt, index));
        Ok(AnswerResult {
            feedback,
            next: self.next(today)?,
        })
    }

    /// R83: a typed answer to a short answer question.
    pub fn answer_text(&mut self, typed: String, today: i64) -> ApiResult<AnswerResult> {
        let attempt = self.active_mut()?;
        let index = current_index(attempt);
        let feedback_given = attempt.submit_text(&typed)?.is_some();
        let feedback = feedback_given.then(|| feedback_for(attempt, index));
        Ok(AnswerResult {
            feedback,
            next: self.next(today)?,
        })
    }

    /// R84: overrule an answer straight after it's marked. `attempt` is set once the quiz
    /// has finished and been saved (the last question), otherwise it's the running quiz.
    pub fn overrule_answer(&mut self, attempt: Option<AttemptId>, index: usize) -> ApiResult<Feedback> {
        match attempt {
            None => {
                let active = self.active_mut()?;
                active.overrule(index)?;
                Ok(feedback_for(active, index))
            }
            Some(id) => {
                let saved = self.overrule_saved(id, index)?;
                Ok(feedback_for(&saved, index))
            }
        }
    }

    /// R84: overrule a short answer on the results page.
    pub fn overrule_result(&mut self, attempt: AttemptId, index: usize) -> ApiResult<ResultsView> {
        self.overrule_saved(attempt, index)?;
        self.results(attempt)
    }

    fn overrule_saved(&mut self, id: AttemptId, index: usize) -> ApiResult<Attempt> {
        let (mut saved, mut reasons) = self.store.attempt(id)?;
        let outcome = saved.overrule(index)?;
        self.store.update_attempt(id, &saved)?;
        self.redo_quiz_schedules(id, &saved)?;
        // A reason only belongs on a wrong answer.
        if outcome == Outcome::Right && reasons.0.remove(&index).is_some() {
            self.store.save_reasons(id, &reasons)?;
        }
        Ok(saved)
    }

    pub fn question_timed_out(&mut self, today: i64) -> ApiResult<AnswerResult> {
        let attempt = self.active_mut()?;
        let index = current_index(attempt);
        attempt.question_timed_out()?;
        let feedback =
            (attempt.settings().mode == Mode::Study).then(|| feedback_for(attempt, index));
        Ok(AnswerResult {
            feedback,
            next: self.next(today)?,
        })
    }

    pub fn total_timed_out(&mut self, today: i64) -> ApiResult<AttemptId> {
        self.active_mut()?.total_timed_out();
        self.save_active(today)
    }

    pub fn quit_quiz(&mut self, today: i64) -> ApiResult<AttemptId> {
        self.active_mut()?.quit();
        self.save_active(today)
    }

    pub fn retry(&mut self, attempt: AttemptId) -> ApiResult<QuizView> {
        let (saved, _) = self.store.attempt(attempt)?;
        // R42: a retry of a retry uses its own questions but links to the original quiz.
        let original = saved.retry_of().unwrap_or(attempt.0 as usize);
        self.active = Some(saved.retry(original)?);
        self.current_question()
    }

    // ---------- After a quiz ----------

    pub fn results(&self, attempt: AttemptId) -> ApiResult<ResultsView> {
        let (saved, reasons) = self.store.attempt(attempt)?;
        let review = review_order(&saved)
            .into_iter()
            .map(|index| {
                let asked = &saved.asked()[index];
                let (status, auto_reason) = match asked.outcome {
                    Some(Outcome::Right) if asked.flagged_guessed => {
                        (ReviewStatus::RightGuessed, None)
                    }
                    Some(Outcome::Right) => (ReviewStatus::Right, None),
                    Some(Outcome::Wrong(auto)) => (ReviewStatus::Wrong, auto),
                    None => (ReviewStatus::Unanswered, None),
                };
                ReviewItem {
                    index,
                    topic: asked.snapshot.topic(),
                    text: asked.snapshot.text().into(),
                    options: asked
                        .snapshot
                        .options()
                        .iter()
                        .map(|option| option.text.clone())
                        .collect(),
                    picked: asked.picked.iter().copied().collect(),
                    correct: asked.snapshot.correct_indices(),
                    explanation: asked.snapshot.explanation().into(),
                    status,
                    auto_reason,
                    reason: reasons.0.get(&index).cloned(),
                    short: short_feedback(asked),
                }
            })
            .collect();
        let retry_count = saved.retry(0).map(|retry| retry.asked().len()).unwrap_or(0);
        Ok(ResultsView {
            attempt,
            status: saved.status(),
            exam: saved.settings().mode == Mode::Exam,
            summary: summarise(&saved),
            review,
            to_prompt: reasons.to_prompt(&saved),
            retry_count,
            moves: self.move_views(attempt)?,
        })
    }

    pub fn give_reason(
        &mut self,
        attempt: AttemptId,
        index: usize,
        reason: GivenReason,
    ) -> ApiResult<ResultsView> {
        let (saved, mut reasons) = self.store.attempt(attempt)?;
        reasons.give(&saved, index, reason)?;
        self.store.save_reasons(attempt, &reasons)?;
        self.results(attempt)
    }

    pub fn history(&self) -> ApiResult<Vec<HistoryRow>> {
        Ok(self.store.history()?)
    }

    pub fn overview(&self, today: i64) -> ApiResult<Overview> {
        let topics: Vec<TopicId> = self
            .store
            .topics()?
            .into_iter()
            .map(|topic| topic.id)
            .collect();
        Ok(overview(&self.store.attempt_records()?, today, &topics))
    }

    pub fn search(&self, query: SearchQuery) -> ApiResult<Vec<Hit>> {
        Ok(self.store.search(&query)?)
    }

    // ---------- Helpers ----------

    fn active_mut(&mut self) -> ApiResult<&mut Attempt> {
        self.active
            .as_mut()
            .ok_or_else(|| ApiError::new("no quiz is running"))
    }

    fn next(&mut self, today: i64) -> ApiResult<Next> {
        match self.current_question() {
            Ok(view) => Ok(Next::Question(view)),
            Err(_) => Ok(Next::Finished(self.save_active(today)?)),
        }
    }

    fn save_active(&mut self, today: i64) -> ApiResult<AttemptId> {
        let attempt = self
            .active
            .take()
            .ok_or_else(|| ApiError::new("no quiz is running"))?;
        let id = self.store.save_attempt(&attempt, today, &Reasons::default())?;
        self.move_quiz_schedules(id, &attempt, today)?;
        Ok(id)
    }
}

fn required<'a>(text: &'a str, message: &str) -> ApiResult<&'a str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        Err(ApiError::new(message))
    } else {
        Ok(trimmed)
    }
}

fn to_question(input: QuestionInput) -> ApiResult<Question> {
    Ok(Question::new(QuestionDraft {
        topic: input.topic,
        text: input.text,
        kind: if input.short.is_some() {
            AnswerKind::Short
        } else if input.multiple {
            AnswerKind::Multiple
        } else {
            AnswerKind::Single
        },
        options: input
            .options
            .into_iter()
            .map(|option| AnswerOption {
                text: option.text,
                correct: option.correct,
            })
            .collect(),
        explanation: input.explanation,
        tags: input
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect(),
        citations: input
            .citations
            .into_iter()
            .map(|citation| Citation {
                source: citation.source,
                location: citation.location,
            })
            .collect(),
        short: input.short,
    })?)
}

fn current_index(attempt: &Attempt) -> usize {
    attempt
        .asked()
        .iter()
        .position(|asked| asked.outcome.is_none())
        .unwrap_or(attempt.asked().len())
}

fn view_of(attempt: &Attempt) -> Option<QuizView> {
    let asked = attempt.current()?;
    let settings = attempt.settings();
    let exam = settings.mode == Mode::Exam;
    let index = current_index(attempt);
    let track = attempt
        .asked()
        .iter()
        .enumerate()
        .map(
            |(position, earlier)| match (position.cmp(&index), earlier.outcome) {
                (std::cmp::Ordering::Equal, _) => TrackCell::Current,
                (std::cmp::Ordering::Greater, _) | (_, None) => TrackCell::Upcoming,
                (_, Some(_)) if exam => TrackCell::Answered,
                (_, Some(Outcome::Right)) if earlier.flagged_guessed => TrackCell::RightGuessed,
                (_, Some(Outcome::Right)) => TrackCell::RightSure,
                (_, Some(Outcome::Wrong(_))) => TrackCell::Wrong,
            },
        )
        .collect();
    Some(QuizView {
        position: index + 1,
        total: attempt.asked().len(),
        exam,
        topic: asked.snapshot.topic(),
        text: asked.snapshot.text().into(),
        options: asked
            .item
            .option_order
            .iter()
            .map(|&original| ShownOption {
                index: original,
                text: asked.snapshot.options()[original].text.clone(),
            })
            .collect(),
        multiple: asked.snapshot.kind() == AnswerKind::Multiple,
        short: asked.snapshot.kind() == AnswerKind::Short,
        answers_to_pick: attempt.answers_to_pick(),
        flagged: asked.flagged_guessed,
        total_seconds: settings.total_timer.map(|timer| timer.as_secs()),
        per_question_seconds: settings.per_question_timer.map(|timer| timer.as_secs()),
        track,
    })
}

fn feedback_for(attempt: &Attempt, index: usize) -> Feedback {
    let asked = &attempt.asked()[index];
    let (right, auto_reason) = match asked.outcome {
        Some(Outcome::Right) => (true, None),
        Some(Outcome::Wrong(auto)) => (false, auto),
        None => (false, None),
    };
    Feedback {
        index,
        right,
        auto_reason,
        correct: asked.snapshot.correct_indices(),
        explanation: asked.snapshot.explanation().into(),
        citations: asked.snapshot.citations().to_vec(),
        short: short_feedback(asked),
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod short_tests;
#[cfg(test)]
mod practice_tests;

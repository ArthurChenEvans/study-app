//! Dates on the calendar (R130 to R137): adding them, what they cover, the preview of what
//! moves, the date's own panel on its day, and the tightened due days everything else uses.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use study_core::dates::{Due, Tightening, tightened};
use study_core::question::TopicId;
use study_store::ModuleId;
use study_store::cards::CardRow;
use study_store::dates::{Covers, DateId, DateInput, DateRecord};
use study_store::practice::{DrillId, DrillRecord, TopicQuiz};

use crate::{ApiResult, App};

/// A date as the calendar shows it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DateView {
    pub id: DateId,
    pub name: String,
    pub day: i64,
    pub kind: String,
    pub covers: Covers,
    /// "Networks module", "CPU and Caches", or empty when it covers nothing.
    pub covers_label: String,
    /// The topics it covers now.
    pub topics: Vec<TopicId>,
    pub tighten: bool,
    /// Days from today, negative once it's passed.
    pub days_left: i64,
}

/// A topic quiz a date moves (R136), or has moved (R135).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DateMove {
    pub topic: TopicId,
    pub name: String,
    pub step: String,
    /// Its next quiz without the date.
    pub from: i64,
    /// With it. The same as `from` when the date doesn't move it.
    pub to: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UnquizzedTopic {
    pub topic: TopicId,
    pub name: String,
    pub questions: usize,
}

/// What adding or saving a date would move (R136).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DatePreview {
    pub quizzes: Vec<DateMove>,
    /// Covered topics never quizzed, which have no schedule to tighten.
    pub unquizzed: Vec<UnquizzedTopic>,
    /// Cards due on or after the date that come in before it.
    pub cards: usize,
    pub drills: usize,
}

/// A date's own panel in the day view (R135).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DatePanel {
    pub date: DateView,
    /// Each covered topic's next quiz, soonest first, with where it was pulled in from.
    pub quizzes: Vec<DateMove>,
    pub unquizzed: Vec<UnquizzedTopic>,
    /// Covered cards (reviewed before, not archived) due before the date.
    pub cards_before: usize,
    /// Of those, the ones pulled in from the date or after it.
    pub cards_pulled: usize,
}

/// Saving a date: what it covers comes as a module or picked topics.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DateSave {
    pub name: String,
    pub day: i64,
    pub kind: String,
    pub module: Option<ModuleId>,
    #[serde(default)]
    pub topics: Vec<TopicId>,
    pub tighten: bool,
}

impl DateSave {
    fn input(&self) -> DateInput {
        let covers = match (self.module, self.topics.is_empty()) {
            (Some(module), _) => Covers::Module(module),
            (None, false) => Covers::Topics(self.topics.clone()),
            (None, true) => Covers::Nothing,
        };
        DateInput { name: self.name.clone(), day: self.day, kind: self.kind.clone(), covers, tighten: self.tighten }
    }
}

/// The dates and "last done" days, read once, to work out tightened due days (R132, R133).
pub(crate) struct Now {
    today: i64,
    by_topic: HashMap<TopicId, Vec<Tightening<DateId>>>,
    names: HashMap<DateId, String>,
    quiz_last: HashMap<TopicId, i64>,
    drill_last: HashMap<DrillId, i64>,
}

impl Now {
    fn due(&self, topic: Option<TopicId>, last: Option<i64>, normal: i64) -> Due<DateId> {
        let dates = topic.and_then(|topic| self.by_topic.get(&topic)).map(Vec::as_slice).unwrap_or_default();
        tightened(last, normal, self.today, dates.iter().copied())
    }

    pub fn quiz(&self, quiz: &TopicQuiz) -> Due<DateId> {
        self.due(Some(quiz.topic), self.quiz_last.get(&quiz.topic).copied(), quiz.due)
    }

    pub fn drill(&self, drill: &DrillRecord) -> Option<Due<DateId>> {
        drill.due.map(|due| self.due(drill.topic, self.drill_last.get(&drill.id).copied(), due))
    }

    /// New cards have no last review, so nothing to tighten.
    pub fn card(&self, card: &CardRow) -> Due<DateId> {
        self.due(Some(card.topic), card.schedule.last_review, card.schedule.due)
    }

    pub fn name(&self, date: DateId) -> Option<String> {
        self.names.get(&date).cloned()
    }

    pub fn tightened_for(&self, due: &Due<DateId>) -> Option<String> {
        due.tightened_by.and_then(|date| self.name(date))
    }
}

/// "CPU", "CPU and Caches", "CPU, Caches and Memory", "5 topics".
fn topic_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] if names.len() <= 3 => format!("{} and {last}", rest.join(", ")),
        _ => format!("{} topics", names.len()),
    }
}

impl App {
    fn topic_modules(&self) -> ApiResult<Vec<(TopicId, Option<ModuleId>)>> {
        Ok(self.store.topics()?.into_iter().map(|topic| (topic.id, topic.module)).collect())
    }

    /// Tightened due days as of `today`, from `dates` instead of the saved ones.
    fn now_from(&self, today: i64, dates: &[DateRecord]) -> ApiResult<Now> {
        let topics = self.topic_modules()?;
        let mut by_topic: HashMap<TopicId, Vec<Tightening<DateId>>> = HashMap::new();
        for date in dates.iter().filter(|date| date.tighten) {
            for topic in date.topics(&topics) {
                by_topic.entry(topic).or_default().push(Tightening { day: date.day, date: date.id });
            }
        }
        Ok(Now {
            today,
            by_topic,
            names: dates.iter().map(|date| (date.id, date.name.clone())).collect(),
            quiz_last: self.store.last_quiz_days()?,
            drill_last: self.store.last_drill_days()?,
        })
    }

    pub(crate) fn now(&self, today: i64) -> ApiResult<Now> {
        self.now_from(today, &self.store.dates()?)
    }

    /// Topic quiz schedules with tightened due days.
    pub(crate) fn quizzes_now(&self, now: &Now) -> ApiResult<Vec<TopicQuiz>> {
        Ok(self.store.topic_quizzes()?.into_iter().map(|quiz| TopicQuiz { due: now.quiz(&quiz).day, ..quiz }).collect())
    }

    /// Drills with tightened due days.
    pub(crate) fn drills_now(&self, now: &Now) -> ApiResult<Vec<DrillRecord>> {
        Ok(self.store.drills()?.into_iter().map(|drill| DrillRecord { due: now.drill(&drill).map(|due| due.day), ..drill }).collect())
    }

    /// Cards with tightened due days.
    pub(crate) fn cards_now(&self, cards: Vec<CardRow>, now: &Now) -> Vec<CardRow> {
        cards
            .into_iter()
            .map(|mut card| {
                card.schedule.due = now.card(&card).day;
                card
            })
            .collect()
    }

    fn date_view(&self, date: &DateRecord, topics: &[(TopicId, Option<ModuleId>)], today: i64) -> ApiResult<DateView> {
        let covered = date.topics(topics);
        let covers_label = match &date.covers {
            Covers::Nothing => String::new(),
            Covers::Module(module) => self.store.modules()?.into_iter().find(|m| m.id == *module).map(|m| format!("{} module", m.name)).unwrap_or_default(),
            Covers::Topics(picked) => {
                let all = self.store.topics()?;
                let mut names: Vec<String> = picked.iter().filter_map(|id| all.iter().find(|t| t.id == *id).map(|t| t.name.clone())).collect();
                // By name, so it reads the same on every computer.
                names.sort_by_key(|name| name.to_lowercase());
                topic_list(&names)
            }
        };
        Ok(DateView { id: date.id, name: date.name.clone(), day: date.day, kind: date.kind.clone(), covers: date.covers.clone(), covers_label, topics: covered, tighten: date.tighten, days_left: date.day - today })
    }

    /// Every date, soonest first (R130, R134).
    pub fn dates(&self, today: i64) -> ApiResult<Vec<DateView>> {
        let topics = self.topic_modules()?;
        self.store.dates()?.iter().map(|date| self.date_view(date, &topics, today)).collect()
    }

    pub fn add_date(&mut self, date: DateSave) -> ApiResult<DateId> {
        Ok(self.store.add_date(&date.input())?)
    }

    pub fn update_date(&mut self, id: DateId, date: DateSave) -> ApiResult<()> {
        Ok(self.store.update_date(id, &date.input())?)
    }

    pub fn delete_date(&mut self, id: DateId) -> ApiResult<()> {
        Ok(self.store.delete_date(id)?)
    }

    /// What saving `date` would move, compared with how things are without it (R136).
    /// `id` is the date being edited, left out of the "before".
    pub fn preview_date(&self, id: Option<DateId>, date: DateSave, today: i64) -> ApiResult<DatePreview> {
        let saved = self.store.dates()?;
        let without: Vec<DateRecord> = saved.iter().filter(|d| Some(d.id) != id).cloned().collect();
        let input = date.input();
        let draft = DateRecord { id: id.unwrap_or(DateId(0)), name: input.name.clone(), day: input.day, kind: input.kind.clone(), covers: input.covers.clone(), tighten: input.tighten && input.covers != Covers::Nothing };
        let mut with = without.clone();
        with.push(draft.clone());
        let (before, after) = (self.now_from(today, &without)?, self.now_from(today, &with)?);
        let topic_rows = self.store.topics()?;
        let covered = draft.topics(&self.topic_modules()?);
        let schedule = self.store.quiz_schedule()?;
        let quizzes_here = self.store.topic_quizzes()?;

        let mut quizzes = vec![];
        let mut unquizzed = vec![];
        for topic in topic_rows.iter().filter(|t| covered.contains(&t.id)) {
            match quizzes_here.iter().find(|q| q.topic == topic.id) {
                Some(quiz) => {
                    let (from, to) = (before.quiz(quiz).day, after.quiz(quiz).day);
                    if to < from {
                        quizzes.push(DateMove { topic: topic.id, name: topic.name.clone(), step: schedule.label(schedule.clamp(quiz.position)), from, to });
                    }
                }
                None => unquizzed.push(UnquizzedTopic { topic: topic.id, name: topic.name.clone(), questions: topic.question_count }),
            }
        }
        quizzes.sort_by_key(|m| (m.to, m.topic));
        let cards = self.store.cards()?.into_iter().filter(|c| !c.archived && covered.contains(&c.topic)).filter(|c| after.card(c).day < before.card(c).day).count();
        let drills = self.store.drills()?.iter().filter(|d| match (before.drill(d), after.drill(d)) {
            (Some(b), Some(a)) => a.day < b.day,
            _ => false,
        }).count();
        Ok(DatePreview { quizzes, unquizzed, cards, drills })
    }

    /// A date's own panel (R135).
    pub fn date_panel(&self, id: DateId, today: i64) -> ApiResult<DatePanel> {
        let saved = self.store.dates()?;
        let date = saved.iter().find(|d| d.id == id).ok_or_else(|| crate::ApiError::new("that date isn't there any more"))?;
        let topics = self.topic_modules()?;
        let covered = date.topics(&topics);
        let without: Vec<DateRecord> = saved.iter().filter(|d| d.id != id).cloned().collect();
        let (before, now) = (self.now_from(today, &without)?, self.now_from(today, &saved)?);
        let topic_rows = self.store.topics()?;
        let schedule = self.store.quiz_schedule()?;
        let quizzes_here = self.store.topic_quizzes()?;
        let mut quizzes = vec![];
        let mut unquizzed = vec![];
        for topic in topic_rows.iter().filter(|t| covered.contains(&t.id)) {
            match quizzes_here.iter().find(|q| q.topic == topic.id) {
                Some(quiz) => quizzes.push(DateMove { topic: topic.id, name: topic.name.clone(), step: schedule.label(schedule.clamp(quiz.position)), from: before.quiz(quiz).day, to: now.quiz(quiz).day }),
                None => unquizzed.push(UnquizzedTopic { topic: topic.id, name: topic.name.clone(), questions: topic.question_count }),
            }
        }
        quizzes.sort_by_key(|m| (m.to, m.topic));
        let cards: Vec<CardRow> = self.store.cards()?.into_iter().filter(|c| !c.archived && !c.schedule.is_new() && covered.contains(&c.topic)).collect();
        let before_it: Vec<&CardRow> = cards.iter().filter(|c| now.card(c).day < date.day).collect();
        let cards_pulled = before_it.iter().filter(|c| before.card(c).day >= date.day).count();
        Ok(DatePanel { date: self.date_view(date, &topics, today)?, quizzes, unquizzed, cards_before: before_it.len(), cards_pulled })
    }
}

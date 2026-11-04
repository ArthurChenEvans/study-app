//! The unified calendar (R98 to R100): what's due each day, what was done on past days, and
//! the counts the sidebar shows. Overdue things count as due today.

use serde::{Deserialize, Serialize};

use study_core::question::TopicId;
use study_store::AttemptId;
use study_store::recall::ChecklistId;

use crate::cards::DueCounts;
use crate::practice::{DrillView, StruggleView};
use crate::{ApiResult, App};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    Past,
    Today,
    Future,
}

/// One day in the month view. For past days these are what was done, otherwise what's due.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDay {
    pub day: i64,
    pub when: When,
    pub cards: usize,
    pub checklists: usize,
    pub quizzes: usize,
    pub drills: usize,
    /// Past days and today only.
    pub struggles: usize,
}

/// Everything due today, for the sidebar (R100).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueToday {
    pub cards: usize,
    pub checklists: usize,
    pub quizzes: usize,
    pub drills: usize,
    pub total: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayCards {
    /// Today: the session's counts. Future: the forecast in `review`. Past: zero.
    pub due: DueCounts,
    pub minutes: usize,
    /// Past days.
    pub reviewed: usize,
    pub again: usize,
    pub seconds: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayChecklist {
    pub id: ChecklistId,
    pub name: String,
    pub step: Option<String>,
    pub points: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayCheck {
    pub id: ChecklistId,
    pub name: String,
    pub score: u8,
    pub passed: bool,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayTopic {
    pub topic: TopicId,
    pub name: String,
    pub step: String,
    pub questions: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayAttempt {
    pub attempt: AttemptId,
    pub topics: Vec<String>,
    pub right: usize,
    pub total: usize,
    /// e.g. "OSI model moved up to Fortnight"
    pub moves: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayDrillRun {
    pub task: String,
    pub rating: study_core::practice::DrillRating,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DayView {
    pub day: i64,
    pub when: When,
    pub cards: DayCards,
    pub checklists: Vec<DayChecklist>,
    pub checks: Vec<DayCheck>,
    pub quizzes: Vec<DayTopic>,
    pub attempts: Vec<DayAttempt>,
    pub drills: Vec<DrillView>,
    pub drill_runs: Vec<DayDrillRun>,
    pub struggles: Vec<StruggleView>,
}

fn when(day: i64, today: i64) -> When {
    match day.cmp(&today) {
        std::cmp::Ordering::Less => When::Past,
        std::cmp::Ordering::Equal => When::Today,
        std::cmp::Ordering::Greater => When::Future,
    }
}

impl App {
    /// Reviewed cards (not new, not archived) due on exactly this day, for the forecast.
    fn cards_forecast(&self, day: i64) -> ApiResult<usize> {
        Ok(self
            .store
            .cards()?
            .iter()
            .filter(|c| !c.archived && !c.schedule.is_new() && c.schedule.due == day)
            .count())
    }

    /// Counts for the month view, from `from` to `to` inclusive (R98).
    pub fn calendar_month(&self, from: i64, to: i64, today: i64) -> ApiResult<Vec<CalendarDay>> {
        let done = self.store.done_counts(from, to.min(today))?;
        let cards = self.store.cards()?;
        let checklists = self.store.checklists()?;
        let quizzes = self.store.topic_quizzes()?;
        let drills = self.store.drills()?;
        let due_today = self.due_today(today)?;
        Ok((from..=to)
            .map(|day| match when(day, today) {
                When::Past => {
                    let counts = done.get(&day).copied().unwrap_or_default();
                    CalendarDay { day, when: When::Past, cards: counts.cards, checklists: counts.checks, quizzes: counts.quizzes, drills: counts.drills, struggles: counts.struggles }
                }
                When::Today => CalendarDay {
                    day,
                    when: When::Today,
                    cards: due_today.cards,
                    checklists: due_today.checklists,
                    quizzes: due_today.quizzes,
                    drills: due_today.drills,
                    struggles: done.get(&day).map_or(0, |c| c.struggles),
                },
                When::Future => CalendarDay {
                    day,
                    when: When::Future,
                    cards: cards.iter().filter(|c| !c.archived && !c.schedule.is_new() && c.schedule.due == day).count(),
                    checklists: checklists.iter().filter(|c| c.points > 0 && c.due == day).count(),
                    quizzes: quizzes.iter().filter(|q| q.due == day).count(),
                    drills: drills.iter().filter(|d| d.due == Some(day)).count(),
                    struggles: 0,
                },
            })
            .collect())
    }

    /// What's due today, overdue included (R98, R100).
    pub fn due_today(&self, today: i64) -> ApiResult<DueToday> {
        let cards = self.cards_due_count(today)?;
        let checklists = self.store.checklists()?.iter().filter(|c| c.points > 0 && c.due <= today).count();
        let quizzes = self.store.topic_quizzes()?.iter().filter(|q| q.due <= today).count();
        let drills = self.store.drills()?.iter().filter(|d| d.due.is_some_and(|due| due <= today)).count();
        Ok(DueToday { cards, checklists, quizzes, drills, total: cards + checklists + quizzes + drills })
    }

    /// One day in detail (R99).
    pub fn calendar_day(&self, day: i64, today: i64) -> ApiResult<DayView> {
        let when = when(day, today);
        let recall_schedule = self.store.schedule()?;
        let quiz_schedule = self.store.quiz_schedule()?;
        let topics = self.store.topics()?;
        let due_on = |due: i64| if when == When::Today { due <= today } else { due == day };

        let mut cards = DayCards { due: DueCounts { new: 0, learning: 0, review: 0 }, minutes: 0, reviewed: 0, again: 0, seconds: 0 };
        match when {
            When::Today => {
                let home = self.cards_home(today)?;
                cards.due = home.due;
                cards.minutes = home.minutes;
            }
            When::Future => cards.due.review = self.cards_forecast(day)?,
            When::Past => {
                for review in self.store.reviews_since(day)?.into_iter().filter(|r| r.day == day) {
                    cards.reviewed += 1;
                    cards.again += usize::from(review.rating == study_core::cards::Rating::Again as u8);
                    cards.seconds += review.seconds;
                }
            }
        }

        let mut checklists = vec![];
        let mut checks = vec![];
        for row in self.store.checklists()? {
            if when == When::Past {
                for check in self.store.checks(row.id)?.into_iter().filter(|c| c.day == day) {
                    checks.push(DayCheck { id: row.id, name: row.subject_name.clone(), score: check.score, passed: check.passed, to: check.to_label });
                }
            } else if row.points > 0 && due_on(row.due) {
                checklists.push(DayChecklist {
                    id: row.id,
                    name: row.subject_name.clone(),
                    step: row.position.map(|p| recall_schedule.label(recall_schedule.clamp(p))),
                    points: row.points,
                });
            }
        }

        let mut quizzes = vec![];
        let mut attempts = vec![];
        if when == When::Past {
            for row in self.store.history()?.into_iter().filter(|h| h.day == day) {
                let tally = row.summary.overall;
                attempts.push(DayAttempt {
                    attempt: row.id,
                    topics: row.topics.iter().filter_map(|id| topics.iter().find(|t| t.id == *id).map(|t| t.name.clone())).collect(),
                    right: tally.right_sure + tally.right_guessed,
                    total: tally.right_sure + tally.right_guessed + tally.wrong + tally.unanswered,
                    moves: self
                        .move_views(row.id)?
                        .into_iter()
                        .map(|m| format!("{} {} {}", m.name, if m.passed { "moved up to" } else { "went back to" }, m.to))
                        .collect(),
                });
            }
        } else {
            for quiz in self.store.topic_quizzes()?.into_iter().filter(|q| due_on(q.due)) {
                if let Some(topic) = topics.iter().find(|t| t.id == quiz.topic) {
                    quizzes.push(DayTopic {
                        topic: quiz.topic,
                        name: topic.name.clone(),
                        step: quiz_schedule.label(quiz_schedule.clamp(quiz.position)),
                        questions: topic.question_count,
                    });
                }
            }
        }

        let mut drills = vec![];
        let mut drill_runs = vec![];
        if when == When::Past {
            let all = self.store.drills()?;
            for run in self.store.drill_runs_on(day)? {
                let task = all.iter().find(|d| d.id == run.drill).map(|d| d.task.clone()).unwrap_or_default();
                drill_runs.push(DayDrillRun { task, rating: run.rating });
            }
        } else {
            for drill in self.store.drills()?.into_iter().filter(|d| d.due.is_some_and(due_on)) {
                drills.push(self.drill_view_of(drill)?);
            }
        }

        let struggles = if when == When::Future {
            vec![]
        } else {
            self.store.struggles_on(day)?.into_iter().map(|s| self.struggle_view(s)).collect::<ApiResult<_>>()?
        };

        Ok(DayView { day, when, cards, checklists, checks, quizzes, attempts, drills, drill_runs, struggles })
    }
}

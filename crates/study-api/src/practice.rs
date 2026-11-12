//! Projects, struggles, session reviews, issues and drills (R87 to R95), and each topic's
//! quiz schedule (R96, R97). Thin over the store and the core's rules.

use serde::{Deserialize, Serialize};

use study_core::attempt::Attempt;
use study_core::practice::{DrillRating, PracticeError, StruggleKind, issue_key, topic_scores};
use study_core::question::TopicId;
use study_core::recall::Schedule;
use study_store::AttemptId;
use study_store::practice::{
    DrillId, DrillRecord, DrillRun, IssueId, NewStruggle, ProjectId, ProjectRecord, QuizMove, SessionId, SessionRecord,
    StruggleId, StruggleOutcome, StruggleRecord, TopicQuiz,
};
use study_store::recall::ChecklistId;

use crate::recall::PointInput;
use crate::{ApiError, ApiResult, App};

/// The moment something happens, from the shell's local clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub day: i64,
    /// Minutes after midnight.
    pub minute: u32,
    /// Unix seconds.
    pub unix: i64,
}

impl From<PracticeError> for ApiError {
    fn from(error: PracticeError) -> Self {
        ApiError::new(error)
    }
}

// ---------- Views ----------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: ProjectId,
    pub name: String,
    pub kind: String,
    pub created: i64,
    /// The last day a struggle was logged or a session started.
    pub last_day: Option<i64>,
    pub struggles: usize,
    pub to_review: usize,
    pub drills_due: usize,
    pub session: Option<SessionRecord>,
}

/// What a reviewed struggle became, ready to show.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OutcomeView {
    Card,
    Checklist { id: ChecklistId, name: String },
    Drill { id: DrillId, due: Option<i64> },
    Nothing,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StruggleView {
    pub id: StruggleId,
    pub project: ProjectId,
    pub project_name: String,
    pub day: i64,
    pub minute: u32,
    pub what: String,
    pub why: String,
    pub kind: StruggleKind,
    pub topic: Option<TopicId>,
    pub topic_name: Option<String>,
    pub issue: Option<String>,
    /// None until it's reviewed.
    pub outcome: Option<OutcomeView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub row: ProjectRow,
    pub struggles: Vec<StruggleView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StruggleBrief {
    pub day: i64,
    pub project_name: String,
    pub what: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DrillView {
    pub id: DrillId,
    pub task: String,
    pub hints: String,
    pub issue: Option<String>,
    pub topic: Option<TopicId>,
    pub topic_name: Option<String>,
    pub created: i64,
    /// None once it's ticked off.
    pub due: Option<i64>,
    pub last: Option<DrillRun>,
    pub runs: usize,
    /// The struggles with this drill's issue, newest first (R93).
    pub from: Vec<StruggleBrief>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IssueView {
    pub id: IssueId,
    pub name: String,
    pub seen: usize,
    pub projects: Vec<String>,
    pub last_seen: Option<i64>,
    pub drill: Option<DrillView>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StruggleInput {
    pub what: String,
    pub why: String,
    pub kind: StruggleKind,
    pub topic: Option<TopicId>,
}

/// What a struggle becomes at review (R90 to R92).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ReviewChoice {
    Card,
    Checklist { checklist: ChecklistId },
    Nothing,
    NewDrill { issue: String, task: String, hints: String, topic: Option<TopicId>, day: i64 },
    DrillAgain { issue: String, drill: DrillId, day: i64 },
}

/// A topic's place on its quiz schedule (R97).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TopicQuizView {
    pub topic: TopicId,
    pub step: String,
    /// Index of the current step, and how many steps the ladder has, for drawing it.
    pub position: usize,
    pub steps: usize,
    /// Tightened for a date when one covers it (R132).
    pub due: i64,
    pub due_today: bool,
    /// Its next quiz without the date.
    pub normal_due: i64,
    pub tightened_for: Option<String>,
    /// Overdue, but past the day's limit, so it waits its turn (R146).
    pub waiting: bool,
}

/// Today's topic quizzes under the daily limit (R146).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuizQueue {
    pub daily: usize,
    /// Due today, most overdue first, at most `daily`.
    pub today: Vec<TopicId>,
    /// Overdue ones waiting their turn, most overdue first.
    pub waiting: Vec<TopicId>,
    /// The day the last waiting one comes in, at `daily` a day.
    pub clears_by: Option<i64>,
    /// How many come due over the next four weeks, to warn when that's more than the
    /// limit can keep up with.
    pub next_four_weeks: usize,
}

/// Where a quiz moved one topic (R97).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MoveView {
    pub topic: TopicId,
    pub name: String,
    pub score: u8,
    pub passed: bool,
    pub from: Option<String>,
    pub to: String,
    /// The next quiz on the schedule. `tightened_due` is earlier when a date pulls it in,
    /// so results can show both (R137).
    pub next_due: i64,
    pub tightened_due: Option<i64>,
    pub tightened_for: Option<String>,
}

impl App {
    // ---------- Projects and sessions (R87, R88) ----------

    pub fn projects(&self, today: i64) -> ApiResult<Vec<ProjectRow>> {
        let projects = self.store.projects()?;
        projects.into_iter().map(|project| self.project_row(project, today)).collect()
    }

    fn project_row(&self, project: ProjectRecord, today: i64) -> ApiResult<ProjectRow> {
        let struggles = self.store.struggles(project.id)?;
        let sessions = self.store.sessions(project.id)?;
        let drills = self.drills_of(&struggles, today)?;
        let last_day = struggles.iter().map(|s| s.day).chain(sessions.iter().map(|s| s.day)).max();
        Ok(ProjectRow {
            id: project.id,
            name: project.name,
            kind: project.kind,
            created: project.created,
            last_day,
            struggles: struggles.len(),
            to_review: struggles.iter().filter(|s| s.outcome.is_none()).count(),
            drills_due: drills.iter().filter(|d| d.due.is_some_and(|due| due <= today)).count(),
            session: sessions.into_iter().find(|s| s.ended.is_none()),
        })
    }

    /// The drills a project's struggles were reviewed into.
    fn drills_of(&self, struggles: &[StruggleRecord], today: i64) -> ApiResult<Vec<DrillRecord>> {
        let mut ids: Vec<DrillId> = struggles
            .iter()
            .filter_map(|s| match s.outcome {
                Some(StruggleOutcome::Drill(id)) => Some(id),
                _ => None,
            })
            .collect();
        ids.sort();
        ids.dedup();
        Ok(self.drills_now(&self.now(today)?)?.into_iter().filter(|d| ids.contains(&d.id)).collect())
    }

    pub fn create_project(&mut self, name: &str, kind: &str, today: i64) -> ApiResult<ProjectId> {
        let name = name.trim();
        if name.is_empty() {
            return Err(PracticeError::NoProjectName.into());
        }
        Ok(self.store.create_project(name, kind.trim(), today)?)
    }

    pub fn delete_project(&mut self, id: ProjectId) -> ApiResult<()> {
        Ok(self.store.delete_project(id)?)
    }

    pub fn project_view(&self, id: ProjectId, today: i64) -> ApiResult<ProjectView> {
        let project = self.store.project(id)?;
        let struggles = self.store.struggles(id)?.into_iter().map(|s| self.struggle_view(s)).collect::<ApiResult<_>>()?;
        Ok(ProjectView { row: self.project_row(project, today)?, struggles })
    }

    pub fn start_session(&mut self, project: ProjectId, clock: Clock) -> ApiResult<ProjectView> {
        if self.store.running_session(project)?.is_some() {
            return Err(PracticeError::SessionRunning.into());
        }
        self.store.start_session(project, clock.day, clock.unix)?;
        self.project_view(project, clock.day)
    }

    /// Ends the running session. Its struggles are then reviewed (R90).
    pub fn end_session(&mut self, project: ProjectId, clock: Clock) -> ApiResult<SessionId> {
        let session = self.store.running_session(project)?.ok_or(PracticeError::NoSession)?;
        self.store.end_session(session.id, clock.unix)?;
        Ok(session.id)
    }

    // ---------- Struggles (R89, R90) ----------

    pub fn log_struggle(&mut self, project: ProjectId, input: StruggleInput, clock: Clock) -> ApiResult<StruggleView> {
        let session = self.store.running_session(project)?.ok_or(PracticeError::NoSession)?;
        let what = input.what.trim();
        if what.is_empty() {
            return Err(PracticeError::NoWhat.into());
        }
        let id = self.store.add_struggle(&NewStruggle {
            project,
            session: session.id,
            day: clock.day,
            minute: clock.minute,
            what,
            why: input.why.trim(),
            kind: input.kind,
            topic: input.topic,
        })?;
        self.struggle_view(self.store.struggle(id)?)
    }

    pub fn delete_struggle(&mut self, id: StruggleId) -> ApiResult<()> {
        Ok(self.store.delete_struggle(id)?)
    }

    /// A project's struggles still to review, oldest first, so a review goes in order.
    pub fn struggles_to_review(&self, project: ProjectId) -> ApiResult<Vec<StruggleView>> {
        let mut rows: Vec<StruggleRecord> = self.store.struggles(project)?.into_iter().filter(|s| s.outcome.is_none()).collect();
        rows.reverse();
        rows.into_iter().map(|s| self.struggle_view(s)).collect()
    }

    pub(crate) fn struggle_view(&self, record: StruggleRecord) -> ApiResult<StruggleView> {
        let project_name = self.store.project(record.project).map(|p| p.name).unwrap_or_default();
        let topic_name = match record.topic {
            Some(topic) => self.store.topics()?.into_iter().find(|t| t.id == topic).map(|t| t.name),
            None => None,
        };
        let issue = match record.issue {
            Some(id) => self.store.issues()?.into_iter().find(|i| i.id == id).map(|i| i.name),
            None => None,
        };
        let outcome = match record.outcome {
            None => None,
            Some(StruggleOutcome::Card) => Some(OutcomeView::Card),
            Some(StruggleOutcome::Nothing) => Some(OutcomeView::Nothing),
            Some(StruggleOutcome::Checklist(id)) => Some(OutcomeView::Checklist {
                id,
                name: self.store.checklist(id).map(|c| c.subject_name).unwrap_or_else(|_| "a deleted checklist".into()),
            }),
            Some(StruggleOutcome::Drill(id)) => Some(OutcomeView::Drill { id, due: self.store.drill(id).ok().and_then(|d| d.due) }),
        };
        Ok(StruggleView {
            id: record.id,
            project: record.project,
            project_name,
            day: record.day,
            minute: record.minute,
            what: record.what,
            why: record.why,
            kind: record.kind,
            topic: record.topic,
            topic_name,
            issue,
            outcome,
        })
    }

    /// Decides what a struggle becomes (R90 to R92). Drills are scheduled today or later.
    pub fn review_struggle(&mut self, id: StruggleId, choice: ReviewChoice, today: i64) -> ApiResult<StruggleView> {
        let struggle = self.store.struggle(id)?;
        if struggle.outcome.is_some() {
            return Err(PracticeError::AlreadyReviewed.into());
        }
        let (issue, outcome) = match choice {
            ReviewChoice::Card => (None, StruggleOutcome::Card),
            ReviewChoice::Nothing => (None, StruggleOutcome::Nothing),
            ReviewChoice::Checklist { checklist } => {
                let mut points: Vec<PointInput> = self
                    .store
                    .points(checklist)?
                    .into_iter()
                    .map(|p| PointInput { text: p.text, question: p.question, note: p.note })
                    .collect();
                points.push(PointInput { text: struggle.what.clone(), question: None, note: None });
                self.save_points(checklist, points)?;
                (None, StruggleOutcome::Checklist(checklist))
            }
            ReviewChoice::NewDrill { issue, task, hints, topic, day } => {
                if task.trim().is_empty() {
                    return Err(PracticeError::NoTask.into());
                }
                if day < today {
                    return Err(PracticeError::PastDay.into());
                }
                let issue = self.issue(&issue)?;
                let drill = self.store.add_drill(Some(issue), topic.or(struggle.topic), task.trim(), hints.trim(), today, day)?;
                (Some(issue), StruggleOutcome::Drill(drill))
            }
            ReviewChoice::DrillAgain { issue, drill, day } => {
                if day < today {
                    return Err(PracticeError::PastDay.into());
                }
                let issue = self.issue(&issue)?;
                self.store.set_drill_due(drill, Some(day))?;
                (Some(issue), StruggleOutcome::Drill(drill))
            }
        };
        self.store.review_struggle(id, issue, outcome)?;
        self.struggle_view(self.store.struggle(id)?)
    }

    fn issue(&mut self, name: &str) -> ApiResult<IssueId> {
        if issue_key(name).is_empty() {
            return Err(PracticeError::NoIssue.into());
        }
        Ok(self.store.issue_named(name)?)
    }

    // ---------- Issues (R91, R94) ----------

    pub fn issues(&self) -> ApiResult<Vec<IssueView>> {
        let struggles = self.store.all_struggles()?;
        let projects = self.store.projects()?;
        self.store
            .issues()?
            .into_iter()
            .map(|issue| {
                let seen: Vec<&StruggleRecord> = struggles.iter().filter(|s| s.issue == Some(issue.id)).collect();
                let mut names: Vec<String> = seen
                    .iter()
                    .filter_map(|s| projects.iter().find(|p| p.id == s.project).map(|p| p.name.clone()))
                    .collect();
                names.sort();
                names.dedup();
                Ok(IssueView {
                    id: issue.id,
                    name: issue.name,
                    seen: seen.len(),
                    projects: names,
                    last_seen: seen.iter().map(|s| s.day).max(),
                    drill: self.store.latest_drill(issue.id)?.map(|d| self.drill_view_of(d)).transpose()?,
                })
            })
            .collect()
    }

    /// The drill to offer again when a review names this issue, if it has one (R91).
    pub fn known_drill(&self, issue: &str) -> ApiResult<Option<DrillView>> {
        let key = issue_key(issue);
        let Some(found) = self.store.issues()?.into_iter().find(|i| issue_key(&i.name) == key) else {
            return Ok(None);
        };
        self.store.latest_drill(found.id)?.map(|d| self.drill_view_of(d)).transpose()
    }

    // ---------- Drills (R92, R93) ----------

    pub(crate) fn drill_view_of(&self, drill: DrillRecord) -> ApiResult<DrillView> {
        let issue = match drill.issue {
            Some(id) => self.store.issues()?.into_iter().find(|i| i.id == id),
            None => None,
        };
        let topic_name = match drill.topic {
            Some(topic) => self.store.topics()?.into_iter().find(|t| t.id == topic).map(|t| t.name),
            None => None,
        };
        let runs = self.store.drill_runs(drill.id)?;
        let projects = self.store.projects()?;
        let from = match &issue {
            Some(issue) => self
                .store
                .all_struggles()?
                .into_iter()
                .filter(|s| s.issue == Some(issue.id))
                .map(|s| StruggleBrief {
                    day: s.day,
                    project_name: projects.iter().find(|p| p.id == s.project).map(|p| p.name.clone()).unwrap_or_default(),
                    what: s.what,
                })
                .collect(),
            None => vec![],
        };
        Ok(DrillView {
            id: drill.id,
            task: drill.task,
            hints: drill.hints,
            issue: issue.map(|i| i.name),
            topic: drill.topic,
            topic_name,
            created: drill.created,
            due: drill.due,
            last: runs.last().cloned(),
            runs: runs.len(),
            from,
        })
    }

    pub fn drill_view(&self, id: DrillId) -> ApiResult<DrillView> {
        self.drill_view_of(self.store.drill(id)?)
    }

    /// Drills due today or overdue, oldest first (R98: overdue counts as today).
    pub fn drills_due(&self, today: i64) -> ApiResult<Vec<DrillView>> {
        self.drills_now(&self.now(today)?)?
            .into_iter()
            .filter(|d| d.due.is_some_and(|due| due <= today))
            .map(|d| self.drill_view_of(d))
            .collect()
    }

    pub fn rate_drill(&mut self, id: DrillId, rating: DrillRating, seconds: u32, note: &str, today: i64) -> ApiResult<DrillView> {
        self.store.record_drill_run(&DrillRun { drill: id, day: today, rating, seconds, note: note.trim().into() })?;
        self.drill_view(id)
    }

    pub fn move_drill(&mut self, id: DrillId, day: i64, today: i64) -> ApiResult<DrillView> {
        if day < today {
            return Err(PracticeError::PastDay.into());
        }
        self.store.set_drill_due(id, Some(day))?;
        self.drill_view(id)
    }

    // ---------- Quiz schedules (R96, R97) ----------

    pub fn quiz_schedule(&self) -> ApiResult<Schedule> {
        Ok(self.store.quiz_schedule()?)
    }

    pub fn set_quiz_schedule(&mut self, schedule: Schedule) -> ApiResult<()> {
        schedule.validate()?;
        Ok(self.store.set_quiz_schedule(&schedule)?)
    }

    pub fn reset_quiz_schedule(&mut self) -> ApiResult<Schedule> {
        self.store.reset_quiz_schedule()?;
        self.quiz_schedule()
    }

    pub fn quizzes_a_day(&self) -> ApiResult<usize> {
        Ok(self.store.quizzes_a_day()?)
    }

    pub fn set_quizzes_a_day(&mut self, count: usize) -> ApiResult<()> {
        Ok(self.store.set_quizzes_a_day(count)?)
    }

    /// Overdue topic quizzes split into today's (at most the daily limit, most overdue
    /// first) and the ones waiting (R146). Waiting doesn't move a topic down its ladder.
    pub fn quiz_queue(&self, today: i64) -> ApiResult<QuizQueue> {
        let daily = self.store.quizzes_a_day()?;
        let mut quizzes = self.quizzes_now(&self.now(today)?)?;
        quizzes.sort_by_key(|quiz| (quiz.due, quiz.topic));
        let overdue: Vec<TopicId> = quizzes.iter().filter(|quiz| quiz.due <= today).map(|quiz| quiz.topic).collect();
        let (first, rest) = overdue.split_at(overdue.len().min(daily));
        let coming = quizzes.iter().filter(|quiz| quiz.due > today && quiz.due <= today + 28).count();
        Ok(QuizQueue {
            daily,
            today: first.to_vec(),
            waiting: rest.to_vec(),
            clears_by: (!rest.is_empty()).then(|| today + rest.len().div_ceil(daily) as i64),
            next_four_weeks: coming,
        })
    }

    pub fn topic_quizzes(&self, today: i64) -> ApiResult<Vec<TopicQuizView>> {
        let schedule = self.store.quiz_schedule()?;
        let now = self.now(today)?;
        let queue = self.quiz_queue(today)?;
        Ok(self
            .store
            .topic_quizzes()?
            .into_iter()
            .map(|quiz| {
                let position = schedule.clamp(quiz.position);
                let due = now.quiz(&quiz);
                TopicQuizView {
                    topic: quiz.topic,
                    step: schedule.label(position),
                    position: position.step,
                    steps: schedule.steps.len(),
                    due: due.day,
                    due_today: queue.today.contains(&quiz.topic),
                    normal_due: quiz.due,
                    tightened_for: now.tightened_for(&due),
                    waiting: queue.waiting.contains(&quiz.topic),
                }
            })
            .collect())
    }

    /// Moves every topic of a finished quiz along its schedule (R96).
    pub(crate) fn move_quiz_schedules(&mut self, attempt: AttemptId, saved: &Attempt, day: i64) -> ApiResult<()> {
        let schedule = self.store.quiz_schedule()?;
        for score in topic_scores(saved) {
            let from = self.store.topic_quiz(score.topic)?.map(|q| (schedule.clamp(q.position), q.due));
            let outcome = schedule.check(from.map(|(position, _)| position), score.right, score.total, day)?;
            self.store.save_quiz_move(&QuizMove {
                attempt,
                topic: score.topic,
                day,
                from,
                score: outcome.score,
                passed: outcome.passed,
                to: outcome.to,
                next_due: outcome.next_due,
            })?;
        }
        Ok(())
    }

    /// After an overrule, redoes the quiz's moves for the topics it's still the latest for.
    pub(crate) fn redo_quiz_schedules(&mut self, attempt: AttemptId, saved: &Attempt) -> ApiResult<()> {
        let schedule = self.store.quiz_schedule()?;
        let moves = self.store.quiz_moves(attempt)?;
        for score in topic_scores(saved) {
            let Some(old) = moves.iter().find(|m| m.topic == score.topic) else {
                continue;
            };
            if self.store.latest_quiz_move(score.topic)? != Some(attempt) {
                continue;
            }
            let outcome = schedule.check(old.from.map(|(position, _)| position), score.right, score.total, old.day)?;
            self.store.save_quiz_move(&QuizMove { score: outcome.score, passed: outcome.passed, to: outcome.to, next_due: outcome.next_due, ..*old })?;
        }
        Ok(())
    }

    pub(crate) fn move_views(&self, attempt: AttemptId) -> ApiResult<Vec<MoveView>> {
        let schedule = self.store.quiz_schedule()?;
        let topics = self.store.topics()?;
        let moves = self.store.quiz_moves(attempt)?;
        // As of the quiz day, so the results say what the date did then.
        let now = self.now(moves.first().map_or(0, |m| m.day))?;
        let latest = |m: &QuizMove| self.store.latest_quiz_move(m.topic).ok().flatten() == Some(m.attempt);
        Ok(moves
            .into_iter()
            .map(|m| {
                // Only the topic's latest quiz still sets its schedule.
                let due = latest(&m).then(|| now.quiz(&TopicQuiz { topic: m.topic, position: m.to, due: m.next_due })).filter(|due| due.tightened_by.is_some());
                MoveView {
                topic: m.topic,
                name: topics.iter().find(|t| t.id == m.topic).map(|t| t.name.clone()).unwrap_or_default(),
                score: m.score,
                passed: m.passed,
                from: m.from.map(|(position, _)| schedule.label(schedule.clamp(position))),
                to: schedule.label(schedule.clamp(m.to)),
                next_due: m.next_due,
                tightened_due: due.map(|due| due.day),
                tightened_for: due.and_then(|due| now.tightened_for(&due)),
            }})
            .collect())
    }
}

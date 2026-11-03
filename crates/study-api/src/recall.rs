//! Free recall commands (R52 to R61). Thin over the store and the core's schedule.

use serde::{Deserialize, Serialize};

use study_core::question::QuestionId;
use study_core::recall::Schedule;
use study_store::recall::{CheckRecord, ChecklistId, ChecklistRow, Point, Subject};

use crate::{ApiError, ApiResult, App};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecallRow {
    pub id: ChecklistId,
    pub subject_name: String,
    pub module: bool,
    /// False once the module or topic has been deleted.
    pub subject_exists: bool,
    pub points: usize,
    /// The current step, or None for a checklist that hasn't had its first check.
    pub step: Option<String>,
    /// The step a pass would move it to.
    pub next_step: String,
    /// Ladder place for drawing the track: step index and repeat.
    pub position: Option<(usize, u32)>,
    pub due: i64,
    pub due_today: bool,
    pub days_late: i64,
    pub last_score: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ChecklistView {
    pub row: RecallRow,
    pub points: Vec<Point>,
    /// Names of the topics the checklist covers, for "Add from questions".
    pub topics: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub question: QuestionId,
    pub topic_name: String,
    pub text: String,
    pub answer: String,
    pub in_checklist: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub score: u8,
    pub passed: bool,
    pub to: String,
    pub next_due: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Missed {
    pub text: String,
    pub missed: usize,
    pub checks: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub score: u8,
    pub recalled: usize,
    pub total: usize,
    pub passed: bool,
    pub previous_score: Option<u8>,
    pub from: Option<String>,
    pub to: String,
    pub position: (usize, u32),
    pub next_due: i64,
    pub missed: Vec<Missed>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CheckView {
    pub day: i64,
    pub score: u8,
    pub passed: bool,
    pub from: Option<String>,
    pub to: String,
    pub dump: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecallHistory {
    pub checks: Vec<CheckView>,
    pub most_missed: Vec<Missed>,
    pub pass_mark: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PointInput {
    pub text: String,
    pub question: Option<QuestionId>,
}

/// How often each point was missed across checks, most missed first.
fn missed_counts(checks: &[CheckRecord], only: Option<&[String]>) -> Vec<Missed> {
    let mut counts: Vec<Missed> = vec![];
    for check in checks {
        for (text, recalled) in &check.points {
            let entry = match counts.iter_mut().find(|m| &m.text == text) {
                Some(entry) => entry,
                None => {
                    counts.push(Missed {
                        text: text.clone(),
                        missed: 0,
                        checks: 0,
                    });
                    counts.last_mut().unwrap()
                }
            };
            entry.checks += 1;
            if !recalled {
                entry.missed += 1;
            }
        }
    }
    if let Some(only) = only {
        counts.retain(|m| only.contains(&m.text));
    }
    counts.sort_by_key(|m| std::cmp::Reverse(m.missed));
    counts
}

impl App {
    fn recall_row(&self, row: ChecklistRow, schedule: &Schedule, today: i64) -> RecallRow {
        let position = row.position.map(|p| schedule.clamp(p));
        RecallRow {
            id: row.id,
            subject_name: row.subject_name,
            module: row.module,
            subject_exists: row.subject.is_some(),
            points: row.points,
            step: position.map(|p| schedule.label(p)),
            next_step: schedule.label(schedule.after_pass(position)),
            position: position.map(|p| (p.step, p.rep)),
            due: row.due,
            due_today: row.due <= today,
            days_late: (today - row.due).max(0),
            last_score: row.last_score,
        }
    }

    /// Every checklist, soonest due first (R58).
    pub fn recall_list(&self, today: i64) -> ApiResult<Vec<RecallRow>> {
        let schedule = self.store.schedule()?;
        Ok(self
            .store
            .checklists()?
            .into_iter()
            .map(|row| self.recall_row(row, &schedule, today))
            .collect())
    }

    pub fn recall_due_count(&self, today: i64) -> ApiResult<usize> {
        Ok(self
            .store
            .checklists()?
            .iter()
            .filter(|row| row.due <= today)
            .count())
    }

    pub fn create_checklist(&mut self, subject: Subject, today: i64) -> ApiResult<ChecklistId> {
        Ok(self.store.create_checklist(subject, today)?)
    }

    pub fn delete_checklist(&mut self, id: ChecklistId) -> ApiResult<()> {
        Ok(self.store.delete_checklist(id)?)
    }

    pub fn checklist(&self, id: ChecklistId, today: i64) -> ApiResult<ChecklistView> {
        let schedule = self.store.schedule()?;
        let row = self.store.checklist(id)?;
        let topics = self
            .subject_topics(row.subject)?
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        Ok(ChecklistView {
            points: self.store.points(id)?,
            topics,
            row: self.recall_row(row, &schedule, today),
        })
    }

    fn subject_topics(
        &self,
        subject: Option<Subject>,
    ) -> ApiResult<Vec<(study_core::question::TopicId, String)>> {
        Ok(self
            .store
            .topics()?
            .into_iter()
            .filter(|topic| match subject {
                Some(Subject::Module(module)) => topic.module == Some(module),
                Some(Subject::Topic(id)) => topic.id == id,
                None => false,
            })
            .map(|topic| (topic.id, topic.name))
            .collect())
    }

    /// Replaces the points (R53). Blank points are dropped.
    pub fn save_points(
        &mut self,
        id: ChecklistId,
        points: Vec<PointInput>,
    ) -> ApiResult<Vec<Point>> {
        let points: Vec<Point> = points
            .into_iter()
            .filter_map(|p| {
                let text = p.text.trim();
                (!text.is_empty()).then(|| Point {
                    text: text.into(),
                    question: p.question,
                })
            })
            .collect();
        self.store.set_points(id, &points)?;
        Ok(points)
    }

    /// MCQs from the checklist's topics that can become points (R54).
    pub fn recall_candidates(&self, id: ChecklistId) -> ApiResult<Vec<Candidate>> {
        let row = self.store.checklist(id)?;
        let in_checklist: Vec<QuestionId> = self
            .store
            .points(id)?
            .into_iter()
            .filter_map(|p| p.question)
            .collect();
        let topics = self.subject_topics(row.subject)?;
        let ids: Vec<_> = topics.iter().map(|(id, _)| *id).collect();
        Ok(self
            .store
            .questions_in(&ids)?
            .into_iter()
            .map(|(question_id, question)| {
                let topic_name = topics
                    .iter()
                    .find(|(id, _)| *id == question.topic())
                    .map(|(_, n)| n.clone())
                    .unwrap_or_default();
                let answer = question
                    .correct_indices()
                    .iter()
                    .map(|&i| question.options()[i].text.trim().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Candidate {
                    question: question_id,
                    topic_name,
                    text: question.text().trim().into(),
                    answer,
                    in_checklist: in_checklist.contains(&question_id),
                }
            })
            .collect())
    }

    /// Adds questions as "question → answer" points and removes points of unticked ones.
    pub fn add_from_questions(
        &mut self,
        id: ChecklistId,
        add: Vec<QuestionId>,
        remove: Vec<QuestionId>,
    ) -> ApiResult<Vec<Point>> {
        let candidates = self.recall_candidates(id)?;
        let mut points: Vec<Point> = self
            .store
            .points(id)?
            .into_iter()
            .filter(|p| !p.question.is_some_and(|q| remove.contains(&q)))
            .collect();
        for question in add {
            if points.iter().any(|p| p.question == Some(question)) {
                continue;
            }
            let candidate = candidates
                .iter()
                .find(|c| c.question == question)
                .ok_or_else(|| ApiError::new("that question isn't in this checklist's topics"))?;
            points.push(Point {
                text: format!("{} → {}", candidate.text, candidate.answer),
                question: Some(question),
            });
        }
        self.store.set_points(id, &points)?;
        Ok(points)
    }

    /// What finishing now would do, shown live while ticking (R56).
    pub fn preview_check(
        &self,
        id: ChecklistId,
        recalled: usize,
        today: i64,
    ) -> ApiResult<Preview> {
        let row = self.store.checklist(id)?;
        let schedule = self.store.schedule()?;
        let outcome = schedule.check(row.position, recalled, row.points, today)?;
        Ok(Preview {
            score: outcome.score,
            passed: outcome.passed,
            to: schedule.label(outcome.to),
            next_due: outcome.next_due,
        })
    }

    /// Saves the check and moves the checklist (R55, R57). `recalled` holds point indexes.
    pub fn finish_check(
        &mut self,
        id: ChecklistId,
        dump: Option<String>,
        recalled: Vec<usize>,
        today: i64,
    ) -> ApiResult<CheckResult> {
        let row = self.store.checklist(id)?;
        let points = self.store.points(id)?;
        if recalled.iter().any(|&i| i >= points.len()) {
            return Err(ApiError::new("a ticked point isn't on this checklist"));
        }
        let schedule = self.store.schedule()?;
        let ticked = recalled
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        let outcome = schedule.check(row.position, ticked, points.len(), today)?;
        let from = row.position.map(|p| schedule.label(p));
        let to = schedule.label(outcome.to);
        let snapshot: Vec<(String, bool)> = points
            .iter()
            .enumerate()
            .map(|(i, p)| (p.text.clone(), recalled.contains(&i)))
            .collect();
        let check = CheckRecord {
            day: today,
            dump: dump.map(|d| d.trim().to_string()).filter(|d| !d.is_empty()),
            points: snapshot.clone(),
            score: outcome.score,
            passed: outcome.passed,
            from_label: from.clone(),
            to_label: to.clone(),
        };
        self.store
            .save_check(id, &check, outcome.to, outcome.next_due)?;
        let missed_now: Vec<String> = snapshot
            .iter()
            .filter(|(_, r)| !r)
            .map(|(t, _)| t.clone())
            .collect();
        let mut missed = missed_counts(&self.store.checks(id)?, Some(&missed_now));
        missed.sort_by_key(|m| missed_now.iter().position(|t| t == &m.text));
        Ok(CheckResult {
            score: outcome.score,
            recalled: ticked,
            total: points.len(),
            passed: outcome.passed,
            previous_score: row.last_score,
            from,
            to,
            position: (outcome.to.step, outcome.to.rep),
            next_due: outcome.next_due,
            missed,
        })
    }

    /// Score per check, step changes, dumps and the most missed points (R59).
    pub fn recall_history(&self, id: ChecklistId) -> ApiResult<RecallHistory> {
        let checks = self.store.checks(id)?;
        let mut most_missed = missed_counts(&checks, None);
        most_missed.retain(|m| m.missed > 0);
        most_missed.truncate(3);
        Ok(RecallHistory {
            checks: checks
                .into_iter()
                .map(|c| CheckView {
                    day: c.day,
                    score: c.score,
                    passed: c.passed,
                    from: c.from_label,
                    to: c.to_label,
                    dump: c.dump,
                })
                .collect(),
            most_missed,
            pass_mark: self.store.schedule()?.pass_mark,
        })
    }

    pub fn schedule(&self) -> ApiResult<Schedule> {
        Ok(self.store.schedule()?)
    }

    /// R60. Applies from each checklist's next check.
    pub fn set_schedule(&mut self, schedule: Schedule) -> ApiResult<()> {
        schedule.validate()?;
        Ok(self.store.set_schedule(&schedule)?)
    }

    pub fn reset_schedule(&mut self) -> ApiResult<Schedule> {
        self.store.reset_schedule()?;
        self.schedule()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OptionInput, QuestionInput};
    use study_store::Store;

    const TODAY: i64 = 20_760; // Tue 3 Nov 2026

    fn app() -> App {
        App::with_seed(Store::in_memory().unwrap(), 7)
    }

    fn texts(points: &[Point]) -> Vec<&str> {
        points.iter().map(|p| p.text.as_str()).collect()
    }

    fn with_checklist(points: usize) -> (App, ChecklistId) {
        let mut app = app();
        let topic = app.create_topic("CPU", None).unwrap();
        let id = app.create_checklist(Subject::Topic(topic), TODAY).unwrap();
        app.save_points(
            id,
            (1..=points)
                .map(|i| PointInput {
                    text: format!("Point {i}"),
                    question: None,
                })
                .collect(),
        )
        .unwrap();
        (app, id)
    }

    #[test]
    fn today_is_tuesday_3_november() {
        assert_eq!(
            study_core::recall::add(
                0,
                study_core::recall::Interval {
                    count: TODAY as u32,
                    unit: study_core::recall::Unit::Days
                }
            ),
            TODAY
        );
        // 20 760 days after 1 Jan 1970
        let date = chrono_free_date(TODAY);
        assert_eq!(date, (2026, 11, 3));
    }

    /// Civil date from a day number without pulling chrono into this crate's tests.
    fn chrono_free_date(day: i64) -> (i64, i64, i64) {
        let z = day + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }

    // free-recall.feature: Write, reorder and remove points
    #[test]
    fn blank_points_are_dropped_and_the_rest_trimmed() {
        let (mut app, id) = with_checklist(0);
        let saved = app
            .save_points(
                id,
                vec![
                    PointInput {
                        text: "  Amdahl's law ".into(),
                        question: None,
                    },
                    PointInput {
                        text: " ".into(),
                        question: None,
                    },
                ],
            )
            .unwrap();
        assert_eq!(texts(&saved), vec!["Amdahl's law"]);
    }

    // free-recall.feature: Add and remove points from questions
    #[test]
    fn questions_become_points_and_can_be_removed() {
        let mut app = app();
        let module = app.create_module("Computer Architecture").unwrap();
        let cpu = app.create_topic("CPU", Some(module)).unwrap();
        let other = app.create_topic("Networks", None).unwrap();
        let question = |topic, text: &str| QuestionInput {
            topic,
            text: text.into(),
            multiple: false,
            options: ["Fetch", "Decode", "Execute"]
                .iter()
                .enumerate()
                .map(|(i, t)| OptionInput {
                    text: (*t).into(),
                    correct: i == 1,
                })
                .collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
        };
        let reads = app
            .save_question(question(cpu, "Which stage reads registers?"))
            .unwrap();
        app.save_question(question(other, "Not in this module"))
            .unwrap();
        let id = app
            .create_checklist(Subject::Module(module), TODAY)
            .unwrap();
        app.save_points(
            id,
            vec![PointInput {
                text: "Written".into(),
                question: None,
            }],
        )
        .unwrap();

        let candidates = app.recall_candidates(id).unwrap();
        assert_eq!(candidates.len(), 1, "only questions in the module's topics");
        assert_eq!(
            (
                candidates[0].topic_name.as_str(),
                candidates[0].answer.as_str(),
                candidates[0].in_checklist
            ),
            ("CPU", "Decode", false)
        );

        let points = app
            .add_from_questions(id, vec![reads, reads], vec![])
            .unwrap();
        assert_eq!(
            texts(&points),
            vec!["Written", "Which stage reads registers? → Decode"]
        );
        assert!(app.recall_candidates(id).unwrap()[0].in_checklist);

        let points = app.add_from_questions(id, vec![], vec![reads]).unwrap();
        assert_eq!(texts(&points), vec!["Written"]);
        assert_eq!(
            app.question(reads).unwrap().text,
            "Which stage reads registers?",
            "the question is unchanged"
        );
        assert_eq!(app.checklist(id, TODAY).unwrap().topics, vec!["CPU"]);
    }

    // free-recall.feature: A checklist needs a point before it can start
    #[test]
    fn a_check_needs_points() {
        let (mut app, id) = with_checklist(0);
        assert_eq!(
            app.finish_check(id, None, vec![], TODAY)
                .unwrap_err()
                .message,
            "add at least one point before starting a recall"
        );
    }

    // free-recall.feature: Dump in the app or outside it, The first check puts a new checklist on Day
    #[test]
    fn first_check_saves_the_dump_and_puts_the_checklist_on_day() {
        let (mut app, id) = with_checklist(10);
        let preview = app.preview_check(id, 3, TODAY).unwrap();
        assert_eq!(
            (preview.score, preview.to.as_str(), preview.next_due),
            (30, "Day", TODAY + 1)
        );
        let result = app
            .finish_check(
                id,
                Some("  Pipelines and caches ".into()),
                vec![0, 1, 2, 2],
                TODAY,
            )
            .unwrap();
        assert_eq!(
            (result.recalled, result.total, result.score),
            (3, 10, 30),
            "a point ticked twice counts once"
        );
        assert_eq!(
            (result.from, result.to.as_str(), result.next_due),
            (None, "Day", TODAY + 1)
        );
        assert_eq!(result.missed.len(), 7);
        let history = app.recall_history(id).unwrap();
        assert_eq!(
            history.checks[0].dump.as_deref(),
            Some("Pipelines and caches")
        );
        app.finish_check(id, Some("   ".into()), (0..10).collect(), TODAY + 1)
            .unwrap();
        assert_eq!(
            app.recall_history(id).unwrap().checks[0].dump,
            None,
            "outside the app saves no dump"
        );
        let row = &app.recall_list(TODAY + 1).unwrap()[0];
        assert_eq!(
            (row.step.as_deref(), row.next_step.as_str(), row.due),
            (Some("Week"), "Fortnight", TODAY + 8)
        );
    }

    // free-recall.feature: The result shows what was missed and how often
    #[test]
    fn missed_points_say_how_often_they_have_been_missed() {
        let (mut app, id) = with_checklist(3);
        app.finish_check(id, None, vec![0, 1], TODAY).unwrap();
        app.finish_check(id, None, vec![0, 1], TODAY + 1).unwrap();
        app.finish_check(id, None, vec![0, 1, 2], TODAY + 8)
            .unwrap();
        let result = app.finish_check(id, None, vec![0], TODAY + 22).unwrap();
        let missed: Vec<_> = result
            .missed
            .iter()
            .map(|m| (m.text.as_str(), m.missed, m.checks))
            .collect();
        assert_eq!(
            missed,
            vec![("Point 2", 1, 4), ("Point 3", 3, 4)],
            "in checklist order, with counts"
        );
        let history = app.recall_history(id).unwrap();
        assert_eq!(history.most_missed[0].text, "Point 3");
        assert_eq!(history.checks.len(), 4);
    }

    // free-recall.feature: Due checklists come first
    #[test]
    fn due_count_and_lateness() {
        let (mut app, id) = with_checklist(2);
        assert_eq!(app.recall_due_count(TODAY).unwrap(), 1);
        app.finish_check(id, None, vec![0, 1], TODAY).unwrap();
        assert_eq!(app.recall_due_count(TODAY).unwrap(), 0);
        let row = &app.recall_list(TODAY + 3).unwrap()[0];
        assert_eq!((row.due_today, row.days_late), (true, 2));
    }

    // free-recall.feature: Edit and reset the schedule
    #[test]
    fn the_schedule_is_checked_before_saving_and_can_be_reset() {
        let (mut app, id) = with_checklist(10);
        let mut schedule = app.schedule().unwrap();
        schedule.pass_mark = 0;
        assert_eq!(
            app.set_schedule(schedule.clone()).unwrap_err().message,
            "the pass mark has to be between 1% and 100%"
        );
        schedule.pass_mark = 80;
        app.set_schedule(schedule).unwrap();
        app.finish_check(id, None, (0..10).collect(), TODAY)
            .unwrap();
        assert!(
            !app.preview_check(id, 7, TODAY + 1).unwrap().passed,
            "uses the new pass mark"
        );
        assert_eq!(app.reset_schedule().unwrap().pass_mark, 70);
    }
}

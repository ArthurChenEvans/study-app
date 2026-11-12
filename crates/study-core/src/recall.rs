//! Free recall scheduling (R57, R60). A checklist climbs a ladder of steps. Passing a check
//! moves it up one step, missing the pass mark moves it back one, and the next date counts
//! from the day the check was done. Days are numbered from 1970-01-01, as elsewhere in the core.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub count: u32,
    pub unit: Unit,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub every: Interval,
    /// How many checks this step lasts. 0 means forever, which only the last step may use.
    pub repeats: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderPass {
    BackOneStep,
    Stay,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub steps: Vec<Step>,
    /// Percent of points that has to be recalled to move up.
    pub pass_mark: u8,
    pub under_pass: UnderPass,
}

/// Where a checklist is on the ladder. `rep` counts from 1 within a repeating step.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub step: usize,
    pub rep: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Rounded percent, for showing. Passing uses the exact ratio.
    pub score: u8,
    pub passed: bool,
    pub to: Position,
    pub next_due: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum RecallError {
    #[error("add at least one point before starting a recall")]
    NoPoints,
    #[error("more points ticked than the checklist has")]
    TooManyTicked,
    #[error("the schedule needs at least one step")]
    NoSteps,
    #[error("every step needs an interval of at least 1")]
    ZeroInterval,
    #[error("only the last step can repeat forever")]
    ForeverBeforeLast,
    #[error("the pass mark has to be between 1% and 100%")]
    PassMark,
    #[error("every step needs a name")]
    UnnamedStep,
}

fn step(name: &str, count: u32, unit: Unit, repeats: u32) -> Step {
    Step {
        name: name.into(),
        every: Interval { count, unit },
        repeats,
    }
}

impl Default for Schedule {
    /// The calendar ladder: Day, Week, Fortnight, Month, Monthly ×6, Seasonal ×4, then Yearly.
    fn default() -> Self {
        Self {
            steps: vec![
                step("Day", 1, Unit::Days, 1),
                step("Week", 1, Unit::Weeks, 1),
                step("Fortnight", 2, Unit::Weeks, 1),
                step("Month", 1, Unit::Months, 1),
                step("Monthly", 1, Unit::Months, 6),
                step("Seasonal", 3, Unit::Months, 4),
                step("Yearly", 1, Unit::Years, 0),
            ],
            pass_mark: 70,
            under_pass: UnderPass::BackOneStep,
        }
    }
}

impl Schedule {
    pub fn validate(&self) -> Result<(), RecallError> {
        if self.steps.is_empty() {
            return Err(RecallError::NoSteps);
        }
        if !(1..=100).contains(&self.pass_mark) {
            return Err(RecallError::PassMark);
        }
        for (index, step) in self.steps.iter().enumerate() {
            if step.name.trim().is_empty() {
                return Err(RecallError::UnnamedStep);
            }
            if step.every.count == 0 {
                return Err(RecallError::ZeroInterval);
            }
            if step.repeats == 0 && index + 1 != self.steps.len() {
                return Err(RecallError::ForeverBeforeLast);
            }
        }
        Ok(())
    }

    /// A position saved under an older schedule, fitted to this one.
    pub fn clamp(&self, position: Position) -> Position {
        let step = position.step.min(self.steps.len() - 1);
        let repeats = self.steps[step].repeats;
        let rep = if repeats <= 1 {
            1
        } else {
            position.rep.clamp(1, repeats)
        };
        Position { step, rep }
    }

    /// "Month", "Monthly 2 of 6", "Yearly".
    pub fn label(&self, position: Position) -> String {
        let position = self.clamp(position);
        let step = &self.steps[position.step];
        if step.repeats > 1 {
            format!("{} {} of {}", step.name, position.rep, step.repeats)
        } else {
            step.name.clone()
        }
    }

    /// Where a pass would put a checklist. A new checklist (None) goes on the first step.
    pub fn after_pass(&self, from: Option<Position>) -> Position {
        from.map_or(Position { step: 0, rep: 1 }, |position| {
            self.up(self.clamp(position))
        })
    }

    fn up(&self, position: Position) -> Position {
        let step = &self.steps[position.step];
        if step.repeats == 0 || position.rep < step.repeats {
            return Position {
                step: position.step,
                rep: if step.repeats == 0 {
                    1
                } else {
                    position.rep + 1
                },
            };
        }
        if position.step + 1 < self.steps.len() {
            Position {
                step: position.step + 1,
                rep: 1,
            }
        } else {
            position
        }
    }

    fn back(&self, position: Position) -> Position {
        if position.rep > 1 {
            return Position {
                step: position.step,
                rep: position.rep - 1,
            };
        }
        if position.step == 0 {
            return position;
        }
        let previous = position.step - 1;
        Position {
            step: previous,
            rep: self.steps[previous].repeats.max(1),
        }
    }

    /// The result of a check done on `day`. `from` is None for a checklist's first check,
    /// which always puts it on the first step.
    pub fn check(
        &self,
        from: Option<Position>,
        recalled: usize,
        total: usize,
        day: i64,
    ) -> Result<Outcome, RecallError> {
        if total == 0 {
            return Err(RecallError::NoPoints);
        }
        if recalled > total {
            return Err(RecallError::TooManyTicked);
        }
        let passed = recalled * 100 >= self.pass_mark as usize * total;
        let score = ((recalled * 100) as f64 / total as f64).round() as u8;
        let to = match from.map(|position| self.clamp(position)) {
            None => Position { step: 0, rep: 1 },
            Some(position) if passed => self.up(position),
            Some(position) => match self.under_pass {
                UnderPass::BackOneStep => self.back(position),
                UnderPass::Stay => position,
            },
        };
        Ok(Outcome {
            score,
            passed,
            to,
            next_due: add(day, self.steps[to.step].every),
        })
    }
}

/// `day` plus an interval, with months and years on the calendar (31 Jan + 1 month is 28 Feb).
pub fn add(day: i64, interval: Interval) -> i64 {
    use chrono::{Months, NaiveDate, TimeDelta};
    let count = interval.count;
    // Days and weeks are plain sums. Nothing here may panic, whatever the day: a clock
    // reset to before 1970 gives a negative one, and a damaged file can hold any number.
    let months = match interval.unit {
        Unit::Days => return day.saturating_add(i64::from(count)),
        Unit::Weeks => return day.saturating_add(7 * i64::from(count)),
        Unit::Months => count,
        Unit::Years => count.saturating_mul(12),
    };
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date");
    TimeDelta::try_days(day)
        .and_then(|since| epoch.checked_add_signed(since))
        .and_then(|date| date.checked_add_months(Months::new(months)))
        .map(|next| (next - epoch).num_days())
        // Further than any calendar goes: count a month as 30 days.
        .unwrap_or_else(|| day.saturating_add(30 * i64::from(months)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn day(y: i32, m: u32, d: u32) -> i64 {
        (NaiveDate::from_ymd_opt(y, m, d).unwrap() - NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days()
    }
    const TUE_3_NOV: (i32, u32, u32) = (2026, 11, 3);
    fn today() -> i64 {
        day(TUE_3_NOV.0, TUE_3_NOV.1, TUE_3_NOV.2)
    }
    fn at(step: usize, rep: u32) -> Option<Position> {
        Some(Position { step, rep })
    }

    // free-recall.feature: The step moves up at the pass mark and back under it (outline)
    #[test]
    fn up_at_the_pass_mark_and_back_under_it() {
        let schedule = Schedule::default();
        let month = at(3, 1);
        let up = schedule.check(month, 7, 10, today()).unwrap();
        assert_eq!(
            (up.score, up.passed, schedule.label(up.to)),
            (70, true, "Monthly 1 of 6".into())
        );
        assert_eq!(up.next_due, day(2026, 12, 3));

        let back = schedule.check(month, 6, 10, today()).unwrap();
        assert_eq!(
            (back.score, back.passed, schedule.label(back.to)),
            (60, false, "Fortnight".into())
        );
        assert_eq!(back.next_due, day(2026, 11, 17));

        let seasonal = schedule.check(at(4, 6), 9, 10, today()).unwrap();
        assert_eq!(schedule.label(seasonal.to), "Seasonal 1 of 4");
        assert_eq!(seasonal.next_due, day(2027, 2, 3));

        let yearly = schedule.check(at(6, 1), 8, 10, today()).unwrap();
        assert_eq!(schedule.label(yearly.to), "Yearly");
        assert_eq!(yearly.next_due, day(2027, 11, 3));

        let stays_on_day = schedule.check(at(0, 1), 2, 10, today()).unwrap();
        assert_eq!(schedule.label(stays_on_day.to), "Day");
        assert_eq!(stays_on_day.next_due, day(2026, 11, 4));
    }

    #[test]
    fn repeats_climb_within_a_step_and_back_steps_into_the_previous_one() {
        let schedule = Schedule::default();
        assert_eq!(
            schedule.label(schedule.check(at(4, 2), 10, 10, today()).unwrap().to),
            "Monthly 3 of 6"
        );
        assert_eq!(
            schedule.label(schedule.check(at(4, 2), 0, 10, today()).unwrap().to),
            "Monthly 1 of 6"
        );
        // Back from the first seasonal check lands on the last monthly one
        assert_eq!(
            schedule.label(schedule.check(at(5, 1), 0, 10, today()).unwrap().to),
            "Monthly 6 of 6"
        );
    }

    // free-recall.feature: The first check puts a new checklist on Day
    #[test]
    fn first_check_puts_a_new_checklist_on_day_whatever_the_score() {
        let schedule = Schedule::default();
        for recalled in [0, 3, 10] {
            let outcome = schedule.check(None, recalled, 10, today()).unwrap();
            assert_eq!(schedule.label(outcome.to), "Day");
            assert_eq!(outcome.next_due, today() + 1);
        }
    }

    // free-recall.feature: A late check counts from the day it was done
    #[test]
    fn a_late_check_counts_from_the_day_it_was_done() {
        let outcome = Schedule::default()
            .check(at(1, 1), 10, 10, today())
            .unwrap();
        assert_eq!(outcome.next_due, day(2026, 11, 17));
    }

    #[test]
    fn passing_uses_the_exact_ratio_not_the_rounded_score() {
        // 8 of 12 is 66.7%, shown as 67, and still under 70
        let outcome = Schedule::default().check(at(3, 1), 8, 12, today()).unwrap();
        assert_eq!((outcome.score, outcome.passed), (67, false));
    }

    #[test]
    fn stay_on_the_same_step_when_set_to() {
        let schedule = Schedule {
            under_pass: UnderPass::Stay,
            ..Schedule::default()
        };
        assert_eq!(
            schedule.label(schedule.check(at(3, 1), 0, 10, today()).unwrap().to),
            "Month"
        );
    }

    // free-recall.feature: A checklist needs a point before it can start
    #[test]
    fn needs_points_and_sensible_ticks() {
        let schedule = Schedule::default();
        assert_eq!(
            schedule.check(None, 0, 0, today()),
            Err(RecallError::NoPoints)
        );
        assert_eq!(
            schedule.check(None, 11, 10, today()),
            Err(RecallError::TooManyTicked)
        );
    }

    // free-recall.feature: Edit and reset the schedule
    #[test]
    fn edited_schedules_are_checked_and_old_positions_fit_them() {
        assert_eq!(Schedule::default().validate(), Ok(()));
        let mut edited = Schedule {
            pass_mark: 80,
            ..Schedule::default()
        };
        edited.steps[4].repeats = 3;
        assert_eq!(edited.validate(), Ok(()));
        assert_eq!(edited.label(Position { step: 4, rep: 5 }), "Monthly 3 of 3");
        assert!(
            !edited.check(at(3, 1), 7, 10, today()).unwrap().passed,
            "70% is under an 80% pass mark"
        );

        let short = Schedule {
            steps: edited.steps[..2].to_vec(),
            ..Schedule::default()
        };
        assert_eq!(
            short.validate(),
            Ok(()),
            "a ladder without a forever step is fine"
        );
        assert_eq!(
            short.clamp(Position { step: 6, rep: 1 }),
            Position { step: 1, rep: 1 }
        );

        let bad = |change: fn(&mut Schedule)| {
            let mut s = Schedule::default();
            change(&mut s);
            s.validate()
        };
        assert_eq!(bad(|s| s.steps.clear()), Err(RecallError::NoSteps));
        assert_eq!(bad(|s| s.pass_mark = 0), Err(RecallError::PassMark));
        assert_eq!(
            bad(|s| s.steps[1].every.count = 0),
            Err(RecallError::ZeroInterval)
        );
        assert_eq!(
            bad(|s| s.steps[1].repeats = 0),
            Err(RecallError::ForeverBeforeLast)
        );
        assert_eq!(
            bad(|s| s.steps[1].name = " ".into()),
            Err(RecallError::UnnamedStep)
        );
    }

    #[test]
    fn after_pass_says_what_comes_next() {
        let schedule = Schedule::default();
        assert_eq!(schedule.label(schedule.after_pass(None)), "Day");
        assert_eq!(
            schedule.label(schedule.after_pass(at(3, 1))),
            "Monthly 1 of 6"
        );
        assert_eq!(schedule.label(schedule.after_pass(at(6, 1))), "Yearly");
    }

    #[test]
    fn months_land_on_the_calendar() {
        assert_eq!(
            add(
                day(2027, 1, 31),
                Interval {
                    count: 1,
                    unit: Unit::Months
                }
            ),
            day(2027, 2, 28)
        );
    }
}

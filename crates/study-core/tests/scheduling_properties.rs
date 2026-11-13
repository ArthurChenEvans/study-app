//! Properties of the schedulers (hazard 11): the recall and quiz ladders, FSRS card
//! scheduling, tightening before a date, and drills. Each holds for whatever inputs
//! proptest invents, including the awkward ones: a review done days early or late, the
//! computer's clock set back, midnight passing, and day numbers no calendar has (which
//! a damaged import or sync file can carry).

use proptest::prelude::*;
use study_core::cards::{CardSettings, Rating, Schedule as CardSchedule};
use study_core::dates::{Tightening, tightened};
use study_core::practice::DrillRating;
use study_core::recall::{Interval, Position, Schedule, Step, UnderPass, Unit, add};

fn unit() -> impl Strategy<Value = Unit> {
    prop_oneof![Just(Unit::Days), Just(Unit::Weeks), Just(Unit::Months), Just(Unit::Years)]
}

/// Any schedule the settings screen would accept.
fn ladder() -> impl Strategy<Value = Schedule> {
    (prop::collection::vec((1u32..400, unit(), 1u32..5), 1..8), 0u32..5, 1u8..=100, any::<bool>()).prop_map(|(steps, last_repeats, pass_mark, back)| {
        let count = steps.len();
        let steps = steps.into_iter().enumerate().map(|(index, (every, unit, repeats))| Step { name: format!("step {index}"), every: Interval { count: every, unit }, repeats: if index + 1 == count { last_repeats } else { repeats } }).collect();
        Schedule { steps, pass_mark, under_pass: if back { UnderPass::BackOneStep } else { UnderPass::Stay } }
    })
}

/// A day anyone could really be on: 1860 to 2500, so before 1970 is included.
fn real_day() -> impl Strategy<Value = i64> {
    -40_000i64..194_000
}

fn order(position: Position) -> (usize, u32) {
    (position.step, position.rep)
}

// Past the last day a calendar can name (a number only a damaged file would hold), a month
// counts as 30 days and a year as 12 of them.
#[test]
fn beyond_any_calendar_a_month_counts_as_thirty_days() {
    let far = 200_000_000_000;
    assert_eq!(add(far, Interval { count: 2, unit: Unit::Months }), far + 60);
    assert_eq!(add(far, Interval { count: 1, unit: Unit::Years }), far + 360);
    assert_eq!(add(-far, Interval { count: 3, unit: Unit::Months }), -far + 90);
    // And on real days months are real months: 31 January plus a month is the end of February.
    assert_eq!(add(19_753, Interval { count: 1, unit: Unit::Months }), 19_782);
    assert_eq!(add(-1, Interval { count: 1, unit: Unit::Months }), 30, "31 December 1969 plus a month is 31 January 1970");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn adding_an_interval_never_panics_and_never_goes_backwards(day in any::<i64>(), count in any::<u32>(), unit in unit()) {
        let next = add(day, Interval { count, unit });
        prop_assert!(next >= day || day > i64::MAX - 5_000_000_000_000, "{day} + {count} {unit:?} = {next}");
    }

    #[test]
    fn a_longer_interval_is_never_due_sooner(day in real_day(), count in 1u32..5000, unit in unit()) {
        let shorter = add(day, Interval { count, unit });
        let longer = add(day, Interval { count: count + 1, unit });
        prop_assert!(day < shorter && shorter < longer, "{day}: {shorter} then {longer}");
    }

    // R57: the next date counts from the day the check was actually done, whenever that
    // was, and a miss goes back at most one step.
    #[test]
    fn a_ladder_always_moves_one_step_at_most_and_schedules_after_the_day_done(
        schedule in ladder(),
        checks in prop::collection::vec((0usize..=20, 1usize..=20, -30i64..400), 1..40),
        start in real_day(),
    ) {
        prop_assume!(schedule.validate().is_ok());
        let mut position: Option<Position> = None;
        let mut day = start;
        for (recalled, total, gap) in checks {
            // The gap can be negative: a check done before the last one's day, as when
            // the clock was wrong and then corrected.
            day += gap;
            let recalled = recalled.min(total);
            let outcome = schedule.check(position, recalled, total, day).unwrap();
            prop_assert!(outcome.next_due > day, "due {} isn't after the day it was done, {day}", outcome.next_due);
            prop_assert!(outcome.score <= 100);
            prop_assert_eq!(outcome.passed, recalled * 100 >= schedule.pass_mark as usize * total);
            prop_assert!(outcome.to.step < schedule.steps.len());
            let last = &schedule.steps[outcome.to.step];
            prop_assert!(outcome.to.rep >= 1 && (last.repeats == 0 || outcome.to.rep <= last.repeats), "{:?} isn't a place on {:?}", outcome.to, last);
            prop_assert_eq!(outcome.next_due, add(day, last.every));
            if let Some(from) = position.map(|position| schedule.clamp(position)) {
                if outcome.passed {
                    prop_assert!(order(outcome.to) >= order(from), "a pass went back: {from:?} to {:?}", outcome.to);
                    prop_assert!(outcome.to.step <= from.step + 1, "a pass skipped a step");
                } else {
                    prop_assert!(order(outcome.to) <= order(from), "a miss went forward: {from:?} to {:?}", outcome.to);
                    prop_assert!(from.step - outcome.to.step <= 1, "a miss went back more than one step");
                    if schedule.under_pass == UnderPass::Stay {
                        prop_assert_eq!(outcome.to, from);
                    }
                }
            } else {
                prop_assert_eq!(outcome.to, Position { step: 0, rep: 1 });
            }
            position = Some(outcome.to);
        }
    }

    // A position saved under an older, longer schedule always fits the current one.
    #[test]
    fn any_saved_position_fits_any_schedule(schedule in ladder(), step in 0usize..50, rep in 0u32..50, recalled in 0usize..=5, day in real_day()) {
        prop_assume!(schedule.validate().is_ok());
        let fitted = schedule.clamp(Position { step, rep });
        prop_assert!(fitted.step < schedule.steps.len() && fitted.rep >= 1);
        let checked = schedule.check(Some(Position { step, rep }), recalled, 5, day);
        prop_assert!(checked.is_ok());
        let _ = schedule.label(Position { step, rep });
    }

    // R66, R74: whatever the answers and whenever they're given, a card comes back on or
    // after the day it was answered, the four buttons stay in order, and its memory
    // state stays a number.
    #[test]
    fn a_card_is_never_due_before_the_day_it_was_answered(
        answers in prop::collection::vec((0u8..4, -30i64..400), 1..40),
        start in real_day(),
        retention in 0.70f32..=0.99,
    ) {
        let settings = CardSettings { desired_retention: retention, ..CardSettings::default() };
        prop_assume!(settings.validate().is_ok());
        let mut day = start;
        let mut card = CardSchedule::new(day);
        for (answer, gap) in answers {
            day += gap; // negative when the clock went back
            let rating = [Rating::Again, Rating::Hard, Rating::Good, Rating::Easy][answer as usize];
            let [again, hard, good, easy] = settings.preview(&card, day).unwrap();
            prop_assert!(again == 0 && 1 <= hard && hard <= good && good < easy, "buttons out of order: {again} {hard} {good} {easy}");
            let before = card;
            card = settings.grade(&card, rating, day).unwrap();
            prop_assert!(card.due >= day, "due {} before the day answered {day}", card.due);
            prop_assert_eq!(card.due == day, rating == Rating::Again);
            prop_assert_eq!(card.last_review, Some(day));
            prop_assert_eq!(card.reps, before.reps + 1);
            prop_assert!(card.lapses >= before.lapses && card.lapses <= before.lapses + 1);
            let memory = card.memory.unwrap();
            prop_assert!(memory.stability.is_finite() && memory.stability > 0.0 && memory.difficulty.is_finite(), "{memory:?}");
            if let Some(chance) = study_core::cards::retrievability(&settings, &card, day) {
                prop_assert!((0.0..=1.0).contains(&chance), "{chance}");
            }
        }
    }

    // R132, R133: tightening only ever pulls a review earlier, never past half the days
    // that were left when it was last done, and stops the day after the date.
    #[test]
    fn tightening_only_pulls_earlier_and_never_past_half_the_days_left(
        last in real_day(),
        normal_after in 0i64..2000,
        today_after in -10i64..2000,
        dates in prop::collection::vec(-50i64..2500, 0..6),
    ) {
        let normal = last + normal_after;
        let today = last + today_after;
        let dates: Vec<Tightening<usize>> = dates.iter().enumerate().map(|(index, after)| Tightening { day: last + after, date: index }).collect();
        let due = tightened(Some(last), normal, today, dates.clone());
        prop_assert!(due.day <= normal, "tightening pushed it later: {} after {normal}", due.day);
        match due.tightened_by {
            None => prop_assert_eq!(due.day, normal),
            Some(index) => {
                let date = dates[index].day;
                prop_assert!(today <= date, "a date that has passed still tightens");
                prop_assert!(due.day > last && due.day < normal);
                prop_assert!(due.day - last <= (date - last) / 2, "a gap of {} with {} days left", due.day - last, date - last);
                prop_assert!(due.day <= date);
            }
        }
        // No other date would have pulled it in further.
        for other in &dates {
            let alone = tightened(Some(last), normal, today, [*other]);
            prop_assert!(due.day <= alone.day);
        }
        // Something never done has nothing to count from.
        prop_assert_eq!(tightened(None, normal, today, dates).day, normal);
    }

    // Day numbers from a damaged file can be anything. Working out a due day from them
    // may give a silly answer, but it must not panic.
    #[test]
    fn no_day_number_makes_tightening_or_a_drill_panic(last in any::<i64>(), normal in any::<i64>(), today in any::<i64>(), date in any::<i64>(), day in any::<i64>()) {
        let due = tightened(Some(last), normal, today, [Tightening { day: date, date: () }]);
        prop_assert!(due.day <= normal);
        prop_assert_eq!(DrillRating::Solid.next_due(day), None);
        for rating in [DrillRating::Struggled, DrillRating::Shaky] {
            let next = rating.next_due(day).unwrap();
            prop_assert!(next > day || day == i64::MAX);
        }
    }

    #[test]
    fn no_day_number_makes_card_scheduling_panic(due in any::<i64>(), last in any::<i64>(), today in any::<i64>(), answer in 0u8..4) {
        let settings = CardSettings::default();
        let card = CardSchedule { memory: Some(study_core::cards::Memory { stability: 3.0, difficulty: 5.0 }), due, last_review: Some(last), reps: 3, lapses: 0 };
        let rating = [Rating::Again, Rating::Hard, Rating::Good, Rating::Easy][answer as usize];
        let _ = settings.preview(&card, today);
        if let Ok(next) = settings.grade(&card, rating, today) {
            prop_assert!(next.due >= today);
        }
        let _ = study_core::cards::retrievability(&settings, &card, today);
        let _ = card.is_due(today);
        let _ = card.is_learning(today);
    }
}

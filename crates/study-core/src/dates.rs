//! Dates on the calendar (R130 to R137): exams, deadlines and anything else with a name
//! and a day, and how a date that tightens reviews pulls in what it covers.
//!
//! Tightening is worked out each time, never stored (R133). An item's due day becomes the
//! earlier of its normal one and "last done + half the days that were left then". So
//! editing or deleting a date puts everything back, and from the day after the date the
//! normal schedule is used again.

/// A date that tightens reviews, as far as one item is concerned: its day, and something
/// to say which date it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tightening<T> {
    pub day: i64,
    pub date: T,
}

/// Where an item is due once the dates covering it are taken into account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Due<T> {
    pub day: i64,
    /// The date that pulled it in, if one did.
    pub tightened_by: Option<T>,
}

/// The day an item is pulled in to by one date, if it is. `last` is the day it was last
/// done (its schedule was set), `normal` its due day.
fn pulled_in(last: i64, normal: i64, today: i64, date: i64) -> Option<i64> {
    // From the day after, things go back to normal.
    if today > date {
        return None;
    }
    // The day before (or on) the date there's nothing left to halve.
    let left = date - last;
    if left < 2 {
        return None;
    }
    let day = last + left / 2;
    (day < normal).then_some(day)
}

/// The item's due day with every date covering it applied. The tighter date wins, and on
/// a tie the earlier date (R132).
pub fn tightened<T: Copy>(last: Option<i64>, normal: i64, today: i64, dates: impl IntoIterator<Item = Tightening<T>>) -> Due<T> {
    let best = last.and_then(|last| {
        dates
            .into_iter()
            .filter_map(|tightening| pulled_in(last, normal, today, tightening.day).map(|day| (day, tightening.day, tightening.date)))
            .min_by_key(|(day, date_day, _)| (*day, *date_day))
    });
    match best {
        Some((day, _, date)) => Due { day, tightened_by: Some(date) },
        None => Due { day: normal, tightened_by: None },
    }
}

/// The kinds suggested when adding a date (R130). Anything else is fine too.
pub const SUGGESTED_KINDS: [&str; 4] = ["Exam", "Deadline", "Coursework", "Mock exam"];

/// Whether a new date of this kind tightens reviews unless told otherwise (R132).
pub fn tightens_by_default(kind: &str) -> bool {
    kind.trim().eq_ignore_ascii_case("exam")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tue 3 Nov is day 3, Mon 23 Nov day 23, and so on.
    const EXAM: Tightening<&str> = Tightening { day: 23, date: "Networks exam" };

    fn due(last: Option<i64>, normal: i64, today: i64, dates: &[Tightening<&'static str>]) -> (i64, Option<&'static str>) {
        let due = tightened(last, normal, today, dates.iter().copied());
        (due.day, due.tightened_by)
    }

    #[test]
    fn pulls_in_to_half_the_days_that_were_left() {
        // Quizzed on the 3rd, normally back on the 28th: 20 days left, so the 13th.
        assert_eq!(due(Some(3), 28, 3, &[EXAM]), (13, Some("Networks exam")));
    }

    #[test]
    fn leaves_alone_what_is_already_soon_enough() {
        assert_eq!(due(Some(3), 5, 3, &[EXAM]), (5, None));
        assert_eq!(due(Some(3), 13, 3, &[EXAM]), (13, None));
    }

    #[test]
    fn the_day_before_there_is_nothing_left_to_halve() {
        assert_eq!(due(Some(22), 36, 22, &[EXAM]), (36, None));
        assert_eq!(due(Some(23), 37, 23, &[EXAM]), (37, None));
        // Two days before still halves to the day before.
        assert_eq!(due(Some(21), 35, 21, &[EXAM]), (22, Some("Networks exam")));
    }

    #[test]
    fn something_done_long_ago_comes_up_straight_away() {
        // Last quizzed on 5 Oct (day -26), normally back on 5 Dec: half of 49 days is 24,
        // so it was due on 29 Oct, which is overdue on the 3rd.
        assert_eq!(due(Some(-26), 35, 3, &[EXAM]), (-2, Some("Networks exam")));
    }

    #[test]
    fn after_the_date_things_go_back_to_normal() {
        assert_eq!(due(Some(3), 28, 23, &[EXAM]), (13, Some("Networks exam")));
        assert_eq!(due(Some(3), 28, 24, &[EXAM]), (28, None));
    }

    #[test]
    fn never_done_has_nothing_to_tighten() {
        assert_eq!(due(None, 28, 3, &[EXAM]), (28, None));
    }

    #[test]
    fn the_tighter_date_wins() {
        let mock = Tightening { day: 13, date: "Networks mock" };
        assert_eq!(due(Some(3), 28, 3, &[EXAM, mock]), (8, Some("Networks mock")));
        // Once the mock has passed, the exam still applies.
        assert_eq!(due(Some(3), 28, 14, &[EXAM, mock]), (13, Some("Networks exam")));
    }

    #[test]
    fn exams_tighten_by_default() {
        assert!(tightens_by_default("Exam"));
        assert!(tightens_by_default(" exam "));
        assert!(!tightens_by_default("Mock exam"));
        assert!(!tightens_by_default(""));
    }
}

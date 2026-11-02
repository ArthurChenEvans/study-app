//! The overview screen (R33): progress worked out from saved attempts only.

use std::collections::BTreeMap;

use crate::question::TopicId;
use crate::results::{Summary, Tally};

/// A saved attempt as the overview needs it. `day` is a day number (days since an epoch),
/// so the core needs no calendar library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRecord {
    pub day: i64,
    /// Order the attempt was saved in (its id). Breaks ties between attempts on the same day.
    pub sequence: u64,
    pub retry: bool,
    pub summary: Summary,
    /// One label per wrong answer that has a reason (see `results::reason_labels`).
    pub reason_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScorePoint {
    pub day: i64,
    pub percent: u32,
    pub retry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// % right over the last 7 days (today and the 6 before). None if no quizzes then.
    pub accuracy: Option<u32>,
    /// Change in points from the 7 days before. None unless both weeks have quizzes.
    pub change: Option<i32>,
    pub answered_last_7_days: usize,
    pub quizzes_last_7_days: usize,
    /// % of right answers (last 7 days) that were flagged as guessed.
    pub guessed_share: Option<u32>,
    /// Every attempt in date order.
    pub scores: Vec<ScorePoint>,
    /// Each quizzed topic's latest result, weakest first.
    pub weakest_topics: Vec<(TopicId, Tally)>,
    pub never_quizzed: Vec<TopicId>,
    /// Reasons on wrong answers in the last 30 days, most common first.
    pub reasons: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overview {
    /// No attempts yet: show first steps instead of charts.
    FirstRun,
    Progress(Progress),
}

pub fn percent(part: usize, whole: usize) -> u32 {
    if whole == 0 {
        return 0;
    }
    ((part * 200 + whole) / (whole * 2)) as u32
}

pub fn overview(records: &[AttemptRecord], today: i64, topics: &[TopicId]) -> Overview {
    if records.is_empty() {
        return Overview::FirstRun;
    }
    let mut ordered: Vec<&AttemptRecord> = records.iter().collect();
    ordered.sort_by_key(|record| (record.day, record.sequence));

    let within = |from: i64, to: i64| {
        ordered
            .iter()
            .copied()
            .filter(move |record| record.day >= from && record.day <= to)
    };
    let sum = |from: i64, to: i64| {
        within(from, to).fold((0, 0, 0, 0), |(right, total, guessed, count), record| {
            let t = &record.summary.overall;
            (
                right + t.right(),
                total + t.total(),
                guessed + t.right_guessed,
                count + 1,
            )
        })
    };
    let (right_now, total_now, guessed_now, quizzes_now) = sum(today - 6, today);
    let (right_before, total_before, _, quizzes_before) = sum(today - 13, today - 7);
    let accuracy = (quizzes_now > 0).then(|| percent(right_now, total_now));
    let change = (quizzes_now > 0 && quizzes_before > 0)
        .then(|| percent(right_now, total_now) as i32 - percent(right_before, total_before) as i32);

    let scores = ordered
        .iter()
        .map(|record| ScorePoint {
            day: record.day,
            percent: percent(
                record.summary.overall.right(),
                record.summary.overall.total(),
            ),
            retry: record.retry,
        })
        .collect();

    let mut latest: BTreeMap<TopicId, Tally> = BTreeMap::new();
    for record in &ordered {
        for (topic, tally) in &record.summary.per_topic {
            latest.insert(*topic, tally.clone());
        }
    }
    let mut weakest_topics: Vec<(TopicId, Tally)> = latest.into_iter().collect();
    weakest_topics.sort_by(|(_, a), (_, b)| (a.right() * b.total()).cmp(&(b.right() * a.total())));
    let never_quizzed = topics
        .iter()
        .copied()
        .filter(|topic| !weakest_topics.iter().any(|(quizzed, _)| quizzed == topic))
        .collect();

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for record in within(today - 29, today) {
        for label in &record.reason_labels {
            *counts.entry(label.clone()).or_default() += 1;
        }
    }
    let mut reasons: Vec<(String, usize)> = counts.into_iter().collect();
    reasons.sort_by(|(a_label, a), (b_label, b)| b.cmp(a).then_with(|| a_label.cmp(b_label)));

    Overview::Progress(Progress {
        accuracy,
        change,
        answered_last_7_days: total_now,
        quizzes_last_7_days: quizzes_now,
        guessed_share: (quizzes_now > 0).then(|| percent(guessed_now, right_now)),
        scores,
        weakest_topics,
        never_quizzed,
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CPU: TopicId = TopicId(1);
    const GPU: TopicId = TopicId(2);
    const OSI: TopicId = TopicId(3);
    const MISC: TopicId = TopicId(4);
    const TODAY: i64 = 100;

    fn tally(right_sure: usize, right_guessed: usize, wrong: usize) -> Tally {
        Tally {
            right_sure,
            right_guessed,
            wrong,
            wrong_flagged: 0,
            unanswered: 0,
        }
    }

    fn record(
        sequence: u64,
        day: i64,
        retry: bool,
        topics: &[(TopicId, Tally)],
        reasons: &[&str],
    ) -> AttemptRecord {
        let mut overall = Tally::default();
        for (_, t) in topics {
            overall.right_sure += t.right_sure;
            overall.right_guessed += t.right_guessed;
            overall.wrong += t.wrong;
        }
        AttemptRecord {
            day,
            sequence,
            retry,
            summary: Summary {
                overall,
                per_topic: topics.iter().cloned().collect(),
            },
            reason_labels: reasons.iter().map(|r| r.to_string()).collect(),
        }
    }

    fn progress(records: &[AttemptRecord]) -> Progress {
        match overview(records, TODAY, &[CPU, GPU, OSI, MISC]) {
            Overview::Progress(progress) => progress,
            Overview::FirstRun => panic!("expected progress"),
        }
    }

    fn sample() -> Vec<AttemptRecord> {
        vec![
            // 7 days before: 12 / 20 right
            record(
                1,
                TODAY - 10,
                false,
                &[(CPU, tally(6, 2, 2)), (GPU, tally(4, 0, 6))],
                &["Blank"],
            ),
            // last 7 days: OSI 4 / 15, CPU+GPU 16 / 20 (3 guessed), retry 6 / 7
            record(
                2,
                TODAY - 4,
                false,
                &[(OSI, tally(3, 1, 11))],
                &["Misremembered", "Blank"],
            ),
            record(
                3,
                TODAY,
                false,
                &[(CPU, tally(8, 1, 1)), (GPU, tally(5, 2, 3))],
                &["Confused with something similar", "Blank"],
            ),
            record(
                4,
                TODAY,
                true,
                &[(CPU, tally(1, 0, 0)), (GPU, tally(4, 1, 1))],
                &[],
            ),
        ]
    }

    #[test]
    fn rounds_to_the_nearest_percent() {
        assert_eq!(percent(16, 20), 80);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(2, 3), 67);
        assert_eq!(percent(0, 0), 0);
    }

    // overview.feature: First run
    #[test]
    fn first_run_has_no_progress() {
        assert_eq!(overview(&[], TODAY, &[CPU]), Overview::FirstRun);
    }

    // overview.feature: Progress summary
    #[test]
    fn summary_of_the_last_seven_days() {
        let p = progress(&sample());
        // 4 + 16 + 6 right of 15 + 20 + 7 = 26 / 42
        assert_eq!(p.accuracy, Some(62));
        assert_eq!(p.answered_last_7_days, 42);
        assert_eq!(p.quizzes_last_7_days, 3);
        // guessed 1 + 3 + 1 = 5 of 26 right
        assert_eq!(p.guessed_share, Some(19));
    }

    // overview.feature: Change from the previous week
    #[test]
    fn change_from_the_week_before() {
        assert_eq!(progress(&sample()).change, Some(62 - 60));
        let only_this_week = &sample()[1..];
        assert_eq!(progress(only_this_week).change, None);
    }

    // overview.feature: Score per quiz / Retries on the chart
    #[test]
    fn scores_in_date_order_with_retries_marked() {
        let mut records = sample();
        records.reverse();
        let scores = progress(&records).scores;
        assert_eq!(
            scores,
            vec![
                ScorePoint {
                    day: TODAY - 10,
                    percent: 60,
                    retry: false
                },
                ScorePoint {
                    day: TODAY - 4,
                    percent: 27,
                    retry: false
                },
                ScorePoint {
                    day: TODAY,
                    percent: 80,
                    retry: false
                },
                ScorePoint {
                    day: TODAY,
                    percent: 86,
                    retry: true
                },
            ]
        );
    }

    // overview.feature: Weakest topics first / Topics never quizzed
    #[test]
    fn topics_ranked_by_their_latest_result() {
        let p = progress(&sample());
        let order: Vec<TopicId> = p.weakest_topics.iter().map(|(topic, _)| *topic).collect();
        // latest: OSI 4/15, GPU 5/6 (retry), CPU 1/1 (retry)
        assert_eq!(order, vec![OSI, GPU, CPU]);
        assert_eq!(p.weakest_topics[0].1, tally(3, 1, 11));
        assert_eq!(p.never_quizzed, vec![MISC]);
    }

    // overview.feature: Reasons ranked
    #[test]
    fn reasons_from_the_last_30_days_most_common_first() {
        let mut records = sample();
        records.push(record(
            0,
            TODAY - 40,
            false,
            &[(CPU, tally(0, 0, 1))],
            &["Careless slip"],
        ));
        let reasons = progress(&records).reasons;
        assert_eq!(
            reasons,
            vec![
                ("Blank".to_string(), 3),
                ("Confused with something similar".to_string(), 1),
                ("Misremembered".to_string(), 1)
            ]
        );
    }
}

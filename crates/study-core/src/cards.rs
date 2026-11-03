//! Flashcards (R62 to R70, R77). A note makes one or more cards, each scheduled with FSRS.
//! Days are numbered from 1970-01-01, as elsewhere in the core.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::question::TopicId;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Basic,
    Reversed,
    Cloze,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub topic: TopicId,
    pub kind: NoteKind,
    /// Basic and Reversed.
    pub front: String,
    pub back: String,
    /// Basic only: more cues for the same back (R63).
    pub cues: Vec<String>,
    /// Cloze only.
    pub text: String,
    /// Cloze only: shown under the answer.
    pub extra: String,
}

/// One card's two sides, as Markdown. `key` names the card within its note, so editing
/// a note keeps the schedule of every card that still exists (R77).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub key: String,
    pub front: String,
    pub back: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum CardError {
    #[error("write the front of the card")]
    NoFront,
    #[error("write the back of the card")]
    NoBack,
    #[error("make at least one gap, like {{{{c1::answer}}}}")]
    NoGaps,
    #[error("a cue can't be empty")]
    EmptyCue,
}

struct Gap {
    number: u32,
    start: usize,
    end: usize,
    answer: String,
    hint: Option<String>,
}

/// Finds `{{cN::answer}}` and `{{cN::answer::hint}}`.
fn gaps(text: &str) -> Vec<Gap> {
    let mut found = vec![];
    let mut from = 0;
    while let Some(open) = text[from..].find("{{c").map(|i| i + from) {
        let Some(close) = text[open..].find("}}").map(|i| i + open) else {
            break;
        };
        let inner = &text[open + 3..close];
        let mut parts = inner.splitn(3, "::");
        let number = parts.next().and_then(|n| n.parse::<u32>().ok());
        let answer = parts.next();
        match (number, answer) {
            (Some(number), Some(answer)) if number > 0 && !answer.trim().is_empty() => {
                let hint = parts
                    .next()
                    .map(str::trim)
                    .filter(|h| !h.is_empty())
                    .map(String::from);
                found.push(Gap {
                    number,
                    start: open,
                    end: close + 2,
                    answer: answer.to_string(),
                    hint,
                });
            }
            _ => {}
        }
        from = close + 2;
    }
    found
}

/// The text with each gap replaced by `pick(gap)`.
fn fill(text: &str, gaps: &[Gap], pick: impl Fn(&Gap) -> String) -> String {
    let mut out = String::new();
    let mut at = 0;
    for gap in gaps {
        out.push_str(&text[at..gap.start]);
        out.push_str(&pick(gap));
        at = gap.end;
    }
    out.push_str(&text[at..]);
    out
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl Note {
    /// The cards this note makes (R62, R63).
    pub fn faces(&self) -> Result<Vec<Face>, CardError> {
        let need = |front: &str, back: &str| {
            if front.trim().is_empty() {
                return Err(CardError::NoFront);
            }
            if back.trim().is_empty() {
                return Err(CardError::NoBack);
            }
            Ok(())
        };
        match self.kind {
            NoteKind::Basic => {
                need(&self.front, &self.back)?;
                let mut faces = vec![Face {
                    key: "main".into(),
                    front: self.front.trim().into(),
                    back: self.back.trim().into(),
                }];
                for (index, cue) in self.cues.iter().enumerate() {
                    if cue.trim().is_empty() {
                        return Err(CardError::EmptyCue);
                    }
                    faces.push(Face {
                        key: format!("cue:{index}"),
                        front: cue.trim().into(),
                        back: self.back.trim().into(),
                    });
                }
                Ok(faces)
            }
            NoteKind::Reversed => {
                need(&self.front, &self.back)?;
                Ok(vec![
                    Face {
                        key: "forward".into(),
                        front: self.front.trim().into(),
                        back: self.back.trim().into(),
                    },
                    Face {
                        key: "reverse".into(),
                        front: self.back.trim().into(),
                        back: self.front.trim().into(),
                    },
                ])
            }
            NoteKind::Cloze => {
                let text = self.text.trim();
                let found = gaps(text);
                if found.is_empty() {
                    return Err(CardError::NoGaps);
                }
                let mut numbers: Vec<u32> = found.iter().map(|g| g.number).collect();
                numbers.sort_unstable();
                numbers.dedup();
                Ok(numbers
                    .into_iter()
                    .map(|number| {
                        let front = fill(text, &found, |g| {
                            if g.number == number {
                                format!(
                                    "<span class=\"cloze-gap\">[{}]</span>",
                                    escape(g.hint.as_deref().unwrap_or("..."))
                                )
                            } else {
                                g.answer.clone()
                            }
                        });
                        let mut back = fill(text, &found, |g| {
                            if g.number == number {
                                format!("<span class=\"cloze-answer\">{}</span>", escape(&g.answer))
                            } else {
                                g.answer.clone()
                            }
                        });
                        if !self.extra.trim().is_empty() {
                            back.push_str("\n\n");
                            back.push_str(self.extra.trim());
                        }
                        Face {
                            key: format!("c{number}"),
                            front,
                            back,
                        }
                    })
                    .collect())
            }
        }
    }
}

// ---------- Scheduling (R65, R66, R70) ----------

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Memory {
    pub stability: f32,
    pub difficulty: f32,
}

/// Where a card stands. `memory` is None for a card that's never been reviewed.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Schedule {
    pub memory: Option<Memory>,
    pub due: i64,
    pub last_review: Option<i64>,
    pub reps: u32,
    /// Times Again was pressed on a card that had been learned. Cleared by editing it (R70).
    pub lapses: u32,
}

impl Schedule {
    pub fn new(today: i64) -> Self {
        Self {
            memory: None,
            due: today,
            last_review: None,
            reps: 0,
            lapses: 0,
        }
    }

    pub fn is_new(&self) -> bool {
        self.memory.is_none()
    }

    /// Answered Again earlier today, so it's waiting in today's learning queue.
    pub fn is_learning(&self, today: i64) -> bool {
        self.memory.is_some() && self.last_review == Some(today) && self.due <= today
    }

    pub fn is_due(&self, today: i64) -> bool {
        self.due <= today
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CardSettings {
    /// Chance of still remembering a card when it comes back (R74).
    pub desired_retention: f32,
    pub new_per_day: u32,
    /// None means no limit.
    pub max_reviews: Option<u32>,
    pub leech_after: u32,
    /// FSRS parameters from "Fit the scheduler to you". Empty means the defaults.
    pub parameters: Vec<f32>,
}

impl Default for CardSettings {
    fn default() -> Self {
        Self {
            desired_retention: 0.9,
            new_per_day: 10,
            max_reviews: Some(200),
            leech_after: 8,
            parameters: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum SchedulerError {
    #[error("how much to remember has to be between 70% and 99%")]
    Retention,
    #[error("the leech threshold has to be at least 2")]
    LeechThreshold,
    #[error("the scheduler couldn't work out the next date: {0}")]
    Fsrs(String),
    #[error("fitting the scheduler needs at least {needed} reviews, you have {have} so far")]
    NotEnoughReviews { needed: usize, have: usize },
    #[error("fitting the scheduler didn't work: {0}")]
    Fit(String),
}

impl CardSettings {
    pub fn validate(&self) -> Result<(), SchedulerError> {
        if !(0.7..=0.99).contains(&self.desired_retention) {
            return Err(SchedulerError::Retention);
        }
        if self.leech_after < 2 {
            return Err(SchedulerError::LeechThreshold);
        }
        Ok(())
    }

    pub fn is_leech(&self, schedule: &Schedule) -> bool {
        schedule.lapses >= self.leech_after
    }

    fn model(&self) -> Result<fsrs::FSRS, SchedulerError> {
        fsrs::FSRS::new(&self.parameters).map_err(|e| SchedulerError::Fsrs(e.to_string()))
    }

    fn next_states(
        &self,
        schedule: &Schedule,
        today: i64,
    ) -> Result<fsrs::NextStates, SchedulerError> {
        let memory = schedule.memory.map(|m| fsrs::MemoryState {
            stability: m.stability,
            difficulty: m.difficulty,
        });
        let elapsed = schedule
            .last_review
            .map_or(0, |last| (today - last).max(0) as u32);
        self.model()?
            .next_states(memory, self.desired_retention, elapsed)
            .map_err(|e| SchedulerError::Fsrs(e.to_string()))
    }

    /// Days until each answer would bring the card back: Again, Hard, Good, Easy.
    /// Again is 0, meaning later in this session (R66).
    pub fn preview(&self, schedule: &Schedule, today: i64) -> Result<[i64; 4], SchedulerError> {
        let states = self.next_states(schedule, today)?;
        let days = |state: &fsrs::ItemState| (state.interval.round() as i64).max(1);
        let (hard, good, easy) = (days(&states.hard), days(&states.good), days(&states.easy));
        // Keep the buttons in order even when FSRS rounds two of them to the same day.
        let good = good.max(hard);
        let easy = easy.max(good + 1);
        Ok([0, hard, good, easy])
    }

    /// The card's schedule after answering `rating` on `today`.
    pub fn grade(
        &self,
        schedule: &Schedule,
        rating: Rating,
        today: i64,
    ) -> Result<Schedule, SchedulerError> {
        let states = self.next_states(schedule, today)?;
        let [_, hard, good, easy] = self.preview(schedule, today)?;
        let (state, due) = match rating {
            Rating::Again => (states.again, today),
            Rating::Hard => (states.hard, today + hard),
            Rating::Good => (states.good, today + good),
            Rating::Easy => (states.easy, today + easy),
        };
        let lapsed =
            rating == Rating::Again && !schedule.is_new() && schedule.last_review != Some(today);
        Ok(Schedule {
            memory: Some(Memory {
                stability: state.memory.stability,
                difficulty: state.memory.difficulty,
            }),
            due,
            last_review: Some(today),
            reps: schedule.reps + 1,
            lapses: schedule.lapses + u32::from(lapsed),
        })
    }
}

/// Chance of remembering the card right now, for the card details panel.
pub fn retrievability(settings: &CardSettings, schedule: &Schedule, today: i64) -> Option<f32> {
    let memory = schedule.memory?;
    let elapsed = (today - schedule.last_review?).max(0) as f32;
    let decay = settings
        .parameters
        .get(20)
        .copied()
        .unwrap_or(fsrs::FSRS6_DEFAULT_DECAY);
    Some(fsrs::current_retrievability(
        fsrs::MemoryState {
            stability: memory.stability,
            difficulty: memory.difficulty,
        },
        elapsed,
        decay,
    ))
}

// ---------- Fit the scheduler to you (R75) ----------

pub const MIN_REVIEWS_TO_FIT: usize = 50;

/// FSRS parameters fitted to review histories: for each card, every answer in order with the
/// days since the answer before it (0 for the first).
pub fn fit(histories: &[Vec<(Rating, u32)>]) -> Result<Vec<f32>, SchedulerError> {
    let have: usize = histories.iter().map(Vec::len).sum();
    if have < MIN_REVIEWS_TO_FIT {
        return Err(SchedulerError::NotEnoughReviews {
            needed: MIN_REVIEWS_TO_FIT,
            have,
        });
    }
    let mut train_set = vec![];
    for history in histories {
        let reviews: Vec<fsrs::FSRSReview> = history
            .iter()
            .map(|(rating, delta_t)| fsrs::FSRSReview {
                rating: *rating as u32,
                delta_t: *delta_t,
            })
            .collect();
        for end in 2..=reviews.len() {
            if reviews[end - 1].delta_t > 0 {
                train_set.push(fsrs::FSRSItem {
                    reviews: reviews[..end].to_vec(),
                });
            }
        }
    }
    if train_set.is_empty() {
        return Err(SchedulerError::NotEnoughReviews {
            needed: MIN_REVIEWS_TO_FIT,
            have,
        });
    }
    fsrs::compute_parameters(fsrs::ComputeParametersInput {
        train_set,
        ..Default::default()
    })
    .map_err(|e| SchedulerError::Fit(e.to_string()))
}

// ---------- A review session (R66, R67, R69) ----------

/// What's due today and goes into a session: learning first, then reviews, then new cards,
/// within the daily limits. `new_today` and `reviewed_today` are what's already been done today.
pub fn session_order<Id: Copy>(
    cards: &[(Id, Schedule)],
    settings: &CardSettings,
    today: i64,
    new_today: u32,
    reviewed_today: u32,
) -> Vec<Id> {
    let due: Vec<&(Id, Schedule)> = cards.iter().filter(|(_, s)| s.is_due(today)).collect();
    let mut learning: Vec<&(Id, Schedule)> = due
        .iter()
        .copied()
        .filter(|(_, s)| s.is_learning(today))
        .collect();
    let mut review: Vec<&(Id, Schedule)> = due
        .iter()
        .copied()
        .filter(|(_, s)| !s.is_new() && !s.is_learning(today))
        .collect();
    let new: Vec<&(Id, Schedule)> = due.iter().copied().filter(|(_, s)| s.is_new()).collect();
    learning.sort_by_key(|(_, s)| s.due);
    review.sort_by_key(|(_, s)| s.due);
    let review_room = settings.max_reviews.map_or(usize::MAX, |max| {
        max.saturating_sub(reviewed_today) as usize
    });
    let new_room = settings.new_per_day.saturating_sub(new_today) as usize;
    learning
        .into_iter()
        .chain(review.into_iter().take(review_room))
        .chain(new.into_iter().take(new_room))
        .map(|(id, _)| *id)
        .collect()
}

/// Again puts a card back this many places later in the session, so it isn't shown twice in a row.
pub const AGAIN_GAP: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: i64 = 20_760;

    fn note(kind: NoteKind) -> Note {
        Note {
            topic: TopicId(1),
            kind,
            front: "Front".into(),
            back: "Back".into(),
            cues: vec![],
            text: String::new(),
            extra: String::new(),
        }
    }

    // flashcards.feature: One note makes one or more cards (outline)
    #[test]
    fn notes_make_the_right_cards() {
        assert_eq!(note(NoteKind::Basic).faces().unwrap().len(), 1);
        let cues = Note {
            cues: vec!["Cue A".into(), "Cue B".into()],
            ..note(NoteKind::Basic)
        };
        let faces = cues.faces().unwrap();
        assert_eq!(
            faces
                .iter()
                .map(|f| (f.key.as_str(), f.front.as_str(), f.back.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("main", "Front", "Back"),
                ("cue:0", "Cue A", "Back"),
                ("cue:1", "Cue B", "Back")
            ]
        );
        let reversed = note(NoteKind::Reversed).faces().unwrap();
        assert_eq!(
            (reversed[1].front.as_str(), reversed[1].back.as_str()),
            ("Back", "Front")
        );
        let cloze = Note {
            text: "{{c1::A}} {{c2::B}} {{c2::C}} {{c3::D}}".into(),
            ..note(NoteKind::Cloze)
        };
        assert_eq!(
            cloze
                .faces()
                .unwrap()
                .iter()
                .map(|f| f.key.as_str())
                .collect::<Vec<_>>(),
            vec!["c1", "c2", "c3"]
        );
    }

    // flashcards.feature: A cloze gap shows its hint and is filled in on the answer
    #[test]
    fn cloze_hides_only_its_own_gaps() {
        let cloze = Note {
            text: "L1 {{c1::4}}, L3 about {{c2::40::tens of cycles}} cycles".into(),
            extra: "From lecture 3".into(),
            ..note(NoteKind::Cloze)
        };
        let faces = cloze.faces().unwrap();
        assert_eq!(
            faces[1].front,
            "L1 4, L3 about <span class=\"cloze-gap\">[tens of cycles]</span> cycles"
        );
        assert_eq!(
            faces[1].back,
            "L1 4, L3 about <span class=\"cloze-answer\">40</span> cycles\n\nFrom lecture 3"
        );
        assert_eq!(
            faces[0].front,
            "L1 <span class=\"cloze-gap\">[...]</span>, L3 about 40 cycles"
        );
        let html = Note {
            text: "{{c1::<b>x</b> & y}}".into(),
            ..note(NoteKind::Cloze)
        }
        .faces()
        .unwrap();
        assert!(
            html[0].back.contains("&lt;b&gt;x&lt;/b&gt; &amp; y"),
            "answers are escaped inside the highlight"
        );
    }

    // flashcards.feature: Notes need content
    #[test]
    fn notes_need_content() {
        assert_eq!(
            Note {
                back: " ".into(),
                ..note(NoteKind::Basic)
            }
            .faces(),
            Err(CardError::NoBack)
        );
        assert_eq!(
            Note {
                front: "".into(),
                ..note(NoteKind::Reversed)
            }
            .faces(),
            Err(CardError::NoFront)
        );
        assert_eq!(
            Note {
                text: "no gaps here {{c::x}} {{c1::}}".into(),
                ..note(NoteKind::Cloze)
            }
            .faces(),
            Err(CardError::NoGaps)
        );
        assert_eq!(
            Note {
                cues: vec![" ".into()],
                ..note(NoteKind::Basic)
            }
            .faces(),
            Err(CardError::EmptyCue)
        );
    }

    // flashcards.feature: Grading moves a card on
    #[test]
    fn grading_moves_a_card_on_and_previews_each_answer() {
        let settings = CardSettings::default();
        let new = Schedule::new(TODAY);
        let [again, hard, good, easy] = settings.preview(&new, TODAY).unwrap();
        assert!(
            again == 0 && 1 <= hard && hard <= good && good < easy,
            "{hard} {good} {easy}"
        );
        let after = settings.grade(&new, Rating::Good, TODAY).unwrap();
        assert_eq!(
            (after.due, after.reps, after.lapses, after.last_review),
            (TODAY + good, 1, 0, Some(TODAY))
        );
        assert!(!after.is_due(TODAY));
        // A second Good much later goes further out
        let later = settings.grade(&after, Rating::Good, after.due).unwrap();
        assert!(later.due - after.due > good, "intervals grow");
    }

    // flashcards.feature: Again shows a card again in the same session
    #[test]
    fn again_keeps_a_card_due_today_and_counts_lapses_on_learned_cards() {
        let settings = CardSettings::default();
        let new = Schedule::new(TODAY);
        let failed_new = settings.grade(&new, Rating::Again, TODAY).unwrap();
        assert_eq!(
            (failed_new.due, failed_new.lapses),
            (TODAY, 0),
            "a new card's first Again isn't a lapse"
        );
        assert!(failed_new.is_learning(TODAY));
        let learned = settings.grade(&failed_new, Rating::Good, TODAY).unwrap();
        let lapse = settings
            .grade(&learned, Rating::Again, learned.due)
            .unwrap();
        assert_eq!(lapse.lapses, 1);
        let again_same_day = settings.grade(&lapse, Rating::Again, learned.due).unwrap();
        assert_eq!(
            again_same_day.lapses, 1,
            "Again twice in one day is one lapse"
        );
    }

    // flashcards.feature: A card that keeps slipping is flagged
    #[test]
    fn leeches_are_flagged_at_the_threshold() {
        let settings = CardSettings::default();
        let card = Schedule {
            lapses: 7,
            ..Schedule::new(TODAY)
        };
        assert!(!settings.is_leech(&card));
        assert!(settings.is_leech(&Schedule { lapses: 8, ..card }));
    }

    // flashcards.feature: Daily limits
    #[test]
    fn sessions_put_learning_first_and_respect_daily_limits() {
        let settings = CardSettings {
            new_per_day: 2,
            max_reviews: Some(1),
            ..CardSettings::default()
        };
        let memory = Some(Memory {
            stability: 3.0,
            difficulty: 5.0,
        });
        let cards = vec![
            (1, Schedule::new(TODAY)),
            (
                2,
                Schedule {
                    memory,
                    due: TODAY - 2,
                    last_review: Some(TODAY - 5),
                    reps: 2,
                    lapses: 0,
                },
            ),
            (
                3,
                Schedule {
                    memory,
                    due: TODAY,
                    last_review: Some(TODAY),
                    reps: 1,
                    lapses: 0,
                },
            ),
            (4, Schedule::new(TODAY)),
            (5, Schedule::new(TODAY)),
            (
                6,
                Schedule {
                    memory,
                    due: TODAY - 1,
                    last_review: Some(TODAY - 4),
                    reps: 2,
                    lapses: 0,
                },
            ),
            (
                7,
                Schedule {
                    memory,
                    due: TODAY + 3,
                    last_review: Some(TODAY - 1),
                    reps: 2,
                    lapses: 0,
                },
            ),
        ];
        assert_eq!(
            session_order(&cards, &settings, TODAY, 0, 0),
            vec![3, 2, 1, 4]
        );
        assert_eq!(
            session_order(&cards, &settings, TODAY, 2, 1),
            vec![3],
            "limits count what's already done today"
        );
        let unlimited = CardSettings {
            max_reviews: None,
            ..settings
        };
        assert_eq!(
            session_order(&cards, &unlimited, TODAY, 0, 0),
            vec![3, 2, 6, 1, 4]
        );
    }

    // flashcards.feature: Fitting the scheduler needs history
    #[test]
    fn fitting_needs_enough_reviews_and_then_gives_usable_parameters() {
        let few = vec![vec![(Rating::Good, 0), (Rating::Good, 3)]; 10];
        assert_eq!(
            fit(&few),
            Err(SchedulerError::NotEnoughReviews {
                needed: 50,
                have: 20
            })
        );
        let ratings = [
            Rating::Good,
            Rating::Again,
            Rating::Good,
            Rating::Hard,
            Rating::Easy,
        ];
        let many: Vec<Vec<(Rating, u32)>> = (0..60)
            .map(|card| {
                (0..5)
                    .map(|i| {
                        (
                            ratings[(card + i) % 5],
                            if i == 0 {
                                0
                            } else {
                                [1, 3, 7, 15][(i - 1) % 4]
                            },
                        )
                    })
                    .collect()
            })
            .collect();
        let parameters = fit(&many).expect("fits");
        let settings = CardSettings {
            parameters,
            ..CardSettings::default()
        };
        assert!(
            settings
                .grade(&Schedule::new(TODAY), Rating::Good, TODAY)
                .unwrap()
                .due
                > TODAY,
            "fitted parameters schedule cards"
        );
    }

    #[test]
    fn retrievability_falls_over_time() {
        let settings = CardSettings::default();
        let seen = Schedule {
            memory: Some(Memory {
                stability: 10.0,
                difficulty: 5.0,
            }),
            due: TODAY + 10,
            last_review: Some(TODAY),
            reps: 1,
            lapses: 0,
        };
        let now = retrievability(&settings, &seen, TODAY).unwrap();
        let later = retrievability(&settings, &seen, TODAY + 10).unwrap();
        assert!(now > 0.99 && (0.85..0.95).contains(&later), "{now} {later}");
        assert_eq!(
            retrievability(&settings, &Schedule::new(TODAY), TODAY),
            None
        );
    }

    #[test]
    fn settings_are_checked() {
        assert_eq!(CardSettings::default().validate(), Ok(()));
        assert_eq!(
            CardSettings {
                desired_retention: 0.5,
                ..Default::default()
            }
            .validate(),
            Err(SchedulerError::Retention)
        );
        assert_eq!(
            CardSettings {
                leech_after: 1,
                ..Default::default()
            }
            .validate(),
            Err(SchedulerError::LeechThreshold)
        );
        let lower = CardSettings {
            desired_retention: 0.8,
            ..Default::default()
        };
        let higher = CardSettings {
            desired_retention: 0.97,
            ..Default::default()
        };
        let seen = Schedule {
            memory: Some(Memory {
                stability: 10.0,
                difficulty: 5.0,
            }),
            due: TODAY,
            last_review: Some(TODAY - 10),
            reps: 3,
            lapses: 0,
        };
        assert!(
            lower.preview(&seen, TODAY).unwrap()[2] > higher.preview(&seen, TODAY).unwrap()[2],
            "remembering more means sooner reviews"
        );
    }
}

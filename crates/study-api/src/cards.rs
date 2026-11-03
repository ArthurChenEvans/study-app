//! Flashcard commands (R62 to R77). Thin over the store and the core's FSRS scheduling.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use study_core::attempt::Outcome;
use study_core::cards::{
    AGAIN_GAP, CardSettings, Note, NoteKind, Rating, Schedule, fit, retrievability, session_order,
};
use study_core::question::TopicId;
use study_store::cards::{CardId, CardRow, NoteId, ReviewRow};
use study_store::recall::Subject;

use crate::{ApiError, ApiResult, App};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DueCounts {
    pub new: usize,
    pub learning: usize,
    pub review: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TopicCounts {
    pub topic: TopicId,
    pub name: String,
    pub module: Option<String>,
    pub due: DueCounts,
    pub total: usize,
    pub retention: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CardsHome {
    pub due: DueCounts,
    pub total_cards: usize,
    pub minutes: usize,
    /// Cards due on each of the next 14 days, starting today.
    pub forecast: Vec<usize>,
    pub retention: Option<u8>,
    pub target: u8,
    pub per_topic: Vec<TopicCounts>,
    pub suggestions: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CardListRow {
    pub id: CardId,
    pub note: NoteId,
    pub front: String,
    pub kind: String,
    pub source: Option<String>,
    pub due: i64,
    pub is_new: bool,
    pub lapses: u32,
    pub leech: bool,
    pub archived: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TopicCards {
    pub topic: TopicId,
    pub name: String,
    pub module: Option<String>,
    pub due: usize,
    pub cards: Vec<CardListRow>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CardDetail {
    pub id: CardId,
    pub key: String,
    pub reps: u32,
    pub lapses: u32,
    pub leech: bool,
    pub due: i64,
    pub is_new: bool,
    pub archived: bool,
    /// Percent chance of remembering it right now.
    pub remember_now: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NoteView {
    pub id: NoteId,
    pub note: Note,
    pub source: Option<String>,
    pub cards: Vec<CardDetail>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ReviewCard {
    pub card: CardId,
    pub note: NoteId,
    pub front: String,
    pub back: String,
    pub topic_name: String,
    pub kind: String,
    pub source: Option<String>,
    /// Days until each answer brings it back: Again (0, later this session), Hard, Good, Easy.
    pub intervals: [i64; 4],
    pub lapses: u32,
    pub leech: bool,
    pub left: DueCounts,
    /// Which queue this card is from: "new", "learning" or "review".
    pub queue: String,
    pub done: usize,
    pub total: usize,
    pub can_undo: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    pub note: NoteId,
    pub topic: TopicId,
    pub front: String,
    pub agains: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ReviewDone {
    pub reviewed: usize,
    pub new: usize,
    pub again: usize,
    pub remembered: Option<u8>,
    pub target: u8,
    pub seconds: u32,
    pub worth_rewriting: Vec<Rewrite>,
    pub due_tomorrow: usize,
    /// The busiest of the next 14 days: (day, cards).
    pub busiest: Option<(i64, usize)>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ReviewStep {
    Card(ReviewCard),
    Done(ReviewDone),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SuggestionKind {
    Recall,
    Quiz,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub key: String,
    pub kind: SuggestionKind,
    pub from: String,
    pub day: i64,
    pub note: String,
    /// Recall: the missed point. Quiz: the question.
    pub text: String,
    /// Quiz only: the correct answer.
    pub answer: Option<String>,
    /// Where a card made from it would go, if known.
    pub topic: Option<TopicId>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NoteInput {
    pub note: Note,
    /// The suggestion this card is made from, if any (R72).
    pub suggestion: Option<String>,
}

/// A review session in progress (R66, R67).
pub struct CardSession {
    queue: VecDeque<CardId>,
    total: usize,
    undo: Vec<Undo>,
    answers: usize,
    new_seen: usize,
    agains: usize,
    seconds: u32,
    agains_by_note: HashMap<NoteId, u32>,
}

struct Undo {
    card: CardId,
    previous: Schedule,
    log: i64,
    queue: VecDeque<CardId>,
    rating: Rating,
    was_new: bool,
    seconds: u32,
}

fn kind_label(card: &CardRow, cards_in_note: usize) -> String {
    match card.key.as_str() {
        "forward" | "reverse" => "Reversed".into(),
        key if key.starts_with('c') && key[1..].parse::<u32>().is_ok() => {
            format!("Cloze {} of {}", &key[1..], cards_in_note)
        }
        _ => "Basic".into(),
    }
}

fn source_label(source: Option<&str>) -> Option<String> {
    source.map(|s| {
        if s.starts_with("quiz:") {
            "From quiz mistake".into()
        } else if s.starts_with("recall:") {
            "From recall miss".into()
        } else {
            "Written".into()
        }
    })
}

fn percent(part: usize, whole: usize) -> Option<u8> {
    (whole > 0).then(|| ((part * 100) as f64 / whole as f64).round() as u8)
}

impl App {
    fn reviews_today(&self, today: i64) -> ApiResult<(u32, u32)> {
        let reviews = self.store.reviews_since(today)?;
        let new = reviews.iter().filter(|r| r.was_new).count() as u32;
        let old = reviews
            .iter()
            .filter(|r| !r.was_new && r.rating != Rating::Again as u8)
            .count() as u32;
        Ok((new, old))
    }

    fn in_scope(&self, scope: Option<Subject>) -> ApiResult<Vec<CardRow>> {
        let topics = self.store.topics()?;
        Ok(self
            .store
            .cards()?
            .into_iter()
            .filter(|card| {
                !card.archived
                    && match scope {
                        None => true,
                        Some(Subject::Topic(topic)) => card.topic == topic,
                        Some(Subject::Module(module)) => topics
                            .iter()
                            .any(|t| t.id == card.topic && t.module == Some(module)),
                    }
            })
            .collect())
    }

    fn due_counts(
        &self,
        cards: &[CardRow],
        settings: &CardSettings,
        today: i64,
    ) -> ApiResult<DueCounts> {
        let (new_today, reviewed_today) = self.reviews_today(today)?;
        let list: Vec<(CardId, Schedule)> = cards.iter().map(|c| (c.id, c.schedule)).collect();
        let order = session_order(&list, settings, today, new_today, reviewed_today);
        Ok(count(&order, cards, today))
    }

    /// Cards due today across every topic, for the sidebar (R68).
    pub fn cards_due_count(&self, today: i64) -> ApiResult<usize> {
        let settings = self.store.card_settings()?;
        let due = self.due_counts(&self.in_scope(None)?, &settings, today)?;
        Ok(due.new + due.learning + due.review)
    }

    /// The Cards screen (R68).
    pub fn cards_home(&self, today: i64) -> ApiResult<CardsHome> {
        let settings = self.store.card_settings()?;
        let cards = self.in_scope(None)?;
        let due = self.due_counts(&cards, &settings, today)?;
        let forecast = (0..14)
            .map(|offset| {
                cards
                    .iter()
                    .filter(|c| {
                        if offset == 0 {
                            c.schedule.due <= today
                        } else {
                            c.schedule.due == today + offset
                        }
                    })
                    .count()
            })
            .collect();
        let reviews = self.store.reviews_since(today - 29)?;
        let graded: Vec<&ReviewRow> = reviews.iter().filter(|r| !r.was_new).collect();
        let retention = percent(
            graded
                .iter()
                .filter(|r| r.rating != Rating::Again as u8)
                .count(),
            graded.len(),
        );
        let topics = self.store.topics()?;
        let modules = self.store.modules()?;
        let card_topics: HashMap<CardId, TopicId> = cards.iter().map(|c| (c.id, c.topic)).collect();
        let mut per_topic = vec![];
        for topic in &topics {
            let in_topic: Vec<CardRow> = cards
                .iter()
                .filter(|c| c.topic == topic.id)
                .cloned()
                .collect();
            let all = self.store.cards_in(topic.id)?.len();
            if all == 0 {
                continue;
            }
            let topic_reviews: Vec<&&ReviewRow> = graded
                .iter()
                .filter(|r| card_topics.get(&r.card) == Some(&topic.id))
                .collect();
            per_topic.push(TopicCounts {
                topic: topic.id,
                name: topic.name.clone(),
                module: topic.module.and_then(|m| {
                    modules
                        .iter()
                        .find(|row| row.id == m)
                        .map(|row| row.name.clone())
                }),
                due: self.due_counts(&in_topic, &settings, today)?,
                total: all,
                retention: percent(
                    topic_reviews
                        .iter()
                        .filter(|r| r.rating != Rating::Again as u8)
                        .count(),
                    topic_reviews.len(),
                ),
            });
        }
        let total = due.new + due.learning + due.review;
        Ok(CardsHome {
            minutes: (total * 25).div_ceil(60),
            due,
            total_cards: self.store.cards()?.len(),
            forecast,
            retention,
            target: (settings.desired_retention * 100.0).round() as u8,
            per_topic,
            suggestions: self.card_suggestions(today)?.len(),
        })
    }

    pub fn topic_cards(&self, topic: TopicId, today: i64) -> ApiResult<TopicCards> {
        let settings = self.store.card_settings()?;
        let topics = self.store.topics()?;
        let row = topics
            .iter()
            .find(|t| t.id == topic)
            .ok_or_else(|| ApiError::new("topic not found"))?;
        let module = row.module.and_then(|m| {
            self.store
                .modules()
                .ok()?
                .into_iter()
                .find(|r| r.id == m)
                .map(|r| r.name)
        });
        let cards = self.store.cards_in(topic)?;
        let mut per_note: HashMap<NoteId, usize> = HashMap::new();
        for card in &cards {
            *per_note.entry(card.note).or_default() += 1;
        }
        let live: Vec<CardRow> = cards.iter().filter(|c| !c.archived).cloned().collect();
        let due = self.due_counts(&live, &settings, today)?;
        Ok(TopicCards {
            topic,
            name: row.name.clone(),
            module,
            due: due.new + due.learning + due.review,
            cards: cards
                .iter()
                .map(|c| CardListRow {
                    id: c.id,
                    note: c.note,
                    front: c.front.clone(),
                    kind: kind_label(c, per_note[&c.note]),
                    source: source_label(c.source.as_deref()),
                    due: c.schedule.due,
                    is_new: c.schedule.is_new(),
                    lapses: c.schedule.lapses,
                    leech: settings.is_leech(&c.schedule),
                    archived: c.archived,
                })
                .collect(),
        })
    }

    pub fn note_view(&self, id: NoteId, today: i64) -> ApiResult<NoteView> {
        let settings = self.store.card_settings()?;
        let (note, source) = self.store.note(id)?;
        let cards = self
            .store
            .cards_of(id)?
            .into_iter()
            .map(|c| CardDetail {
                id: c.id,
                key: c.key.clone(),
                reps: c.schedule.reps,
                lapses: c.schedule.lapses,
                leech: settings.is_leech(&c.schedule),
                due: c.schedule.due,
                is_new: c.schedule.is_new(),
                archived: c.archived,
                remember_now: retrievability(&settings, &c.schedule, today)
                    .map(|r| (r * 100.0).round() as u8),
            })
            .collect();
        Ok(NoteView {
            id,
            note,
            source,
            cards,
        })
    }

    /// Saves a new or edited note (R62 to R64, R77). A note made from a suggestion clears it (R72).
    pub fn save_note(
        &mut self,
        id: Option<NoteId>,
        input: NoteInput,
        today: i64,
    ) -> ApiResult<NoteId> {
        let mut note = input.note;
        note.cues.retain(|cue| !cue.trim().is_empty());
        let faces = note.faces()?;
        let source = input.suggestion.as_deref();
        let saved = self.store.save_note(id, &note, &faces, source, today)?;
        if let Some(key) = source {
            self.store.mark_suggestion(key, true)?;
        }
        Ok(saved)
    }

    pub fn delete_note(&mut self, id: NoteId) -> ApiResult<()> {
        Ok(self.store.delete_note(id)?)
    }

    /// R71.
    pub fn archive_card(&mut self, id: CardId, archived: bool) -> ApiResult<()> {
        Ok(self.store.set_archived(id, archived)?)
    }

    // ---------- Reviewing (R66, R67, R69, R76) ----------

    pub fn start_review(&mut self, scope: Option<Subject>, today: i64) -> ApiResult<ReviewStep> {
        let settings = self.store.card_settings()?;
        let cards = self.in_scope(scope)?;
        let (new_today, reviewed_today) = self.reviews_today(today)?;
        let list: Vec<(CardId, Schedule)> = cards.iter().map(|c| (c.id, c.schedule)).collect();
        let queue: VecDeque<CardId> =
            session_order(&list, &settings, today, new_today, reviewed_today).into();
        self.card_session = Some(CardSession {
            total: queue.len(),
            queue,
            undo: vec![],
            answers: 0,
            new_seen: 0,
            agains: 0,
            seconds: 0,
            agains_by_note: HashMap::new(),
        });
        self.review_step(today)
    }

    fn review_step(&self, today: i64) -> ApiResult<ReviewStep> {
        let session = self
            .card_session
            .as_ref()
            .ok_or_else(|| ApiError::new("no review in progress"))?;
        let settings = self.store.card_settings()?;
        let Some(&id) = session.queue.front() else {
            return Ok(ReviewStep::Done(self.review_done(today)?));
        };
        let card = self.store.card(id)?;
        let siblings = self.store.cards_of(card.note)?.len();
        let topic_name = self
            .store
            .topics()?
            .into_iter()
            .find(|t| t.id == card.topic)
            .map(|t| t.name)
            .unwrap_or_default();
        let queued: Vec<CardRow> = session
            .queue
            .iter()
            .filter_map(|id| self.store.card(*id).ok())
            .collect();
        let left = count(
            &session.queue.iter().copied().collect::<Vec<_>>(),
            &queued,
            today,
        );
        let queue = if card.schedule.is_new() {
            "new"
        } else if card.schedule.is_learning(today) {
            "learning"
        } else {
            "review"
        };
        Ok(ReviewStep::Card(ReviewCard {
            card: id,
            note: card.note,
            kind: kind_label(&card, siblings),
            source: source_label(card.source.as_deref()),
            intervals: settings.preview(&card.schedule, today)?,
            lapses: card.schedule.lapses,
            leech: settings.is_leech(&card.schedule),
            front: card.front,
            back: card.back,
            topic_name,
            left,
            queue: queue.into(),
            done: session.total.saturating_sub(session.queue.len()),
            total: session.total.max(session.answers + session.queue.len()),
            can_undo: !session.undo.is_empty(),
        }))
    }

    pub fn current_card(&self, today: i64) -> ApiResult<ReviewStep> {
        self.review_step(today)
    }

    /// Answers the card at the front of the session. `seconds` is how long it took.
    pub fn grade_card(
        &mut self,
        rating: Rating,
        seconds: u32,
        today: i64,
    ) -> ApiResult<ReviewStep> {
        let settings = self.store.card_settings()?;
        let session = self
            .card_session
            .as_mut()
            .ok_or_else(|| ApiError::new("no review in progress"))?;
        let id = *session
            .queue
            .front()
            .ok_or_else(|| ApiError::new("nothing left to review"))?;
        let card = self.store.card(id)?;
        let next = settings.grade(&card.schedule, rating, today)?;
        let elapsed = card
            .schedule
            .last_review
            .map_or(0, |last| (today - last).max(0) as u32);
        let was_new = card.schedule.is_new();
        let log = self.store.record_review(
            id,
            &next,
            &ReviewRow {
                card: id,
                day: today,
                rating: rating as u8,
                elapsed,
                was_new,
                seconds,
            },
        )?;
        let session = self.card_session.as_mut().expect("checked above");
        let before = session.queue.clone();
        session.queue.pop_front();
        if rating == Rating::Again {
            let at = AGAIN_GAP.min(session.queue.len());
            session.queue.insert(at, id);
            session.agains += 1;
            *session.agains_by_note.entry(card.note).or_default() += 1;
        }
        session.answers += 1;
        session.new_seen += usize::from(was_new);
        session.seconds += seconds;
        session.undo.push(Undo {
            card: id,
            previous: card.schedule,
            log,
            queue: before,
            rating,
            was_new,
            seconds,
        });
        self.review_step(today)
    }

    /// R67: takes back the last answer and shows that card again.
    pub fn undo_card(&mut self, today: i64) -> ApiResult<ReviewStep> {
        let session = self
            .card_session
            .as_mut()
            .ok_or_else(|| ApiError::new("no review in progress"))?;
        let undo = session
            .undo
            .pop()
            .ok_or_else(|| ApiError::new("there's nothing to undo"))?;
        session.queue = undo.queue;
        session.answers -= 1;
        session.new_seen -= usize::from(undo.was_new);
        session.seconds -= undo.seconds;
        if undo.rating == Rating::Again {
            session.agains -= 1;
            let note = self.store.card(undo.card)?.note;
            if let Some(count) = session.agains_by_note.get_mut(&note) {
                *count = count.saturating_sub(1);
            }
        }
        self.store
            .undo_review(undo.card, &undo.previous, undo.log)?;
        self.review_step(today)
    }

    pub fn leave_review(&mut self) {
        self.card_session = None;
    }

    fn review_done(&self, today: i64) -> ApiResult<ReviewDone> {
        let session = self
            .card_session
            .as_ref()
            .ok_or_else(|| ApiError::new("no review in progress"))?;
        let settings = self.store.card_settings()?;
        let mut worth_rewriting = vec![];
        for (note, agains) in &session.agains_by_note {
            if *agains < 2 {
                continue;
            }
            if let Some(card) = self.store.cards_of(*note)?.into_iter().next() {
                worth_rewriting.push(Rewrite {
                    note: *note,
                    topic: card.topic,
                    front: card.front,
                    agains: *agains,
                });
            }
        }
        worth_rewriting.sort_by_key(|r| std::cmp::Reverse(r.agains));
        let cards = self.in_scope(None)?;
        let busiest = (1..14)
            .map(|offset| {
                (
                    today + offset,
                    cards
                        .iter()
                        .filter(|c| c.schedule.due == today + offset)
                        .count(),
                )
            })
            .max_by_key(|(_, n)| *n)
            .filter(|(_, n)| *n > 0);
        Ok(ReviewDone {
            reviewed: session.answers,
            new: session.new_seen,
            again: session.agains,
            remembered: percent(session.answers - session.agains, session.answers),
            target: (settings.desired_retention * 100.0).round() as u8,
            seconds: session.seconds,
            worth_rewriting,
            due_tomorrow: cards.iter().filter(|c| c.schedule.due == today + 1).count(),
            busiest,
        })
    }

    // ---------- Suggestions (R72, R73) ----------

    pub fn card_suggestions(&self, today: i64) -> ApiResult<Vec<Suggestion>> {
        let done = self.store.suggestions_done()?;
        let topics = self.store.topics()?;
        let mut suggestions = vec![];
        for checklist in self.store.checklists()? {
            let checks = self.store.checks(checklist.id)?;
            let Some(latest) = checks.first() else {
                continue;
            };
            for (text, recalled) in &latest.points {
                if *recalled {
                    continue;
                }
                let key = format!("recall:{}:{}", checklist.id.0, text);
                if done.contains(&key) {
                    continue;
                }
                let missed = checks
                    .iter()
                    .filter(|c| c.points.iter().any(|(t, r)| t == text && !r))
                    .count();
                let seen = checks
                    .iter()
                    .filter(|c| c.points.iter().any(|(t, _)| t == text))
                    .count();
                suggestions.push(Suggestion {
                    key,
                    kind: SuggestionKind::Recall,
                    from: format!("Recall: {}", checklist.subject_name),
                    day: latest.day,
                    note: format!(
                        "Missed {missed} of {seen} {}",
                        if seen == 1 { "check" } else { "checks" }
                    ),
                    text: text.clone(),
                    answer: None,
                    topic: match checklist.subject {
                        Some(Subject::Topic(topic)) => Some(topic),
                        _ => None,
                    },
                });
            }
        }
        let mut seen_questions = vec![];
        for row in self
            .store
            .history()?
            .into_iter()
            .filter(|row| row.day >= today - 30)
        {
            let (attempt, _) = self.store.attempt(row.id)?;
            for asked in attempt.asked() {
                let wrong = matches!(asked.outcome, Some(Outcome::Wrong(_)));
                let guessed = asked.outcome == Some(Outcome::Right) && asked.flagged_guessed;
                if !(wrong || guessed) {
                    continue;
                }
                let key = format!("quiz:{}", asked.item.question.0);
                if done.contains(&key) || seen_questions.contains(&key) {
                    continue;
                }
                seen_questions.push(key.clone());
                let question = &asked.snapshot;
                let topic_name = topics
                    .iter()
                    .find(|t| t.id == question.topic())
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "deleted topic".into());
                let picked: Vec<String> = asked
                    .picked
                    .iter()
                    .map(|&i| question.options()[i].text.trim().to_string())
                    .collect();
                suggestions.push(Suggestion {
                    key,
                    kind: SuggestionKind::Quiz,
                    from: format!("Quiz: {topic_name}"),
                    day: row.day,
                    note: if guessed {
                        "Right but guessed".into()
                    } else if picked.is_empty() {
                        "Not answered".into()
                    } else {
                        format!("Wrong, you picked “{}”", picked.join(", "))
                    },
                    text: question.text().trim().into(),
                    answer: Some(
                        question
                            .correct_indices()
                            .iter()
                            .map(|&i| question.options()[i].text.trim().to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    topic: topics
                        .iter()
                        .any(|t| t.id == question.topic())
                        .then(|| question.topic()),
                });
            }
        }
        suggestions.sort_by_key(|s| std::cmp::Reverse(s.day));
        Ok(suggestions)
    }

    pub fn dismiss_suggestion(&mut self, key: &str) -> ApiResult<()> {
        Ok(self.store.mark_suggestion(key, false)?)
    }

    // ---------- Settings (R74, R75) ----------

    pub fn card_settings(&self) -> ApiResult<CardSettings> {
        Ok(self.store.card_settings()?)
    }

    pub fn set_card_settings(&mut self, settings: CardSettings) -> ApiResult<()> {
        settings.validate()?;
        Ok(self.store.set_card_settings(&settings)?)
    }

    pub fn reset_card_settings(&mut self) -> ApiResult<CardSettings> {
        self.store.reset_card_settings()?;
        self.card_settings()
    }

    /// Fits FSRS to the review history and saves the parameters. Returns how many reviews it used.
    pub fn fit_scheduler(&mut self) -> ApiResult<usize> {
        let histories = self.store.review_histories()?;
        let used = histories.iter().map(Vec::len).sum();
        let parameters = fit(&histories)?;
        let mut settings = self.store.card_settings()?;
        settings.parameters = parameters;
        self.store.set_card_settings(&settings)?;
        Ok(used)
    }
}

/// New, learning and review counts for the cards in `order`.
fn count(order: &[CardId], cards: &[CardRow], today: i64) -> DueCounts {
    let mut counts = DueCounts {
        new: 0,
        learning: 0,
        review: 0,
    };
    let mut seen = vec![];
    for id in order {
        if seen.contains(id) {
            continue;
        }
        seen.push(*id);
        if let Some(card) = cards.iter().find(|c| c.id == *id) {
            if card.schedule.is_new() {
                counts.new += 1
            } else if card.schedule.is_learning(today) {
                counts.learning += 1
            } else {
                counts.review += 1
            }
        }
    }
    counts
}

/// Kept so the kind is visible to the frontend types.
pub const NOTE_KINDS: [NoteKind; 3] = [NoteKind::Basic, NoteKind::Reversed, NoteKind::Cloze];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OptionInput, QuestionInput, QuizSetupInput};
    use study_store::Store;

    const TODAY: i64 = 20_760;

    fn app() -> (App, TopicId) {
        let mut app = App::with_seed(Store::in_memory().unwrap(), 7);
        let topic = app.create_topic("CPU", None).unwrap();
        (app, topic)
    }

    fn basic(topic: TopicId, front: &str) -> NoteInput {
        NoteInput {
            note: Note {
                topic,
                kind: NoteKind::Basic,
                front: front.into(),
                back: "Back".into(),
                cues: vec![],
                text: String::new(),
                extra: String::new(),
            },
            suggestion: None,
        }
    }

    fn card(step: &ReviewStep) -> &ReviewCard {
        match step {
            ReviewStep::Card(card) => card,
            ReviewStep::Done(_) => panic!("expected a card"),
        }
    }

    // flashcards.feature: One note makes one or more cards, Notes need content
    #[test]
    fn notes_are_checked_and_blank_cues_dropped() {
        let (mut app, topic) = app();
        let mut input = basic(topic, "Front");
        input.note.cues = vec!["Cue".into(), "  ".into()];
        let id = app.save_note(None, input, TODAY).unwrap();
        assert_eq!(app.note_view(id, TODAY).unwrap().cards.len(), 2);
        let mut cloze = basic(topic, "");
        cloze.note.kind = NoteKind::Cloze;
        cloze.note.text = "no gaps".into();
        assert_eq!(
            app.save_note(None, cloze, TODAY).unwrap_err().message,
            "make at least one gap, like {{c1::answer}}"
        );
        let listed = app.topic_cards(topic, TODAY).unwrap();
        assert_eq!(
            (
                listed.due,
                listed.cards[0].kind.as_str(),
                listed.cards[0].is_new
            ),
            (2, "Basic", true)
        );
    }

    // flashcards.feature: Grading moves a card on, Again shows a card again in the same session, Undo
    #[test]
    fn a_session_grades_requeues_after_again_and_undoes() {
        let (mut app, topic) = app();
        for front in ["A", "B"] {
            app.save_note(None, basic(topic, front), TODAY).unwrap();
        }
        let first = app.start_review(None, TODAY).unwrap();
        let first = card(&first).clone();
        assert_eq!(
            (
                first.front.as_str(),
                first.left.new,
                first.queue.as_str(),
                first.can_undo
            ),
            ("A", 2, "new", false)
        );
        assert_eq!(first.intervals[0], 0);
        let second = app.grade_card(Rating::Again, 5, TODAY).unwrap();
        assert_eq!(card(&second).front, "B", "Again puts A after B");
        assert!(card(&second).can_undo);
        let undone = app.undo_card(TODAY).unwrap();
        assert_eq!(card(&undone).front, "A", "Undo shows the same card again");
        assert!(
            app.store.reviews_since(TODAY).unwrap().is_empty(),
            "and drops the answer"
        );

        app.grade_card(Rating::Again, 5, TODAY).unwrap();
        let back_to_a = app.grade_card(Rating::Good, 4, TODAY).unwrap();
        assert_eq!(
            (
                card(&back_to_a).front.as_str(),
                card(&back_to_a).queue.as_str()
            ),
            ("A", "learning")
        );
        let done = app.grade_card(Rating::Good, 3, TODAY).unwrap();
        let ReviewStep::Done(done) = done else {
            panic!("expected done")
        };
        assert_eq!(
            (
                done.reviewed,
                done.new,
                done.again,
                done.remembered,
                done.seconds
            ),
            (3, 2, 1, Some(67), 12)
        );
        assert_eq!(
            app.cards_due_count(TODAY).unwrap(),
            0,
            "nothing due once reviewed"
        );
        let home = app.cards_home(TODAY).unwrap();
        assert_eq!(
            (home.due.new, home.total_cards, home.forecast.len()),
            (0, 2, 14)
        );
    }

    // flashcards.feature: Daily limits
    #[test]
    fn new_cards_a_day_limits_the_session() {
        let (mut app, topic) = app();
        for i in 0..15 {
            app.save_note(None, basic(topic, &format!("Card {i}")), TODAY)
                .unwrap();
        }
        let step = app.start_review(None, TODAY).unwrap();
        assert_eq!(card(&step).total, 10);
        assert_eq!(app.cards_home(TODAY).unwrap().due.new, 10);
    }

    // flashcards.feature: Archived cards are left out
    #[test]
    fn archived_cards_are_left_out() {
        let (mut app, topic) = app();
        let id = app.save_note(None, basic(topic, "A"), TODAY).unwrap();
        let card_id = app.note_view(id, TODAY).unwrap().cards[0].id;
        app.archive_card(card_id, true).unwrap();
        assert_eq!(app.cards_due_count(TODAY).unwrap(), 0);
        assert!(matches!(
            app.start_review(None, TODAY).unwrap(),
            ReviewStep::Done(_)
        ));
        app.archive_card(card_id, false).unwrap();
        assert_eq!(app.cards_due_count(TODAY).unwrap(), 1);
    }

    // flashcards.feature: Suggestions from recalls and quizzes
    #[test]
    fn suggestions_come_from_recall_misses_and_quiz_mistakes() {
        let (mut app, topic) = app();
        let checklist = app.create_checklist(Subject::Topic(topic), TODAY).unwrap();
        app.save_points(
            checklist,
            vec![
                crate::recall::PointInput {
                    text: "Memory-mapped I/O".into(),
                    question: None,
                },
                crate::recall::PointInput {
                    text: "Pipelines".into(),
                    question: None,
                },
            ],
        )
        .unwrap();
        app.finish_check(checklist, None, vec![1], TODAY).unwrap();
        app.save_question(QuestionInput {
            topic,
            text: "What does the TLB cache?".into(),
            multiple: false,
            options: ["Page translations", "Instructions", "Data"]
                .iter()
                .enumerate()
                .map(|(i, t)| OptionInput {
                    text: (*t).into(),
                    correct: i == 0,
                })
                .collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
        })
        .unwrap();
        app.start_quiz(QuizSetupInput {
            topics: vec![topic],
            count: None,
            even: false,
            shuffle_questions: false,
            shuffle_options: false,
            exam: false,
            total_minutes: None,
            per_question_seconds: None,
            easy: false,
        })
        .unwrap();
        app.answer(vec![1], TODAY).unwrap();

        let suggestions = app.card_suggestions(TODAY).unwrap();
        assert_eq!(suggestions.len(), 2);
        let quiz = suggestions
            .iter()
            .find(|s| s.kind == SuggestionKind::Quiz)
            .unwrap();
        assert_eq!(
            (
                quiz.text.as_str(),
                quiz.answer.as_deref(),
                quiz.note.as_str()
            ),
            (
                "What does the TLB cache?",
                Some("Page translations"),
                "Wrong, you picked “Instructions”"
            )
        );
        let recall = suggestions
            .iter()
            .find(|s| s.kind == SuggestionKind::Recall)
            .unwrap();
        assert_eq!(
            (recall.text.as_str(), recall.topic, recall.note.as_str()),
            ("Memory-mapped I/O", Some(topic), "Missed 1 of 1 check")
        );

        let mut made = basic(topic, &quiz.text);
        made.note.back = quiz.answer.clone().unwrap();
        made.suggestion = Some(quiz.key.clone());
        let note = app.save_note(None, made, TODAY).unwrap();
        assert_eq!(
            app.topic_cards(topic, TODAY)
                .unwrap()
                .cards
                .iter()
                .find(|c| c.note == note)
                .unwrap()
                .source
                .as_deref(),
            Some("From quiz mistake")
        );
        app.dismiss_suggestion(&recall.key).unwrap();
        assert!(app.card_suggestions(TODAY).unwrap().is_empty());
    }

    // flashcards.feature: A card that keeps slipping is flagged
    #[test]
    fn leeches_show_in_review_and_editing_clears_them() {
        let (mut app, topic) = app();
        let id = app
            .save_note(None, basic(topic, "Slippery"), TODAY)
            .unwrap();
        app.set_card_settings(CardSettings {
            leech_after: 2,
            ..CardSettings::default()
        })
        .unwrap();
        let mut day = TODAY;
        for _ in 0..2 {
            app.start_review(None, day).unwrap();
            app.grade_card(Rating::Good, 1, day).unwrap();
            day = app.note_view(id, day).unwrap().cards[0].due;
            app.start_review(None, day).unwrap();
            app.grade_card(Rating::Again, 1, day).unwrap();
            app.grade_card(Rating::Good, 1, day).unwrap();
            day = app.note_view(id, day).unwrap().cards[0].due;
        }
        let step = app.start_review(None, day).unwrap();
        assert!(card(&step).leech);
        assert!(app.topic_cards(topic, day).unwrap().cards[0].leech);
        let due = app.note_view(id, day).unwrap().cards[0].due;
        let mut edited = basic(topic, "Less slippery");
        edited.note.back = "Back".into();
        app.save_note(Some(id), edited, day).unwrap();
        let after = &app.note_view(id, day).unwrap().cards[0];
        assert_eq!(
            (after.leech, after.due),
            (false, due),
            "flag cleared, schedule kept"
        );
    }

    // flashcards.feature: Fitting the scheduler needs history
    #[test]
    fn fitting_says_when_there_isnt_enough_history() {
        let (mut app, _) = app();
        assert_eq!(
            app.fit_scheduler().unwrap_err().message,
            "fitting the scheduler needs at least 50 reviews, you have 0 so far"
        );
        assert_eq!(
            app.set_card_settings(CardSettings {
                desired_retention: 0.5,
                ..CardSettings::default()
            })
            .unwrap_err()
            .message,
            "how much to remember has to be between 70% and 99%"
        );
        assert_eq!(app.reset_card_settings().unwrap(), CardSettings::default());
    }
}

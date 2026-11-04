//! Flashcard storage (R62 to R77): notes, their cards with FSRS state, and the review log.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use study_core::cards::{CardSettings, Face, Memory, Note, Rating, Schedule};
use study_core::question::TopicId;

use crate::{Result, Store, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NoteId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CardId(pub i64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardRow {
    pub id: CardId,
    pub note: NoteId,
    pub topic: TopicId,
    pub key: String,
    pub front: String,
    pub back: String,
    pub schedule: Schedule,
    pub archived: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReviewRow {
    pub card: CardId,
    pub day: i64,
    pub rating: u8,
    pub elapsed: u32,
    pub was_new: bool,
    pub seconds: u32,
}

const SETTINGS_KEY: &str = "card_settings";

pub(crate) fn rating_from(value: i64) -> Rating {
    match value {
        1 => Rating::Again,
        2 => Rating::Hard,
        4 => Rating::Easy,
        _ => Rating::Good,
    }
}

impl Store {
    /// Saves a note and its cards. New cards are due `today`. On an edit, cards whose key
    /// still exists keep their schedule, edited ones lose their leech flag (R70, R77), and
    /// cards whose cue or gap is gone are removed.
    pub fn save_note(
        &mut self,
        id: Option<NoteId>,
        note: &Note,
        faces: &[Face],
        source: Option<&str>,
        today: i64,
    ) -> Result<NoteId> {
        let transaction = self.connection.transaction()?;
        let json = serde_json::to_string(note)?;
        let id = match id {
            Some(id) => {
                if transaction.execute(
                    "UPDATE notes SET topic_id = ?2, note = ?3 WHERE id = ?1",
                    params![id.0, note.topic.0, json],
                )? == 0
                {
                    return Err(StoreError::NotFound("note"));
                }
                id
            }
            None => {
                transaction.execute(
                    "INSERT INTO notes (topic_id, note, source, created) VALUES (?1, ?2, ?3, ?4)",
                    params![note.topic.0, json, source, today],
                )?;
                NoteId(transaction.last_insert_rowid())
            }
        };
        let keys: Vec<&str> = faces.iter().map(|f| f.key.as_str()).collect();
        let existing: Vec<(i64, String, String, String)> = {
            let mut statement =
                transaction.prepare("SELECT id, key, front, back FROM cards WHERE note_id = ?1")?;
            statement
                .query_map([id.0], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })?
                .collect::<rusqlite::Result<_>>()?
        };
        for (card, key, _, _) in &existing {
            if !keys.contains(&key.as_str()) {
                transaction.execute("DELETE FROM cards WHERE id = ?1", [card])?;
            }
        }
        for face in faces {
            match existing.iter().find(|(_, key, _, _)| key == &face.key) {
                Some((card, _, front, back)) => {
                    let changed = front != &face.front || back != &face.back;
                    transaction.execute(
                        "UPDATE cards SET front = ?2, back = ?3, lapses = CASE WHEN ?4 THEN 0 ELSE lapses END WHERE id = ?1",
                        params![card, face.front, face.back, changed],
                    )?;
                }
                None => {
                    transaction.execute("INSERT INTO cards (note_id, key, front, back, due) VALUES (?1, ?2, ?3, ?4, ?5)", params![id.0, face.key, face.front, face.back, today])?;
                }
            }
        }
        transaction.commit()?;
        Ok(id)
    }

    pub fn note(&self, id: NoteId) -> Result<(Note, Option<String>)> {
        let (json, source): (String, Option<String>) = self
            .connection
            .query_row(
                "SELECT note, source FROM notes WHERE id = ?1",
                [id.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or(StoreError::NotFound("note"))?;
        Ok((serde_json::from_str(&json)?, source))
    }

    pub fn delete_note(&mut self, id: NoteId) -> Result<()> {
        if self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", [id.0])?
            == 0
        {
            return Err(StoreError::NotFound("note"));
        }
        Ok(())
    }

    fn card_rows(&self, filter: &str, values: &[&dyn rusqlite::ToSql]) -> Result<Vec<CardRow>> {
        let sql = format!(
            "SELECT c.id, c.note_id, n.topic_id, c.key, c.front, c.back, c.stability, c.difficulty, c.due, c.last_review, c.reps, c.lapses, c.archived, n.source
             FROM cards c JOIN notes n ON n.id = c.note_id WHERE {filter} ORDER BY c.due, c.id"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(values, |row| {
            let stability: Option<f64> = row.get(6)?;
            let difficulty: Option<f64> = row.get(7)?;
            Ok(CardRow {
                id: CardId(row.get(0)?),
                note: NoteId(row.get(1)?),
                topic: TopicId(row.get(2)?),
                key: row.get(3)?,
                front: row.get(4)?,
                back: row.get(5)?,
                schedule: Schedule {
                    memory: stability.zip(difficulty).map(|(s, d)| Memory {
                        stability: s as f32,
                        difficulty: d as f32,
                    }),
                    due: row.get(8)?,
                    last_review: row.get(9)?,
                    reps: row.get::<_, i64>(10)? as u32,
                    lapses: row.get::<_, i64>(11)? as u32,
                },
                archived: row.get(12)?,
                source: row.get(13)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every card, soonest due first.
    pub fn cards(&self) -> Result<Vec<CardRow>> {
        self.card_rows("1 = 1", params![])
    }

    pub fn cards_in(&self, topic: TopicId) -> Result<Vec<CardRow>> {
        self.card_rows("n.topic_id = ?1", params![topic.0])
    }

    pub fn cards_of(&self, note: NoteId) -> Result<Vec<CardRow>> {
        self.card_rows("c.note_id = ?1", params![note.0])
    }

    pub fn card(&self, id: CardId) -> Result<CardRow> {
        self.card_rows("c.id = ?1", params![id.0])?
            .pop()
            .ok_or(StoreError::NotFound("card"))
    }

    /// R71: archived cards are left out of reviews.
    pub fn set_archived(&mut self, id: CardId, archived: bool) -> Result<()> {
        if self.connection.execute(
            "UPDATE cards SET archived = ?2 WHERE id = ?1",
            params![id.0, archived],
        )? == 0
        {
            return Err(StoreError::NotFound("card"));
        }
        Ok(())
    }

    /// Saves an answer: the card's new schedule and a line in the review log. Returns the log id.
    pub fn record_review(
        &mut self,
        id: CardId,
        schedule: &Schedule,
        review: &ReviewRow,
    ) -> Result<i64> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE cards SET stability = ?2, difficulty = ?3, due = ?4, last_review = ?5, reps = ?6, lapses = ?7 WHERE id = ?1",
            params![id.0, schedule.memory.map(|m| m.stability as f64), schedule.memory.map(|m| m.difficulty as f64), schedule.due, schedule.last_review, schedule.reps, schedule.lapses],
        )?;
        if changed == 0 {
            return Err(StoreError::NotFound("card"));
        }
        transaction.execute(
            "INSERT INTO card_reviews (card_id, day, rating, elapsed, was_new, seconds) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id.0, review.day, review.rating, review.elapsed, review.was_new, review.seconds],
        )?;
        let log = transaction.last_insert_rowid();
        transaction.commit()?;
        Ok(log)
    }

    /// R67: puts the card back as it was and drops the log line.
    pub fn undo_review(&mut self, id: CardId, previous: &Schedule, log: i64) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE cards SET stability = ?2, difficulty = ?3, due = ?4, last_review = ?5, reps = ?6, lapses = ?7 WHERE id = ?1",
            params![id.0, previous.memory.map(|m| m.stability as f64), previous.memory.map(|m| m.difficulty as f64), previous.due, previous.last_review, previous.reps, previous.lapses],
        )?;
        transaction.execute(
            "DELETE FROM card_reviews WHERE id = ?1 AND card_id = ?2",
            params![log, id.0],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn reviews_since(&self, day: i64) -> Result<Vec<ReviewRow>> {
        let mut statement = self.connection.prepare("SELECT card_id, day, rating, elapsed, was_new, seconds FROM card_reviews WHERE day >= ?1 ORDER BY id")?;
        let rows = statement.query_map([day], |row| {
            Ok(ReviewRow {
                card: CardId(row.get(0)?),
                day: row.get(1)?,
                rating: row.get::<_, i64>(2)? as u8,
                elapsed: row.get::<_, i64>(3)? as u32,
                was_new: row.get(4)?,
                seconds: row.get::<_, i64>(5)? as u32,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every card's answers in order, for fitting the scheduler (R75).
    pub fn review_histories(&self) -> Result<Vec<Vec<(Rating, u32)>>> {
        let mut statement = self
            .connection
            .prepare("SELECT card_id, rating, elapsed FROM card_reviews ORDER BY card_id, id")?;
        let rows: Vec<(i64, i64, i64)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut histories: Vec<Vec<(Rating, u32)>> = vec![];
        let mut last = None;
        for (card, rating, elapsed) in rows {
            if last != Some(card) {
                histories.push(vec![]);
                last = Some(card);
            }
            histories
                .last_mut()
                .expect("pushed")
                .push((rating_from(rating), elapsed as u32));
        }
        Ok(histories)
    }

    pub fn card_settings(&self) -> Result<CardSettings> {
        let saved: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [SETTINGS_KEY],
                |row| row.get(0),
            )
            .optional()?;
        Ok(match saved {
            Some(json) => serde_json::from_str(&json)?,
            None => CardSettings::default(),
        })
    }

    pub fn set_card_settings(&mut self, settings: &CardSettings) -> Result<()> {
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SETTINGS_KEY, serde_json::to_string(settings)?],
        )?;
        Ok(())
    }

    pub fn reset_card_settings(&mut self) -> Result<()> {
        self.connection
            .execute("DELETE FROM settings WHERE key = ?1", [SETTINGS_KEY])?;
        Ok(())
    }

    /// Suggestion keys that were made into cards or dismissed (R72).
    pub fn suggestions_done(&self) -> Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT key FROM card_suggestions_done")?;
        Ok(statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn mark_suggestion(&mut self, key: &str, made: bool) -> Result<()> {
        self.connection.execute(
            "INSERT INTO card_suggestions_done (key, status) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET status = excluded.status",
            params![key, if made { "made" } else { "dismissed" }],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use study_core::cards::NoteKind;

    const TODAY: i64 = 20_760;

    fn basic(topic: TopicId, cues: &[&str]) -> Note {
        Note {
            topic,
            kind: NoteKind::Basic,
            front: "Front".into(),
            back: "Back".into(),
            cues: cues.iter().map(|c| c.to_string()).collect(),
            text: String::new(),
            extra: String::new(),
        }
    }

    fn setup() -> (Store, TopicId) {
        let mut store = Store::in_memory().unwrap();
        let topic = store.create_topic("CPU", None).unwrap();
        (store, topic)
    }

    // flashcards.feature: One note makes one or more cards
    #[test]
    fn a_note_saves_its_cards_new_and_due_today() {
        let (mut store, topic) = setup();
        let note = basic(topic, &["Cue"]);
        let id = store
            .save_note(None, &note, &note.faces().unwrap(), Some("quiz:7"), TODAY)
            .unwrap();
        let cards = store.cards_of(id).unwrap();
        assert_eq!(cards.len(), 2);
        assert!(cards.iter().all(|c| c.schedule.is_new()
            && c.schedule.due == TODAY
            && c.source.as_deref() == Some("quiz:7")));
        assert_eq!(store.note(id).unwrap().0, note);
        assert_eq!(store.cards_in(topic).unwrap().len(), 2);
    }

    // flashcards.feature: A card that keeps slipping is flagged (editing clears it), R77
    #[test]
    fn editing_keeps_schedules_clears_lapses_on_changed_cards_and_drops_removed_ones() {
        let (mut store, topic) = setup();
        let note = basic(topic, &["Cue A", "Cue B"]);
        let id = store
            .save_note(None, &note, &note.faces().unwrap(), None, TODAY)
            .unwrap();
        let main = store.cards_of(id).unwrap()[0].clone();
        let reviewed = Schedule {
            memory: Some(Memory {
                stability: 4.0,
                difficulty: 6.0,
            }),
            due: TODAY + 4,
            last_review: Some(TODAY),
            reps: 3,
            lapses: 8,
        };
        store
            .record_review(
                main.id,
                &reviewed,
                &ReviewRow {
                    card: main.id,
                    day: TODAY,
                    rating: 3,
                    elapsed: 0,
                    was_new: true,
                    seconds: 5,
                },
            )
            .unwrap();

        let edited = Note {
            front: "Better front".into(),
            cues: vec!["Cue A".into()],
            ..note
        };
        store
            .save_note(Some(id), &edited, &edited.faces().unwrap(), None, TODAY + 1)
            .unwrap();
        let cards = store.cards_of(id).unwrap();
        assert_eq!(cards.len(), 2, "the second cue's card is gone");
        let main = cards.iter().find(|c| c.key == "main").unwrap();
        assert_eq!(
            (main.front.as_str(), main.schedule.due, main.schedule.lapses),
            ("Better front", TODAY + 4, 0)
        );
    }

    // flashcards.feature: Undo the last answer, Archived cards are left out
    #[test]
    fn reviews_can_be_undone_and_cards_archived() {
        let (mut store, topic) = setup();
        let note = basic(topic, &[]);
        let id = store
            .save_note(None, &note, &note.faces().unwrap(), None, TODAY)
            .unwrap();
        let card = store.cards_of(id).unwrap()[0].clone();
        let next = Schedule {
            memory: Some(Memory {
                stability: 2.0,
                difficulty: 5.0,
            }),
            due: TODAY + 2,
            last_review: Some(TODAY),
            reps: 1,
            lapses: 0,
        };
        let log = store
            .record_review(
                card.id,
                &next,
                &ReviewRow {
                    card: card.id,
                    day: TODAY,
                    rating: 3,
                    elapsed: 0,
                    was_new: true,
                    seconds: 4,
                },
            )
            .unwrap();
        assert_eq!(store.reviews_since(TODAY).unwrap().len(), 1);
        assert_eq!(
            store.review_histories().unwrap(),
            vec![vec![(Rating::Good, 0)]]
        );
        store.undo_review(card.id, &card.schedule, log).unwrap();
        assert_eq!(store.card(card.id).unwrap().schedule, card.schedule);
        assert!(store.reviews_since(TODAY).unwrap().is_empty());
        store.set_archived(card.id, true).unwrap();
        assert!(store.card(card.id).unwrap().archived);
    }

    #[test]
    fn settings_and_suggestions_persist() {
        let (mut store, _) = setup();
        assert_eq!(store.card_settings().unwrap(), CardSettings::default());
        store
            .set_card_settings(&CardSettings {
                new_per_day: 5,
                ..CardSettings::default()
            })
            .unwrap();
        assert_eq!(store.card_settings().unwrap().new_per_day, 5);
        store.reset_card_settings().unwrap();
        assert_eq!(store.card_settings().unwrap().new_per_day, 10);
        store.mark_suggestion("quiz:3", true).unwrap();
        store.mark_suggestion("recall:1:A", false).unwrap();
        let mut done = store.suggestions_done().unwrap();
        done.sort();
        assert_eq!(done, vec!["quiz:3", "recall:1:A"]);
    }
}

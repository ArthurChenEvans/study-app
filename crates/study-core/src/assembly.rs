//! Setting up a quiz: R15 (topics, all or a set number, random or even),
//! R16 (question and option order), R17 (timers), R18 (mode), R19 (difficulty).
//!
//! Open questions Q2, Q3 and Q4 in specs/README.md are not decided, so those
//! cases return `AssemblyError::Undecided` instead of guessing.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use rand::Rng;
use rand::seq::SliceRandom;
use thiserror::Error;

use crate::question::{Question, QuestionId, TopicId};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    All,
    Count(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Random,
    Even,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Keep,
    Shuffle,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Study,
    Exam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Hard,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuizSettings {
    pub topics: Vec<TopicId>,
    pub amount: Amount,
    pub pick: Pick,
    pub question_order: Order,
    pub option_order: Order,
    pub mode: Mode,
    pub total_timer: Option<Duration>,
    pub per_question_timer: Option<Duration>,
    pub difficulty: Difficulty,
}

impl QuizSettings {
    /// Settings with only the topics chosen. Everything else starts from the
    /// setup screen's defaults, including hard difficulty (R19).
    pub fn for_topics(topics: Vec<TopicId>) -> Self {
        Self {
            topics,
            amount: Amount::All,
            pick: Pick::Random,
            question_order: Order::Keep,
            option_order: Order::Keep,
            mode: Mode::Study,
            total_timer: None,
            per_question_timer: None,
            difficulty: Difficulty::default(),
        }
    }
}

/// One question as it will be asked: which question, and the order its options are shown in.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuizItem {
    pub question: QuestionId,
    pub option_order: Vec<usize>,
}

/// An open question from specs/README.md that blocks a behaviour.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenQuestion {
    /// Q2: more questions asked for than the chosen topics have.
    MoreThanAvailable,
    /// Q3: an even split that doesn't divide, or a topic short of its share.
    UnevenSplit,
    /// Q4: a chosen topic with no questions.
    EmptyTopic,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblyError {
    #[error("pick at least one topic to start")]
    NoTopics,
    #[error("ask for at least one question")]
    ZeroQuestions,
    #[error("not decided yet in the specs: {0:?}")]
    Undecided(OpenQuestion),
}

/// A question in the pool the quiz draws from.
pub struct PoolEntry<'a> {
    pub id: QuestionId,
    pub question: &'a Question,
}

pub fn assemble<R: Rng + ?Sized>(
    pool: &[PoolEntry],
    settings: &QuizSettings,
    rng: &mut R,
) -> Result<Vec<QuizItem>, AssemblyError> {
    if settings.topics.is_empty() {
        return Err(AssemblyError::NoTopics);
    }
    if settings.amount == Amount::Count(0) {
        return Err(AssemblyError::ZeroQuestions);
    }

    // Questions per chosen topic, in the order the topics were chosen, each sorted by id.
    let per_topic: Vec<Vec<&PoolEntry>> = settings
        .topics
        .iter()
        .map(|topic| {
            let mut entries: Vec<&PoolEntry> = pool
                .iter()
                .filter(|entry| entry.question.topic() == *topic)
                .collect();
            entries.sort_by_key(|entry| entry.id);
            entries
        })
        .collect();
    if per_topic.iter().any(Vec::is_empty) {
        return Err(AssemblyError::Undecided(OpenQuestion::EmptyTopic));
    }
    let available: usize = per_topic.iter().map(Vec::len).sum();

    let mut chosen: Vec<&PoolEntry> = match settings.amount {
        Amount::All => per_topic.into_iter().flatten().collect(),
        Amount::Count(count) if count > available => {
            return Err(AssemblyError::Undecided(OpenQuestion::MoreThanAvailable));
        }
        Amount::Count(count) => match settings.pick {
            Pick::Random => {
                let mut all: Vec<&PoolEntry> = per_topic.into_iter().flatten().collect();
                all.shuffle(rng);
                all.truncate(count);
                all
            }
            Pick::Even => {
                let topics = settings.topics.len();
                let share = count / topics;
                if count % topics != 0 || per_topic.iter().any(|entries| entries.len() < share) {
                    return Err(AssemblyError::Undecided(OpenQuestion::UnevenSplit));
                }
                per_topic
                    .into_iter()
                    .flat_map(|mut entries| {
                        entries.shuffle(rng);
                        entries.truncate(share);
                        entries
                    })
                    .collect()
            }
        },
    };

    match settings.question_order {
        Order::Keep => {
            let rank = |entry: &PoolEntry| {
                settings
                    .topics
                    .iter()
                    .position(|topic| *topic == entry.question.topic())
            };
            chosen.sort_by_key(|entry| (rank(entry), entry.id));
        }
        Order::Shuffle => chosen.shuffle(rng),
    }

    Ok(chosen
        .into_iter()
        .map(|entry| {
            let mut option_order: Vec<usize> = (0..entry.question.options().len()).collect();
            if settings.option_order == Order::Shuffle {
                option_order.shuffle(rng);
            }
            QuizItem {
                question: entry.id,
                option_order,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::question::{AnswerKind, AnswerOption, QuestionDraft};
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashSet;

    const CPU: TopicId = TopicId(1);
    const GPU: TopicId = TopicId(2);
    const EMPTY: TopicId = TopicId(3);

    fn question(topic: TopicId) -> Question {
        Question::new(QuestionDraft {
            topic,
            text: "Q".into(),
            kind: AnswerKind::Single,
            options: ["A", "B", "C", "D"]
                .iter()
                .enumerate()
                .map(|(index, text)| AnswerOption {
                    text: (*text).into(),
                    correct: index == 0,
                })
                .collect(),
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
        })
        .unwrap()
    }

    /// 10 CPU questions (ids 1..=10) and 10 GPU questions (ids 11..=20), as in quiz-setup.feature.
    fn questions() -> Vec<(QuestionId, Question)> {
        (1..=20)
            .map(|id| (QuestionId(id), question(if id <= 10 { CPU } else { GPU })))
            .collect()
    }

    fn pool(questions: &[(QuestionId, Question)]) -> Vec<PoolEntry<'_>> {
        questions
            .iter()
            .map(|(id, question)| PoolEntry { id: *id, question })
            .collect()
    }

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn topic_of(id: QuestionId) -> TopicId {
        if id.0 <= 10 { CPU } else { GPU }
    }

    // quiz-setup.feature: Quiz on several topics
    #[test]
    fn all_questions_from_several_topics() {
        let questions = questions();
        let items = assemble(
            &pool(&questions),
            &QuizSettings::for_topics(vec![CPU, GPU]),
            &mut rng(),
        )
        .unwrap();
        assert_eq!(items.len(), 20);
        let topics: HashSet<_> = items.iter().map(|item| topic_of(item.question)).collect();
        assert_eq!(topics, HashSet::from([CPU, GPU]));
    }

    #[test]
    fn only_chosen_topics_are_used() {
        let questions = questions();
        let items = assemble(
            &pool(&questions),
            &QuizSettings::for_topics(vec![GPU]),
            &mut rng(),
        )
        .unwrap();
        assert_eq!(items.len(), 10);
        assert!(items.iter().all(|item| topic_of(item.question) == GPU));
    }

    // quiz-setup.feature: All questions or a set number (outline)
    #[test]
    fn set_number_of_questions() {
        let questions = questions();
        let settings = QuizSettings {
            amount: Amount::Count(8),
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        let items = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        assert_eq!(items.len(), 8);
        let unique: HashSet<_> = items.iter().map(|item| item.question).collect();
        assert_eq!(unique.len(), 8, "no question appears twice");
    }

    // quiz-setup.feature: Random pick across topics
    #[test]
    fn random_pick_draws_from_both_topics_combined() {
        let questions = questions();
        let settings = QuizSettings {
            amount: Amount::Count(8),
            pick: Pick::Random,
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        let mut seen = HashSet::new();
        for seed in 0..20 {
            let items = assemble(
                &pool(&questions),
                &settings,
                &mut StdRng::seed_from_u64(seed),
            )
            .unwrap();
            seen.extend(items.iter().map(|item| item.question));
        }
        assert!(seen.len() > 8, "different seeds draw different questions");
    }

    // quiz-setup.feature: Even split across topics
    #[test]
    fn even_pick_takes_the_same_number_from_each_topic() {
        let questions = questions();
        let settings = QuizSettings {
            amount: Amount::Count(8),
            pick: Pick::Even,
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        let items = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        let cpu = items
            .iter()
            .filter(|item| topic_of(item.question) == CPU)
            .count();
        assert_eq!((cpu, items.len() - cpu), (4, 4));
    }

    // quiz-setup.feature: Order options (outline)
    #[test]
    fn kept_question_order_follows_topic_then_question() {
        let questions = questions();
        let items = assemble(
            &pool(&questions),
            &QuizSettings::for_topics(vec![GPU, CPU]),
            &mut rng(),
        )
        .unwrap();
        let ids: Vec<i64> = items.iter().map(|item| item.question.0).collect();
        let expected: Vec<i64> = (11..=20).chain(1..=10).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn shuffled_question_order_differs_from_kept() {
        let questions = questions();
        let settings = QuizSettings {
            question_order: Order::Shuffle,
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        let items = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        let ids: Vec<i64> = items.iter().map(|item| item.question.0).collect();
        assert_ne!(ids, (1..=20).collect::<Vec<_>>());
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            (1..=20).collect::<Vec<_>>(),
            "same questions, different order"
        );
    }

    #[test]
    fn option_order_kept_or_shuffled() {
        let questions = questions();
        let kept = assemble(
            &pool(&questions),
            &QuizSettings::for_topics(vec![CPU]),
            &mut rng(),
        )
        .unwrap();
        assert!(
            kept.iter()
                .all(|item| item.option_order == vec![0, 1, 2, 3])
        );

        let settings = QuizSettings {
            option_order: Order::Shuffle,
            ..QuizSettings::for_topics(vec![CPU])
        };
        let shuffled = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        assert!(
            shuffled
                .iter()
                .any(|item| item.option_order != vec![0, 1, 2, 3])
        );
        for item in &shuffled {
            let mut sorted = item.option_order.clone();
            sorted.sort();
            assert_eq!(sorted, vec![0, 1, 2, 3], "every option still shown once");
        }
    }

    // quiz-setup.feature: Hard mode is the default
    #[test]
    fn hard_is_the_default_difficulty() {
        assert_eq!(
            QuizSettings::for_topics(vec![CPU]).difficulty,
            Difficulty::Hard
        );
    }

    #[test]
    fn needs_at_least_one_topic() {
        let questions = questions();
        assert_eq!(
            assemble(
                &pool(&questions),
                &QuizSettings::for_topics(vec![]),
                &mut rng()
            ),
            Err(AssemblyError::NoTopics)
        );
    }

    #[test]
    fn needs_at_least_one_question() {
        let questions = questions();
        let settings = QuizSettings {
            amount: Amount::Count(0),
            ..QuizSettings::for_topics(vec![CPU])
        };
        assert_eq!(
            assemble(&pool(&questions), &settings, &mut rng()),
            Err(AssemblyError::ZeroQuestions)
        );
    }

    // Open questions: no behaviour is invented, the caller is told it's undecided.
    #[test]
    fn open_question_cases_are_reported_not_guessed() {
        let questions = questions();
        let pool = pool(&questions);
        let more = QuizSettings {
            amount: Amount::Count(21),
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        assert_eq!(
            assemble(&pool, &more, &mut rng()),
            Err(AssemblyError::Undecided(OpenQuestion::MoreThanAvailable))
        );

        let uneven = QuizSettings {
            amount: Amount::Count(7),
            pick: Pick::Even,
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        assert_eq!(
            assemble(&pool, &uneven, &mut rng()),
            Err(AssemblyError::Undecided(OpenQuestion::UnevenSplit))
        );

        let empty = QuizSettings::for_topics(vec![CPU, EMPTY]);
        assert_eq!(
            assemble(&pool, &empty, &mut rng()),
            Err(AssemblyError::Undecided(OpenQuestion::EmptyTopic))
        );
    }
}

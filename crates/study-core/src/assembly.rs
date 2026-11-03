//! Setting up a quiz: R15 (topics, all or a set number, random or even),
//! R16 (question and option order), R17 (timers), R18 (mode), R19 (difficulty).
//!
//! R36: asking for more than exist uses all of them.
//! R37: an even split gives extras to topics in order; short topics are made up by others.
//! R38: topics with no questions are refused.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use rand::Rng;
use rand::seq::SliceRandom;
use thiserror::Error;

use crate::question::{AnswerKind, Question, QuestionId, TopicId};

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
    #[serde(default)]
    pub types: QuestionTypes,
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
            types: QuestionTypes::default(),
        }
    }
}

/// One question as it will be asked: which question, and the order its options are shown in.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuizItem {
    pub question: QuestionId,
    pub option_order: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblyError {
    #[error("pick at least one topic to start")]
    NoTopics,
    #[error("ask for at least one question")]
    ZeroQuestions,
    #[error("a chosen topic has no questions yet")]
    EmptyTopic(TopicId),
    #[error("a chosen topic has no questions of that type yet")]
    NoneOfType(TopicId),
}

/// Which kinds of question a quiz asks (R85).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuestionTypes {
    #[default]
    Both,
    Mcq,
    Short,
}

impl QuestionTypes {
    pub fn includes(self, kind: AnswerKind) -> bool {
        match self {
            Self::Both => true,
            Self::Mcq => kind != AnswerKind::Short,
            Self::Short => kind == AnswerKind::Short,
        }
    }
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
    if let Some(index) = per_topic.iter().position(Vec::is_empty) {
        return Err(AssemblyError::EmptyTopic(settings.topics[index]));
    }
    // R85: only the question types asked for.
    let per_topic: Vec<Vec<&PoolEntry>> = per_topic
        .into_iter()
        .map(|entries| entries.into_iter().filter(|entry| settings.types.includes(entry.question.kind())).collect())
        .collect();
    if let Some(index) = per_topic.iter().position(Vec::is_empty) {
        return Err(AssemblyError::NoneOfType(settings.topics[index]));
    }
    let available: usize = per_topic.iter().map(Vec::len).sum();

    let mut chosen: Vec<&PoolEntry> = match settings.amount {
        Amount::All => per_topic.into_iter().flatten().collect(),
        Amount::Count(count) if count >= available => per_topic.into_iter().flatten().collect(),
        Amount::Count(count) => match settings.pick {
            Pick::Random => {
                let mut all: Vec<&PoolEntry> = per_topic.into_iter().flatten().collect();
                all.shuffle(rng);
                all.truncate(count);
                all
            }
            Pick::Even => {
                // Hand out one question per topic in turn, skipping topics that have run out.
                let mut shares = vec![0usize; per_topic.len()];
                let mut left = count;
                while left > 0 {
                    for (index, entries) in per_topic.iter().enumerate() {
                        if left > 0 && shares[index] < entries.len() {
                            shares[index] += 1;
                            left -= 1;
                        }
                    }
                }
                per_topic
                    .into_iter()
                    .zip(shares)
                    .flat_map(|(mut entries, share)| {
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
    use crate::question::{AnswerOption, QuestionDraft};
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
            short: None,
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

    fn count_per_topic(items: &[QuizItem], topic: fn(QuestionId) -> TopicId, t: TopicId) -> usize {
        items.iter().filter(|item| topic(item.question) == t).count()
    }

    // R36: more than available uses all of them
    #[test]
    fn more_than_available_uses_all() {
        let questions = questions();
        let settings = QuizSettings {
            amount: Amount::Count(25),
            ..QuizSettings::for_topics(vec![CPU, GPU])
        };
        let items = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        assert_eq!(items.len(), 20);
        let even = QuizSettings { pick: Pick::Even, ..settings };
        assert_eq!(assemble(&pool(&questions), &even, &mut rng()).unwrap().len(), 20);
    }

    const MEM: TopicId = TopicId(4);
    fn topic3(id: QuestionId) -> TopicId {
        if id.0 <= 10 { CPU } else if id.0 <= 20 { GPU } else { MEM }
    }
    fn three_topics(mem: i64) -> Vec<(QuestionId, Question)> {
        let mut all = questions();
        all.extend((21..21 + mem).map(|id| (QuestionId(id), question(MEM))));
        all
    }

    // R37: extras go to topics in order
    #[test]
    fn even_split_gives_extras_in_order() {
        let questions = three_topics(10);
        let settings = QuizSettings {
            amount: Amount::Count(10),
            pick: Pick::Even,
            ..QuizSettings::for_topics(vec![CPU, GPU, MEM])
        };
        let items = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        let counts = [CPU, GPU, MEM].map(|t| count_per_topic(&items, topic3, t));
        assert_eq!(counts, [4, 3, 3]);
    }

    // R37: a short topic gives all it has, the others make up the rest
    #[test]
    fn short_topic_is_made_up_by_others() {
        let questions = three_topics(1);
        let settings = QuizSettings {
            amount: Amount::Count(9),
            pick: Pick::Even,
            ..QuizSettings::for_topics(vec![CPU, GPU, MEM])
        };
        let items = assemble(&pool(&questions), &settings, &mut rng()).unwrap();
        let counts = [CPU, GPU, MEM].map(|t| count_per_topic(&items, topic3, t));
        assert_eq!(counts, [4, 4, 1]);
    }

    // R38: empty topics are refused
    #[test]
    fn empty_topic_is_refused() {
        let questions = questions();
        assert_eq!(
            assemble(&pool(&questions), &QuizSettings::for_topics(vec![CPU, EMPTY]), &mut rng()),
            Err(AssemblyError::EmptyTopic(EMPTY))
        );
    }

    fn short_question(topic: TopicId) -> Question {
        use crate::question::{Check, ShortAnswer};
        Question::new(QuestionDraft {
            kind: AnswerKind::Short,
            options: vec![],
            short: Some(ShortAnswer { check: Check::Accepted { answers: vec!["A".into()], typos: true }, model_answer: String::new() }),
            ..QuestionDraft { topic, text: "Q".into(), kind: AnswerKind::Single, options: vec![], explanation: String::new(), tags: vec![], citations: vec![], short: None }
        })
        .unwrap()
    }

    // short-answer.feature: Choose the question types for a quiz
    #[test]
    fn quizzes_can_take_one_type_or_both() {
        let mut questions = questions();
        questions.extend((21..=23).map(|id| (QuestionId(id), short_question(CPU))));
        let ids = |types| {
            let settings = QuizSettings { types, option_order: Order::Shuffle, ..QuizSettings::for_topics(vec![CPU]) };
            assemble(&pool(&questions), &settings, &mut rng()).unwrap()
        };
        assert_eq!(ids(QuestionTypes::Both).len(), 13);
        assert_eq!(ids(QuestionTypes::Mcq).len(), 10);
        let short = ids(QuestionTypes::Short);
        assert_eq!(short.iter().map(|item| item.question.0).collect::<Vec<_>>(), vec![21, 22, 23]);
        assert!(short.iter().all(|item| item.option_order.is_empty()), "short answers have no options to order");

        let settings = QuizSettings { types: QuestionTypes::Short, ..QuizSettings::for_topics(vec![CPU, GPU]) };
        assert_eq!(assemble(&pool(&questions), &settings, &mut rng()), Err(AssemblyError::NoneOfType(GPU)));
        assert_eq!(AssemblyError::NoneOfType(GPU).to_string(), "a chosen topic has no questions of that type yet");
        assert_eq!(QuizSettings::for_topics(vec![CPU]).types, QuestionTypes::Both, "both by default");
    }
}

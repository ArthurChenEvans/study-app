//! Search (R31): matches as you type, category tabs, combinable filters, sortable results.
//!
//! Open question Q16 (which tab matching questions belong under) isn't decided,
//! so questions only appear under All for now.

use serde::{Deserialize, Serialize};

use study_core::question::{QuestionId, SourceId, TopicId};

use crate::{ModuleId, Result, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    All,
    Modules,
    Topics,
    Tags,
    Sources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortBy {
    Question,
    Topic,
    #[default]
    Edited,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub tab: Tab,
    pub module: Option<ModuleId>,
    pub tag: Option<String>,
    pub sort: SortBy,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hit {
    Question {
        id: QuestionId,
        text: String,
        topic: TopicId,
        topic_name: String,
        tags: Vec<String>,
    },
    Module {
        id: ModuleId,
        name: String,
    },
    Topic {
        id: TopicId,
        name: String,
    },
    Tag {
        name: String,
    },
    Source {
        id: SourceId,
        apa: String,
    },
}

impl Store {
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<Hit>> {
        let text = query.text.trim().to_lowercase();
        let filtered = query.module.is_some() || query.tag.is_some();
        let matches = |haystack: &str| text.is_empty() || haystack.to_lowercase().contains(&text);
        let mut hits = vec![];

        if query.tab == Tab::All {
            hits.extend(self.search_questions(&text, query)?);
        }
        let show_other =
            |tab: Tab| query.tab == tab || (query.tab == Tab::All && !text.is_empty() && !filtered);
        if show_other(Tab::Modules) {
            hits.extend(
                self.modules()?
                    .into_iter()
                    .filter(|m| matches(&m.name))
                    .map(|m| Hit::Module {
                        id: m.id,
                        name: m.name,
                    }),
            );
        }
        if show_other(Tab::Topics) {
            hits.extend(
                self.topics()?
                    .into_iter()
                    .filter(|t| matches(&t.name))
                    .map(|t| Hit::Topic {
                        id: t.id,
                        name: t.name,
                    }),
            );
        }
        if show_other(Tab::Tags) {
            let mut statement = self.connection.prepare(
                "SELECT tag FROM question_tags UNION SELECT tag FROM topic_tags ORDER BY 1",
            )?;
            let tags: Vec<String> = statement
                .query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            hits.extend(
                tags.into_iter()
                    .filter(|tag| matches(tag))
                    .map(|name| Hit::Tag { name }),
            );
        }
        if show_other(Tab::Sources) {
            hits.extend(
                self.sources()?
                    .into_iter()
                    .filter(|s| matches(&s.apa.replace('*', "")))
                    .map(|s| Hit::Source {
                        id: s.id,
                        apa: s.apa,
                    }),
            );
        }
        Ok(hits)
    }

    fn search_questions(&self, text: &str, query: &SearchQuery) -> Result<Vec<Hit>> {
        let column = match query.sort {
            SortBy::Question => "lower(q.text)",
            SortBy::Topic => "lower(t.name)",
            SortBy::Edited => "q.edited_at",
        };
        let direction = if query.descending { "DESC" } else { "ASC" };
        let sql = format!(
            "SELECT q.id, q.text, q.topic_id, t.name FROM questions q JOIN topics t ON t.id = q.topic_id
             WHERE (?1 = '' OR instr(lower(q.text), ?1) > 0 OR instr(lower(q.explanation), ?1) > 0
                    OR EXISTS (SELECT 1 FROM options o WHERE o.question_id = q.id AND instr(lower(o.text), ?1) > 0)
                    OR EXISTS (SELECT 1 FROM question_tags g WHERE g.question_id = q.id AND instr(lower(g.tag), ?1) > 0))
               AND (?2 IS NULL OR t.module_id = ?2)
               AND (?3 IS NULL OR EXISTS (SELECT 1 FROM question_tags g WHERE g.question_id = q.id AND g.tag = ?3))
             ORDER BY {column} {direction}, q.id {direction}"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params![text, query.module.map(|m| m.0), query.tag],
            |row| {
                Ok((
                    QuestionId(row.get(0)?),
                    row.get::<_, String>(1)?,
                    TopicId(row.get(2)?),
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        let mut tags = self
            .connection
            .prepare("SELECT tag FROM question_tags WHERE question_id = ?1 ORDER BY tag")?;
        let mut hits = vec![];
        for row in rows {
            let (id, text, topic, topic_name) = row?;
            let tags = tags
                .query_map([id.0], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            hits.push(Hit::Question {
                id,
                text,
                topic,
                topic_name,
                tags,
            });
        }
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use study_core::bibliography::SourceFields;
    use study_core::question::{AnswerKind, AnswerOption, Question, QuestionDraft};

    struct Fixture {
        store: Store,
        architecture: ModuleId,
        cpu: TopicId,
        gpu: TopicId,
    }

    fn question(
        topic: TopicId,
        text: &str,
        option: &str,
        explanation: &str,
        tags: &[&str],
    ) -> Question {
        Question::new(QuestionDraft {
            topic,
            text: text.into(),
            kind: AnswerKind::Single,
            options: vec![
                AnswerOption {
                    text: option.into(),
                    correct: true,
                },
                AnswerOption {
                    text: "Something else".into(),
                    correct: false,
                },
                AnswerOption {
                    text: "Another thing".into(),
                    correct: false,
                },
            ],
            explanation: explanation.into(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            citations: vec![],
        })
        .unwrap()
    }

    /// search.feature Background, plus a networking topic outside any module.
    fn fixture() -> Fixture {
        let mut store = Store::in_memory().unwrap();
        let architecture = store.create_module("Computer Architecture").unwrap();
        let cpu = store.create_topic("CPU", Some(architecture)).unwrap();
        let gpu = store.create_topic("GPU", Some(architecture)).unwrap();
        let networking = store.create_topic("Networking", None).unwrap();
        store
            .save_question(&question(
                gpu,
                "What is a shader core?",
                "A small ALU",
                "Shaders run on many cores.",
                &["hardware"],
            ))
            .unwrap();
        store
            .save_question(&question(
                cpu,
                "Which stage reads registers?",
                "Decode",
                "Operands are read in decode.",
                &["hardware", "pipeline"],
            ))
            .unwrap();
        store
            .save_question(&question(
                networking,
                "What does a switch forward?",
                "Frames",
                "Layer 2 devices forward frames.",
                &["hardware"],
            ))
            .unwrap();
        store
            .add_source(&SourceFields {
                authors: "Hennessy, J. L.".into(),
                year: Some("2019".into()),
                title: "Pipeline design".into(),
                ..Default::default()
            })
            .unwrap();
        Fixture {
            store,
            architecture,
            cpu,
            gpu,
        }
    }

    fn texts(hits: &[Hit]) -> Vec<String> {
        hits.iter()
            .map(|hit| match hit {
                Hit::Question { text, .. } => format!("Q:{text}"),
                Hit::Module { name, .. } => format!("M:{name}"),
                Hit::Topic { name, .. } => format!("T:{name}"),
                Hit::Tag { name } => format!("#:{name}"),
                Hit::Source { apa, .. } => format!("S:{apa}"),
            })
            .collect()
    }

    fn search(store: &Store, query: SearchQuery) -> Vec<String> {
        texts(&store.search(&query).unwrap())
    }

    // search.feature: Search as you type
    #[test]
    fn matches_part_of_a_word_ignoring_case() {
        let f = fixture();
        assert_eq!(
            search(
                &f.store,
                SearchQuery {
                    text: "SHAD".into(),
                    ..Default::default()
                }
            ),
            vec!["Q:What is a shader core?"]
        );
    }

    // search.feature: Search question content (outline)
    #[test]
    fn searches_question_text_options_and_explanations() {
        let f = fixture();
        for text in ["shader", "small alu", "many cores"] {
            assert_eq!(
                search(
                    &f.store,
                    SearchQuery {
                        text: text.into(),
                        ..Default::default()
                    }
                ),
                vec!["Q:What is a shader core?"],
                "{text}"
            );
        }
    }

    // search.feature: Search across categories (outline)
    #[test]
    fn each_tab_shows_its_own_category() {
        let f = fixture();
        let tab = |tab: Tab, text: &str| {
            search(
                &f.store,
                SearchQuery {
                    text: text.into(),
                    tab,
                    ..Default::default()
                },
            )
        };
        assert_eq!(tab(Tab::Topics, "gpu"), vec!["T:GPU"]);
        assert_eq!(
            tab(Tab::Modules, "architecture"),
            vec!["M:Computer Architecture"]
        );
        assert_eq!(tab(Tab::Tags, "pipe"), vec!["#:pipeline"]);
        assert_eq!(
            tab(Tab::Sources, "pipeline"),
            vec!["S:Hennessy, J. L. (2019). *Pipeline design*."]
        );
    }

    #[test]
    fn all_tab_mixes_every_category() {
        let f = fixture();
        let hits = search(
            &f.store,
            SearchQuery {
                text: "pipeline".into(),
                ..Default::default()
            },
        );
        assert!(hits.contains(&"#:pipeline".to_string()));
        assert!(hits.contains(&"S:Hennessy, J. L. (2019). *Pipeline design*.".to_string()));
        assert!(
            hits.contains(&"Q:Which stage reads registers?".to_string()),
            "tagged pipeline"
        );
    }

    // search.feature: Combine filters
    #[test]
    fn filters_combine() {
        let f = fixture();
        let query = SearchQuery {
            tag: Some("hardware".into()),
            module: Some(f.architecture),
            sort: SortBy::Question,
            ..Default::default()
        };
        assert_eq!(
            search(&f.store, query),
            vec!["Q:What is a shader core?", "Q:Which stage reads registers?"]
        );
    }

    // search.feature: Sort results
    #[test]
    fn sorts_by_a_column_either_way() {
        let f = fixture();
        let by_topic = SearchQuery {
            tag: Some("hardware".into()),
            sort: SortBy::Topic,
            ..Default::default()
        };
        let topics: Vec<TopicId> = f
            .store
            .search(&by_topic)
            .unwrap()
            .iter()
            .filter_map(|hit| {
                if let Hit::Question { topic, .. } = hit {
                    Some(*topic)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(&topics[..2], &[f.cpu, f.gpu]);
        let descending = SearchQuery {
            descending: true,
            ..by_topic
        };
        let first = f.store.search(&descending).unwrap();
        assert!(
            matches!(&first[0], Hit::Question { topic_name, .. } if topic_name == "Networking")
        );
    }

    #[test]
    fn empty_search_with_no_filters_lists_every_question() {
        let f = fixture();
        assert_eq!(
            f.store
                .search(&SearchQuery::default())
                .unwrap()
                .iter()
                .filter(|hit| matches!(hit, Hit::Question { .. }))
                .count(),
            3
        );
    }

    #[test]
    fn no_matches_is_an_empty_list() {
        let f = fixture();
        assert!(
            f.store
                .search(&SearchQuery {
                    text: "tensor core".into(),
                    ..Default::default()
                })
                .unwrap()
                .is_empty()
        );
    }
}

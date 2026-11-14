//! import-export.feature: spreadsheets, JSON, Markdown checklists and Moodle questions
//! (R174 to R176).
//!
//! Every plain format is read into `Items` (topics holding questions, cards and recall
//! checklists), checked with the same rules as the editors, shown as an `ItemsPreview`, then
//! turned into the records a `.studyapp` import uses: a backup first, nothing doubled up when
//! the same file comes in again, and Undo.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use study_core::cards::{Note as CardNote, NoteKind};
use study_core::question::{AnswerKind, AnswerOption, Question as CheckedQuestion, QuestionDraft};
use study_core::short::{Check, Keyword, ShortAnswer};
use study_store::portable::{self, Snapshot};

use crate::anki::{DeckTarget, stable_uid};
use crate::practice::Clock;
use crate::transfer::ExportFormat;
use crate::{ApiError, ApiResult, App, ModuleId, TopicId};

const NOT_READABLE: &str = "it isn't a kind of file Shudhana can import, so nothing was added. Try a .studyapp, an Anki .apkg or .colpkg, a .csv, .json, .md, or a Moodle .txt";
const LETTERS: [&str; 5] = ["A", "B", "C", "D", "E"];

// ---------- The shape every plain format is read into ----------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Items {
    pub topics: Vec<ItemTopic>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemTopic {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<ItemQuestion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<ItemCard>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checklists: Vec<ItemChecklist>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemOption {
    pub text: String,
    #[serde(default)]
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ItemQuestion {
    Mcq {
        text: String,
        options: Vec<ItemOption>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        explanation: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
    },
    Short {
        text: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        model_answer: String,
        /// Right when the answer is one of these.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        answers: Vec<String>,
        /// Or right when it has all of these, in any order. `word|other spelling`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        keywords: Vec<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        explanation: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemCard {
    /// "basic", "reversed" or "cloze".
    #[serde(default = "basic")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub front: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub back: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub extra: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<String>,
}

fn basic() -> String {
    "basic".into()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemChecklist {
    pub points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonFile {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    version: Option<u32>,
    topics: Vec<ItemTopic>,
}

// ---------- Spreadsheets ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CsvKind {
    Mcq,
    Short,
    Card,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Question,
    Option,
    Correct,
    Explanation,
    Type,
    ModelAnswer,
    Keywords,
    Front,
    Back,
    Text,
    Extra,
    CardKind,
    Topic,
    Module,
    Tags,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsvMapping {
    pub kind: CsvKind,
    pub header: bool,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvInfo {
    pub columns: Vec<String>,
    pub header: bool,
    pub kind: CsvKind,
    pub roles: Vec<Role>,
    /// The first rows, to show beside the column choices.
    pub sample: Vec<Vec<String>>,
    pub total_rows: usize,
}

fn role_for(name: &str) -> Role {
    let name = name.trim().to_lowercase();
    match name.as_str() {
        "question" | "q" | "prompt" => Role::Question,
        "a" | "b" | "c" | "d" | "e" => Role::Option,
        _ if name.starts_with("option") || name.starts_with("choice") || name.starts_with("distractor") => Role::Option,
        "answer" | "correct" | "correct answer" | "right answer" | "key" => Role::Correct,
        "explanation" | "why" | "feedback" | "reason" => Role::Explanation,
        "type" => Role::Type,
        "model answer" | "model" | "model_answer" => Role::ModelAnswer,
        "keywords" | "keyword" => Role::Keywords,
        "front" | "term" | "cue" => Role::Front,
        "back" | "definition" | "meaning" => Role::Back,
        "text" | "cloze" => Role::Text,
        "extra" | "back extra" => Role::Extra,
        "kind" | "card type" => Role::CardKind,
        "topic" | "deck" => Role::Topic,
        "module" => Role::Module,
        "tags" | "tag" => Role::Tags,
        _ => Role::Ignore,
    }
}

fn guess_mapping(first: &[String]) -> CsvMapping {
    let roles: Vec<Role> = first.iter().map(|name| role_for(name)).collect();
    let header = roles.iter().any(|role| *role != Role::Ignore);
    if !header {
        // No names: question, options, then the answer.
        let count = first.len();
        let roles = (0..count).map(|index| if index == 0 { Role::Question } else if index + 1 == count && count > 2 { Role::Correct } else { Role::Option }).collect();
        return CsvMapping { kind: CsvKind::Mcq, header: false, roles };
    }
    let kind = if roles.iter().any(|role| matches!(role, Role::Front | Role::Back | Role::Text | Role::CardKind)) {
        CsvKind::Card
    } else if !roles.contains(&Role::Option) && roles.iter().any(|role| matches!(role, Role::ModelAnswer | Role::Keywords)) {
        CsvKind::Short
    } else {
        CsvKind::Mcq
    };
    CsvMapping { kind, header: true, roles }
}

/// One problem with one row or question, said so it can be fixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// "Row 14" or "Question 3".
    pub at: String,
    pub text: String,
    pub why: String,
}

fn split_list(text: &str) -> Vec<String> {
    text.split([';', '\n']).map(str::trim).filter(|part| !part.is_empty()).map(String::from).collect()
}

fn read_csv(bytes: &[u8], file_stem: &str, mapping: Option<CsvMapping>) -> ApiResult<(Items, Vec<Problem>, CsvInfo)> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}');
    let delimiter = if text.lines().next().is_some_and(|line| line.matches('\t').count() > line.matches(',').count()) { b'\t' } else if text.lines().next().is_some_and(|line| line.matches(';').count() > line.matches(',').count()) { b';' } else { b',' };
    let mut reader = csv::ReaderBuilder::new().has_headers(false).flexible(true).delimiter(delimiter).from_reader(text.as_bytes());
    let rows: Vec<Vec<String>> = reader.records().filter_map(Result::ok).map(|record| record.iter().map(|cell| cell.trim().to_string()).collect()).filter(|row: &Vec<String>| row.iter().any(|cell| !cell.is_empty())).collect();
    let first = rows.first().cloned().ok_or_else(|| ApiError::new("that spreadsheet is empty"))?;
    let mapping = mapping.unwrap_or_else(|| guess_mapping(&first));
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut roles = mapping.roles.clone();
    roles.resize(width, Role::Ignore);
    let columns = if mapping.header { first.clone() } else { (1..=width).map(|index| format!("Column {index}")).collect() };
    let data: Vec<(usize, &Vec<String>)> = rows.iter().enumerate().skip(usize::from(mapping.header)).map(|(index, row)| (index + 1, row)).collect();

    let mut topics: BTreeMap<(Option<String>, String), ItemTopic> = BTreeMap::new();
    let mut problems = vec![];
    for (number, row) in &data {
        let cell = |role: Role| -> String { roles.iter().enumerate().filter(|(_, wanted)| **wanted == role).filter_map(|(index, _)| row.get(index)).find(|cell| !cell.is_empty()).cloned().unwrap_or_default() };
        let all = |role: Role| -> Vec<String> { roles.iter().enumerate().filter(|(_, wanted)| **wanted == role).filter_map(|(index, _)| row.get(index)).filter(|cell| !cell.is_empty()).cloned().collect() };
        let at = format!("Row {number}");
        let topic_name = Some(cell(Role::Topic)).filter(|name| !name.is_empty()).unwrap_or_else(|| file_stem.to_string());
        let module = Some(cell(Role::Module)).filter(|name| !name.is_empty());
        let entry = topics.entry((module.clone(), topic_name.clone())).or_insert_with(|| ItemTopic { name: topic_name.clone(), module: module.clone(), ..Default::default() });
        let tags = split_list(&cell(Role::Tags));
        let kind = match (mapping.kind, cell(Role::Type).to_lowercase().as_str()) {
            (CsvKind::Card, _) => CsvKind::Card,
            (_, "short" | "short answer") => CsvKind::Short,
            (_, "single" | "multiple" | "mcq") => CsvKind::Mcq,
            (kind, _) => kind,
        };
        match kind {
            CsvKind::Card => {
                let card_kind = match cell(Role::CardKind).to_lowercase().as_str() {
                    "reversed" | "basic and reversed" | "basic (and reversed card)" => "reversed",
                    "cloze" => "cloze",
                    _ if !cell(Role::Text).is_empty() && cell(Role::Front).is_empty() => "cloze",
                    _ => "basic",
                };
                entry.cards.push(ItemCard { kind: card_kind.into(), front: cell(Role::Front), back: cell(Role::Back), text: cell(Role::Text), extra: cell(Role::Extra), cues: vec![] });
                let _ = &at;
                continue;
            }
            CsvKind::Short => {
                let text = cell(Role::Question);
                let model = cell(Role::ModelAnswer);
                let keywords = split_list(&cell(Role::Keywords));
                let answers = if keywords.is_empty() { split_list(&cell(Role::Correct)) } else { vec![] };
                if text.is_empty() {
                    problems.push(Problem { at, text: String::new(), why: "The question is empty. Write one, or delete the row.".into() });
                    continue;
                }
                entry.questions.push(ItemQuestion::Short { text, model_answer: model, answers, keywords, explanation: cell(Role::Explanation) });
            }
            CsvKind::Mcq => {
                let text = cell(Role::Question);
                let options = all(Role::Option);
                let answer = cell(Role::Correct);
                if text.is_empty() {
                    problems.push(Problem { at, text: String::new(), why: "The question is empty. Write one, or delete the row.".into() });
                    continue;
                }
                let mut right = vec![false; options.len()];
                let mut understood = !answer.is_empty();
                for part in answer.split(';').map(str::trim).filter(|part| !part.is_empty()) {
                    let index = if part.len() == 1 && part.chars().all(|letter| letter.is_ascii_alphabetic()) {
                        Some((part.to_ascii_uppercase().as_bytes()[0] - b'A') as usize)
                    } else if let Some(found) = options.iter().position(|option| option.eq_ignore_ascii_case(part)) {
                        Some(found)
                    } else if let Ok(number) = part.parse::<usize>() {
                        number.checked_sub(1)
                    } else {
                        None
                    };
                    match index.filter(|index| *index < options.len()) {
                        Some(index) => right[index] = true,
                        None => {
                            understood = false;
                            let choices = LETTERS[..options.len().min(5)].join(", ");
                            let choices = match choices.rsplit_once(", ") {
                                Some((start, last)) => format!("{start} or {last}"),
                                None => choices,
                            };
                            problems.push(Problem { at: at.clone(), text: text.clone(), why: format!("The answer says {part}, but there are only {} options. Put {choices} in the answer column.", options.len()) });
                        }
                    }
                }
                if !understood {
                    if answer.is_empty() {
                        problems.push(Problem { at, text, why: "There's no answer. Put the right option's letter in the answer column.".into() });
                    }
                    continue;
                }
                entry.questions.push(ItemQuestion::Mcq { text, options: options.into_iter().zip(right).map(|(text, correct)| ItemOption { text, correct }).collect(), explanation: cell(Role::Explanation), tags });
            }
        }
    }
    let info = CsvInfo { columns, header: mapping.header, kind: mapping.kind, roles: mapping.roles.clone(), sample: data.iter().take(4).map(|(_, row)| (*row).clone()).collect(), total_rows: data.len() };
    Ok((Items { topics: topics.into_values().filter(|topic| !topic.questions.is_empty() || !topic.cards.is_empty() || !topic.checklists.is_empty()).collect() }, problems, info))
}

// ---------- Markdown checklists ----------

fn read_markdown(text: &str, file_stem: &str) -> Items {
    let mut topics: Vec<ItemTopic> = vec![];
    for line in text.lines() {
        let line = line.trim();
        if let Some(heading) = line.strip_prefix('#') {
            let name = heading.trim_start_matches('#').trim().to_string();
            if !name.is_empty() {
                topics.push(ItemTopic { name, checklists: vec![ItemChecklist::default()], ..Default::default() });
            }
            continue;
        }
        let point = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")).or_else(|| line.strip_prefix("+ ")).or_else(|| line.split_once(". ").filter(|(number, _)| !number.is_empty() && number.chars().all(|digit| digit.is_ascii_digit())).map(|(_, rest)| rest));
        if let Some(point) = point.map(str::trim).filter(|point| !point.is_empty()) {
            if topics.is_empty() {
                topics.push(ItemTopic { name: file_stem.to_string(), checklists: vec![ItemChecklist::default()], ..Default::default() });
            }
            topics.last_mut().and_then(|topic| topic.checklists.last_mut()).map(|checklist| checklist.points.push(point.to_string()));
        }
    }
    Items { topics: topics.into_iter().filter(|topic| topic.checklists.iter().any(|checklist| !checklist.points.is_empty())).collect() }
}

// ---------- Moodle: GIFT and Aiken ----------

fn gift_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for letter in text.chars() {
        if matches!(letter, '~' | '=' | '#' | '{' | '}' | ':') {
            out.push('\\');
        }
        out.push(letter);
    }
    out.replace('\n', "\\n")
}

fn gift_unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut letters = text.chars();
    while let Some(letter) = letters.next() {
        if letter == '\\' {
            match letters.next() {
                Some('n') => out.push('\n'),
                Some(next) => out.push(next),
                None => {}
            }
        } else {
            out.push(letter);
        }
    }
    out.trim().to_string()
}

/// Splits on `marker` characters that aren't escaped.
fn split_unescaped(text: &str, markers: &[char]) -> Vec<(char, String)> {
    let mut parts = vec![];
    let mut current = String::new();
    let mut kind = ' ';
    let mut escaped = false;
    for letter in text.chars() {
        if escaped {
            current.push('\\');
            current.push(letter);
            escaped = false;
        } else if letter == '\\' {
            escaped = true;
        } else if markers.contains(&letter) {
            parts.push((kind, std::mem::take(&mut current)));
            kind = letter;
        } else {
            current.push(letter);
        }
    }
    parts.push((kind, current));
    parts
}

fn category_place(category: &str) -> (Option<String>, String) {
    let parts: Vec<&str> = category.split('/').map(str::trim).filter(|part| !part.is_empty() && !part.starts_with('$') && *part != "top" && *part != "Default").collect();
    match parts.as_slice() {
        [] => (None, String::new()),
        [only] => (None, (*only).to_string()),
        [first, .., last] => (Some((*first).to_string()), (*last).to_string()),
    }
}

fn read_gift(text: &str, file_stem: &str) -> (Items, Vec<Problem>) {
    let mut topics: Vec<ItemTopic> = vec![ItemTopic { name: file_stem.into(), ..Default::default() }];
    let mut problems = vec![];
    let cleaned: Vec<&str> = text.lines().filter(|line| !line.trim_start().starts_with("//")).collect();
    let mut number = 0;
    for block in cleaned.join("\n").split("\n\n").map(str::trim).filter(|block| !block.is_empty()) {
        if let Some(category) = block.strip_prefix("$CATEGORY:") {
            let (module, name) = category_place(category.lines().next().unwrap_or(""));
            if !name.is_empty() {
                topics.push(ItemTopic { name, module, ..Default::default() });
            }
            let rest = category.split_once('\n').map(|(_, rest)| rest.trim()).unwrap_or("");
            if rest.is_empty() {
                continue;
            }
        }
        let block = block.lines().filter(|line| !line.starts_with("$CATEGORY:")).collect::<Vec<_>>().join("\n");
        if block.trim().is_empty() {
            continue;
        }
        number += 1;
        let at = format!("Question {number}");
        let body = match block.trim_start().strip_prefix("::").and_then(|rest| rest.split_once("::")) {
            Some((_, rest)) => rest.to_string(),
            None => block.clone(),
        };
        let (Some(open), Some(close)) = (find_unescaped(&body, '{'), body.rfind('}')) else {
            problems.push(Problem { at, text: gift_unescape(&body), why: "There are no answers in curly brackets, so there's nothing to check.".into() });
            continue;
        };
        let question = gift_unescape(&format!("{} {}", &body[..open], &body[close + 1..]));
        let answers = &body[open + 1..close];
        let mut explanation = String::new();
        let mut options = vec![];
        let mut accepted = vec![];
        let parts = split_unescaped(answers, &['=', '~']);
        for (kind, part) in parts.into_iter().filter(|(kind, _)| *kind != ' ') {
            let (part, general) = match part.split_once("####") {
                Some((part, general)) => (part.to_string(), Some(general.to_string())),
                None => (part, None),
            };
            if let Some(general) = general {
                explanation = gift_unescape(&general);
            }
            let answer = split_unescaped(&part, &['#']).into_iter().next().map(|(_, text)| text).unwrap_or_default();
            let (weight, answer) = match answer.trim().strip_prefix('%').and_then(|rest| rest.split_once('%')) {
                Some((weight, rest)) => (weight.trim().parse::<f64>().ok(), rest.to_string()),
                None => (None, answer),
            };
            let answer = gift_unescape(&answer);
            if answer.is_empty() {
                continue;
            }
            let correct = kind == '=' || weight.is_some_and(|weight| weight > 0.0);
            if kind == '=' {
                accepted.push(answer.clone());
            }
            options.push(ItemOption { text: answer, correct });
        }
        let only_right = options.iter().all(|option| option.correct);
        let item = if !options.is_empty() && only_right {
            ItemQuestion::Short { text: question.clone(), model_answer: String::new(), answers: accepted, keywords: vec![], explanation }
        } else {
            ItemQuestion::Mcq { text: question.clone(), options, explanation, tags: vec![] }
        };
        topics.last_mut().expect("a topic").questions.push(item);
    }
    (Items { topics: topics.into_iter().filter(|topic| !topic.questions.is_empty()).collect() }, problems)
}

fn find_unescaped(text: &str, wanted: char) -> Option<usize> {
    let mut escaped = false;
    for (at, letter) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if letter == '\\' {
            escaped = true;
        } else if letter == wanted {
            return Some(at);
        }
    }
    None
}

fn read_aiken(text: &str, file_stem: &str) -> (Items, Vec<Problem>) {
    let mut topic = ItemTopic { name: file_stem.into(), ..Default::default() };
    let mut problems = vec![];
    let mut question: Vec<String> = vec![];
    let mut options: Vec<String> = vec![];
    let mut number = 0;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        let option = line.get(..2).filter(|start| {
            let mut letters = start.chars();
            letters.next().is_some_and(|letter| letter.is_ascii_uppercase()) && matches!(letters.next(), Some('.') | Some(')'))
        });
        if let Some(answer) = line.strip_prefix("ANSWER:") {
            number += 1;
            let letter = answer.trim().to_ascii_uppercase();
            let index = letter.bytes().next().map(|byte| byte.wrapping_sub(b'A') as usize);
            let text = question.join("\n");
            match index.filter(|index| *index < options.len()) {
                Some(index) if !text.is_empty() => topic.questions.push(ItemQuestion::Mcq {
                    text,
                    options: options.iter().enumerate().map(|(at, option)| ItemOption { text: option.clone(), correct: at == index }).collect(),
                    explanation: String::new(),
                    tags: vec![],
                }),
                _ => problems.push(Problem { at: format!("Question {number}"), text, why: format!("The answer says {letter}, but there are only {} options.", options.len()) }),
            }
            question.clear();
            options.clear();
        } else if option.is_some() && !question.is_empty() {
            options.push(line[2..].trim().to_string());
        } else {
            question.push(line.to_string());
        }
    }
    (Items { topics: if topic.questions.is_empty() { vec![] } else { vec![topic] } }, problems)
}

// ---------- Reading any of them ----------

fn stem(file_name: &str) -> String {
    let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name).to_string()
}

fn read(bytes: &[u8], file_name: &str, mapping: Option<CsvMapping>) -> ApiResult<(String, Items, Vec<Problem>, Option<CsvInfo>)> {
    let extension = file_name.rsplit_once('.').map(|(_, extension)| extension.to_lowercase()).unwrap_or_default();
    let file_stem = stem(file_name);
    let text = || -> ApiResult<String> { String::from_utf8(bytes.to_vec()).map_err(|_| ApiError::new(NOT_READABLE)) };
    match extension.as_str() {
        "csv" | "tsv" => {
            let (items, problems, info) = read_csv(bytes, &file_stem, mapping)?;
            Ok(("csv".into(), items, problems, Some(info)))
        }
        "json" => {
            let file: JsonFile = serde_json::from_str(&text()?).map_err(|error| ApiError::new(format!("that JSON file couldn't be read: {error}")))?;
            let _ = (file.format, file.version);
            Ok(("json".into(), Items { topics: file.topics }, vec![], None))
        }
        "md" | "markdown" => Ok(("markdown".into(), read_markdown(&text()?, &file_stem), vec![], None)),
        "gift" => {
            let (items, problems) = read_gift(&text()?, &file_stem);
            Ok(("gift".into(), items, problems, None))
        }
        "txt" => {
            let text = text()?;
            if text.lines().any(|line| line.trim_start().starts_with("ANSWER:")) {
                let (items, problems) = read_aiken(&text, &file_stem);
                Ok(("aiken".into(), items, problems, None))
            } else {
                let (items, problems) = read_gift(&text, &file_stem);
                Ok(("gift".into(), items, problems, None))
            }
        }
        _ => Err(ApiError::new(NOT_READABLE)),
    }
}

// ---------- Checking and turning into records ----------

fn topic_key(topic: &ItemTopic) -> String {
    match &topic.module {
        Some(module) => format!("{module}::{}", topic.name),
        None => topic.name.clone(),
    }
}

fn short_of(model_answer: &str, answers: &[String], keywords: &[String]) -> ShortAnswer {
    let check = if keywords.is_empty() {
        let answers = if answers.is_empty() && !model_answer.trim().is_empty() { vec![model_answer.trim().to_string()] } else { answers.to_vec() };
        Check::Accepted { answers, typos: true }
    } else {
        Check::Keywords {
            keywords: keywords
                .iter()
                .map(|keyword| {
                    let mut spellings = keyword.split('|').map(str::trim).filter(|word| !word.is_empty()).map(String::from);
                    Keyword { word: spellings.next().unwrap_or_default(), also: spellings.collect() }
                })
                .collect(),
            needed: None,
        }
    };
    ShortAnswer { check, model_answer: model_answer.to_string() }
}

/// A question as the app would save it, or why not.
fn checked(question: &ItemQuestion) -> Result<(String, Vec<portable::QuestionOption>, Option<ShortAnswer>, String, Vec<String>, String), String> {
    let (text, explanation, tags, draft) = match question {
        ItemQuestion::Mcq { text, options, explanation, tags } => {
            let correct = options.iter().filter(|option| option.correct).count();
            let kind = if correct > 1 { AnswerKind::Multiple } else { AnswerKind::Single };
            let draft = QuestionDraft { topic: TopicId(0), text: text.clone(), kind, options: options.iter().map(|option| AnswerOption { text: option.text.clone(), correct: option.correct }).collect(), explanation: explanation.clone(), tags: tags.clone(), citations: vec![], short: None };
            (text, explanation, tags.clone(), draft)
        }
        ItemQuestion::Short { text, model_answer, answers, keywords, explanation } => {
            let draft = QuestionDraft { topic: TopicId(0), text: text.clone(), kind: AnswerKind::Short, options: vec![], explanation: explanation.clone(), tags: vec![], citations: vec![], short: Some(short_of(model_answer, answers, keywords)) };
            (text, explanation, vec![], draft)
        }
    };
    if text.trim().is_empty() {
        return Err("The question is empty. Write one, or delete it.".into());
    }
    let kind = match draft.kind {
        AnswerKind::Single => "single",
        AnswerKind::Multiple => "multiple",
        AnswerKind::Short => "short",
    };
    let short = draft.short.clone();
    let options = draft.options.iter().map(|option| portable::QuestionOption { text: option.text.clone(), correct: option.correct }).collect();
    CheckedQuestion::new(draft).map_err(|error| {
        let message = error.to_string();
        let mut letters = message.chars();
        letters.next().map(|first| first.to_uppercase().collect::<String>() + letters.as_str() + ".").unwrap_or_default()
    })?;
    Ok((text.trim().to_string(), options, short, explanation.clone(), tags, kind.to_string()))
}

fn card_note(card: &ItemCard) -> CardNote {
    let kind = match card.kind.to_lowercase().as_str() {
        "reversed" => NoteKind::Reversed,
        "cloze" => NoteKind::Cloze,
        _ => NoteKind::Basic,
    };
    CardNote { topic: TopicId(0), kind, front: card.front.clone(), back: card.back.clone(), cues: card.cues.clone(), text: card.text.clone(), extra: card.extra.clone() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemsTopicRow {
    /// What a choice names this topic by.
    pub key: String,
    pub name: String,
    pub module: Option<String>,
    pub questions: usize,
    pub cards: usize,
    pub checklists: usize,
    pub target: DeckTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemsPreview {
    pub file_name: String,
    /// "csv", "json", "markdown", "gift" or "aiken".
    pub format: String,
    pub csv: Option<CsvInfo>,
    pub topics: Vec<ItemsTopicRow>,
    pub problems: Vec<Problem>,
    /// Questions, cards and checklists that will come in.
    pub ready: usize,
    /// Of those, ones here from an earlier import, which stay as they are.
    pub already_here: usize,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ItemsChoice {
    /// Topics left out go where the preview suggested.
    #[serde(default)]
    pub targets: Vec<(String, DeckTarget)>,
}

/// A read file, kept between the preview and the import.
#[derive(Debug, Clone)]
pub struct PendingItems {
    items: Items,
    file_name: String,
}

fn question_uid(key: &str, text: &str) -> String {
    stable_uid(&format!("items-question:{key}:{}", text.trim()))
}

fn card_uid(key: &str, note: &CardNote) -> String {
    stable_uid(&format!("items-card:{key}:{}{}", note.front.trim(), note.text.trim()))
}

impl App {
    /// Reads a spreadsheet, JSON, Markdown or Moodle file and says what importing it would
    /// do, changing nothing (R174, R175). A spreadsheet's columns can be chosen with `mapping`.
    pub fn items_preview(&mut self, bytes: &[u8], file_name: &str, mapping: Option<CsvMapping>) -> ApiResult<ItemsPreview> {
        self.pending_items = None;
        let (format, mut items, mut problems, csv) = read(bytes, file_name, mapping)?;
        let here = self.store.stamps_here()?;
        let mut ready = 0;
        let mut already_here = 0;
        let mut rows = vec![];
        for topic in &mut items.topics {
            let key = topic_key(topic);
            let mut questions = vec![];
            for (index, question) in topic.questions.iter().enumerate() {
                match checked(question) {
                    Ok((text, ..)) => {
                        already_here += usize::from(here.contains_key(&question_uid(&key, &text)));
                        questions.push(question.clone());
                    }
                    Err(why) => {
                        let text = match question {
                            ItemQuestion::Mcq { text, .. } | ItemQuestion::Short { text, .. } => text.clone(),
                        };
                        problems.push(Problem { at: format!("{}, question {}", topic.name, index + 1), text, why });
                    }
                }
            }
            topic.questions = questions;
            let mut cards = vec![];
            for (index, card) in topic.cards.iter().enumerate() {
                let note = card_note(card);
                match note.faces() {
                    Ok(_) => {
                        already_here += usize::from(here.contains_key(&card_uid(&key, &note)));
                        cards.push(card.clone());
                    }
                    Err(error) => problems.push(Problem { at: format!("{}, card {}", topic.name, index + 1), text: if note.front.is_empty() { note.text.clone() } else { note.front.clone() }, why: format!("{error}.") }),
                }
            }
            topic.cards = cards;
            for (index, _) in topic.checklists.iter().enumerate() {
                already_here += usize::from(here.contains_key(&stable_uid(&format!("items-checklist:{key}:{index}"))));
            }
            ready += topic.questions.len() + topic.cards.len() + topic.checklists.len();
            rows.push(ItemsTopicRow { target: DeckTarget::NewTopic { name: topic.name.clone(), module: topic.module.clone() }, key, name: topic.name.clone(), module: topic.module.clone(), questions: topic.questions.len(), cards: topic.cards.len(), checklists: topic.checklists.len() });
        }
        if ready == 0 && problems.is_empty() {
            return Err(ApiError::new("there's nothing in that file to import"));
        }
        self.pending_items = Some(PendingItems { items, file_name: file_name.to_string() });
        Ok(ItemsPreview { file_name: file_name.to_string(), format, csv, topics: rows, problems, ready, already_here })
    }

    /// Imports the previewed file as chosen: a backup first, then everything at once (R174).
    pub fn items_apply(&mut self, choice: ItemsChoice, clock: Clock) -> ApiResult<crate::transfer::ImportResult> {
        let pending = self.pending_items.take().ok_or_else(|| ApiError::new("pick the file to import again"))?;
        let snapshot = self.items_snapshot(&pending.items, &choice, clock)?;
        let result = self.apply_snapshot(snapshot, HashSet::new(), clock)?;
        self.remember_import(&pending.file_name, &result, clock);
        Ok(result)
    }

    fn items_snapshot(&self, items: &Items, choice: &ItemsChoice, clock: Clock) -> ApiResult<Snapshot> {
        let chosen: HashMap<&str, &DeckTarget> = choice.targets.iter().map(|(key, target)| (key.as_str(), target)).collect();
        let stamp = format!("{:016}-0000-import", 0);
        let edited = {
            let seconds = clock.unix;
            let (day, time) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
            format!("{} {:02}:{:02}:{:02}", crate::backup::date_label(day), time / 3600, time / 60 % 60, time % 60)
        };
        let modules_here: HashMap<String, ModuleId> = self.modules()?.into_iter().map(|module| (module.name.to_lowercase(), module.id)).collect();
        let mut module_uids: HashMap<String, String> = HashMap::new();
        let mut snapshot = Snapshot::default();
        for topic in &items.topics {
            let key = topic_key(topic);
            let target = chosen.get(key.as_str()).map(|target| (*target).clone()).unwrap_or_else(|| DeckTarget::NewTopic { name: topic.name.clone(), module: topic.module.clone() });
            let (topic_uid, topic_name) = match target {
                DeckTarget::Skip => continue,
                DeckTarget::Topic { topic: id } => {
                    let name = self.topics()?.into_iter().find(|row| row.id == id).map(|row| row.name).unwrap_or_else(|| topic.name.clone());
                    (self.store.uid_of("topics", id.0)?.ok_or_else(|| ApiError::new("one of the chosen topics is gone, pick again"))?, name)
                }
                DeckTarget::NewTopic { name, module } => {
                    let module = match module {
                        None => None,
                        Some(module) => Some(match modules_here.get(&module.to_lowercase()) {
                            Some(id) => self.store.uid_of("modules", id.0)?.ok_or_else(|| ApiError::new("one of the modules is gone, pick again"))?,
                            None => module_uids
                                .entry(module.to_lowercase())
                                .or_insert_with(|| {
                                    let uid = stable_uid(&format!("items-module:{}", module.to_lowercase()));
                                    snapshot.modules.push(portable::Module { uid: uid.clone(), changed: stamp.clone(), name: module.clone() });
                                    uid
                                })
                                .clone(),
                        }),
                    };
                    let uid = stable_uid(&format!("items-topic:{key}"));
                    snapshot.topics.push(portable::Topic { uid: uid.clone(), changed: stamp.clone(), name: name.trim().to_string(), module, tags: vec![], schedule: None });
                    (uid, name)
                }
            };
            for question in &topic.questions {
                let Ok((text, options, short, explanation, tags, kind)) = checked(question) else { continue };
                snapshot.questions.push(portable::Question {
                    uid: question_uid(&key, &text),
                    changed: stamp.clone(),
                    topic: topic_uid.clone(),
                    kind,
                    text,
                    explanation,
                    short: short.map(|short| serde_json::to_value(short).unwrap_or(Value::Null)),
                    edited_at: edited.clone(),
                    options,
                    tags,
                    citations: vec![],
                });
            }
            for card in &topic.cards {
                let note = card_note(card);
                let Ok(faces) = note.faces() else { continue };
                let uid = card_uid(&key, &note);
                let mut value = serde_json::to_value(&note).map_err(|error| ApiError::new(error.to_string()))?;
                if let Value::Object(fields) = &mut value {
                    fields.remove("topic");
                }
                let cards = faces.into_iter().map(|face| portable::Card { uid: format!("{uid}:{}", face.key), changed: stamp.clone(), key: face.key, front: face.front, back: face.back, archived: false, schedule: None }).collect();
                snapshot.notes.push(portable::Note { uid, changed: stamp.clone(), topic: topic_uid.clone(), note: value, source: None, created: clock.day, cards });
            }
            for (index, checklist) in topic.checklists.iter().enumerate() {
                snapshot.checklists.push(portable::Checklist {
                    uid: stable_uid(&format!("items-checklist:{key}:{index}")),
                    changed: stamp.clone(),
                    module: None,
                    topic: Some(topic_uid.clone()),
                    subject_name: topic_name.clone(),
                    subject_kind: "Topic".into(),
                    created: clock.day,
                    schedule: None,
                    points: checklist.points.iter().map(|point| portable::Point { text: point.clone(), question: None, note: None }).collect(),
                });
            }
        }
        Ok(snapshot)
    }
}

// ---------- Writing ----------

/// The topics in `snapshot` as `Items`, with module names.
fn items_of(snapshot: &Snapshot) -> Items {
    let modules: HashMap<&str, &str> = snapshot.modules.iter().map(|module| (module.uid.as_str(), module.name.as_str())).collect();
    let mut topics: Vec<(String, ItemTopic)> = snapshot.topics.iter().map(|topic| (topic.uid.clone(), ItemTopic { name: topic.name.clone(), module: topic.module.as_deref().and_then(|module| modules.get(module)).map(|name| (*name).to_string()), ..Default::default() })).collect();
    let index: HashMap<String, usize> = topics.iter().enumerate().map(|(at, (uid, _))| (uid.clone(), at)).collect();
    for question in &snapshot.questions {
        let Some(&at) = index.get(&question.topic) else { continue };
        let item = if question.kind == "short" {
            let short: Option<ShortAnswer> = question.short.clone().and_then(|value| serde_json::from_value(value).ok());
            let (answers, keywords) = match short.as_ref().map(|short| &short.check) {
                Some(Check::Accepted { answers, .. }) => (answers.clone(), vec![]),
                Some(Check::Keywords { keywords, .. }) => (vec![], keywords.iter().map(|keyword| std::iter::once(keyword.word.clone()).chain(keyword.also.clone()).collect::<Vec<_>>().join("|")).collect()),
                None => (vec![], vec![]),
            };
            ItemQuestion::Short { text: question.text.clone(), model_answer: short.map(|short| short.model_answer).unwrap_or_default(), answers, keywords, explanation: question.explanation.clone() }
        } else {
            ItemQuestion::Mcq { text: question.text.clone(), options: question.options.iter().map(|option| ItemOption { text: option.text.clone(), correct: option.correct }).collect(), explanation: question.explanation.clone(), tags: question.tags.clone() }
        };
        topics[at].1.questions.push(item);
    }
    for note in &snapshot.notes {
        let Some(&at) = index.get(&note.topic) else { continue };
        let text = |key: &str| note.note.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        let cues = note.note.get("cues").and_then(Value::as_array).map(|cues| cues.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
        topics[at].1.cards.push(ItemCard { kind: text("kind").to_lowercase(), front: text("front"), back: text("back"), text: text("text"), extra: text("extra"), cues });
    }
    for checklist in &snapshot.checklists {
        let points = checklist.points.iter().map(|point| point.text.clone()).collect();
        match checklist.topic.as_ref().and_then(|topic| index.get(topic)) {
            Some(&at) => topics[at].1.checklists.push(ItemChecklist { points }),
            None => topics.push((String::new(), ItemTopic { name: checklist.subject_name.clone(), checklists: vec![ItemChecklist { points }], ..Default::default() })),
        }
    }
    Items { topics: topics.into_iter().map(|(_, topic)| topic).collect() }
}

fn csv_bytes(header: &[&str], rows: Vec<Vec<String>>) -> ApiResult<Vec<u8>> {
    let mut writer = csv::Writer::from_writer(vec![]);
    let failed = |error: csv::Error| ApiError::new(format!("couldn't write the spreadsheet: {error}"));
    writer.write_record(header).map_err(failed)?;
    for row in rows {
        writer.write_record(&row).map_err(failed)?;
    }
    writer.into_inner().map_err(|error| ApiError::new(format!("couldn't write the spreadsheet: {error}")))
}

/// A file in one of the plain formats, for `snapshot` (R174 to R176).
pub fn export(snapshot: &Snapshot, format: ExportFormat) -> ApiResult<Vec<u8>> {
    let items = items_of(snapshot);
    match format {
        ExportFormat::Json => {
            let file = JsonFile { format: Some("shudhana".into()), version: Some(1), topics: items.topics };
            let mut text = serde_json::to_string_pretty(&file).map_err(|error| ApiError::new(error.to_string()))?;
            text.push('\n');
            Ok(text.into_bytes())
        }
        ExportFormat::QuestionsCsv => {
            let mut rows = vec![];
            for topic in &items.topics {
                for question in &topic.questions {
                    let mut row = vec![String::new(); 13];
                    row[11] = topic.name.clone();
                    row[12] = topic.module.clone().unwrap_or_default();
                    match question {
                        ItemQuestion::Mcq { text, options, explanation, .. } => {
                            row[0] = text.clone();
                            for (index, option) in options.iter().take(5).enumerate() {
                                row[1 + index] = option.text.clone();
                            }
                            row[6] = options.iter().enumerate().filter(|(_, option)| option.correct).map(|(index, _)| LETTERS[index.min(4)]).collect::<Vec<_>>().join(";");
                            row[7] = explanation.clone();
                            row[8] = if options.iter().filter(|option| option.correct).count() > 1 { "multiple" } else { "single" }.into();
                        }
                        ItemQuestion::Short { text, model_answer, answers, keywords, explanation } => {
                            row[0] = text.clone();
                            row[6] = answers.join(";");
                            row[7] = explanation.clone();
                            row[8] = "short".into();
                            row[9] = model_answer.clone();
                            row[10] = keywords.join(";");
                        }
                    }
                    rows.push(row);
                }
            }
            csv_bytes(&["question", "a", "b", "c", "d", "e", "answer", "explanation", "type", "model answer", "keywords", "topic", "module"], rows)
        }
        ExportFormat::CardsCsv => {
            let rows = items.topics.iter().flat_map(|topic| topic.cards.iter().map(move |card| vec![card.kind.clone(), card.front.clone(), card.back.clone(), card.text.clone(), card.extra.clone(), topic.name.clone(), topic.module.clone().unwrap_or_default()])).collect();
            csv_bytes(&["kind", "front", "back", "text", "extra", "topic", "module"], rows)
        }
        ExportFormat::Markdown => {
            let mut text = String::new();
            for topic in &items.topics {
                for checklist in topic.checklists.iter().filter(|checklist| !checklist.points.is_empty()) {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&format!("# {}\n\n", topic.name));
                    for point in &checklist.points {
                        text.push_str(&format!("- {}\n", point.replace('\n', " ")));
                    }
                }
            }
            Ok(text.into_bytes())
        }
        ExportFormat::Gift => {
            let mut text = String::new();
            for topic in items.topics.iter().filter(|topic| !topic.questions.is_empty()) {
                let category = match &topic.module {
                    Some(module) => format!("{}/{}", module.replace('/', "-"), topic.name.replace('/', "-")),
                    None => topic.name.replace('/', "-"),
                };
                text.push_str(&format!("$CATEGORY: {category}\n\n"));
                for question in &topic.questions {
                    match question {
                        ItemQuestion::Mcq { text: body, options, explanation, .. } => {
                            let right = options.iter().filter(|option| option.correct).count();
                            text.push_str(&format!("{} {{\n", gift_escape(body)));
                            for option in options {
                                let line = if right == 1 {
                                    format!("{}{}", if option.correct { "=" } else { "~" }, gift_escape(&option.text))
                                } else if option.correct {
                                    let weight = match right {
                                        2 => "50",
                                        3 => "33.33333",
                                        4 => "25",
                                        _ => "20",
                                    };
                                    format!("~%{weight}%{}", gift_escape(&option.text))
                                } else {
                                    format!("~%-100%{}", gift_escape(&option.text))
                                };
                                text.push_str(&format!("\t{line}\n"));
                            }
                            if !explanation.is_empty() {
                                text.push_str(&format!("\t####{}\n", gift_escape(explanation)));
                            }
                            text.push_str("}\n\n");
                        }
                        ItemQuestion::Short { text: body, model_answer, answers, explanation, .. } => {
                            let accepted: Vec<&String> = if answers.is_empty() { std::iter::once(model_answer).filter(|answer| !answer.is_empty()).collect() } else { answers.iter().collect() };
                            if accepted.is_empty() {
                                continue;
                            }
                            text.push_str(&format!("{} {{\n", gift_escape(body)));
                            for answer in accepted {
                                text.push_str(&format!("\t={}\n", gift_escape(answer)));
                            }
                            if !explanation.is_empty() {
                                text.push_str(&format!("\t####{}\n", gift_escape(explanation)));
                            }
                            text.push_str("}\n\n");
                        }
                    }
                }
            }
            Ok(text.into_bytes())
        }
        ExportFormat::Aiken => {
            let mut text = String::new();
            for question in items.topics.iter().flat_map(|topic| topic.questions.iter()) {
                // Aiken holds multiple choice with one right answer, up to five options.
                let ItemQuestion::Mcq { text: body, options, .. } = question else { continue };
                if options.iter().filter(|option| option.correct).count() != 1 || options.len() > 5 {
                    continue;
                }
                text.push_str(&format!("{}\n", body.replace('\n', " ")));
                for (index, option) in options.iter().enumerate() {
                    text.push_str(&format!("{}. {}\n", LETTERS[index], option.text.replace('\n', " ")));
                }
                let right = options.iter().position(|option| option.correct).unwrap_or(0);
                text.push_str(&format!("ANSWER: {}\n\n", LETTERS[right]));
            }
            Ok(text.into_bytes())
        }
        ExportFormat::StudyApp | ExportFormat::Anki => Err(ApiError::new("that format is written elsewhere")),
    }
}

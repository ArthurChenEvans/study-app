//! A heavy profile for testing how the screens cope with a lot of data: about 160 topics in
//! 14 modules, thousands of questions and cards, four months of daily use, dozens of
//! checklists and projects. The text is generated, so it reads like filler, but names
//! and questions vary in length like real ones do. Deterministic, like the demo profile.

use std::collections::{HashMap, HashSet};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use study_core::bibliography::SourceFields;
use study_core::cards::{Note, NoteKind};
use study_core::practice::{DrillRating, StruggleKind};
use study_core::question::TopicId;
use study_core::short::{Check, ShortAnswer};
use study_store::recall::Subject;

use crate::cards::NoteInput;
use crate::recall::PointInput;
use crate::seed::{SeedSummary, StruggleSeed, quiz, review_day, work_session};
use crate::{ApiError, ApiResult, App, CitationInput, OptionInput, QuestionInput};

/// How far back the large profile's history goes.
pub const LARGE_DAYS: i64 = 120;

const MODULES: &[&str] = &[
    "Computer Architecture", "Networks", "Security Fundamentals", "Cryptography", "Operating Systems",
    "Databases", "Algorithms and Data Structures", "Software Engineering", "Digital Forensics",
    "Web Application Security", "Penetration Testing", "Law, Ethics and Professional Practice",
    "Discrete Mathematics", "Cloud and Virtualisation",
];

const SUBJECTS: &[&str] = &[
    "Caches", "Pipelining", "Branch prediction", "Virtual memory", "Paging and segmentation", "Interrupts",
    "TCP congestion control", "Subnetting", "Routing protocols", "DNS", "TLS handshakes", "Public key infrastructure",
    "Block cipher modes", "Hash functions", "Access control models", "Threat modelling", "SQL injection",
    "Cross-site scripting", "Session management", "Normal forms", "Transactions and isolation levels", "Indexes",
    "Graph search", "Dynamic programming", "Heaps", "Hash tables", "Sorting", "Recurrences", "Scheduling",
    "Deadlock", "File systems", "Memory forensics", "Chain of custody", "Incident response", "GDPR",
    "Computer Misuse Act", "Set theory", "Proof by induction", "Containers", "Hypervisors",
];

const QUALIFIERS: &[&str] = &[
    "", "", "", " basics", " in practice", " and their trade-offs", ": worked examples", " revision",
    " for the exam", " (lecture 7)", " deep dive", ", common mistakes",
];

const STEMS: &[&str] = &[
    "Which of these best describes", "What is the main purpose of", "Which statement about", "When would you choose",
    "What goes wrong if you misconfigure", "Which is NOT true of", "In a typical exam scenario, what happens to",
    "Given a system under heavy load, which part of",
];

fn topic_name(index: usize) -> String {
    let subject = SUBJECTS[index % SUBJECTS.len()];
    let qualifier = QUALIFIERS[(index * 7 + index / SUBJECTS.len()) % QUALIFIERS.len()];
    let round = index / SUBJECTS.len();
    if round == 0 { format!("{subject}{qualifier}") } else { format!("{subject}{qualifier} {}", round + 1) }
}

/// Fills an empty database with the large profile.
pub fn seed_large(app: &mut App, today: i64) -> ApiResult<SeedSummary> {
    if !app.topics()?.is_empty() || !app.modules()?.is_empty() {
        return Err(ApiError::new("the database already has data, so it wasn't seeded. Start from an empty one"));
    }
    let mut rng = StdRng::seed_from_u64(20_261_103);
    let mut summary = SeedSummary::default();
    let start = today - LARGE_DAYS;

    let mut sources = vec![];
    for index in 0..60 {
        sources.push(app.add_source(SourceFields {
            authors: format!("{}, {}.", ["Patterson", "Tanenbaum", "Hepburn", "Stallings", "Anderson", "Cormen", "Silberschatz", "Ross"][index % 8], ["A", "B", "J", "M", "R"][index % 5]),
            year: Some(format!("{}", 2005 + index % 21)),
            title: format!("{}: notes and readings, part {}", topic_name(index * 3), index + 1),
            edition: None,
            description: (index % 3 == 0).then(|| "Lecture slides".into()),
            publisher: Some("University College Isle of Man".into()),
            url: None,
        })?);
    }
    summary.sources = sources.len();

    let mut topics: Vec<TopicId> = vec![];
    let mut module_of: Vec<Option<usize>> = vec![];
    let mut modules = vec![];
    let mut correct: HashMap<String, Vec<usize>> = HashMap::new();
    let mut typed_right: HashMap<String, String> = HashMap::new();
    let mut names = vec![];
    for (module_index, module) in MODULES.iter().enumerate() {
        modules.push(app.create_module(module)?);
        summary.modules += 1;
        let count = rng.random_range(8..=16);
        for _ in 0..count {
            let name = topic_name(names.len());
            topics.push(app.create_topic(&name, Some(modules[module_index]))?);
            module_of.push(Some(module_index));
            names.push(name);
        }
    }
    for _ in 0..6 {
        let name = topic_name(names.len());
        topics.push(app.create_topic(&name, None)?);
        module_of.push(None);
        names.push(name);
    }
    summary.topics = topics.len();

    for (topic_index, &topic) in topics.iter().enumerate() {
        let tags: Vec<String> = (0..rng.random_range(0..3)).map(|_| format!("week {}", rng.random_range(1..13))).collect::<HashSet<_>>().into_iter().collect();
        app.set_topic_tags(topic, tags)?;
        for number in 0..rng.random_range(10..=40) {
            let stem = STEMS[rng.random_range(0..STEMS.len())];
            let long = if rng.random_bool(0.2) { " Assume the default settings from the lecture and that nothing else on the system has changed since the last check." } else { "" };
            let text = format!("{stem} {} (Q{}.{number})?{long}", names[topic_index].to_lowercase(), topic_index + 1);
            let options: Vec<OptionInput> = (0..4).map(|option| OptionInput {
                text: if option == 0 { format!("The right answer for question {number}") } else { format!("A plausible distractor, number {option}") },
                correct: option == 0,
            }).collect();
            let citations = if number % 5 == 0 { vec![CitationInput { source: sources[rng.random_range(0..sources.len())], location: format!("p. {}", rng.random_range(1..400)) }] } else { vec![] };
            app.save_question(QuestionInput { topic, text: text.clone(), multiple: false, options, explanation: "Generated for the large test profile.".into(), tags: vec![], citations, short: None })?;
            correct.insert(text, vec![0]);
            summary.questions += 1;
        }
        for number in 0..rng.random_range(0..=4) {
            let text = format!("In one word, what is the key idea in {} (S{}.{number})?", names[topic_index].to_lowercase(), topic_index + 1);
            let answer = format!("answer{number}");
            app.save_question(QuestionInput {
                topic, text: text.clone(), multiple: false, options: vec![], explanation: String::new(), tags: vec![], citations: vec![],
                short: Some(ShortAnswer { check: Check::Accepted { answers: vec![answer.clone()], typos: false }, model_answer: answer.clone() }),
            })?;
            typed_right.insert(text, answer);
            summary.questions += 1;
            summary.short_answers += 1;
        }
        for number in 0..rng.random_range(10..=40) {
            let note = if number % 6 == 5 {
                Note { topic, kind: NoteKind::Cloze, front: String::new(), back: String::new(), cues: vec![], text: format!("In {}, the {{{{c1::first idea}}}} leads to the {{{{c2::second idea}}}} (card {number}).", names[topic_index]), extra: String::new() }
            } else {
                Note { topic, kind: if number % 9 == 4 { NoteKind::Reversed } else { NoteKind::Basic }, front: format!("{} card {number}: what does this term mean?", names[topic_index]), back: "Its generated definition.".into(), cues: vec![], text: String::new(), extra: String::new() }
            };
            let day = start + rng.random_range(0..LARGE_DAYS);
            app.save_note(None, NoteInput { note, suggestion: None }, day)?;
            summary.notes += 1;
        }
    }

    // Checklists: one per module and about a quarter of the topics, spread over the months.
    let mut checklists = vec![];
    for (index, &module) in modules.iter().enumerate() {
        checklists.push((Subject::Module(module), start + index as i64));
    }
    for (index, &topic) in topics.iter().enumerate().filter(|(index, _)| index % 4 == 0) {
        checklists.push((Subject::Topic(topic), start + (index as i64 % 60)));
    }
    let mut due = vec![];
    for (subject, created) in checklists {
        let id = app.create_checklist(subject, created)?;
        let points = rng.random_range(4..=14);
        app.save_points(id, (0..points).map(|point| PointInput { text: format!("Point {}: explain it from memory, with an example", point + 1), question: None, note: None }).collect())?;
        due.push((id, points, created + 1));
        summary.checklists += 1;
    }

    // Projects with generated struggles. A few issues come up across projects.
    let issues: Vec<(&'static str, &'static str, &'static str)> = (0..40).map(|index| {
        let name: &'static str = Box::leak(format!("{} mistake {}", SUBJECTS[index % SUBJECTS.len()], index / SUBJECTS.len() + 1).into_boxed_str());
        let task: &'static str = Box::leak(format!("Do a small exercise on {} without notes", SUBJECTS[index % SUBJECTS.len()].to_lowercase()).into_boxed_str());
        (name, task, "Start from the smallest case")
    }).collect();
    let kinds = ["Coding", "Past paper", "Blender", "Lab", "Writing"];
    let mut projects = vec![];
    let mut pools: Vec<Vec<StruggleSeed>> = vec![];
    for index in 0..18 {
        projects.push(app.create_project(&format!("{} {}", ["Lab", "Past paper", "Coursework", "Side project", "Revision"][index % 5], topic_name(index * 5)), kinds[index % kinds.len()], start + index as i64)?);
        summary.projects += 1;
        pools.push((0..12).map(|number| {
            let what: &'static str = Box::leak(format!("Struggle {number} in project {}: got stuck on a step", index + 1).into_boxed_str());
            let kind = [StruggleKind::Recall, StruggleKind::Apply, StruggleKind::Unsure][number % 3];
            let issue = (kind == StruggleKind::Apply).then(|| issues[(index * 3 + number) % issues.len()]);
            (what, "It looked like something else at first", kind, issue)
        }).collect());
    }
    let mut next = vec![0usize; projects.len()];

    for day in start..today {
        let progress = (day - start) as f64 / LARGE_DAYS as f64;
        for drill in app.drills_due(day)? {
            let rating = if drill.runs == 0 && rng.random_bool(0.5) { DrillRating::Shaky } else { DrillRating::Solid };
            app.rate_drill(drill.id, rating, rng.random_range(240..1200), "", day)?;
            summary.drill_runs += 1;
        }
        if day % 2 == 0 {
            let which = rng.random_range(0..projects.len());
            work_session(app, projects[which], &pools[which], &mut next[which], day, rng.random_bool(0.9), &mut rng, &mut summary)?;
        }
        if rng.random_bool(0.85) {
            summary.reviews += review_day(app, day, &HashSet::new(), &mut rng)?;
        }
        for _ in 0..rng.random_range(1..=2) {
            quiz(app, day, progress, &topics, &correct, &typed_right, &mut rng)?;
            summary.quizzes += 1;
        }
        for (id, points, next_due) in due.iter_mut() {
            if *next_due != day || rng.random_bool(0.15) {
                if *next_due == day { *next_due += 1; }
                continue;
            }
            let share = (0.55 + progress * 0.4).clamp(0.2, 1.0);
            let recalled: Vec<usize> = (0..*points).filter(|_| rng.random_bool(share)).collect();
            let result = app.finish_check(*id, None, recalled, day)?;
            *next_due = result.next_due.max(day + 1);
            summary.checks += 1;
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use study_store::Store;

    #[test]
    #[ignore = "slow: run with --ignored to check the large profile still builds"]
    fn the_large_profile_builds() {
        let mut app = App::with_seed(Store::in_memory().unwrap(), 3);
        let summary = seed_large(&mut app, 20_760).unwrap();
        assert!(summary.topics >= 120 && summary.questions >= 3_000 && summary.notes >= 3_000, "{summary:?}");
        assert!(summary.quizzes >= 120 && summary.checklists >= 40 && summary.projects == 18, "{summary:?}");
    }
}

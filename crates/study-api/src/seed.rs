//! A realistic demo profile for trying the app out: modules, topics, questions with
//! sources, about two months of quizzes, recall checks and card reviews, a few cards that
//! keep slipping, and three projects with sessions, struggles, shared issues and drills. Everything goes through the same `App` functions the screens use,
//! with past day numbers, so the history is exactly what real use would leave behind.
//! Deterministic: the same `today` always gives the same profile.

use std::collections::{HashMap, HashSet};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use study_core::bibliography::SourceFields;
use study_core::cards::{Note, NoteKind, Rating};
use study_core::question::{SourceId, TopicId};
use study_core::results::GivenReason;
use study_store::recall::Subject;

use crate::cards::{NoteInput, ReviewStep};
use crate::practice::{Clock, ReviewChoice, StruggleInput};
use study_core::practice::{DrillRating, StruggleKind};
use study_store::practice::ProjectId;
use crate::recall::PointInput;
use crate::{ApiError, ApiResult, App, CitationInput, Next, OptionInput, QuestionInput, QuizSetupInput};
use study_core::short::{Check, Keyword, ShortAnswer};

/// How far back the history goes.
pub const DAYS: i64 = 60;

/// What was made, for the command line to report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SeedSummary {
    pub modules: usize,
    pub topics: usize,
    pub questions: usize,
    pub short_answers: usize,
    pub sources: usize,
    pub quizzes: usize,
    pub checklists: usize,
    pub checks: usize,
    pub notes: usize,
    pub reviews: usize,
    pub projects: usize,
    pub sessions: usize,
    pub struggles: usize,
    pub drill_runs: usize,
}

/// (what tripped you up, why it wasn't obvious, kind, the issue and its drill if it needs one)
pub(crate) type StruggleSeed = (&'static str, &'static str, StruggleKind, Option<(&'static str, &'static str, &'static str)>);

const OFF_BY_ONE: (&str, &str, &str) = ("Off-by-one in loops", "Write a loop over a slice without an off-by-one", "Index with 0..len, or skip indexes with .iter()");

const PROJECTS: &[(&str, &str, &[StruggleSeed])] = &[
    ("Study app", "Coding", &[
        ("Loop ran one too many times reading the file", "Used <= out of habit from the C course", StruggleKind::Apply, Some(OFF_BY_ONE)),
        ("Forgot how to slice a string from the end", "I was thinking in Python's negative indexes", StruggleKind::Recall, None),
        ("Couldn't borrow inside a closure that outlives the loop", "I thought of it as a lifetime problem, not an ownership one", StruggleKind::Apply, Some(("Borrowing in closures", "Spawn 3 threads that each use a value from a Vec, without cloning the Vec", "move it in, and Arc if they share it"))),
        ("Lifetime error returning a reference from a match", "Didn't see the match arm was borrowing the temporary", StruggleKind::Unsure, None),
    ]),
    ("Blender mug", "Blender", &[
        ("Mirror modifier left a seam down the middle", "Didn't know Clipping had to be on", StruggleKind::Apply, Some(("Mirror seams", "Mirror a simple mug with a clean centre seam", "Apply scale first, then turn on Clipping"))),
        ("Couldn't remember which modifier keeps a bevel even", "Mixed up Bevel and Solidify", StruggleKind::Recall, None),
        ("Handle normals came out inverted", "Forgot to recalculate after the boolean", StruggleKind::Unsure, None),
    ]),
    ("Networks past paper 2024", "Past paper", &[
        ("Counted one extra host in the subnet", "Forgot the network and broadcast addresses", StruggleKind::Apply, Some(OFF_BY_ONE)),
        ("Mixed up which layer adds port numbers", "Learned the layers as a list, not by what each does", StruggleKind::Recall, None),
        ("The /27 mask took ages", "Still doing powers of two by hand", StruggleKind::Apply, Some(("Subnet maths", "Work out the hosts and mask for /24 to /30 without a table", "Hosts = 2^(32 - n) - 2"))),
    ]),
];

fn at(day: i64, minute: u32) -> Clock {
    Clock { day, minute, unix: day * 86_400 + minute as i64 * 60 }
}

struct TopicSeed {
    name: &'static str,
    tags: &'static [&'static str],
    /// (question, options with the correct ones marked by a leading '*', explanation)
    questions: &'static [(&'static str, &'static [&'static str], &'static str)],
    /// (front, back): Basic cards. A front starting with "<>" makes a reversed card.
    cards: &'static [(&'static str, &'static str)],
    /// Cloze texts.
    clozes: &'static [&'static str],
}

struct ModuleSeed {
    name: &'static str,
    topics: &'static [TopicSeed],
}

const MODULES: &[ModuleSeed] = &[
    ModuleSeed {
        name: "Computer Architecture",
        topics: &[
            TopicSeed {
                name: "CPU",
                tags: &["exam", "week 3"],
                questions: &[
                    ("Which pipeline stage reads the registers?", &["Fetch", "*Decode", "Execute", "Write back"], "Operands are read from the register file in **decode**, ready for execute."),
                    ("What does a branch predictor guess?", &["The next instruction's opcode", "*Whether a branch is taken", "Which cache line to evict", "The result of an ALU operation"], "It guesses taken or not taken so fetch can carry on without waiting."),
                    ("Which hazard does forwarding *not* fully remove?", &["Write after write", "*Load-use", "Structural", "Control"], "The loaded value only exists after the memory stage, so one stall is still needed."),
                    ("A program spends 60% of its time in code sped up 4×. What's the overall speedup?", &["4×", "2.4×", "*About 1.82×", "1.6×"], "Amdahl's law: $S = \\frac{1}{(1-p) + p/n} = \\frac{1}{0.4 + 0.15} \\approx 1.82$"),
                    ("Which of these are RISC traits? Pick two.", &["*Fixed-length instructions", "Many addressing modes", "*Load/store architecture", "Microcoded complex instructions"], "RISC keeps instructions simple and uniform and only touches memory with loads and stores."),
                ],
                cards: &[
                    ("I need the CPU to fetch an instruction and its data in the same cycle. Which design lets it?", "**Harvard architecture**: separate instruction and data memory."),
                    ("A load is followed straight away by an instruction using its result. What still forces a stall with forwarding?", "A **load-use hazard**. The value isn't ready until after the memory stage."),
                    ("I've sped up part of a program. How do I work out the overall speedup?", "Amdahl's law: $S = \\frac{1}{(1-p) + p/n}$"),
                    ("The compiler keeps caching a memory-mapped register in a CPU register. What stops it?", "Declare it `volatile`:\n\n```c\nvolatile uint32_t *status = (uint32_t *)0x40021000;\n```"),
                    ("<>Superscalar", "Issues more than one instruction per clock cycle"),
                ],
                clozes: &["The classic RISC pipeline is {{c1::fetch}}, {{c2::decode}}, execute, {{c3::memory}} and write back."],
            },
            TopicSeed {
                name: "Memory",
                tags: &["exam"],
                questions: &[
                    ("What does the TLB cache?", &["Instructions", "*Page table entries", "Dirty cache lines", "Disk blocks"], "The TLB holds recent virtual to physical translations."),
                    ("Which replacement policy can suffer Belady's anomaly?", &["LRU", "*FIFO", "Optimal", "LFU"], "With FIFO, adding frames can increase the number of page faults."),
                    ("Write-back caches write to memory when…", &["every write happens", "*a dirty line is evicted", "the TLB misses", "a read misses"], "Writes stay in the cache until the line is evicted, marked by a dirty bit."),
                    ("Roughly how many cycles does an L1 hit take?", &["*About 4", "About 40", "About 200", "About 1,000"], "L1 is around 4 cycles, L2 around 12, L3 around 40, DRAM in the hundreds."),
                ],
                cards: &[
                    ("I keep translating the same pages. What caches the translations?", "The **TLB**, a cache of page table entries."),
                    ("Why might adding frames make FIFO worse?", "**Belady's anomaly**: FIFO isn't a stack algorithm, so more frames can mean more faults."),
                    ("<>Thrashing", "Spending more time paging than running"),
                    ("A write only reaches memory when its line is evicted. What kind of cache is this?", "**Write-back**, using a dirty bit."),
                ],
                clozes: &["A cache hit takes about {{c1::4::a few}} cycles at L1, {{c2::12}} at L2 and {{c3::40::tens of}} at L3."],
            },
        ],
    },
    ModuleSeed {
        name: "Networks",
        topics: &[
            TopicSeed {
                name: "OSI model",
                tags: &["week 1"],
                questions: &[
                    ("Which layer adds port numbers?", &["Network", "*Transport", "Session", "Data link"], "TCP and UDP ports live at the **transport** layer."),
                    ("Which layer do routers work at?", &["Physical", "Data link", "*Network", "Application"], "Routers forward by IP address, the network layer."),
                    ("What's the PDU at the data link layer?", &["Segment", "Packet", "*Frame", "Bit"], "Segments at transport, packets at network, frames at data link."),
                    ("Which layer handles encryption and character encoding in the OSI model?", &["Application", "*Presentation", "Session", "Transport"], "Presentation is about how data is represented."),
                ],
                cards: &[
                    ("A packet needs to cross to another network. Which layer picks the route?", "The **network** layer (layer 3), using IP addresses."),
                    ("Two apps on one machine both use the network. What tells their traffic apart?", "**Port numbers**, at the transport layer."),
                    ("<>Frame", "The PDU at the data link layer"),
                ],
                clozes: &["Top to bottom: {{c1::application}}, {{c2::presentation}}, session, {{c3::transport}}, network, data link, physical."],
            },
            TopicSeed {
                name: "TCP/IP",
                tags: &[],
                questions: &[
                    ("What's the order of the TCP handshake?", &["ACK, SYN, SYN-ACK", "*SYN, SYN-ACK, ACK", "SYN, ACK, FIN", "SYN-ACK, SYN, ACK"], "Client SYN, server SYN-ACK, client ACK."),
                    ("Which protocol resolves an IP address to a MAC address?", &["DNS", "DHCP", "*ARP", "ICMP"], "ARP asks who has an IP on the local network."),
                    ("Which of these use UDP? Pick two.", &["*DNS lookups", "HTTP/1.1", "*DHCP", "SSH"], "DNS queries and DHCP are small and connectionless."),
                ],
                cards: &[
                    ("I know a machine's IP on my LAN but need its hardware address. What finds it?", "**ARP**"),
                    ("A connection opens with three messages. What are they?", "SYN, SYN-ACK, ACK"),
                ],
                clozes: &["DHCP hands out addresses in four steps: {{c1::discover}}, {{c2::offer}}, request, {{c3::acknowledge}}."],
            },
        ],
    },
    ModuleSeed {
        name: "Cyber Security",
        topics: &[TopicSeed {
            name: "Cryptography",
            tags: &["exam", "week 5"],
            questions: &[
                ("Which is a symmetric cipher?", &["RSA", "*AES", "ECDSA", "Diffie-Hellman"], "AES uses one shared key for both directions."),
                ("What does a salt protect password hashes against?", &["Brute force on one hash", "*Precomputed rainbow tables", "Keyloggers", "Phishing"], "A unique salt per password makes precomputed tables useless."),
                ("Which property means you can't find two inputs with the same hash?", &["Preimage resistance", "*Collision resistance", "Avalanche effect", "Determinism"], "Collision resistance is about any two inputs, preimage about a given output."),
                ("Which should store passwords? Pick two.", &["SHA-256 on its own", "*Argon2", "*bcrypt", "MD5"], "Password hashes should be slow and salted. Argon2 and bcrypt are built for it."),
            ],
            cards: &[
                ("Two people want to agree a key over a channel anyone can read. What lets them?", "**Diffie-Hellman** key exchange"),
                ("Someone has a table of hashes for common passwords. What makes it useless?", "A unique **salt** per password"),
                ("<>Collision resistance", "Hard to find any two inputs with the same hash"),
                ("I need to store passwords. Which hash should I use?", "A slow, salted one built for passwords: **Argon2** or **bcrypt**."),
            ],
            clozes: &["AES is a {{c1::symmetric}} cipher, RSA is {{c2::asymmetric}}."],
        }],
    },
];

/// Short answer questions: (topic, question, accepted answers or keywords, model answer, keywords?).
const SHORT_ANSWERS: &[(&str, &str, &[&str], &str, bool)] = &[
    ("CPU", "Which pipeline stage reads the register file?", &["Decode", "Instruction decode", "ID"], "**Decode**, so the operands are ready for execute.", false),
    ("CPU", "What's the name for running more than one instruction per cycle?", &["Superscalar", "Superscalar execution"], "", false),
    ("Memory", "What caches recent virtual to physical translations?", &["TLB", "Translation lookaside buffer"], "The **TLB** (translation lookaside buffer)", false),
    ("OSI model", "Which OSI layer gives each application its own port number?", &["Transport layer", "Transport", "Layer 4"], "The **transport** layer (layer 4). TCP and UDP ports live here.", false),
    ("TCP/IP", "What three messages open a TCP connection, in order?", &["SYN", "SYN-ACK|synack", "ACK"], "SYN from the client, SYN-ACK from the server, then ACK from the client.", true),
    ("Cryptography", "What makes a table of precomputed password hashes useless?", &["Salt", "A salt", "Salting", "Unique salt"], "A unique **salt** per password", false),
];

/// Fronts that the simulated you keeps getting wrong, so they end up flagged to rewrite.
const SLIPPERY: &[&str] = &[
    "I've sped up part of a program. How do I work out the overall speedup?",
    "Why might adding frames make FIFO worse?",
];

const CHECKLISTS: &[(&str, &[&str])] = &[
    ("Computer Architecture", &["Five pipeline stages and what each does", "Data, control and structural hazards", "Forwarding and when it can't help", "Amdahl's law: $S = \\frac{1}{(1-p) + p/n}$", "Cache levels and rough latencies", "Write-back vs write-through", "What the TLB does", "Belady's anomaly"]),
    ("OSI model", &["All seven layers in order", "The PDU at each layer", "Which devices work at which layer", "Where ports and IP addresses live", "Encapsulation going down the stack"]),
    ("Cryptography", &["Symmetric vs asymmetric, with an example of each", "What a hash function guarantees", "Salts and why they matter", "Diffie-Hellman in one sentence", "Which hashes to use for passwords", "Digital signatures: sign with private, check with public"]),
];

/// Fills an empty database with the demo profile, as if used for the last [`DAYS`] days.
pub fn seed_profile(app: &mut App, today: i64) -> ApiResult<SeedSummary> {
    if !app.topics()?.is_empty() || !app.modules()?.is_empty() {
        return Err(ApiError::new("the database already has data, so it wasn't seeded. Start from an empty one"));
    }
    let mut rng = StdRng::seed_from_u64(20_260_922);
    let mut summary = SeedSummary::default();
    let start = today - DAYS;

    let sources = add_sources(app)?;
    summary.sources = sources.len();

    // Modules, topics and questions, remembering which options are right for the quizzes.
    let mut correct: HashMap<String, Vec<usize>> = HashMap::new();
    let mut topic_ids: Vec<(TopicId, &TopicSeed)> = Vec::new();
    let mut names: HashMap<&str, Subject> = HashMap::new();
    for module in MODULES {
        let module_id = app.create_module(module.name)?;
        names.insert(module.name, Subject::Module(module_id));
        summary.modules += 1;
        for topic in module.topics {
            let id = app.create_topic(topic.name, Some(module_id))?;
            names.insert(topic.name, Subject::Topic(id));
            app.set_topic_tags(id, topic.tags.iter().map(|tag| tag.to_string()).collect())?;
            for (index, (text, options, explanation)) in topic.questions.iter().enumerate() {
                let picks: Vec<usize> = (0..options.len()).filter(|&i| options[i].starts_with('*')).collect();
                let citations = if index == 0 {
                    vec![CitationInput { source: sources[topic_ids.len() % sources.len()], location: format!("p. {}", 40 + index * 7) }]
                } else {
                    vec![]
                };
                app.save_question(QuestionInput {
                    topic: id,
                    text: text.to_string(),
                    multiple: picks.len() > 1,
                    options: options.iter().map(|option| OptionInput { text: option.trim_start_matches('*').to_string(), correct: option.starts_with('*') }).collect(),
                    explanation: explanation.to_string(),
                    tags: vec![],
                    citations,
                    short: None,
                })?;
                correct.insert(text.to_string(), picks);
                summary.questions += 1;
            }
            topic_ids.push((id, topic));
            summary.topics += 1;
        }
    }

    // Short answer questions, remembering a right answer to type for each.
    let mut typed_right: HashMap<String, String> = HashMap::new();
    for (topic, text, answers, model, keywords) in SHORT_ANSWERS {
        let Subject::Topic(id) = names[topic] else { continue };
        let check = if *keywords {
            Check::Keywords {
                keywords: answers.iter().map(|entry| {
                    let mut forms = entry.split('|');
                    Keyword { word: forms.next().unwrap_or_default().into(), also: forms.map(String::from).collect() }
                }).collect(),
                needed: None,
            }
        } else {
            Check::Accepted { answers: answers.iter().map(|answer| answer.to_string()).collect(), typos: true }
        };
        app.save_question(QuestionInput {
            topic: id,
            text: text.to_string(),
            multiple: false,
            options: vec![],
            explanation: String::new(),
            tags: vec![],
            citations: vec![],
            short: Some(ShortAnswer { check, model_answer: model.to_string() }),
        })?;
        let right = if *keywords { answers.iter().map(|entry| entry.split('|').next().unwrap_or_default()).collect::<Vec<_>>().join(", ") } else { answers[0].to_string() };
        typed_right.insert(text.to_string(), right);
        summary.questions += 1;
        summary.short_answers += 1;
    }

    // Cards: most written at the start, a few more recently, and a couple today, so all
    // three queues have something in them.
    let mut slippery_notes = HashSet::new();
    for (topic_index, (topic, seed)) in topic_ids.iter().enumerate() {
        let notes = seed.cards.iter().map(|(front, back)| {
            let (kind, front) = match front.strip_prefix("<>") {
                Some(rest) => (NoteKind::Reversed, rest),
                None => (NoteKind::Basic, *front),
            };
            Note { topic: *topic, kind, front: front.into(), back: back.to_string(), cues: vec![], text: String::new(), extra: String::new() }
        });
        let clozes = seed.clozes.iter().map(|text| Note { topic: *topic, kind: NoteKind::Cloze, front: String::new(), back: String::new(), cues: vec![], text: text.to_string(), extra: String::new() });
        for (index, note) in notes.chain(clozes).enumerate() {
            let day = match (topic_index + index) % 9 {
                7 => today - 3,
                8 => today,
                _ => start,
            };
            let slippery = SLIPPERY.contains(&note.front.as_str());
            let id = app.save_note(None, NoteInput { note, suggestion: None }, day)?;
            if slippery {
                slippery_notes.insert(id);
            }
            summary.notes += 1;
        }
    }

    // Recall checklists, created at the start.
    let mut checklists = Vec::new();
    for (name, points) in CHECKLISTS {
        let id = app.create_checklist(names[name], start)?;
        app.save_points(id, points.iter().map(|text| PointInput { text: text.to_string(), question: None, note: None }).collect())?;
        checklists.push((id, points.len(), start));
        summary.checklists += 1;
    }

    // Day by day: cards every day you study, a quiz every few days, recall checks when due.
    // Today is left untouched so there's something due.
    // Projects: made a little after the start, worked on every few days (R87).
    let mut projects: Vec<ProjectId> = vec![];
    for (name, kind, _) in PROJECTS {
        projects.push(app.create_project(name, kind, start + 3)?);
        summary.projects += 1;
    }
    let mut next_struggle = [0usize; 3];

    for day in start..today {
        let progress = (day - start) as f64 / DAYS as f64;
        // Drills due today get done first, getting better on the second go (R93).
        for drill in app.drills_due(day)? {
            let rating = match (drill.runs, rng.random_range(0..10)) {
                (0, 0..2) => DrillRating::Struggled,
                (0, 2..6) => DrillRating::Shaky,
                (_, 0..2) => DrillRating::Shaky,
                _ => DrillRating::Solid,
            };
            app.rate_drill(drill.id, rating, rng.random_range(240..1200), "", day)?;
            summary.drill_runs += 1;
        }
        if day >= start + 4 && (day - start) % 5 == 4 {
            let which = summary.sessions % PROJECTS.len();
            work_session(app, projects[which], PROJECTS[which].2, &mut next_struggle[which], day, true, &mut rng, &mut summary)?;
        }
        if rng.random_bool(0.85) {
            summary.reviews += review_day(app, day, &slippery_notes, &mut rng)?;
        }
        if (day - start) % 4 == 1 {
            let ids: Vec<TopicId> = topic_ids.iter().map(|(id, _)| *id).collect();
            quiz(app, day, progress, &ids, &correct, &typed_right, &mut rng)?;
            summary.quizzes += 1;
        }
        for (index, (id, points, due)) in checklists.iter_mut().enumerate() {
            if *due != day {
                continue;
            }
            let share = (0.6 + progress * 0.35 + rng.random_range(-0.1..0.1)).clamp(0.2, 1.0);
            let recalled: Vec<usize> = (0..*points).filter(|_| rng.random_bool(share)).collect();
            // The first checklist skips the check that would take it past today, so it's late.
            if index == 0 && (day > today - 8 || app.preview_check(*id, recalled.len(), day)?.next_due >= today - 2) {
                continue;
            }
            let dump = (!recalled.is_empty()).then(|| "Wrote down what I remembered before checking.".to_string());
            let result = app.finish_check(*id, dump, recalled, day)?;
            *due = result.next_due.max(day + 1);
            summary.checks += 1;
        }
    }
    // Yesterday's session on the study app: the off-by-one came back, and one struggle is
    // left to review, so there's a drill due today and something waiting in the project.
    next_struggle[0] = 0;
    work_session(app, projects[0], PROJECTS[0].2, &mut next_struggle[0], today - 1, true, &mut rng, &mut summary)?;
    app.start_session(projects[0], at(today - 1, 21 * 60))?;
    app.log_struggle(projects[0], StruggleInput { what: "Iterator chain wouldn't collect into a HashMap".into(), why: "Didn't know collect needs the target type spelled out".into(), kind: StruggleKind::Unsure, topic: None }, at(today - 1, 21 * 60 + 20))?;
    app.end_session(projects[0], at(today - 1, 21 * 60 + 45))?;
    summary.sessions += 1;
    summary.struggles += 1;
    Ok(summary)
}

/// One work session: log one to three struggles, then review them like the app asks (R90, R91).
#[allow(clippy::too_many_arguments)]
pub(crate) fn work_session(app: &mut App, project: ProjectId, pool: &[StruggleSeed], next: &mut usize, day: i64, review: bool, rng: &mut StdRng, summary: &mut SeedSummary) -> ApiResult<()> {
    app.start_session(project, at(day, 18 * 60))?;
    let count = rng.random_range(1..=3).min(pool.len());
    let mut logged = vec![];
    for offset in 0..count {
        let (what, why, kind, issue) = pool[(*next + offset) % pool.len()];
        let view = app.log_struggle(project, StruggleInput { what: what.into(), why: why.into(), kind, topic: None }, at(day, 18 * 60 + 10 + offset as u32 * 20))?;
        logged.push((view.id, kind, issue));
        summary.struggles += 1;
    }
    *next += count;
    app.end_session(project, at(day, 19 * 60 + 30))?;
    summary.sessions += 1;
    if !review {
        return Ok(());
    }
    for (id, kind, issue) in logged {
        let choice = match (kind, issue) {
            (StruggleKind::Recall, _) => ReviewChoice::Card,
            (StruggleKind::Apply, Some((name, task, hints))) => match app.known_drill(name)? {
                Some(drill) if drill.due.is_some() => ReviewChoice::Nothing,
                Some(drill) => ReviewChoice::DrillAgain { issue: name.into(), drill: drill.id, day: day + 1 },
                None => ReviewChoice::NewDrill { issue: name.into(), task: task.into(), hints: hints.into(), topic: None, day: day + 1 },
            },
            _ => ReviewChoice::Nothing,
        };
        app.review_struggle(id, choice, day)?;
    }
    Ok(())
}

fn add_sources(app: &mut App) -> ApiResult<Vec<SourceId>> {
    let source = |authors: &str, year: &str, title: &str, edition: Option<&str>, description: Option<&str>, publisher: Option<&str>| SourceFields {
        authors: authors.into(),
        year: Some(year.into()),
        title: title.into(),
        edition: edition.map(Into::into),
        description: description.map(Into::into),
        publisher: publisher.map(Into::into),
        url: None,
    };
    [
        source("Patterson, D. A., & Hennessy, J. L.", "2020", "Computer organization and design", Some("RISC-V"), None, Some("Morgan Kaufmann")),
        source("Tanenbaum, A. S., & Wetherall, D. J.", "2021", "Computer networks", Some("6th"), None, Some("Pearson")),
        source("Hepburn, M.", "2026", "Week 5: Cryptography basics", None, Some("Lecture slides"), Some("University College Isle of Man")),
    ]
    .into_iter()
    .map(|fields| app.add_source(fields))
    .collect()
}

/// One day's review session. Returns how many grades were given.
pub(crate) fn review_day(app: &mut App, day: i64, slippery: &HashSet<study_store::cards::NoteId>, rng: &mut StdRng) -> ApiResult<usize> {
    let mut step = app.start_review(None, day)?;
    let mut seen = HashSet::new();
    let mut graded = 0;
    while let ReviewStep::Card(card) = step {
        // A card shown again after Again gets Hard, so every session ends.
        let rating = if !seen.insert(card.card) {
            Rating::Hard
        } else if slippery.contains(&card.note) {
            if rng.random_bool(0.6) { Rating::Again } else { Rating::Hard }
        } else {
            match rng.random_range(0..100) {
                0..6 => Rating::Again,
                6..20 => Rating::Hard,
                22..90 => Rating::Good,
                _ => Rating::Easy,
            }
        };
        step = app.grade_card(rating, rng.random_range(4..40), day)?;
        graded += 1;
        if graded > 2_000 {
            return Err(ApiError::new("the seed's review session never ended"));
        }
    }
    app.leave_review();
    Ok(graded)
}

/// One quiz over two or three topics, getting better as the weeks go on.
/// A typed answer with two neighbouring letters of its longest word swapped.
fn with_typo(answer: &str) -> String {
    let longest = answer.split_whitespace().max_by_key(|word| word.len()).unwrap_or(answer);
    if longest.len() < 6 || !longest.chars().all(char::is_alphabetic) {
        return answer.to_lowercase();
    }
    let mut letters: Vec<char> = longest.chars().collect();
    let middle = letters.len() / 2;
    letters.swap(middle, middle + 1);
    answer.replacen(longest, &letters.into_iter().collect::<String>(), 1).to_lowercase()
}

pub(crate) fn quiz(app: &mut App, day: i64, progress: f64, topics: &[TopicId], correct: &HashMap<String, Vec<usize>>, typed_right: &HashMap<String, String>, rng: &mut StdRng) -> ApiResult<()> {
    let first = rng.random_range(0..topics.len());
    let chosen: Vec<TopicId> = (0..rng.random_range(2..=3)).map(|offset| topics[(first + offset) % topics.len()]).collect();
    let exam = rng.random_bool(0.25);
    let mut view = app.start_quiz(QuizSetupInput {
        topics: chosen,
        count: Some(8),
        even: true,
        shuffle_questions: true,
        shuffle_options: false,
        exam,
        total_minutes: None,
        per_question_seconds: None,
        easy: false,
        types: crate::QuestionTypes::Both,
    })?;
    let skill = 0.55 + progress * 0.3;
    let attempt = loop {
        if view.short {
            let typed = if rng.random_bool(skill) {
                let right = &typed_right[&view.text];
                if rng.random_bool(0.2) { with_typo(right) } else { right.clone() }
            } else {
                ["not sure", "the network one", "something about caching"][rng.random_range(0..3)].to_string()
            };
            match app.answer_text(typed, day)?.next {
                Next::Question(next) => view = next,
                Next::Finished(attempt) => break attempt,
            }
            continue;
        }
        let right = &correct[&view.text];
        let picked = if rng.random_bool(skill) {
            if rng.random_bool(0.15) {
                app.toggle_flag()?; // right, but guessed
            }
            right.clone()
        } else {
            let wrong = (0..view.options.len()).find(|index| !right.contains(index)).unwrap_or(0);
            if right.len() > 1 { vec![right[0], wrong] } else { vec![wrong] }
        };
        match app.answer(picked, day)?.next {
            Next::Question(next) => view = next,
            Next::Finished(attempt) => break attempt,
        }
    };
    // Now and then a wrong short answer is overruled, as if it was close enough.
    let wrong_short: Vec<usize> = app.results(attempt)?.review.iter().filter(|item| item.short.as_ref().is_some_and(|short| !short.marking.right)).map(|item| item.index).collect();
    for index in wrong_short {
        if rng.random_bool(0.15) {
            app.overrule_result(attempt, index)?;
        }
    }
    let reasons = [GivenReason::Misremembered, GivenReason::ConfusedWithSimilar, GivenReason::CarelessSlip, GivenReason::NeverLearned, GivenReason::SecondGuessed];
    for index in app.results(attempt)?.to_prompt {
        if rng.random_bool(0.7) {
            app.give_reason(attempt, index, reasons[rng.random_range(0..reasons.len())].clone())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use study_core::progress::Overview;
    use study_store::Store;

    const TODAY: i64 = 20_760;

    fn seeded() -> (App, SeedSummary) {
        let mut app = App::with_seed(Store::in_memory().unwrap(), 3);
        let summary = seed_profile(&mut app, TODAY).unwrap();
        (app, summary)
    }

    #[test]
    fn fills_every_part_of_the_app() {
        let (app, summary) = seeded();
        assert_eq!(summary.modules, 3);
        assert_eq!(summary.topics, 5);
        assert_eq!(summary.questions, 26);
        assert_eq!(summary.short_answers, 6);
        assert_eq!(app.topics().unwrap().iter().map(|topic| topic.short_count).sum::<usize>(), 6);
        assert_eq!(summary.sources, 3);
        assert_eq!(app.sources().unwrap().iter().filter(|source| source.cited_by_questions > 0).count(), 3);
        assert!(summary.quizzes >= 14, "{summary:?}");
        assert_eq!(app.history().unwrap().len(), summary.quizzes);
        assert!(matches!(app.overview(TODAY).unwrap(), Overview::Progress(_)));
        assert!(summary.checks >= 9, "{summary:?}");
    }

    #[test]
    fn projects_have_struggles_issues_drills_and_quiz_schedules() {
        let (app, summary) = seeded();
        assert_eq!(summary.projects, 3);
        assert!(summary.sessions >= 10 && summary.struggles >= 15 && summary.drill_runs >= 3, "{summary:?}");
        let projects = app.projects(TODAY).unwrap();
        assert!(projects.iter().all(|p| p.struggles > 0 && p.session.is_none()));
        assert_eq!(projects.iter().map(|p| p.to_review).sum::<usize>(), 1, "one struggle waits for review");
        let issues = app.issues().unwrap();
        assert!(issues.len() >= 3);
        assert!(issues.iter().any(|i| i.projects.len() == 2), "an issue shared by two projects");
        assert!(!app.drills_due(TODAY).unwrap().is_empty(), "a drill is due today");
        assert!(app.card_suggestions(TODAY).unwrap().iter().any(|s| s.kind == crate::cards::SuggestionKind::Struggle));
        assert!(app.topic_quizzes(TODAY).unwrap().len() >= 4, "quizzes started the topics' schedules");
        let month = app.calendar_month(TODAY - 30, TODAY + 14, TODAY).unwrap();
        assert!(month.iter().any(|d| d.struggles > 0) && month.iter().any(|d| d.day > TODAY && d.quizzes > 0));
    }

    #[test]
    fn cards_have_history_due_cards_and_leeches() {
        let (mut app, summary) = seeded();
        assert!(summary.reviews >= 250, "two months of reviews: {summary:?}");
        let home = app.cards_home(TODAY).unwrap();
        assert!(home.due.review > 0 && home.due.new > 0, "{:?}", home.due);
        assert!(home.retention.is_some());
        assert!(home.forecast.iter().skip(1).any(|&count| count > 0));
        assert!(home.suggestions > 0, "recall misses and quiz mistakes become suggestions");
        let leeches: usize = home.per_topic.iter().map(|row| app.topic_cards(row.topic, TODAY).unwrap().cards.iter().filter(|card| card.leech).count()).sum();
        assert!(leeches >= 2, "the slippery cards are flagged");
        assert!(app.fit_scheduler().unwrap() >= 50, "enough history to fit the scheduler");
    }

    #[test]
    fn something_is_due_for_recall_and_nothing_is_in_the_future_history() {
        let (app, _) = seeded();
        let rows = app.recall_list(TODAY).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.last_score.is_some()));
        assert!(rows.iter().any(|row| row.due_today && row.days_late > 0), "one checklist is late");
        assert!(app.history().unwrap().iter().all(|row| row.day < TODAY));
    }

    #[test]
    fn quizzes_include_typed_short_answers_some_overruled() {
        let (app, _) = seeded();
        let mut typed = 0;
        let mut typos = 0;
        let mut overruled = 0;
        for row in app.history().unwrap() {
            for item in app.results(row.id).unwrap().review {
                if let Some(short) = item.short {
                    typed += 1;
                    typos += usize::from(matches!(short.marking.detail, study_core::short::Detail::Matched { typo: Some(_), .. }));
                    overruled += usize::from(short.overruled);
                }
            }
        }
        assert!(typed >= 20 && typos >= 1 && overruled >= 1, "typed {typed}, typos {typos}, overruled {overruled}");
    }

    #[test]
    fn is_the_same_every_time() {
        let (_, first) = seeded();
        let (_, second) = seeded();
        assert_eq!(first, second);
    }

    #[test]
    fn refuses_a_database_with_data() {
        let mut app = App::with_seed(Store::in_memory().unwrap(), 3);
        app.create_topic("Mine", None).unwrap();
        let error = seed_profile(&mut app, TODAY).unwrap_err();
        assert!(error.message.contains("already has data"));
        assert_eq!(app.topics().unwrap().len(), 1);
    }
}

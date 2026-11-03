# Study App MVP Specs

Behaviour specs in Gherkin for everything a user does in the MVP. Every scenario traces to an approved requirement below. Anything that couldn't be traced is listed under Open questions instead of being specified.

## Requirements (approved)

| ID | Requirement |
|---|---|
| R1 | A topic is the deck. Questions are grouped by topic |
| R2 | A topic sits in at most one module, or in none |
| R3 | Tags can be added to topics and to questions |
| R4 | A question has 3 to 5 options |
| R5 | Each question is set to single or multiple correct answers |
| R6 | A question has an explanation |
| R7 | Question text, options and explanation are Markdown with images, fenced code (syntax highlighted) and `$...$` LaTeX |
| R8 | Images are pasted or picked from a file, and are kept with the question |
| R9 | A question needs at least one correct option to be saved |
| R10 | A question belongs to exactly one topic |
| R11 | Files (Markdown, txt, PDF, PowerPoint, EPUB) can be opened beside the editor for reading, and are not stored |
| R12 | Several files can be open while writing, switched with tabs |
| R13 | A global bibliography of sources, stored as fields and shown in APA, with a view of which topics use each source |
| R14 | A question can optionally cite one or more sources, each with a typed page, slide or line |
| R15 | Quiz setup: one or more topics, all questions or a set number, picked randomly or evenly across topics |
| R16 | Quiz setup: question order kept or shuffled, option order kept or shuffled |
| R17 | Quiz setup: optional total timer and optional per-question timer |
| R18 | Quiz setup: study mode (feedback after each question) or exam mode (feedback at the end) |
| R19 | Quiz setup: hard (correct answer count hidden, the default) or easy (count shown) |
| R20 | Exam mode does not allow going back to change an answer |
| R21 | A question can be flagged as guessed, in both modes |
| R22 | A per-question timeout counts as wrong and moves on. A total timeout ends the quiz |
| R23 | Multiple-answer questions are scored all or nothing |
| R24 | Results show the score, the number guessed, a per-topic breakdown and a review of each question with its explanation |
| R25 | A wrong answer can be given a reason: preset or free text, optional but prompted. Incomplete (multi-answer) and timed out are recorded automatically |
| R26 | Retry redoes the wrong ones and the guessed ones, with the same settings, saved as a retry |
| R27 | Attempts are saved and listed on a history screen |
| R28 | History shows each question as it was when answered |
| R29 | Quitting a quiz halfway saves it as an incomplete attempt |
| R30 | Deleting a topic or question asks for confirmation and keeps past history intact |
| R31 | Search updates as you type, with category tabs (modules, topics, tags, sources), sortable columns and combinable filters, across question text, options, explanations, tags and sources |
| R32 | Keyboard shortcuts in quizzes: 1 to 5 pick an option, Enter submits, F flags as guessed |
| R33 | An overview screen shows progress from saved attempts: accuracy for the last 7 days and its change from the 7 days before, questions answered, share of right answers that were guessed, score per quiz over time, topics ranked by their latest score, and wrong-answer reasons ranked by count. With no attempts it shows first steps instead |
| R34 | The app has a dark and a light theme, starts in dark, and remembers the chosen theme |

Preset wrong-answer reasons (R25): blank, misremembered, confused with something similar, misread the question, careless slip, second-guessed, never learned it.

## Example maps

**Organising (topics.feature)**
- Rule R2: a topic has at most one module. Example: "CPU" in "Computer Architecture". Example: "Misc" in no module.
- Rule R3: tags on topics and questions. Example: tag "hardware" on topic "GPU".
- Rule R30: deleting keeps history. Example: delete "CPU" after a quiz, history still shows its questions.

**Writing questions (questions.feature)**
- Rule R4: 3 to 5 options. Examples: 2 rejected, 3 ok, 5 ok, 6 rejected.
- Rule R9: at least one correct. Example: no correct option, save refused.
- Rule R5: single or multiple. Example: single-answer question with two correct marked.
- Rule R7/R8: rich content. Example: pasted image in an option, LaTeX in an explanation.

**Sources (sources.feature)**
- Rule R11: read, don't store. Example: open a PDF, close the app, the file isn't kept.
- Rule R12: tabs. Example: two lecture decks open at once.
- Rule R13/R14: bibliography and citations. Example: question cites "Patterson 2020, p. 42" and "Lecture 3, slide 12".

**Quiz setup and taking (quiz-setup.feature, taking-quiz.feature)**
- Rule R15: set number, random or even. Example: 10 from CPU and GPU, even split gives 5 and 5.
- Rule R18/R20: study vs exam. Example: study shows correct after each answer, exam only at the end.
- Rule R22: timeouts. Example: per-question timer runs out, counted wrong.
- Rule R23: all or nothing. Example: 1 of 2 correct picked, marked wrong and incomplete.

**After a quiz (results.feature, retry.feature, history.feature)**
- Rule R24: results contents. Example: 8/10 correct, 2 guessed.
- Rule R26: retry set. Example: 2 wrong plus 1 guessed-right gives a 3-question retry.
- Rule R28: history snapshot. Example: edit a question after a quiz, history shows the old text.

**Overview (overview.feature)**
- Rule R33: progress comes only from saved attempts. Example: 6 quizzes in the last 7 days at 63%, previous 7 days at 59%, shows "+4 points".
- Example: OSI model at 27% is listed above CPU at 85%.
- Example: no attempts yet, shows first steps instead of charts.

**Search (search.feature)**
- Rule R31: example: filter tag "hardware" in module "Computer Architecture".

## Traceability

| Req | Scenarios |
|---|---|
| R1 | topics: Create a topic |
| R2 | topics: Put a topic in a module, Topic outside any module, Move a topic to another module |
| R3 | topics: Tag a topic, questions: Tag a question |
| R4 | questions: Option count limits (outline) |
| R5 | questions: Single-answer question, Multiple-answer question |
| R6 | questions: Write a question with an explanation |
| R7 | questions: Rich content in every part (outline) |
| R8 | questions: Add an image by pasting or picking (outline) |
| R9 | questions: No correct option |
| R10 | questions: A question belongs to one topic |
| R11 | sources: Open a file beside the editor (outline), Files are not kept |
| R12 | sources: Several files in tabs |
| R13 | sources: Add a source to the bibliography, See which topics use a source |
| R14 | sources: Cite sources on a question, Question without a source |
| R15 | quiz-setup: Quiz on several topics, All questions or a set number, Even split |
| R16 | quiz-setup: Order options (outline) |
| R17 | quiz-setup: Timers are optional (outline) |
| R18 | taking-quiz: Study mode feedback, Exam mode feedback |
| R19 | taking-quiz: Easy and hard mode (outline) |
| R20 | taking-quiz: No going back in exam mode |
| R21 | taking-quiz: Flag a question as guessed (outline) |
| R22 | taking-quiz: Per-question timeout, Total timeout |
| R23 | taking-quiz: All or nothing scoring (outline) |
| R24 | results: Results summary, Review each question |
| R25 | results: Give a reason, Skip the reason, Automatic reasons (outline) |
| R26 | retry: Retry wrong and guessed, Retry uses same settings, Nothing to retry |
| R27 | history: Attempts listed in history |
| R28 | history: History shows the question as it was |
| R29 | taking-quiz: Quit halfway |
| R30 | topics: Delete a topic that is in history, questions: Delete a question that is in history |
| R31 | search: Search as you type, Search question content (outline), Search across categories (outline), Combine filters, Sort results |
| R32 | taking-quiz: Keyboard shortcuts (outline) |
| R33 | overview: Progress summary, Change from the previous week, Score per quiz, Weakest topics first, Topics never quizzed, Reasons ranked, Retries on the chart, First run |
| R34 | appearance: Dark by default, Switch to light, Choice is remembered |

## Decided open questions

All 16 open questions were answered on 2 Nov 2026 and are now requirements.

| ID | Was | Requirement |
|---|---|---|
| R35 | Q1 | A file that can't be opened or converted shows why inside its own tab. Other tabs keep working |
| R36 | Q2 | Asking for more questions than the chosen topics have uses all of them, and setup says how many are available before starting |
| R37 | Q3 | An even split gives the extra questions to topics in order (10 across 3 is 4, 3, 3). A topic short of its share gives all it has and the others make up the rest |
| R38 | Q4 | Topics with no questions can't be picked for a quiz. Setup shows them greyed out with "no questions yet" |
| R39 | Q5 | Study mode shows the explanation and sources straight after each answer |
| R40 | Q6 | There's no going back to an earlier question in either mode |
| R41 | Q7 | "N guessed" counts only right answers that were guesses. Wrong guesses count as wrong |
| R42 | Q8 | A retry can be retried, using its own wrong and guessed questions, and stays linked to the original quiz |
| R43 | Q9 | Deleting a module asks first, then moves its topics to "No module". Nothing else is deleted |
| R44 | Q10 | Deleting a cited source asks first, saying how many questions cite it, then removes those citations. History keeps them |
| R45 | Q11 | Editing a source updates what questions show. History keeps the old details |
| R46 | Q12 | A source needs a title. Every other field is optional |
| R47 | Q13 | An incomplete attempt is final. Retry covers what's left |
| R48 | Q14 | `/` focuses search from anywhere, and Ctrl+S saves the question in the editor |
| R49 | Q15 | Switching a question from multiple to single answer keeps the first tick, clears the rest and says so |
| R50 | Q16 | Search has a Questions tab: All, Questions, Modules, Topics, Tags, Sources |
| R51 | New | Topic tags can be edited from the Topics screen |

## Free recall (approved 3 Nov 2026)

| ID | Requirement |
|---|---|
| R52 | A checklist is about one module or one topic, picked when it's made |
| R53 | A checklist has points, written by hand. Each point is Markdown with `$...$` maths, and can be edited, reordered and removed |
| R54 | Points can be added from questions in the checklist's topics (MCQ now, short answer and flashcards when built) as "question → answer". Unticking an added question removes its point. The question is never changed |
| R55 | A session starts with the dump: typed in the app (saved with the check) or done outside it. The checklist stays hidden until the dump is finished |
| R56 | The check shows the dump beside the checklist. Each point is ticked recalled or not. The score, the pass mark and what will happen next update as you tick |
| R57 | Finishing a check moves the checklist up one step at or over the pass mark, or back one step under it. The next date counts from the day the check was done. The result lists missed points and how often each has been missed |
| R58 | The recall list shows what's due today first, then everything else by next date. The sidebar shows how many are due |
| R59 | Each checklist has a history: score per check against the pass mark, each check's step change and dump, and the points missed most often |
| R60 | The schedule (steps, intervals, repeats, pass mark, what happens under it) is editable and can be reset. Changes apply from each checklist's next check |
| R61 | Keyboard: Ctrl+Enter finishes the dump and the check, Space ticks the focused point |

Default schedule: the first check puts a new checklist on Day, then Week, Fortnight, Month, Monthly ×6, Seasonal ×4, then Yearly for good. Pass mark 70%. Going back from Day stays on Day.

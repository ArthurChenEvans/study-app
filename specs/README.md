# Shudhana Specs

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
| R54 | Points can be added from MCQs and flashcards in the checklist's topics (short answer when built). An MCQ becomes "question → answer". A basic or reversed card becomes "front → back", and a cloze card becomes its text with the gaps filled in and the answers in bold. A note with every card archived isn't offered. Unticking an added item removes its point. The question or card is never changed |
| R55 | A session starts with the dump: typed in the app (saved with the check) or done outside it. The checklist stays hidden until the dump is finished |
| R56 | The check shows the dump beside the checklist. Each point is ticked recalled or not. The score, the pass mark and what will happen next update as you tick |
| R57 | Finishing a check moves the checklist up one step at or over the pass mark, or back one step under it. The next date counts from the day the check was done. The result lists missed points and how often each has been missed |
| R58 | The recall list shows what's due today first, then everything else by next date. The sidebar shows how many are due |
| R59 | Each checklist has a history: score per check against the pass mark, each check's step change and dump, and the points missed most often |
| R60 | The schedule (steps, intervals, repeats, pass mark, what happens under it) is editable and can be reset. Changes apply from each checklist's next check |
| R61 | Keyboard: Ctrl+Enter finishes the dump and the check, Space ticks the focused point |

Default schedule: the first check puts a new checklist on Day, then Week, Fortnight, Month, Monthly ×6, Seasonal ×4, then Yearly for good. Pass mark 70%. Going back from Day stays on Day.

## Flashcards with FSRS (approved 3 Nov 2026)

| ID | Requirement |
|---|---|
| R62 | Cards live in topics. A note makes one or more cards: Basic (front and back), Basic and reversed (two cards, one each way) or Cloze (one card per gap number, `{{c1::answer}}` or `{{c1::answer::hint}}`, with optional extra text on the back) |
| R63 | A Basic note can have more cues for the same answer. Each cue is its own card with the same back |
| R64 | Fronts, backs and cloze text are Markdown with code, `$...$` maths and images, like questions. The cloze editor previews every card it will make |
| R65 | Scheduling uses FSRS (the fsrs crate). Each card keeps its memory state, due day, reviews and Agains |
| R66 | Review shows the front, then the answer on Show answer (Space), then Again, Hard, Good, Easy (1 to 4, Space is Good), each showing when the card would come back. Again shows the card again later in the same session |
| R67 | A review session has Undo for the last answer and can be left at any time. Answers already given are kept |
| R68 | The Cards screen shows what's due today as new, learning and to review, a 14-day forecast, retention over the last 30 days, and counts per topic. The sidebar shows how many are due |
| R69 | New cards a day and most reviews a day are limits on a session |
| R70 | A card is a leech once it has had Again a set number of times (8 by default). The review offers to rewrite it, the editor explains why, and saving a change clears the flag but keeps the schedule |
| R71 | Cards can be archived (left out of reviews) and brought back |
| R72 | Suggestions list points missed in the latest check of each recall checklist and questions answered wrong or right-but-guessed in quizzes. Nothing becomes a card until you make it. A suggestion can be dismissed |
| R73 | A card made from a quiz mistake starts as the question and its answer. One made from a recall miss starts with the point as the back and the front left for you |
| R74 | Settings: desired retention (90% by default), new cards a day (10), most reviews a day (200), the leech threshold (8). Reset to default is offered |
| R75 | "Fit the scheduler to you" runs the FSRS optimiser on your review history and says if there isn't enough yet |
| R76 | The done screen shows how many were reviewed, the share remembered against the target, time taken and cards worth rewriting |
| R77 | Editing a card keeps its schedule. Removing a cue or cloze gap removes its card |

## Short answer (approved 3 Nov 2026)

| ID | Requirement |
|---|---|
| R78 | A question is multiple choice or short answer, chosen in the editor. Switching keeps the question, explanation, sources and tags. Short answers live in the same topics, quizzes, results and history as MCQs |
| R79 | A short answer question is checked by accepted answers or by keywords, and needs at least one to be saved. It has an optional model answer (Markdown), shown after answering. Without one, the first accepted answer is shown |
| R80 | Accepted answers: right when the answer matches any of them, ignoring case, extra spaces, punctuation and a leading "the", "a" or "an". With "Allow small typos" (on by default) a word of 4 to 7 letters may have one wrong letter and a word of 8 or more two. Never in numbers or in words of 3 letters or fewer |
| R81 | Keywords: right when the answer contains all of them, or at least a chosen number, in any order. A keyword can have other spellings. Each word of the answer counts once, so "SYN-ACK" doesn't also count as "SYN" and "ACK" |
| R82 | The editor's "Try an answer" shows how an answer would be marked. Nothing is recorded |
| R83 | In a quiz the answer is typed. Enter submits, Shift+Enter adds a line, Ctrl+G flags it as guessed. Study mode shows the verdict (correct, correct with a typo, or wrong), the keywords hit and missed, the model answer and the explanation |
| R84 | A short answer can be overruled: counted as right or as wrong, in study feedback or on the results page, and undone. Results, history, Overview, retry and suggestions all use the overruled outcome, and the answer is labelled as overruled |
| R85 | Quiz setup can include both question types, only MCQs or only short answers. Topic rows show how many of each. Difficulty and option order only change MCQs |
| R86 | Short answer questions can be added to recall checklists as "question → model answer" (or the first accepted answer). Wrong or guessed ones are card suggestions that start with the question and the model answer, and show what was written |

Decided with the design: keywords are checked in any order, every answer is right or wrong (no half marks), and the typo rule is as in R80.

## Projects, drills, quiz scheduling and the calendar (approved 3 Nov 2026)

| ID | Requirement |
|---|---|
| R87 | A project is anything applied (code, Blender, a past paper). It has a name and a kind, and is never scheduled. Projects list their last worked day, struggles, struggles to review and drills due |
| R88 | Work on a project happens in sessions. A session is started and ended, with a timer. One session at a time per project |
| R89 | During a session a struggle can be logged (Ctrl+L focuses it): what tripped you up, why it wasn't obvious, couldn't recall / couldn't apply / not sure yet, and an optional topic. It records the day and time. Struggles are listed by day, searchable, and show what became of them |
| R90 | Ending a session reviews its struggles one at a time. Each can become a card suggestion, a point on a recall checklist, a drill, or nothing. "Finish later" leaves the rest to review, and they can be reviewed later from the project |
| R91 | A struggle that becomes a drill is given an issue: one used before, or a new one by name. Issues are shared by all projects, matched ignoring case. If the issue already has a drill, the review offers to schedule that drill again or write a different one |
| R92 | A drill is a small task with an optional topic and hints. Scheduling one asks for the day, with tomorrow picked. A drill can be moved to another day |
| R93 | Doing a drill shows the task, a timer, the hints if asked for, the struggles that led to it and a note, then Struggled, Shaky or Solid (keys 1 to 3). Solid ticks it off. Struggled or Shaky brings it back the next day. A ticked-off drill only comes back if it's scheduled again from a review |
| R94 | The Issues tab lists every issue, how often and in which projects it came up, when it was last seen, and the state of its latest drill |
| R95 | Card suggestions from struggles start with what tripped you up as the front, show why it wasn't obvious, and wait to be accepted like the others |
| R96 | Each topic has a quiz schedule on its own ladder (the same steps and pass mark as recall by default, edited separately). It starts the first time the topic is quizzed. In a finished quiz each topic is scored on its own questions: at or over the pass mark it moves up a step, under it goes back one, and the next quiz date counts from the quiz day. Retries and quits don't move it. An overrule on the results page redoes the move if that quiz is still the topic's latest |
| R97 | Topics show each topic's quiz step and next quiz date, with a way to quiz every due topic at once. Results show where each topic moved |
| R98 | The calendar's month view shows, for each day, what's due (cards, checklists, quizzes, drills) with counts and words. Past days show what was done and struggles logged. Today is outlined. Overdue things count as due today |
| R99 | A day view lists everything due that day with a start button for each, or for a past day what was done and the struggles logged. Days can be stepped through |
| R100 | The sidebar shows today's total next to Calendar, quizzes due next to Topics, and drills due next to Projects |
| R101 | A backup is a full copy of the database, made while the app is open without interrupting study, and never left half-written |
| R102 | Once the database has anything in it, a backup is made each day the app is used, plus one before every restore and before an app update changes the database |
| R103 | Backups are kept for the last 7 days, 4 weeks and 12 months (each changeable). Anything made today is always kept |
| R104 | "Your data" shows when the last backup was, in words, and lists the backups with when, why and size. "Back up now" makes one straight away |
| R105 | Restoring a backup first shows what will be undone, backs up the data as it is now, then puts the backup back, upgrading it if it's from an older version |
| R106 | Backups can also be copied to a second folder (another drive, a USB stick). If that folder can't be reached, the app says so and keeps backing up as normal |
| R107 | Backup settings live with the backups, not in the database, so restoring a backup never changes them |
| R108 | The sidebar shows the backup status in one line, and it links to Your data |
| R109 | Every record (module, topic, question, source, card, review, quiz, checklist, project, drill and the rest) has an ID that's the same on every computer and never reused. Records from before this get one when the app updates |
| R110 | Every record you can edit has a "last changed" stamp that moves when it, or a part of it (a question's options, a topic's tags, a checklist's points), changes. Stamps from one computer only ever go up, even if its clock is set back. Reviewing a card or doing a check isn't an edit of the card or checklist |
| R111 | Deleting a record, including the ones deleted along with it, leaves a marker with its ID, so other computers can delete it too |
| R112 | Everything can be exported to one `.studyapp` file (a zip of readable JSON and the images), with or without history, to move to another computer |
| R113 | Some topics can be exported to share: their questions, cards, checklists, sources and the images they use, with history left out unless asked for. Without history, cards and checklists start fresh for whoever imports them |
| R114 | Importing a file shows first what's new, what's here already and what's changed in both places, and changes nothing until confirmed. A backup is made first, and the import goes in all at once or not at all |
| R115 | Records are matched by ID, so importing a newer version of the same file updates what came in before instead of copying it |
| R116 | For things changed in both places, the choice is: whichever changed last, keep mine, take the file's, or look at each one. Shared topics start on keep mine, a full export on whichever changed last |
| R117 | History is only ever added: reviews, quizzes and checks already here are never replaced, and nothing here is deleted. When both sides reviewed a card (or quizzed a topic, or checked a checklist), the side that did it more recently keeps its schedule |
| R118 | A file that isn't a .studyapp file, was made by a newer version, refers to things it doesn't have, has an image that isn't a picture, or unpacks to more than 1 GB is refused with a reason, and nothing changes |
| R119 | Sync keeps computers (and the browser version) in step through a folder another app keeps the same on each one (Syncthing, Nextcloud, a network drive, a USB stick). No account, no server. The database itself never goes in the folder, only files of changes, and each file only ever has one computer writing to it |
| R120 | Setting up sync is: pick the folder, name this computer, then pick the same folder on the other one. A backup is made first. Sync can be turned off, the folder changed and the computer renamed |
| R121 | The app syncs when it starts, every 30 seconds while it's open, and on "Sync now". It waits while a quiz or a card review is in progress |
| R122 | Things that are only ever added (card reviews, quizzes, recall checks, drill runs, struggles, project sessions) all end up on every computer, with nothing to choose between |
| R123 | Edits are merged field by field: changing a card's front on one computer and its back on the other keeps both. When both computers changed the same field before seeing each other's change, the newer one is used everywhere and the pair is listed under "things to check", where the other version can be put back. Editing the field settles it |
| R124 | Deleting something deletes it on the other computers too, unless something was changed or added to it there before they saw the delete. Then it's kept on every computer and listed to check, so nothing is lost by accident |
| R125 | When the same card was reviewed on two computers before they synced, its schedule is worked out again from all its reviews, so both end up with the same one. Other schedules (a topic's quiz step, a checklist's step, a drill's due date) take the newer one |
| R126 | Something that refers to a thing that hasn't arrived yet (a question whose topic is still on its way from a third computer) waits for it rather than being dropped or breaking the sync |
| R127 | If the folder can't be found, changes are kept and counted, and go out once it's back. A change file that's half-written is read once it's complete. A computer on a newer version of the app is skipped with a message to update |
| R128 | Restoring a backup pauses sync, since the other computers would bring the newer changes straight back. Your data and the sidebar say whether sync is on, paused or has things to check, in words |
| R129 | Right, wrong and guessed use standard green and red by default, with a switch for colour-blind safe blue and orange, remembered on this device. Both always show an icon and a word, and guessed is always hatched |

## Dates on the calendar (approved 12 Nov 2026)

| ID | Requirement |
|---|---|
| R130 | A date has a name, a day and an optional kind (Exam, Deadline, Coursework and Mock exam are suggested, anything else is fine). Dates can be added, edited and deleted from the calendar, where "Add a date" is the primary button |
| R131 | A date covers nothing, a module (every topic in it, including ones added later) or picked topics. A deleted topic or module drops out of what it covers |
| R132 | A date that covers something can tighten reviews, on by default for the Exam kind. Until the day, no topic quiz, card or drill for a covered topic waits longer than half the days that were left when it was last done, so nothing lands after the date. Recall checklists aren't tightened. When two dates cover the same topic, the tighter one wins |
| R133 | Tightening is worked out, not stored: each item's due day is the earlier of its normal one and "last done + half the days that were left then". From the day after the date, items go back to their normal schedule. Editing or deleting a date puts everything back. The day before a date there's nothing left to halve, so an item done then is scheduled normally. Cards keep their FSRS memory, only the due day is pulled in. Never-quizzed topics and new cards have nothing to tighten |
| R134 | The month view shows dates as a chip at the top of their day (a cap for exams, a flag for the rest), and a "Coming up" line above the grid gives the next dates with how many days are left. Counts on each day use the tightened due days |
| R135 | The day view shows a date's own panel: how many days are left, what it covers, each covered topic's next quiz and where it was pulled in from, how many of its cards are due before it and how many were pulled in, and the never-quizzed topics with a Quiz button. A day before a covering date says how many days are left, and a tightened quiz says which date it's tightened for |
| R136 | Adding or editing a date previews what moves before saving: each topic quiz moving from one day to another, topics with no schedule to tighten, and how many cards come in before it |
| R137 | Topics show "Tightened for" a date under a tightened next quiz, and results show what the next quiz date would have been without it. Dates sync and go into full exports like everything else |

## Lists at scale (approved 12 Nov 2026)

| ID | Requirement |
|---|---|
| R138 | Long lists (Topics, the topic pickers in New quiz and the date dialog, Cards by topic) have a module rail with All, every module and No module, each with its count. Picking one narrows the list |
| R139 | A filter box narrows the list as you type, on names, modules and tags, with the match marked and "9 of 171" beside it. `/` jumps to it and `Esc` clears it. When nothing matches, it says so and offers to clear the filter, look in all modules or search inside questions |
| R140 | A status filter shows the states that matter on the screen, with counts (Topics: All, Due, Tightened, Not started) |
| R141 | Clicking a column header sorts by it, again flips it, and says which way in words. "Columns" shows or hides the optional ones (Topics: tags, questions, short answers, quiz step, next quiz), remembered on this device |
| R142 | The list scrolls in its own pane with the headers staying put, so the page is never much taller than the window and the filter, rail and item card stay in view |
| R143 | Picking a row shows its details and actions in an item card beside the list, so rows don't each carry their own buttons. `↑` `↓` move, `Enter` edits, `Space` ticks, double-click edits |
| R144 | Rows can be ticked one by one, all shown at once, or a whole module, for "Quiz them", "Move to module" or "Tag" from a bar under the list |
| R145 | New quiz picks topics with the same rail, filter and ticks, with "Tick all N in a module". Your quiz lists the ticked topics as chips that can be removed |
| R146 | At most a set number of topic quizzes are due a day (5 by default, changed in Quiz schedule), most overdue first. The rest wait, keeping their step, and Today says how many are waiting and when the last comes in. Quiz schedule warns when more come due a day than the limit |
| R147 | Search shows recent searches and recently edited questions until you type, then results 50 at a time with "Show 50 more" |
| R148 | History's chart shows the last 30 quizzes by default (90 days and All one click away), and the table is grouped by month, folded after the first two, with a filter |
| R149 | Suggestions are grouped by topic, biggest groups first and folded after the first, with "Dismiss these N" per group and a filter. Bibliography is one compact sortable row per source with a filter, the full reference in the item card. Overview shows the 8 weakest topics with a module picker |

## Celebrations and toasts (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R150 | Which celebrations you want is saved as one setting that syncs to your other computers. Sound volume, "Sound for big moments only" and motion (Follow my system, Full, Calm) stay on each computer |
| R151 | Small pops land about every 15 to 40 cards, worked out from the queue size when the session starts: a quarter of the queue, kept in range and rounded to a friendly number. Fewer and More move the range to 25 to 60 and 10 to 25. Sessions under 15 cards get no pops. Near half and three quarters the pop becomes Halfway there and Last stretch. Pops are never closer than 8 cards or within half a gap of the end |
| R152 | Celebrations settings, from the sidebar: a master switch, each big moment as Full screen, Small pop or Off, card count pops with how often, Halfway and Last stretch, answer streaks of 5 or 10, a Keep going nudge after 3, 5 or 10 quiet minutes, and toasts. Every row has a Play button. Reset to defaults puts it all back. With the master switch off the end-of-session summary still shows, plainly |
| R153 | At most one small moment per answer. A streak wins over a count or marker on the same card. The nudge comes on the next right answer after a quiet stretch, then waits for another |
| R154 | Finishing today's reviews, a quiz or a free recall check shows a full-screen moment in beats: rays, the badge jumping and landing on the chord with confetti, the title word by word, each number counting up with a tick, then Continue. A quiz scales with its score (under 80, 80 and up, 100). Continue ignores presses in the first half second, Escape and Enter close it once it shows, and a second moment waits its turn. Calm mode and reduced motion keep it to fades with no confetti |
| R155 | Adding, saving, moving, importing, exporting and deleting say what happened in a calm toast at the bottom centre, two at most, with an icon, the words, a close button and an action where it helps. Errors are announced straight away, marked with an icon and the word Error, and stay until closed |
| R156 | Every celebration sound is synthesised, follows the Sound switch and the volume, and has the same thing said in words on screen |
| R157 | Deleting a topic, module, card, source or date takes it off screen at once with Undo for six seconds, and only deletes it for real after that, or when the app closes. Topics with questions and modules still ask first. If the real delete fails, it comes back with an error |

## Navigation and small screens (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R158 | The sidebar groups its links into Today (Overview, Calendar), Study (Topics, Cards, Recall, Projects) and Library (Bibliography, History), with a Search box at the top (still `/`), New quiz under the groups, and the backup or sync line and Settings at the bottom. Theme, colour-blind safe colours, sound and celebrations live on a Settings screen, which links to Your data |
| R159 | From 600px to 1099px wide the sidebar is an icon rail with short labels and due counts: Search, Overview, Calendar, Topics, Cards, Recall, Projects, More and Settings. More opens a panel with Bibliography, History, New quiz and Your data |
| R160 | Under 600px wide the navigation is a dock along the bottom: Today, Calendar, More, Topics and Cards, with More in the middle. More opens a sheet with Recall, Projects, Bibliography, History, Search, New quiz, Your data, Settings and the backup or sync line. Due counts show on the tabs, and More shows a dot when something under it is due |
| R161 | No screen scrolls sideways on a phone or tablet. Side gutters shrink, two-column layouts stack, module rails become a row of chips, a picked row's details come up as a sheet with a close button, the calendar shows a dot for each kind of work on phones, card grades are a 2 by 2 grid, and dialogs fill the screen |
| R162 | On phones a list table is a stack of cards that scrolls down: the tick box on the left, the name on top, and each other value on its own line with its column's name in front. The column headers become a row of sort chips, so sorting still works. Keyboard hints are left out |
| R163 | The calendar shows a Month or a Week, remembered on this device, and phones start on the Week. A week is a list of its seven days from Monday, each with its dates and everything due or done in full, with Previous week, Next week and Today. Tapping a day opens it |

## Packaging (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R164 | The app builds as an .rpm, .deb and .AppImage for Linux and an .exe installer (and .msi) for Windows, with the full icon set, Education category and a short description. Windows builds run on GitHub Actions (Release, on a push to the release branch or by hand). On Windows the slide preview finds LibreOffice's soffice.exe on the PATH or in Program Files, and its throwaway profile uses a proper file URL |

## Updates (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R165 | The app updates only from published, signed releases, never from commits. Each push to the release branch passes the Fedora update test, gets the next version (0.1.N), builds the .rpm signed with the key in the TAURI_SIGNING_PRIVATE_KEY secret, and publishes it as a GitHub Release with latest.json. "New" means a newer version than the one installed: the same or an older one is not an update |
| R166 | With Auto update on (the default), the app checks on open and downloads a newer release in the background, then says "Version X is ready" with Install now, and that it installs on close and the computer will ask for a password. On close it installs a downloaded update, or checks first if the last check was over an hour ago, showing "Installing the update" while it does |
| R167 | Settings shows the installed version, an Auto update switch and Check for updates. A found update shows its version, its notes, the password note and an Update button. Being up to date, offline or a failed install each say so in plain words |
| R168 | Before installing, the database is backed up (a "before update" backup). A package whose signature doesn't match the public key built into the app is refused and never installed. Being offline is quiet on open and close, and only shown when checking by hand. `study-app --update-now` does the same with no window (exit 0 installed, 3 nothing newer, 4 failed) for the end-to-end test |

## Shudhana and Windows (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R169 | The app is called Shudhana everywhere it shows its name. Its internal ID stays com.arthurchenevans.studyapp so it keeps the same data folder, and its .rpm (now `shudhana`) obsoletes `study-app`, so an update from Study App replaces it rather than installing beside it. `.studyapp` files keep their extension |
| R170 | Each release also builds a signed Windows installer (.exe) and publishes it beside the .rpm, with both in latest.json, so Windows copies update too. Only Linux installs mention a password |


## Importing and exporting other files (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R171 | An Anki `.apkg` or `.colpkg`, old or new format, imports with its notes, images and maths. Each deck becomes a topic (a parent deck becomes its module), or goes into a topic already here, or is skipped. Card types that don't map cleanly (like Image Occlusion) are flagged "Check these". Sounds are left out and said so |
| R172 | Importing from Anki can keep Anki's progress (when each card is due) or start fresh. Notes imported before are matched by their Anki ID and stay as they are |
| R173 | Export writes an Anki `.apkg` that Anki opens: cards and cloze as Anki notes, images as media, maths as MathJax, and optionally quiz questions as cards and my progress |
| R174 | A `.csv` or `.tsv` imports as questions or cards. Columns are matched by their names, each can be changed, and rows with problems are listed with why and left out |
| R175 | JSON, Markdown checklists, and Moodle GIFT and Aiken files import and export. Exporting to them never includes history |
| R176 | Every import shows what it will add before anything changes, and backs up first |
| R177 | After an import a toast offers Undo, and Your data shows the last import with Undo. Undo puts everything back to the backup made just before. The last import is kept beside the backups, never in the data, so it never travels with an export or sync |
| R178 | A file can be imported from the Import button, by dropping it anywhere on the window (except a card or question field, which takes images), or by opening a `.studyapp` with the app. A big import shows its progress and can be stopped, which adds nothing. A file it can't read says why and offers another |

## Finding your place in a source (approved 13 Nov 2026)

| ID | Requirement |
|---|---|
| R179 | The source preview has a table of contents: a PDF's bookmarks, an EPUB's own contents (or its chapter titles), each slide's title, or a Markdown file's headings. The entry for where you're reading is marked, and picking one goes there |
| R180 | A box goes straight to a page, slide or line, and Ctrl+G jumps to it. PDF pages are drawn only as they come near the screen, so going to page 300 doesn't wait for the pages before it |
| R181 | A citation with a page, slide or line has a button that shows that spot in the preview |
| R182 | The first time, it asks which open file is that source and remembers the file's name on this computer. The file itself is never stored. When that file isn't open, it says which file to open, and opening it shows the spot |

## Colour themes (approved 14 Nov 2026)

| ID | Requirement |
|---|---|
| R183 | Nine colour themes, built in OKLCH: Mono, Coffee, Honey, Brick, Rose, Lavender, Ocean, Miku and Sage, each in light and dark. Coffee is the default. In every theme, mode and palette, text passes 4.5:1 (and APCA Lc 60 for body text, Lc 45 for muted and bold labels), fills pass 3:1, and colour-blind safe right and wrong stay apart under protanopia, deuteranopia and tritanopia |
| R184 | Settings picks the colour theme. Each option previews itself in the current mode and palette, picking one applies it straight away, and the choice is remembered on this computer |

## Editing and deleting everything you make (approved 14 Nov 2026)

| ID | Requirement |
|---|---|
| R185 | Everything you make can be renamed or edited and deleted where it lives. Modules (from a labelled Module menu on Topics) and topics can be renamed, and a recall checklist on one takes the new name. Quiz questions can be deleted from the editor, and past quiz results keep their own copy. Recall checklists can be deleted from the Recall list. Projects can be renamed (name and kind) and deleted. Struggles can be edited and deleted. Issues can be renamed (not to a name another issue has) and deleted, with their drills. Drills can be edited (task and hints) and deleted. Every delete asks first and says what goes with it |
| R186 | Every screen comes in with the same entrance, once, when you go to it. Nothing inside a screen adds a second entrance of its own |

## Every platform (approved 14 Nov 2026)

| ID | Requirement |
|---|---|
| R187 | Each release publishes an .rpm, a .deb, a Windows .exe and a macOS .dmg (one universal app for Apple silicon and Intel), and latest.json lists each, so installed copies update themselves whichever they came from. The Mac app isn't notarised, so the first open goes through Open Anyway |
| R188 | Importing from Anki can take only the notes with chosen tags (a tag brings the tags under it, and case doesn't matter, as in Anki), for decks kept as one deck filtered by tags. Suspended cards are left out by default or kept paused (archived). A note with only some cards suspended comes in with those cards paused |
| R189 | The navigation uses plain names: Quizzes (each topic's questions make a quiz), Free Recall and Practice Log. A Manage group holds Topics & Modules, Import & Export, Bibliography and History. Each sidebar group folds from its heading and stays folded, Manage starts folded, a folded group still shows its due count, and a group holding the page you're on always shows. The phone dock is Overview, Quizzes, More, Cards, Free Recall. The sidebar's button is Start a quiz. Quizzes makes a new quiz (its topic) straight into the question editor, and deleting a quiz takes only its questions. Topics & Modules makes, renames, moves, tags and deletes topics and modules, and says what goes with a topic before deleting it. Import & Export is its own page |
| R190 | Making a practice log always shows the kind box, with a placeholder. Quick picks fill it, and :: nests a kind (Coding::Rust), so the kind filter takes everything under the top of it |
| R191 | Cards opens on Review: what's due, the forecast and stats, with no card fronts or answers. Browse all lists every card in every topic, with a search box (words, -word, topic:, module:, tag:, is:due/new/learning/paused, type:) and a filter panel (topics by module, status, card type, tags) that folds away from the Filters button and stays how you left it. A card due today shows "Hidden until you review it" in place of its answer, and hidden answers are never searched. Ticked cards can be moved to a topic, paused or unpaused, and deleted with Undo |
| R192 | A card (its note) has its own tags, typed in the card editor with commas between and :: for a hierarchy. A new card keeps the last card's tags. Anki import brings each note's tags across as its own (they never make modules or topics), Anki export sends them back, and Browse all filters and searches a card's own tags along with its topic's |

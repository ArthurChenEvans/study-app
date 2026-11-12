# Study App Designs

Clickable HTML prototype of every MVP screen, built with the `studyapp` DaisyUI theme from `docs/design-rules.md`. Open `index.html` in a browser. The dark bar in the bottom right switches each screen between its states.

## Screen inventory

| Screen | States | Requirements | Feature files |
|---|---|---|---|
| Overview | normal, new user | R33 | overview |
| Topics | normal, empty, delete confirmation | R1, R2, R3, R30 | topics |
| Question editor + source preview | normal, error (no correct option), no file open | R4 to R12, R14 | questions, sources |
| Bibliography | normal, empty, add source | R13 | sources |
| Quiz setup | normal, error (no topic picked) | R15 to R19 | quiz-setup |
| Quiz | study (with feedback), exam, timer low, incomplete multi-answer | R18 to R23, R29, R32 | taking-quiz |
| Results | normal, perfect (nothing to retry) | R24 to R26 | results, retry |
| History | normal, empty | R26, R27, R29 | history, retry |
| Search | normal, no results | R31 | search |

Open questions from `specs/README.md` are shown in place as dashed blue "Open Q" badges instead of being designed: Q1 (unreadable files), Q2 to Q4 (quiz setup limits), Q5 and Q6 (study mode), Q7 (guessed count), Q8 and Q13 (history), Q10 to Q12 (sources), Q15 (single/multiple switch), Q16 (search tabs).

## Review checklist results

Checked with screenshots of every screen and state, including a greyscale plus blur squint pass.

**Fixed during review**
- Topic table actions wrapped onto two lines and columns didn't line up between modules. Now a fixed column layout.
- Review list dividers rendered near-black. Now base-300.
- Selected option in segmented controls (Study/Exam etc.) was nearly invisible in greyscale. Now uses the neutral solid style.
- Shortcut hints inside dark buttons were unreadable. Now outlined in the button's text colour.
- Diagram text fell back to a serif font when IBM Plex wasn't loaded. Now falls back to sans-serif.

**Passing**
- Squint and greyscale: score, current question and primary action stand out on every screen. Correct, wrong and guessed all have an icon and a word as well as colour.
- Contrast: all theme text pairs pass AA (see design rules). Field borders use the 3.9:1 field-border shade.
- One primary button per screen and dialog.
- Quiz works by keyboard alone: 1 to 5, Enter, F.
- Nielsen's 10: status (timer, progress, file slide count), real-world words, cancel on every dialog, error prevention (disabled add/remove at 3 and 5 options, delete confirmation, disabled submit with nothing picked), recognition (topic list with counts in setup), shortcuts, plain error messages with the fix.

**Known gaps**
- Fonts, Lucide, KaTeX and Shiki load from CDNs. The real app should bundle them for offline use.
- The source preview is a mock slide, the real one is PDF.js.
- Search filters and sorting are shown but not interactive.

## Component checkup (round 2)

Every control was inventoried from the DOM and screenshotted at 2x in its states (default, hover, focus, selected, disabled, error). Severity uses Nielsen's 0 to 4 scale.

| Control | Used in | Finding | Severity | Fix |
|---|---|---|---|---|
| Segmented control | Quiz setup (7 rows), editor answer type | Ghost and solid buttons side by side didn't read as one control, and looked like two separate buttons | 3 | New `.seg` component: one bordered unit, selected item solid neutral, others muted text |
| Text inputs, selects, textareas | Editor, setup, bibliography, results, search | Focus drew daisyUI's offset outline plus the border, giving a heavy double ring | 3 | One ring: border turns primary with a 1px matching shadow, no outline |
| Search field | Search | Inner input could draw its own focus outline inside the field | 2 | Inner outline removed, the field's ring is the only one |
| Toggle | Quiz setup timers | On state only darkened the thumb, so on and off were hard to tell apart, especially in greyscale | 2 | On state fills the track with primary and a light thumb |
| Filter chips + Add filter | Search | Chips and the Add filter button were different heights, and the filter name and value had the same weight | 2 | Chips and button both 32px, filter name muted |
| Tabs with counts | Search | The result count sat off the label's baseline | 1 | Count styled as 12px muted text on the same baseline |
| Shortcut hints in disabled buttons | Quiz | Hint stayed full strength while the label faded | 1 | Hint fades with the label |
| Quit button | Quiz bar | Bold, label-only button heavier than the rest of the bar | 1 | Regular weight with an icon |

Checked after the fixes: segmented controls and inputs share the same 32px height and baseline, contrast for the new seg text is 5.5:1 (muted on base-200) and 10.7:1 (selected), greyscale still shows the selected item clearly, and every screen still has exactly one primary button.

## Round 3: data visualisation and experience

Diagnosis: the previous version was clean but forgettable (every screen a heading and a table, no focal point, nothing showing whether you're improving).

- **New Overview screen** (now the home screen): accuracy for the last 7 days with the change from the 7 days before, questions answered, share of right answers that were guesses, score-per-quiz line chart, topics ranked weakest first, and wrong-answer reasons ranked. Everything is derived from saved attempts (R24, R25, R27), so no new behaviour. Includes a first-run state.
- **Results**: the score is the hero, with the change from the last quiz on the same topics, a tall right / guessed / wrong bar, a question-by-question grid and per-topic bars.
- **History**: score-per-quiz chart above the table, and a result bar on every row.
- **Topics**: each topic shows its latest quiz as a bar and %, plus a Quiz shortcut that opens setup.
- **Quiz setup**: settings grouped in panels, with a live summary on the right (question count, split per topic, mode, timer, last score on these topics) holding the Start button.
- **Quiz**: progress is a 20-cell track that fills with right / guessed / wrong in study mode (neutral in exam mode, so nothing is given away). Feedback appears in its own panel.
- **Visual system**: raised panel surface, app mark in the sidebar, press feedback on buttons, short rise-in motion (off with reduced motion).

All chart rules are in `docs/design-rules.md` section 9.

## Round 4: accessible colour

- Quiz outcomes moved off red and green, which collapse into one colour for protanopia (validator ΔE 1.0). Right is now blue (#025884), wrong is orange (#cc6c13), and guessed is a hatched light blue, meaning "right, but unsure". Validator: CVD separation 22.1, normal-vision 31.5, all at least 3:1 on the surface.
- In greyscale the three read as dark solid, light hatched and mid solid, so they separate without colour. Checked with screenshots in colour, greyscale and a protanopia simulation.
- Every outcome also has an icon and a word (✓ Correct, ✕ Your pick, ? guessed), a legend and a table view.
- Text colours raised to WCAG AAA (7:1) on the page: secondary text, success, error, info and the new right/wrong text inks.
- Red and green are now only used for system messages (validation errors, delete, score going up or down), always with an icon and words.

## Round 5: dark mode

- Dark is the default, with a Dark / Light switch in the sidebar. The choice is remembered (R34).
- Every hard-coded colour in the prototype was moved to a theme token, so both themes change together: field borders, the pipeline diagram, label colours inside chart fills, shadows, code highlighting.
- Dark palette and its contrast checks are in `docs/design-rules.md` section 11. All text is AAA, and the chart pair passes the validator against the dark surface.

---

# Free recall (proposed, waiting for a greenlight)

Clickable prototype: `free-recall.html`, same theme and components as the MVP prototype. The dark bar in the bottom right switches screens and states.

## What was decided (3 Nov 2026)

- A checklist is about a module or a topic, chosen per checklist.
- Points are written by hand, and can be added from questions. MCQ now, short answer and flashcards once they exist. Unticking an added question removes its point.
- The dump can be typed in the app or done anywhere else. Only ticking the checklist is required.
- Schedule is the calendar ladder by default: Day, Week, Fortnight, Month, Monthly ×6, Seasonal ×4, then Yearly, pass mark 70%. Answered after review: under the pass mark it goes back **one step**, and the next date counts from the day the check was **actually done**. Steps, intervals, repeats and the pass mark are all editable.

## Screen inventory

| Screen | States | Proposed requirement |
|---|---|---|
| Recall list | two due, nothing due, first time, schedule settings | R52, R57, R58 |
| Checklist | points, add from questions, new (no points yet), history | R52 to R54, R59 |
| Recall session | dump typed, dump outside the app, check, result up, result back | R55 to R57 |

## Proposed requirements

| ID | Requirement |
|---|---|
| R52 | A checklist is about one module or one topic, picked when it's made |
| R53 | A checklist has points, written by hand. Each point is Markdown with `$...$` maths, and can be edited, reordered and removed |
| R54 | Points can be added from questions in the checklist's topics (MCQ now, short answer and flashcards when built) as "question → answer". Unticking an added question removes its point. The question is never changed |
| R55 | A session starts with the dump: typed in the app (saved with the check) or done outside it. The checklist stays hidden until the dump is finished |
| R56 | The check shows the dump beside the checklist. Each point is ticked recalled or not. The score, the pass mark and what will happen next update as you tick |
| R57 | Finishing a check moves the checklist up a step at or over the pass mark, or back under it, and sets the next date from the ladder anchored to the start date. The result lists missed points, and how often each has been missed |
| R58 | The recall list shows what's due today first, then everything else by next date. The sidebar shows how many are due |
| R59 | Each checklist has a history: score per check against the pass mark, each check's step change and dump, and the points missed most often |
| R60 | The schedule (steps, intervals, repeats, pass mark, what happens under it) is editable and can be reset. Changes apply from each checklist's next check |
| R61 | Keyboard: Ctrl+Enter finishes the dump and the check, Space ticks the focused point |

## Open questions (answered 3 Nov: one step, from the day it was done)

- **A.** Under the pass mark, go back **one step** (Monthly 3 → Monthly 2) or back **a whole band** (Seasonal → start of Monthly)? Your notes say "drops it back a tier".
- **B.** When a check is done late, does the next date count from the **planned** date (keeps checklists staggered) or the day it was **actually done**?

## Small calls made in the design, to confirm or change

- Ticks are yes or no, no "partly". Keeps the 70% meaning simple.
- The dump has a quiet timer that counts up and never stops you.
- A new checklist starts at Day the first time you do it, not the day it's made.
- During a check you can add a point the list was missing.
- A session hides the sidebar, like a quiz.
- Missed points are listed on the result. Later, these are what would turn into flashcards.

## UI/UX test results

**During the build** (screenshots of all 13 states in both themes, against the design rules checklist). Found and fixed:

| Finding | Severity | Fix |
|---|---|---|
| Step names under the schedule track truncated ("For…", "Mo…") | 3 | Track draws bars only, with one caption: current step and what's next |
| Already-added questions locked in the import dialog, so imports couldn't be removed | 3 | Ticked and editable, unticking marks the point for removal |
| Sidebar stayed visible during a session | 2 | Hidden, like the quiz |
| "Finish check" scrolled out of view on a long checklist | 2 | Score panel is sticky and holds the button |
| Every point was a bordered input, reading as a form | 2 | Points read as text, click to edit |
| "Written" badge on every point carried no information | 2 | Only added points get a badge ("From MCQ") |
| Score chart clipped "100%" and "20 Oct", pass label collided with the line | 2 | Wider margins, pass mark moved to a legend |
| "Recall about" label beside its field, error shown before trying | 2 | Label above, hint instead of an error |
| No way to add or remove schedule steps | 2 | Remove per step, "Add a step before Yearly" |
| "Missed" repeated on every missed row under a "Missed" heading | 1 | Icon only |
| "1 day late" as a heavy black chip | 1 | Muted text with a clock icon |
| "$LaTeX$" in help text rendered as maths | 1 | Shown as code |
| Due badge still said 2 when nothing was due, table not sorted by date | 1 | Badge follows the state, table sorted by next date |

**After the build** (measured, every state, both themes):
- Text contrast: every text node passes WCAG 2.2 AA once animations finish (4.5:1, or 3:1 for large text). Measured by compositing each text colour over its real background.
- Targets: every control is at least 24px, or sits inside a larger clickable row or label.
- One primary button per view and dialog. "Nothing due" has none on purpose.
- Keyboard: 20 tab stops on the check screen, all with a visible focus ring. Space ticks a point and keeps focus, Ctrl+Enter finishes.
- Squint and greyscale: the main thing on each screen (Start recall, the live score, the result) still stands out. Recalled vs not reads without colour (filled box and a word).
- No console errors, no horizontal scroll at 1440 px.

**Nielsen's 10:** status (live score and what happens next, timer, step track, due count), real-world words (checklist, point, recalled, pass mark), control (Leave on every session step, Cancel on dialogs, Reset schedule), consistency (same sidebar, panels, segmented controls and shortcuts as quizzes), error prevention (checklist hidden during the dump, can't start with no points, import button says exactly what will change), recognition (next step and date shown before you commit), efficiency (Ctrl+Enter, Space), minimalism (one primary per view), recovery (hint says what's needed), help (one-line hints under each control).

## Built (3 Nov 2026)

Free recall is built from these designs, test-first, with requirements R52 to R61 in `specs/README.md` and scenarios in `specs/features/free-recall.feature`.

Differences from the prototype, both for consistency with the rest of the app:
- "New checklist" opens a small dialog to pick the module or topic (like "New topic"), then goes straight to the editor.
- Leaving a session with something typed or ticked asks first, since nothing is saved until the check is finished.

Flashcards were added to "Add from questions" once they were built (now "Add from questions or cards"). The dialog's Flashcards tab lists notes from the checklist's topics, skipping ones with every card archived. A basic or reversed card becomes "front → back", a cloze card its text with the answers in bold, and the point is badged "From card". The audit of that dialog found its tab counts at 4.37:1 in the light theme (faded with opacity), now full strength.

Checked in the real app (`e2e/recall_test.py`, run by `e2e/run.sh`): makes a checklist, writes points with maths, adds one from an MCQ, types a dump with the checklist hidden, ticks under the pass mark with the live outcome, and checks the result, the recall list and the history. With `AUDIT` set it also measures every screen in both themes for text contrast, target size and primary buttons. That pass found the score chart's table header at 4.06:1 in the light theme (the same on the Overview chart). Fixed to the muted text colour, and now every screen passes.

---

# Flashcards with FSRS (built)

Clickable prototype: `flashcards.html`. Same theme and components as the other prototypes. The dark bar in the bottom right switches screens and states. Scheduling would use the [fsrs crate](https://docs.rs/crate/fsrs/6.6.2), which gives the interval for each answer button (`next_states`), a desired-retention setting, and fitting the parameters to your own review history (`compute_parameters`).

## Screen inventory

| Screen | States |
|---|---|
| Cards | due today, all done, first time, settings |
| Topic cards | card list, new card, edit a leech card |
| Review | front, answer, leech warning, done for today |
| Suggestions | list, make a card |

## How it works

- **Cards live in topics,** like questions, so a module or topic can be reviewed on its own or everything at once.
- **Review is Anki's loop:** front, Space to show the answer, then Again, Hard, Good or Easy (keys 1 to 4, Space is Good), each showing when the card comes back. Again shows it again later in the same session. E edits the card, and there's Undo.
- **Counts** at the top read "9 new · 2 learning · 6 to review", with the current card's queue underlined.
- **Cues, not names.** The editor's front hint says to write the thought you'll actually have, and "More cues for the same answer" makes extra cards that share one back, each scheduled on its own.
- **Leeches:** after 8 Agains (editable) the card is flagged, the review offers "Rewrite now", and the editor asks why the answer wasn't obvious. Saving clears the flag and keeps the schedule. Cards can be archived.
- **Suggestions** collect points missed in recalls and questions got wrong or guessed in quizzes. A recall miss leaves the front for you to write (the cue that should have brought it up). A quiz mistake fills in question and answer.
- **Settings:** how much to remember (desired retention, default 90%), new cards a day (10), most reviews a day (200), the leech threshold, and "Fit the scheduler to you".

## Answered (3 Nov 2026)

- **A. All three card types.** Basic, Basic and reversed (two cards, one each way, scheduled separately), and Cloze (`{{c1::answer}}` or `{{c1::answer::hint}}`, one card per gap number, made with Ctrl+Shift+C or the "Make a gap" button, with a preview of every card it will make). "More cues for the same answer" stays on Basic only.
- **B. Suggestions wait for acceptance.** Nothing becomes a card until you make it.

Recheck after adding cloze and reversed: the revealed cloze answer first used the "guessed" blue, which already means something else, so it's now an underlined neutral highlight. "Save card" on a reversed card now says "Save 2 cards". The measured audit and keyboard checks pass again on every state in both themes.

## Calls made in the design, to confirm or change

- Cards are per topic, not a separate deck system.
- Keys 1 to 4 and Space follow Anki, so muscle memory carries over.
- Again re-shows the card within the session. The fsrs crate doesn't document same-day learning steps, so this is the app's own rule.
- 10 new cards a day, 200 reviews a day, 90% retention, leech at 8 Agains.
- Editing a card keeps its schedule.

## UI/UX test results

**During the build** (screenshots of all 15 states in both themes against the design rules):

| Finding | Severity | Fix |
|---|---|---|
| Leech editor showed raw LaTeX with no way to see the card | 2 | Preview panel with the front and rendered back |
| Review counts didn't show which queue the current card is from | 2 | Current queue's count underlined, and said in the label |
| "Saving resets how often it's come up" was unclear about the schedule | 2 | "Saving a change clears this flag. The card keeps its schedule." |
| Front had a fixed height, leaving a big gap above the answer | 1 | Removed, the answer sits right under the front |
| Suggestions count was a heavy black chip | 1 | Muted number |
| 14-day forecast didn't label its busiest day | 1 | "Mon 9 Nov · 31" under the peak |
| "Remembered 88%" on the done screen had nothing to compare to | 1 | "Aiming for 90%" |

**After the build** (measured, every state, both themes):

| Finding | Fix |
|---|---|
| Card editor had two primary buttons (Review CPU and Save card) | Review becomes outline while editing |
| Archived row faded with opacity, its grey text 3.77:1 in light | No opacity, the front goes muted instead |
| One-line card fronts 21 px tall, "Change topic" and the retention slider 20 px | Padded to 24 px or more |

Then: every text node passes WCAG 2.2 AA, every target is at least 24 px, one primary per view (none on "all done" and the suggestions list, on purpose). Keyboard: Space shows the answer, 3 grades Good, E opens the rewrite from a leech, and all 10 tab stops on the answer view show a focus ring. The squint test keeps the count and Start review, the Good button, "Done for today" and Save card on top. No console errors, no horizontal scroll at 1440 px.

## UI/UX recheck (3 Nov 2026): bright bars

Reported: "a long white bar in random places on each page".

- **Cause 1, the prototypes' own switcher.** The screen and state picker floated over the page, fixed to the bottom right. In the dark theme it's near-white, so wherever you scrolled it sat across tables, buttons and hints. It's now a slim strip above the app in all three prototypes (`index.html`, `free-recall.html`, `flashcards.html`), so it never covers content. The sidebar and sticky headers sit under it.
- **Cause 2, full-page screenshots.** The sidebar is one screen tall, so long pages showed a band of page colour under it in the captures. Screenshots now render at the page's full height.
- **Cause 3, the retention slider.** DaisyUI's `range` fill is as tall as its thumb, a thick near-white slab in dark. Replaced with a hand-built slider (6px track, 16px thumb, 24px hit area) and added to the design rules.
- **Cause 4, the quiz track's current question (in the real app too).** It was a solid near-white segment, the loudest thing on the quiz screen. Now an outlined segment, so it still reads as "you are here" without shouting, and stays distinct from answered (caramel) and upcoming in greyscale.

Checked after: an automated scan of 55 screens (every dark prototype state plus the real app's recall flow) for light horizontal bands 300 px or wider found none. The contrast, target-size and keyboard audits still pass on both prototypes and in the real app.

## Built (3 Nov 2026)

Flashcards are built from these designs, test-first, with requirements R62 to R77 in `specs/README.md` and scenarios in `specs/features/flashcards.feature`. Scheduling uses the fsrs crate 6.6.2.

Differences from the prototype:
- "New card" on the Cards home opens the editor with a Topic picker (it starts on the topic with the most cards). Saving into another topic opens that topic.
- The review's "Again" label says "Later today" instead of "<10 min", since Again brings the card back three cards later in the same session, not after a set time.
- Z undoes the last grade, as well as the Undo button.
- The done screen says "Under a minute" for very short sessions.
- The sidebar stays hidden on the done screen, and "Back to Cards" returns to it.

Checked in the real app (`e2e/flashcards_test.py`, run by `e2e/run.sh`): writes a basic card with an extra cue into a picked topic, a reversed card and a cloze card with two gaps (one from Ctrl+Shift+C, one from the button), checks the preview and the topic list, reviews all 6 with the keyboard (one Again that comes back, one Undo), checks the summary, and turns a recall miss into a card from Suggestions. With `AUDIT` set it measures every screen in both themes. That run found one real bug, fixed: after accepting the last suggestion, the "Card saved" message disappeared with the list. Every screen passes contrast, target size and one-primary checks.


---

# Short answer (built)

Clickable prototype: `short-answer.html`. Same theme and components as the other prototypes, built into the existing editor, quiz, results and setup screens rather than new ones.

## Answered (3 Nov 2026)

- **Marking:** the app checks, and you can overrule it.
- **Where:** in the same topics and quizzes as MCQs.
- **Feeds:** wrong or guessed short answers become card suggestions, and short answer questions can be added to recall checklists.

## Screen inventory

| Screen | States |
|---|---|
| Editor | accepted answers, keywords, try an answer, needs an answer, multiple choice |
| Quiz | typing, right, right with a typo, wrong, keywords missed, overruled, exam |
| Results | review, overruled |
| Quiz setup | both types, short answer only |
| Checklist import | short answer tab |
| Suggestions | make a card from a short answer |

## How it works

- **Question type** is a switch at the top of the editor: Multiple choice or Short answer. Switching keeps the question, explanation, sources and tags.
- **Two ways to check:**
  - *Accepted answers:* right when what you type matches any of them. Case, extra spaces, punctuation and a leading "the" or "a" don't matter. "Allow small typos" (on by default) accepts one wrong letter in words of 4 to 7 letters and two from 8, never in numbers or answers of 3 letters or fewer.
  - *Keywords:* right when your answer contains all of them, or at least N. Each keyword can have other spellings after a comma. Each word you write counts once, so "SYN-ACK" doesn't also count as "SYN" and "ACK".
- **Model answer** (optional, Markdown) is what you see after answering. Left blank, the first accepted answer is shown.
- **Try an answer** in the editor shows how the app would mark something before saving. Nothing is recorded.
- **In a quiz** you type the answer: Enter submits, Shift+Enter adds a line, Ctrl+G flags it as guessed (F can't work while you're typing). In study mode the feedback says Correct, Correct with a typo (and the word), or Wrong, with the keywords you hit and missed and the model answer.
- **Overruling:** "Count it as right" or "Count it as wrong" (O) in study feedback, and on every short answer in Results, including exam mode. An overruled answer is labelled "You overruled this" and can be undone. The score, history, Overview, retry and suggestions all use your decision.
- **Quiz setup** gets Question types: Both, MCQ or Short answer. Topic rows show "5 MCQ · 2 short answer". Difficulty and option order say they only change multiple choice.
- **Checklist import:** the Short answer tab becomes "question → model answer" (or the first accepted answer).
- **Suggestions:** a wrong or guessed short answer shows what you wrote, and making a card starts with the question as the front and the model answer as the back.

## Open questions (answered 3 Nov 2026: any order, right or wrong only, the typo rule as written)

1. **Keyword order.** The TCP handshake question says "in order", but keywords are checked in any order. Should there be an "in this order" option per question, or is any order fine?
2. **Half marks.** "2 of 3 keywords" counts as wrong (or right, if you set "at least 2"), so every answer is right or wrong and the tallies stay simple. Is that OK, or do you want half marks?
3. **The typo rule** above: right amount of forgiveness, or stricter or looser?

## Calls made in the design, to confirm or change

- Overruling is allowed any time from Results, not just straight after answering.
- Ctrl+G for "I guessed" on short answers. MCQs keep F.
- A short answer question needs at least one accepted answer or keyword to save. It can't be "mark it yourself only".

## UI/UX test results

- Every screen and state screenshotted in both themes at 1440 × 900: no console errors, no blocked requests, no horizontal scroll.
- Measured audit (contrast, 24px targets, one primary per view) passes everywhere. The audit now measures a checkbox through the label that wraps it, since the whole row is what you click. The MCQ "is correct" checkboxes are 20px on their own, as in the real editor today, and pass WCAG 2.5.8's spacing exception (12px clear around them).
- The first pass found the quiz's current-question marker drawn as a solid bright bar, which the app already replaced with an outline after the bright-bar recheck. Fixed to match. It also found the keyword rule needed spelling out ("SYN-ACK" vs "SYN" and "ACK").
- Squint test (greyscale, blurred) on the keyword editor, a wrong answer and the results review: the question, your answer, then the verdict read in that order, and the one primary (Next or Save) stands out.
- Verdicts never rely on colour alone: every one has an icon and a word, and keyword chips have a tick or cross plus solid vs dashed borders.

## Built (3 Nov 2026)

Short answer is built from these designs, test-first, with requirements R78 to R86 in `specs/README.md` and scenarios in `specs/features/short-answer.feature`. Keywords are checked in any order, every answer is right or wrong (no half marks), and the typo rule is as written above.

Difference from the prototype: when a question has its own model answer, "Also accepted" lists every accepted answer, since none of them is the one already shown.

Checked in the real app (`e2e/short_answer_test.py`, run by `e2e/run.sh`): writes a question checked by accepted answers and one by keywords, uses Try an answer on both (a typo allowed, and "SYN-ACK" not also counting as "SYN" and "ACK"), picks short answers only in setup, answers wrongly and overrules with O, gets the keyword question right, then undoes the overrule on the results page and sees the score drop. It also checks the wrong answer becomes a card suggestion and both questions can be checklist points. With `AUDIT` set it measured the editor and quiz setup for the first time, which found two older problems, both fixed: the source preview's file input was a 1px "screen reader only" element, so it was an invisible extra Tab stop announced as a second "Open file" (now fully hidden, like the image picker), and the quiz setup's timer toggles were 20px tall (now the 24px size). Every screen passes contrast, target size and one-primary checks in both themes.


---

# Projects, drills and the calendar (built)

Clickable prototype: `projects-drills-calendar.html`. Same theme and components as the other prototypes.

## Answered (3 Nov 2026)

- **Projects are not scheduled.** A project is anything applied (coding, Blender, past papers). It's where you log struggles while you work: what tripped you up, why it wasn't obvious, and the day.
- **Struggles are reviewed at the end of each work session**, and can become a card suggestion, a recall checklist point or a drill, only when you choose.
- **Issues are shared across projects.** When a struggle is given an issue that already has a drill, the app offers: schedule that drill again, or do a different drill.
- **Drills are one-off and reactive.** A drill is scheduled once. Rated Solid, it's ticked off. Rated Struggled or Shaky, it comes back the next day until it's Solid. It only returns after that if its issue turns up again in a session review.
- **Scheduling a drill** suggests tomorrow and asks you to confirm or pick another day.
- **Quizzes are scheduled per topic** on a ladder like recall's.
- **A unified calendar:** a month view of what's due on each day, and a day view listing everything.

## Screen inventory

| Screen | States |
|---|---|
| Calendar | month, today, a past day, a future day, move a drill |
| Projects | list, first time, issues |
| A project | struggles, session running, log a struggle |
| Session review | recall miss, apply miss with a known drill, new drill, confirm the date, done |
| Doing a drill | doing it, rated shaky, rated solid |
| Quiz scheduling | topics with quiz due dates, results moving topics up or down, schedule settings |

## How it works

- **Sidebar:** Calendar sits under Overview with today's total. Projects sits after Recall with the drills due today. Topics shows the quizzes due.
- **Month view:** weeks start on Monday. Each day lists what's due with an icon, a count and a word ("12 cards", "1 checklist", "2 quizzes", "1 drill"), so nothing relies on the icon alone. Past days show what you did, plus struggles logged. Today is outlined. Future card counts are FSRS's forecast and move as you review.
- **Day view:** a section per type with a start button for each item, and one primary button ("Start with the drills"). Past days show what you did, with links to quiz results and the struggles logged that day. Drills can be moved to another day from here.
- **A project:** "Start a session" starts a timer. While it runs, "Log a struggle" (Ctrl+L) takes what tripped you up, why it wasn't obvious, whether you couldn't recall it, couldn't apply it or aren't sure yet, and an optional topic. Struggles are grouped by day, searchable, and show what became of them.
- **Session review** ("End session and review") steps through that session's struggles:
  - *Couldn't recall it* suggests a card by default. The front starts as what tripped you up, with your "why" beside it to help write the cue. It can also be a checklist point, a drill, or nothing.
  - *Couldn't apply it* asks for an issue (pick one you've had before, or name a new one). If the issue has a drill, you're offered "Schedule this drill again" or "Do a different drill". Otherwise you write a new drill: a small, specific task, a topic, and optional hints.
  - Scheduling a drill asks when, with tomorrow picked. "Finish later" leaves the rest marked to review.
- **Doing a drill:** the task, a timer, hints you can open, the struggles that led to it, and an optional note. Then Struggled, Shaky or Solid (keys 1 to 3). Solid is ticked off, the other two come back tomorrow.
- **Issues tab:** every issue, how often and where it came up, and the state of its drill.
- **Quiz scheduling:** Topics gets "Quiz step" and "Next quiz" columns and "Quiz the 2 due topics". Each topic in a quiz is scored on its own questions. At or above the pass mark (70%) it moves up, under it goes back one step, and the next date counts from the quiz day, the same rules as recall. Results show where each topic moved. The ladder and pass mark are editable in "Quiz schedule", separate from recall's.

## Open question

1. **Exam dates.** Want to add your own dates (exams, deadlines) to the calendar? It could also squeeze a topic's schedule as an exam gets close, which fits "more frequent reviews when there's an end date".

## Calls made in the design, to confirm or change

- Overdue things carry over to today, rather than staying on the day they were due.
- Struggles are logged inside a running session, since the review happens when a session ends.
- An issue can have more than one drill over time. The most recent one is offered again.
- A topic's quiz schedule starts the first time you quiz it, and retries don't move it.

## UI/UX test results

- Every screen and state screenshotted in both themes at 1440 × 900: no console errors, no blocked requests, no horizontal scroll.
- Measured audit (contrast, 24px targets, at most one primary per view) passes everywhere. Read-only views like the month grid have no primary, on purpose.
- The first pass found the month grid's small icons for cards and drills too alike at 14px, so every count now has a word. Today's quizzes each got their own Quiz button, as well as "Quiz both together". "Log it" became the primary while a session runs, since logging is the main thing you do then.
- Squint test (greyscale, blurred) on the month, today, a session review and a drill: today's outline and the one primary stand out, and everything else reads as calm detail. No bright wide bars: current positions are outlined, like the quiz track and ladder.

## Built (3 Nov 2026)

Built from these designs, test-first, with requirements R87 to R100 in `specs/README.md` and scenarios in `specs/features/projects.feature` and `specs/features/calendar.feature`. Exam dates (the open question) aren't in this build.

Differences from the prototype:
- The session review has no Back button. Each struggle is saved as soon as you move on, so "Finish later" leaves the rest to review instead.
- The day view's primary button starts whatever comes first: drills, then cards, then recall, then quizzes.
- Leaving a drill session returns to the calendar day you started from.
- Recall's schedule editor is now shared with the quiz schedule (`ScheduleDialog`), so both work the same way.

Checked in the real app (`e2e/projects_calendar_test.py`, run by `e2e/run.sh`): makes a project, starts a session, logs two struggles (Ctrl+Enter, the button, and Ctrl+L to jump to the log), reviews them into a card suggestion and a new drill for tomorrow, checks the project, the Issues tab and Suggestions, does the drill from today's calendar view (Shaky brings it back tomorrow), and checks a finished quiz starts its topic's quiz schedule. With `AUDIT` set every new screen passes contrast, target size and one-primary checks in both themes.

Found while building, all fixed: dates mid-sentence kept their capital ("started Today", "Last done Yesterday"), drill dates were lowercased whole ("fri 6 nov"), and the Cards first-time button said "1 suggestions".


# Dates on the calendar (exam-dates.html)

Waiting for a greenlight. Open `design/exam-dates.html`, the strip at the top switches screens and states.

| Screen | States |
|---|---|
| Calendar | month with dates, the exam's day, the week before, add a date (covers a module), covers topics, a plain date, edit a date |
| Topics and results | next quiz tightened, results with a tightened move |

## How it works

- **Any named date:** a name, a day and an optional kind (Exam, Deadline, Coursework, Mock exam are suggested, anything else is fine). "Add a date" is the Calendar's one primary button.
- **Covers:** nothing, a module (every topic in it, including ones added later) or picked topics.
- **"Tighten reviews before this":** a tick on each date, on by default when the kind is Exam, and only usable when the date covers something. Until the day, no topic quiz, card or drill for those topics waits longer than half the days left, and nothing lands after the date. Recall checklists aren't squeezed.
- **Afterwards:** from the day after, everything goes back to its normal schedule, carrying on from where it is.
- **Month view:** dates show as an outlined chip at the top of their day (a cap for exams, a flag for the rest), and a "Coming up" line above the grid gives the next dates with a countdown.
- **Day view:** a date gets its own panel with the countdown, what it covers, each topic's next quiz and where it was pulled in from, and how many cards were brought forward. Topics never quizzed are listed with a Quiz button, since they have no schedule to tighten.
- **Topics and results:** a tightened next quiz says "Tightened for Networks exam" under the date, and results show what the date would have been.
- **Adding a date** previews what moves ("TCP/IP quiz moves from Sat 28 Nov to Tue 10 Nov") before you save.

## Calls made in the design, to confirm or change

- **The squeeze is worked out, not stored.** Each item's due date becomes the earlier of its normal date and "last review + half the days that were left then". So editing or deleting a date puts everything back, and items reviewed long ago can come up as due today straight away.
- **The day before the date** there's nothing left to halve, so a review then is scheduled normally (after the date).
- **Two dates covering the same topic:** the tighter one wins.
- **Cards:** only the due date is pulled in. FSRS sees an early review and handles it as it normally does, its memory model isn't changed.
- **Drills** count when their topic is covered. Not-Solid drills are back tomorrow anyway, so this only matters for a drill moved far out.
- **Never-quizzed topics** aren't made due automatically, they're flagged on the date's panel instead.

# Lists at scale (lists-at-scale.html)

Waiting for a greenlight. Built after testing the real app against the large profile (`STUDY_SEED=large`), where half the screens grew to 10 to 300 screens tall. Laid out like SkyUI's inventory: categories down the side, a filter box, sortable columns you can show or hide, and an item card for whatever's picked. The prototype has 171 generated topics and the filter, sort, rail, ticks and keyboard all work.

| Screen | States |
|---|---|
| Topics | all, one module, filtering, due only, a topic picked, three ticked, columns, nothing matches |
| New quiz | picking, whole module ticked, filtered |
| Today | capped quizzes, quiz schedule |
| Search | empty, typed, more results |
| History | by month, filtered |
| Suggestions | grouped by topic |
| Bibliography | list, a source picked |
| Overview | weakest few |
| Cards | by topic |
| Exam topics | picking topics (the exam dates dialog) |

## The shared pieces

- **Module rail:** every module with its topic count, plus All and No module. Picking one narrows the list, like SkyUI's category column.
- **Filter box:** filters as you type, on names, modules and tags, with the match underlined and "9 of 171" beside it. `/` jumps to it from anywhere on the screen, `Esc` clears it. Looking inside questions stays Search's job, and "nothing matches" offers it.
- **Status filter:** a segmented control for the states that matter on that screen, with counts (Topics: All, Due, Tightened, Not started).
- **Sortable columns:** click a header to sort, again to flip it. "Columns" shows or hides the optional ones (tags, cards, last score). Headers stay put while the list scrolls.
- **The list scrolls in its own pane,** so the page is never taller than the window and the filter, rail and item card are always in view.
- **Item card:** picking a row shows its details and actions on the right, so rows don't each carry Quiz, Edit and Delete. `↑` `↓` move, `Enter` edits, `Space` ticks.
- **Ticks:** tick rows (or a whole module) for "Quiz them", "Move to module" or "Tag" from a bar under the list.

## Per screen

- **Topics:** all of the above. The primary is "Quiz today's 5".
- **New quiz and the exam's topics:** the same picker, with "Tick all 9 in Networks" when a module is picked. Your quiz lists the ticked topics as chips you can remove, and defaults to 20 questions spread evenly instead of every question.
- **Today:** quizzes are capped at 5 topics a day, most overdue first. The rest wait in a folded "66 more waiting" with when the last of them comes in. A waiting topic keeps its step. "Quiz all 5 together" is 20 questions. Quiz schedule gets "Topics to quiz a day", and warns when more come due each day than that, since the queue would only grow.
- **Search:** shows recent searches and recently edited questions until you type, then 50 results at a time with "Show 50 more".
- **History:** the chart defaults to the last 30 quizzes (90 days and All one click away). The table is grouped by month, folded after the first two, with a filter and a kind filter.
- **Suggestions:** grouped by topic (or module, day, none), biggest groups first, folded after the first, with "Dismiss these 14" per group and a filter.
- **Bibliography:** one compact row per source (author, year, title, "20 in 9 topics"), sortable, with a filter. The full reference and the topics citing it go in the item card.
- **Overview:** the 8 weakest topics with a module picker, and "See all in Topics".
- **Cards:** the By topic table uses the rail, filter and pane, filtered to topics with cards due by default.

## Calls made in the design, to confirm or change

- **5 quizzes a day** by default (you can change it). Most overdue first, and waiting doesn't move a topic down its ladder.
- **Row actions move to the item card.** One click to pick, then Quiz or Edit, instead of a button on every row. Double-click or `Enter` still edits straight away.
- **Search starts empty** rather than listing everything.
- **The rail and sort aren't remembered** between visits yet. SkyUI's lack of this is a common complaint, so the build could remember them per screen.

## UI/UX test results

- Every screen and state screenshotted in both themes at 1440 × 900: no console errors, no blocked requests, no horizontal scroll. The tallest page is now History at 1.4 window heights (Topics was 13.8 in the real app, Search 306).
- Audit (contrast, 24px targets, at most one primary) passes everywhere. The first pass flagged the 20px checkboxes, now 24px.
- Interaction test in the prototype: `/` focuses the filter, typing filters without losing focus, arrows move the selection into the item card, `Esc` clears, a header sorts both ways, the rail and Due narrow the list, and the page stays one window tall.
- Found while building: DaisyUI's own `list` class made tables shrink to fit (renamed), and hidden screen-reader labels escaped the scroll pane and made the page 10,000px tall (fixed by containing them).

# Backup, export, import and sync (backup-and-sync.html)

Built (see the "built" sections below). One new screen, "Your data" (in the sidebar under Search), plus a status line at the bottom of the sidebar. Designed together because they share the same foundations (IDs that are unique everywhere, a record of changes, one file format), and built in stages: backup, then the ID change and export/import, then sync.

| State | What it shows |
|---|---|
| all good / sync on | backed up today, synced 2 minutes ago, three devices |
| first time | no backups yet, sync off |
| backing up | a backup running |
| restore a backup | what you'd lose, that the current data is backed up first, and that sync pauses |
| export | everything, or some topics to share, with or without your history |
| import preview / import a shared topic | what's new, already here and changed, and what to do about clashes |
| set up sync | pick a synced folder, name this computer, then pick the same folder on the other one |
| a sync conflict | the same card edited on both computers, side by side |
| sync folder missing | sync paused, with how many changes are waiting |

## View goals

- **Your data.** *Goal:* know your study is safe, and fix it if it isn't. *Most important:* when the last backup was and whether sync is working, so that's the first thing on the page, in words and an icon (never colour alone), read out by screen readers when it changes. The one primary is whatever fixes the problem (Find the folder, Check it), otherwise Back up now. *Not needed up front:* file paths and the full backup list (five shown, the rest one click away). *Disabilities:* status in words, not a green or red dot; the path is selectable text, not a picture.
- **Restore.** *Goal:* go back to a point in time without regret. *Most important:* what will be undone, in counts you recognise ("183 card reviews"). Focus starts on Cancel, the button names the backup, and the current data is backed up first so a restore can itself be undone.
- **Import.** *Goal:* bring in only what you want. *Most important:* new, already here and changed, as a table a screen reader can read cell by cell. A backup is made first.
- **Set up sync.** *Goal:* connect two computers once and forget about it. *Most important:* which folder, and that the other computer just picks the same one. Three numbered steps with their state in the number (a tick when done), not only in colour.
- **A sync conflict.** *Goal:* pick the right version quickly. *Most important:* what differs. Differences are underlined (newer) or struck through (older), with a key, so they don't depend on colour. The newer one is already in use and the other is kept, so ignoring it loses nothing.

## How it works

**Backups**
- A full copy made with SQLite's own `VACUUM INTO`, which is safe while the app is open.
- Made once a day, and also before an import, a restore, a migration (app update) and turning on sync.
- Keeps the last 7 days, 4 weeks and 12 months, about 60 MB for the large profile.
- Saved in the app's data folder, with an optional second place (a USB stick, another drive, a synced folder).
- Restoring backs up the current data first, then swaps the file. With sync on, sync pauses after a restore, because the other computers would send the newer changes straight back.

**IDs and changes (migration 8)**
- Every row gets an ID that's unique on every computer (UUIDv7, which sorts by time). The numbers the app uses today stay as a local key, so most queries don't change.
- Editable rows (topics, questions, options, notes, checklists and their points, sources, projects, issues, drills, settings) get "last changed" stamps from a hybrid logical clock: wall time, a counter and the device. This stays in order even when two computers' clocks disagree a little.
- Deleting leaves a small marker so other devices learn about it.

**The `.studyapp` file (export and import)**
- A zip holding `manifest.json` (format version, app version, when, which device, what's in it, counts), one JSON Lines file per kind of record, and the images as files.
- Readable with any zip tool, so your data never depends on this app.
- **Everything** includes history and moves a whole profile. **Some topics** is for sharing: questions, cards, checklists, sources and images, with history off by default so a friend starts fresh.
- Import matches by ID. Importing a newer version of a shared topic updates it instead of making copies.

**Sync, through a folder**
- Nothing to pay for and no always-on server. The app never puts its database in the folder, because syncing a live SQLite file corrupts it. Instead each device appends its own changes to its own files: `devices/<device>.json` and `changes/<device>/<month>.jsonl`.
- Each file only ever has one writer, so the syncing app never has two versions of one file to choose between.
- Any folder kept the same across computers works: Syncthing (free, open source, device to device), Nextcloud, a network drive, or a USB stick carried between them.
- The browser version joins by mounting the same folder in `compose.yaml`, which also makes two-device testing easy.
- The app reads the other devices' files when it starts, every 30 seconds while it's open, and on "Sync now".
- **Things that only get added** (card reviews, quiz attempts, recall checks, drill runs, struggles) just merge, with no conflicts possible.
- **Schedules worked out from those** (a card's FSRS state, a topic's quiz step, a checklist's step, a drill's due date) are replayed from the merged history, so both computers end up with the same schedule.
- **Edits** are compared field by field, and the latest change wins. Only when both computers changed the same field before seeing each other's change is it listed under "things to check", with the other version kept.
- **Edit against delete:** the edit wins and the delete is listed to check, so nothing is lost by accident.
- A quiz or review still in progress isn't synced until it's finished.

## Calls made in the design, to confirm or change

- **Syncthing-style folder sync** rather than a server or direct device-to-device connection. It's free, needs no account, and works offline. The catch is that both computers need the folder synced at some point (Syncthing syncs whenever both are on).
- **Newest edit wins, conflicts are listed, nothing is thrown away.** No "pick one before you can carry on" popups.
- **Schedules are replayed from history** after a sync, not copied. This needs the FSRS replay to give the same result on both computers (same parameters), which the build will test.
- **Backups keep 7 days, 4 weeks and 12 months.**
- **Shared topics leave history out by default.**
- **New dependency:** a folder and file picker needs Tauri's dialog plugin (`tauri-plugin-dialog`), so the Nix pnpm hash and Cargo lock change when it's built.

## UI/UX test results

- Every state screenshotted in both themes at 1440 × 900: no console errors, no blocked requests, no horizontal scroll.
- Audit (contrast, 24px targets, exactly one primary per view) passes everywhere.
- Found while building: DaisyUI's own `status` and `diff` classes squashed the status card and the conflict text into one word per line (renamed). The set-up-sync state also said "Synced 2 minutes ago" behind the dialog (fixed to "Sync is off").

## Backups built (4 Nov 2026)

Backups are built from this design, test-first: requirements R101 to R108 in `specs/README.md`, with scenarios in `specs/features/backup.feature`. The ID change, export and import, and sync come next, in that order.

Differences from the prototype:
- **Your data only shows Backups for now.** Sync and export/import appear when they're built, rather than as buttons that don't work yet.
- **The daily backup waits until there's something to back up.** A brand-new, empty database isn't backed up.
- **"Open" the backups folder became "Copy path"**, and **"Also copy to" takes a typed full path.** Both avoid adding Tauri plugins (an opener and a folder picker) just for this. The folder picker comes with sync, which needs it anyway.
- **What a restore undoes is counted, not listed as edits:** card reviews, quizzes, recall checks, drill runs, struggles, and questions, cards and topics added since. Edits to existing things aren't recorded yet, which the ID change for sync adds.


## IDs built (4 Nov 2026)

Migration 8, test-first: requirements R109 to R111, scenarios in `specs/features/ids.feature`. Nothing on screen changes.

- **IDs:** every record with its own number also gets a UUIDv7 (`uid`). The numbers stay as the local key, so no screen or command changed.
- **Change stamps:** a `changed` stamp from a hybrid logical clock (`ids.rs`). A card's FSRS fields and a checklist's step don't count as edits, since sync will replay them from the review and check history.
- **Deletion markers:** they go in a `deleted` table, for things removed along with others too (a topic's questions, notes and cards).
- **Stamped per record, not per field:** field-by-field comparison will come from sync's change log, which records which fields changed.
- **Triggers do all of it,** calling `new_uid()`, `stamp_now()` and `device_id()`, which the app registers on its database connection. Other SQLite tools can still read the file, but they can't add rows to it.
- **Cost:** opening and upgrading the large test profile took 0.2 seconds (with a "Before update" backup first). The file grew from 5.5 MB to 8.2 MB, and seeding it took about 8% longer.

## Export and import built (4 Nov 2026)

Built test-first: requirements R112 to R118, scenarios in `specs/features/transfer.feature`.

Differences from the prototype:
- **Export saves to Downloads (desktop) or downloads in the browser,** instead of asking where to save. A save dialog needs a Tauri plugin. "Import…" uses the system's file picker, which needs none.
- **"Everything" is picked first** in the export dialog, since moving to another computer is the case where getting it wrong costs the most. Picking "Some topics" unticks history.
- **Clashes are settled before importing, not listed afterwards.** The choices are whichever changed last (the default for a full export), keep mine (the default for shared topics), take the file's, or look at each one, which lists each clash with Mine or The file's. A list of things to check afterwards comes with sync.
- **The topic picker** is a filterable list grouped by module, with a tick for a whole module.
- **Schedules:** when both sides have a card, checklist or topic, the side that reviewed, checked or quizzed it more recently keeps its schedule. Replaying the full history is left for sync.
- **Sharing some topics with history** includes card reviews, recall checks and quiz steps, but not quiz results, since those mix topics.
- **Deletions don't travel** in an import. Nothing here is ever deleted by one.

Tests:
- **Round trip:** the demo profile is exported, imported into a database that already has other records, and exported again. The two files match record for record, and history, every quiz's results, checklists, projects, due counts and suggestions all match too, including one that was dismissed.
- **Hostile files:** a newer format, a zip bomb, a reference to something missing, a repeated ID, a script posing as an image and an image named `../../etc/passwd` are each refused, and nothing changes.
- **Large profile:** a 0.7 MB file, exported in 0.3 seconds and imported in 0.5.

## Sync built (4 Nov 2026)

Built test-first: requirements R119 to R128, scenarios in `specs/features/sync.feature`. Backup, IDs, export and import, and sync are now all built from this design.

How it works:
- **A shadow of every record as last synced**, flattened into fields, with which computer changed each field and in which of its change lines. Syncing compares the database with the shadow, writes only the fields that changed, and merges the other computers' lines field by field. The shadow matches the database after every sync, so syncing again writes nothing.
- **Each line says which of every computer's lines it had seen.** Two changes to the same field are either one after the other (the later one wins quietly, and settles any clash over that field) or made at the same time (the newer stamp wins and the pair is listed to check).
- **A note's front and back are separate fields,** so editing the front on one computer and the back on the other keeps both. A card's own copy of its note's text is never listed, so a clash shows once.
- **Deletes:** a computer deletes what goes with a record first, so a parent never goes while its children are there. A delete loses to a change, or to something added to it, that the deleting computer hadn't seen, and both computers keep it and list it.
- **Something that refers to a thing still on its way waits,** and so does a record only heard about through an edit, until its creation arrives.

Differences from the design:
- **The folder is a typed path,** like "Also copy to" for backups, instead of a folder picker. That keeps out the Tauri dialog plugin. Say if you'd rather have the picker.
- **The computers table's last column is "Changes waiting":** for this computer, changes not in the folder yet (or waiting for it). For the others, changes they've made that haven't arrived here yet.
- **Things to check are kept per computer.** Putting the other version back goes to every computer and settles it everywhere. "Keep" only dismisses it on the computer you're using.
- **Other schedules (a topic's quiz step, a checklist's step, a drill's due date) take the newer one** rather than being replayed. A card reviewed on two computers at once is worked out again from all its reviews on both, so they end up the same.
- **"Sync now" during a quiz or review says to finish first,** and the 30-second tick waits quietly.
- **No "Edit the card" button** on things to check yet. Both versions are shown and either can be kept.
- **Found while building:** writing a record's stamp when it already had that stamp counted as an edit to the database triggers, a quiz's per-topic results came out in local row order, and a review's schedule stamp leaked into a card's "last changed". All three would have kept two computers sending each other the same change forever, and are fixed.

Tests:
- **Two and three computers** as separate databases sharing a real folder: setting up, studying on both, different and same fields, deletes, a delete losing to an addition, the same card reviewed on both, something arriving before what it belongs to, an edit before its creation, a missing folder, a half-written file and a junk line, a newer app, a quiz in progress, a restore, renaming, moving and turning off.
- **Random:** three computers doing random edits, deletes, reviews and syncs in a random order, 20 runs in the suite (and 660 while building), all ending with exactly the same data and nothing left to send.
- **Mutation checks:** ten deliberate breaks in the merge (an off-by-one in what was seen, no child check on deletes, no replay, last arrival wins, parents deleted first and so on) are each caught.
- **In the real app:** the desktop app syncs with a browser-version server through one folder. Audit clean in both themes.
- **Large profile:** the first sync takes 0.7 seconds to send and 1.2 to receive (10 MB of change files), a sync with nothing new 0.1 seconds.


## Playful redesign (exploration)

`playful-redesign/index.html` redraws all 22 screens in a chunky, game-like style and lets you flip each one against a screenshot of the current app (`playful-redesign/shots/`). It's a comparison piece, not the agreed design, and it doesn't follow `docs/design-rules.md` yet.

- **Theme** switch: Gleam colours (faff pink, aged plastic yellow, underwater blue) or the current warm paper neutrals.
- **Colours** switch: standard green/red for right and wrong, or the colour-blind safe blue/orange pair. Icon, word and the guessed hatch stay in both.
- The page ends with what the style costs against the design rules (type, radii, density, red meaning both wrong and destructive).

Fonts load from Google Fonts, so open it online.

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

Checked in the real app (`e2e/recall_test.py`, run by `e2e/run.sh`): makes a checklist, writes points with maths, adds one from an MCQ, types a dump with the checklist hidden, ticks under the pass mark with the live outcome, and checks the result, the recall list and the history. With `AUDIT` set it also measures every screen in both themes for text contrast, target size and primary buttons. That pass found the score chart's table header at 4.06:1 in the light theme (the same on the Overview chart). Fixed to the muted text colour, and now every screen passes.

---

# Flashcards with FSRS (proposed, waiting for a greenlight)

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

## Open questions (dashed badges in the prototype)

- **A.** Card types: just front and back, or also cloze (`{{c1::…}}`) and reversed cards?
- **B.** Suggestions: offer them for you to accept (as designed), or turn misses into cards automatically?

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

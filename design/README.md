# Study App Designs

Clickable HTML prototype of every MVP screen, built with the `studyapp` DaisyUI theme from `docs/design-rules.md`. Open `index.html` in a browser. The dark bar in the bottom right switches each screen between its states.

## Screen inventory

| Screen | States | Requirements | Feature files |
|---|---|---|---|
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

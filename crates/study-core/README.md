# study-core

Pure domain rules for the Study App, built test-first from `specs/`. No IO, no database, no UI.

| Module | Requirements | Feature files |
|---|---|---|
| `question` | R4, R5, R6, R9, R10, R14 | questions, sources |
| `assembly` | R15, R16, R17, R18, R19 | quiz-setup |
| `attempt` | R18 to R23, R26, R28, R29 | taking-quiz, retry, history |
| `results` | R24, R25 | results |
| `progress` | R33 | overview |

Each test is commented with the scenario it covers. Run with `cargo test -p study-core`.

## Open questions

Cases still open in `specs/README.md` return an error instead of guessing:

- Q2, Q3, Q4 (quiz setup limits): `AssemblyError::Undecided`
- Q6 (going back in study mode): `AttemptError::GoingBackInStudyUndecided`
- Q7 (do wrong guesses count as "guessed"): `Tally` keeps `wrong_flagged` separate so either answer can be shown

## Decisions made while building

- A single-answer question with several options marked correct can't be saved (follows from R5). Q15, what happens when you switch a question from multiple to single, is still open.
- When a quiz ends early (total timer or quitting), unanswered questions count towards the total, so a score reads "4 / 15" as in the designs.
- Overview accuracy is right answers over questions asked in the window, matching how scores are shown.

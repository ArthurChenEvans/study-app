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

## Decided rules

- R36 to R38 (quiz setup limits): capped to what's available, extras given out in topic order, empty topics refused with `AssemblyError::EmptyTopic`
- R40: `go_back` always returns `AttemptError::NoGoingBack`
- R41: "guessed" counts right guesses only. `Tally::wrong_flagged` is kept for the record

## Decisions made while building

- A single-answer question with several options marked correct can't be saved (follows from R5). Switching multiple to single keeps the first tick (R49), handled in the editor.
- When a quiz ends early (total timer or quitting), unanswered questions count towards the total, so a score reads "4 / 15" as in the designs.
- Overview accuracy is right answers over questions asked in the window, matching how scores are shown.

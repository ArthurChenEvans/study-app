# study-store

SQLite storage for the Study App (rusqlite, bundled SQLite), built test-first against in-memory databases.

| Area | Requirements | Feature files |
|---|---|---|
| Modules, topics, topic tags | R1, R2, R3, R30 | topics |
| Questions with options, tags, citations | R4 to R10, R14 | questions, sources |
| Bibliography in APA with "used by" topics | R13 | sources |
| Attempts, reasons, history, overview records | R24 to R29, R33 | results, retry, history, overview |
| Search with tabs, filters and sorting | R31 | search |

- Migrations are numbered SQL blocks tracked with `PRAGMA user_version`. A database from a newer app version is refused instead of being damaged.
- History is stored as self-contained snapshots with no foreign keys, so editing or deleting topics and questions never changes past attempts.

## Not implemented (open questions)

- Deleting a module (Q9), deleting or editing a cited source (Q10, Q11), resuming an incomplete attempt (Q13).
- Q16: matching questions only appear under the All search tab until it's decided which tab they belong to.

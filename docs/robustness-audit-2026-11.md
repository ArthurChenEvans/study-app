# Robustness audit, November 2026

What can break, leak, corrupt or lose data, or slow down, tested against the running app. Each confirmed problem has a test that failed before its fix. Branch `audit/robustness`, starting from commit `434522d`.

Run everything with `scripts/test-all.sh` (about 10 minutes), or `scripts/test-all.sh --quick` for the tests that don't need a release build.

## The register, with outcomes

L and D are likelihood and damage as judged before testing.

| # | Hazard | Outcome | What testing showed, and the fix |
|---|---|---|---|
| 1 | A running quiz lost on close or crash | **Confirmed, fixed** | Every answer was lost. Now saved after each step and resumed on start. Checked in Chrome: left mid-quiz, reopened at question 2 of 5 |
| 2 | A panic locks the app until restart | **Confirmed, fixed** | On the running app one panic left every command failing. The lock now recovers. Five panics found by fuzzing are fixed (calendar range, dates far away, timer overflow, clock before 1970, day sums) |
| 3 | Restoring a bad backup | **Confirmed, fixed** | A damaged backup was reported as restored and every screen then failed. Now checked after restoring and undone if not sound |
| 4 | Upgrade interrupted | **Tested, clean** | 90 kills during upgrades from each of the 9 older versions with rows in every table: no damage, no rows lost, the pre-update backup always sound |
| 5 | Kill during a save | **Tested, clean** (one leftover fixed) | 40 kills under constant writes: `integrity_check` and `foreign_key_check` clean, 368 acknowledged saves all present, no half-saved question, note or checklist. Half-written backup files were left behind, now cleared |
| 6 | Two copies on one database | **Tested, clean** (one note) | 800 interleaved saves from two copies: none lost, no duplicate IDs. Two copies can make the same change stamp in the same millisecond. Not fixed: a stamp only orders changes for sync |
| 7 | Damaged backup settings prune backups | **Confirmed, fixed** | A cut-short `settings.json` made the next backup delete 30 of 40 backups. Unreadable settings now thin nothing, and the file is written whole |
| 8 | Disk full, read-only database | **Tested, clean** (one leftover fixed) | On a full 3 MB disk, saves failed with "database or disk is full", nothing acknowledged was lost, the database stayed sound. A leftover journal file is now cleared. Read-only was not tested (tests ran as root, which ignores file permissions) |
| 9 | Hostile sync folder | **Confirmed, fixed** | Damaged lines were merged and then Cards, Calendar and Projects failed to load. A merge is now undone unless everything reads back |
| 10 | Hostile import | **Confirmed, fixed** | The zip limits held. But files with values the app can't read were imported and broke History, Overview, Cards and Calendar. Same fix as 9 |
| 11 | Wrong clock, midnight | **Tested, clean** (overflow fixed) | Property tests on the ladders, FSRS and tightening all held, including reviews done before the previous one. Day numbers no calendar has overflowed three sums, now saturating |
| 12 | Web version accepts other sites' requests | **Confirmed, fixed** | With another site's Origin a request created a module, with a foreign Host it read the data. Now refused: Host, Origin and JSON content type are checked |
| 13 | Web version sends no security headers | **Confirmed, fixed** | None were sent. Now the desktop policy plus frame, form and base restrictions. The container still runs as root: not changed, since it couldn't be tested here |
| 14 | Rendered text can restyle or cover the app | **Confirmed, fixed** | No script ran on any screen, but question text covered the whole window and showed its own form. Style, class, id, forms and media are now stripped |
| 15 | Script in the window can call every command | **Tested, clean** (policy tightened) | Hostile text in every field on all 11 screens in Chrome ran nothing. The desktop policy gains three directives and a test pins it and the permissions |
| 16 | Links navigate the app away | **Confirmed, fixed** | A click navigated the window and lost the quiz. Links no longer navigate. The browser version opens them in a new tab, the desktop app opens nothing |
| 17 | Folder settings accept any path | **Tested, by design** | You choose those folders. The remote route to them (12) is closed. Restore names were already limited to listed backups |
| 18 | Slide conversion leftovers | **Confirmed, fixed** | A timeout left the converter's child running, the temp folder was world-readable, and over 64 KB of converter output blocked it until the timeout |
| 19 | Big or hostile EPUB and PDF | **EPUB confirmed, fixed. PDF not tested** | A 41 KB EPUB unpacked to 40 MB. Now capped while unpacking. The PDF view still draws every page up front |
| 20 | Vulnerable dependencies | **Confirmed, fixed** | Four advisories in `zip`, in a function the app doesn't call. Updated. Three remain with no fix available, none reachable (see below) |
| 21 | Slow lists and calendar | **Confirmed, fixed** | On 3 years of use: Cards 549 ms to 83 ms, today's calendar 500 ms to 28 ms. Calendar range capped at 400 days. All screens now under 100 ms |
| 22 | Long actions freeze the window | **Not tested** | Needs the desktop shell, which can't be built here. Measured in the backend: export 1.1 s, backup 87 ms, sync 0.6 s, the read-back check 0.9 s |
| 23 | Rendering cost | **Partly tested, not fixed** | Renderer failures no longer break a screen. A 1 MB question with 20,000 formulas takes seconds to render. List screens were not timed in a browser on the big seed |
| 24 | Growth | **Measured, fine** | Three years of heavy use is 25 MB, and a backup of it takes 87 ms |

## Not tested, and why

- **The desktop shell itself.** WebKitGTK isn't available in the test environment, so nothing in `src-tauri` was compiled or run. Two changes there are small but unverified: `study_api::locked` in `with`, and three added directives in `tauri.conf.json`. Build and open the app once before relying on them.
- **Row 22**, for the same reason. If the window does freeze during an export or a sync, the likely fix is marking those commands `#[tauri::command(async)]`.
- **PDF previews** with huge or malformed files, and list screens' render time in a browser on the big seed.
- **A read-only database**, since the tests ran as root.
- **Podman itself.** The web server was run directly, not in the container.
- Load testing, multi-user access control and network pen testing, as agreed.

## Decisions made along the way

- **Resuming a quiz restarts its timer from full.** The app doesn't record time already spent.
- **Sync is refused while a quiz is running**, as before. With quizzes now surviving a restart, the app opens straight onto a running quiz so it isn't left blocking sync unnoticed.
- **Links in the desktop app open nothing.** Opening the system browser needs Tauri's opener plugin, which is a new permission and your call.
- **Raw HTML in question text still works** for formatting (`<sub>`, `<kbd>`, tables). It can no longer carry style, class or id.
- **Dates must fall between 1970 and 2300**, quiz timers are capped at a week, and the calendar lists at most 400 days per request.
- **`STUDY_HOSTS`** lists extra host names the web version answers to. Without it, only localhost.

## Evidence

### Each test failing before its fix

Recorded at the point each test existed and its fix didn't.

| Hazard | Test | Before the fix |
|---|---|---|
| 1 | `a_quiz_in_progress_survives_the_app_closing` | "no quiz is running" after reopening |
| 1 | `App.resume.test.ts` | question text not found |
| 2 | `a_date_absurdly_far_away_is_refused...` | day -9223372036854775808 accepted |
| 2 | `a_clock_set_before_1970...` | panic in `recall.rs:234` |
| 2 | `a_quiz_timer_longer_than_time_itself_is_refused` | panic, multiply with overflow |
| 2, 21 | `the_calendar_refuses_a_range_no_screen_would_ask_for` | panic, capacity overflow |
| 2 | `a_panic_in_one_command_doesnt_lock_the_app_for_good` | every command failed after one panic |
| 3 | `a_backup_that_doesnt_restore_cleanly_is_undone` | reported restored, then "no such column" |
| 5, 8 | `half_written_backups_from_a_crash_are_cleared_by_the_next_backup` | stale files left |
| 7 | `damaged_backup_settings_never_thin_out_backups` | 40 backups became 10 |
| 9 | `damaged_lines_in_the_sync_folder_never_break_this_computer` | Cards, Calendar, Projects failed |
| 10 | `a_damaged_import_is_refused_whole_or_leaves_every_screen_working` | History, Overview, Cards, Calendar failed |
| 11 | `no_day_number_makes_*` (proptest) | subtract with overflow in `cards.rs:339` |
| 12, 13 | `another_website_cannot_call_the_server`, `every_response_carries_the_security_headers` | request went through, no headers |
| 14 | `markdown.security.test.ts` | style, class, forms kept |
| 16 | `links.test.ts` | no link handling |
| 14, 19 | `epub.security.test.ts` | no size limit |
| 18 | `a_stuck_slide_conversion_leaves_no_process...` | one child left running, folder mode 755 |
| 18 | `a_failed_slide_conversion_reports_the_converters_own_message` | timed out after 20 s |
| 21 | `perf.rs` | 549 ms and 500 ms against 100 ms |

### Mutation testing (cargo-mutants)

Run on the Rust lines this audit changed (`--in-diff`), each crate against its own tests.

| Code | Mutants | Caught | Missed | Couldn't build |
|---|---|---|---|---|
| `study-core` and `study-store` | 45 | 34 | 6 | 5 |
| `study-web` | 22 | 19 | 2 | 1 |
| `study-api`, the functions this audit wrote or reworked | 45 | 27 | 15 | 3 |

What the misses were, and what happened to them:

- **Store import and sync functions (6).** The store's own tests don't cover them, the API's do. Rerun against both: 5 caught, 1 survives.
- **API edges (15).** The calendar limit, the hour before a half-written backup is cleared, "no settings file" against "unreadable settings", the minutes sum. Tests were added and the 28 mutants in those functions rerun: 27 caught, 1 survives.
- **Earlier in the run**, three gaps in core and store were found and tested before the table above was produced: `Store::health`, the trial and running-quiz functions, and the month sum past the end of the calendar.

Surviving mutants:

| Mutant | Why it survives |
|---|---|
| `Store::sync_take_other` replaced with `Ok(())` | Nothing asserts the effect of taking the other computer's version in a conflict. This predates the audit, which changed one line of it |
| `>` to `>=` in `clear_stale_partials` | Only differs for a file exactly one hour old to the nanosecond |
| `64 * 1024` to `64 + 1024` in `slides_to_pdf` | The cap on how much converter output is read. The message shown is its last line either way |
| `Config::from_env`, `main` in `study-web` | Read the environment and start the server. The kill tests exercise them, the unit tests don't |
| `App::store_mut`, `App::check_data` | Only compiled for the performance test |

Not run: about 110 mutants in API functions the audit touched lightly (a line or two in `start_quiz`, `answer`, `sync_now` and similar). Each takes about 80 seconds, so the run was limited to the functions with new logic. The quiz timer cap was missed in a first partial run and has a boundary test since, which wasn't rerun under mutation.

### Dependency audits

- `cargo audit`: 0 vulnerabilities after the `zip` update. Two warnings remain, both from Tauri's GTK bindings with no fixed version: `proc-macro-error` (unmaintained, build time only) and `glib` (`VariantStrIter`, which the app doesn't use).
- `cargo deny check advisories bans sources`: passes with `deny.toml`.
- `pnpm audit`: one low-severity advisory in `deepmerge`, a build tool dependency with no patch.

### Performance on three years of seeded use

170 topics, 4,600 questions, 1,600 quizzes, 43,000 card reviews, release build, best of three.

| Screen | Slowest command | Time | Budget |
|---|---|---|---|
| Cards | `cards_home` | 80 to 92 ms | 100 ms |
| Overview | `overview` | 34 ms | 100 ms |
| History | `history` | 30 ms | 100 ms |
| Calendar | `calendar_month` | 31 ms | 100 ms |
| Search | `search` | 14 ms | 100 ms |
| Quiz | `start_quiz` across every topic | 120 to 160 ms | 2 s |
| Your data | `export_file` | 1.1 s | 2 s |
| Your data | read-back check | 0.9 s | 2 s |

The Cards screen is the closest to its budget. No statement these commands run does a full table scan.

## Left open

1. Build and run the desktop app once to confirm the two `src-tauri` changes.
2. Row 22: check whether export, sync and backup freeze the desktop window.
3. Lazy page rendering in the PDF view.
4. The surviving mutants listed above.
5. Running the container as a non-root user.
6. Whether links in the desktop app should open the system browser.

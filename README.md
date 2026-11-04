# study-app

Vibecoded study app to help study for exams. All-in-one tool for MCQ quizzes, built with Tauri 2, Svelte 5 and Rust (SQLite). Linux first.

## Run it (NixOS)

```bash
nix develop          # Rust, Node, pnpm, WebKitGTK, LibreOffice
pnpm install
pnpm tauri dev       # opens the app
```

## Try it with demo data

`study-web` runs the same app in a browser, and can fill a database with a demo profile:
5 topics across 3 modules, MCQ and short answer questions with sources, and 60 days of
quizzes, recall checks and card reviews (including cards flagged to rewrite and a late
checklist), plus three projects with sessions, struggles, shared issues and drills, so the
calendar, quiz schedules and today's drills all have something in them.

### In a container

```bash
podman compose up --build -d   # http://localhost:7392, seeded on the first start
podman compose down            # stop, keep the database
podman compose down -v         # stop and delete the database, the next up seeds again
```

Set `STUDY_SEED: "0"` in `compose.yaml` to start empty. `STUDY_SEED=large podman compose up -d`
(after a `down -v`) seeds a heavy profile instead: about 170 topics in 14 modules, thousands of
questions and cards and four months of history, for checking how screens cope with a lot of data.
Outside a container that's `study-web seed <database> --large`. PowerPoint previews need
`SLIDES: "1"` there, which adds LibreOffice to the image (much bigger). Docker works the same.
It's on port 7392, only open to this computer since there are no logins. If that's taken,
`STUDY_PORT=7400 podman compose up -d` uses another. Outside a container, `STUDY_ADDRESS=127.0.0.1:7400` does the same.

### Without a container (inside `nix develop`)

```bash
# Desktop app on a throwaway seeded database
export XDG_DATA_HOME=$(mktemp -d)
cargo run -p study-web -- seed "$XDG_DATA_HOME/com.arthurchenevans.studyapp/study.sqlite"
pnpm tauri dev

# Or the browser version with hot reload: the server on :7392, Vite on :5173
STUDY_DB=$(mktemp -d)/study.sqlite STUDY_SEED=1 cargo run -p study-web &
pnpm dev
```

The browser and desktop versions each have their own database. They don't sync.

## Backups

The app backs itself up once a day (once there's something in it), and before a restore or an update that upgrades the database. Backups are full copies, named like `study 2026-11-04 09-12 daily.sqlite`, in:

- **Desktop:** `~/.local/share/com.arthurchenevans.studyapp/backups`
- **Browser version:** `backups` next to its database, so `/data/backups` in the container's volume (`down -v` deletes them too). `STUDY_BACKUPS` points it somewhere else.

"Your data" in the sidebar lists them, backs up by hand, restores, sets how many to keep (7 days, 4 weeks and 12 months to start with) and can copy every backup to a second folder, like a USB stick. Each backup is an ordinary SQLite file, so you can also copy one into place by hand while the app is closed.

Every record has an ID that's the same on every computer, a "last changed" stamp and, once deleted, a marker. The database's triggers fill these in using functions the app provides, so other SQLite tools can read the database but can't add or change rows in it.

## Export and import

"Your data" exports everything, or some topics to share, as one `.studyapp` file: a zip of readable JSON (`manifest.json`, `data/topics.jsonl` and so on) plus the images, so the data is never locked inside the app. The desktop app saves it to your Downloads folder, and the browser version downloads it.

Importing shows what's new, what's here already and what's changed in both places before anything changes, then makes a backup and adds it all at once. Records are matched by ID, so importing a newer version of a shared file updates it instead of copying it. History is only ever added, and nothing here is deleted.

## Sync

"Your data" can keep computers (and the browser version) in step through any folder another app keeps the same on each one: Syncthing (free, no account), Nextcloud, a network drive, or a USB stick you carry. Pick the folder, name the computer, then pick the same folder on the other one. There's no server and nothing to pay for.

The database never goes in the folder, since syncing a live SQLite file corrupts it. Each computer appends its changes to its own files (`devices/<id>.json` and `changes/<id>/<month>.jsonl`), so no file ever has two writers. The app reads the others' files when it starts, every 30 seconds while it's open, and on "Sync now". Edits are merged field by field. When the same field changed on two computers before they saw each other's change, the newer one is used and the pair is listed under "things to check", where the other version can be put back. A delete loses to a change made elsewhere before that computer saw it, so nothing is lost by accident. Restoring a backup pauses sync.

The browser version joins by mounting the same folder in `compose.yaml` (see the comment there) and typing its path in the set-up dialog.

## Install

### Nix (NixOS)

```sh
nix --extra-experimental-features "nix-command flakes" build
./result/bin/study-app
```

Or add it to your system flake: `inputs.study-app.url = "github:ArthurChenEvans/study-app/mvp-planning";` and put `study-app.packages.x86_64-linux.default` in `environment.systemPackages`. LibreOffice comes with it for PowerPoint previews.

The first build stops with a hash mismatch for the pnpm dependencies. Copy the `got: sha256-...` value into `hash =` in `flake.nix` and build again. Do the same whenever `pnpm-lock.yaml` changes.

### AppImage (other distros)

```sh
nix develop --extra-experimental-features "nix-command flakes"
pnpm install
pnpm tauri build --bundles appimage
```

The file lands in `target/release/bundle/appimage/`. On NixOS run it with `nix run nixpkgs#appimage-run -- "Study App_0.1.0_amd64.AppImage"`. PowerPoint previews need LibreOffice installed on the system, since it isn't bundled.

## Tests

```bash
cargo test           # domain, storage, command layer and the web server
pnpm test            # Svelte screens (Vitest)
```

## Layout

| Path | What it is |
|---|---|
| `specs/` | Requirements (R1 to R34), Gherkin feature files, open questions |
| `docs/design-rules.md` | Design system: tokens, type, colour, charts, both themes |
| `design/` | Clickable HTML prototype of every screen |
| `crates/study-core` | Domain rules, no IO |
| `crates/study-store` | SQLite storage and search |
| `crates/study-api` | Everything the app's commands do, as testable functions |
| `src-tauri` | Desktop shell, forwards commands to study-api |
| `src` | Svelte frontend |

## End-to-end tests

Drive the real app window through tauri-driver, on a fresh throwaway database:

- `e2e/slides_test.py` converts a slide deck over the same IPC the preview uses
- `e2e/rich_content_test.py` writes a question with a pasted image, a picked image and highlighted code, then takes a quiz on it and checks the images decode and the code is coloured in both themes

```sh
cargo install tauri-driver   # once; also needs WebKitWebDriver from webkitgtk and python selenium
e2e/run.sh                   # builds a debug app, then runs both (starts Xvfb if there's no display)
```

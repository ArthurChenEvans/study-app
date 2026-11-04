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

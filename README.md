# study-app

Vibecoded study app to help study for exams. All-in-one tool for MCQ quizzes, built with Tauri 2, Svelte 5 and Rust (SQLite). Linux first.

## Run it (NixOS)

```bash
nix develop          # Rust, Node, pnpm, WebKitGTK, LibreOffice
pnpm install
pnpm tauri dev       # opens the app
```

## Tests

```bash
cargo test           # domain, storage and command layer
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

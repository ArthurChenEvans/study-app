# study-app

Vibecoded study app to help study for exams. All-in-one tool for MCQ quizzes, built with Tauri 2, Svelte 5 and Rust (SQLite). Linux first.

## Run it (NixOS)

```bash
nix develop          # Rust, Node, pnpm, WebKitGTK, LibreOffice
pnpm install
pnpm tauri dev       # opens the app
```

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

#!/usr/bin/env bash
# Every test the app has, in one command:
#
#   scripts/test-all.sh           everything (about 10 minutes)
#   scripts/test-all.sh --quick   the unit, property, fuzz and frontend tests (about 3 minutes)
#
# The full run adds the ones that need a release build: performance on three years of
# seeded use, and killing the running app while it saves and while it upgrades.
# Stops at the first failure and says which step it was.
set -euo pipefail
cd "$(dirname "$0")/.."

quick=0
[ "${1:-}" = "--quick" ] && quick=1

step() { printf '\n\033[1m== %s\033[0m\n' "$1"; }
trap 'echo; echo "FAILED at: ${current:-setup}" >&2' ERR

current="Rust tests (unit, properties, command fuzzing, damaged files, HTTP)"; step "$current"
cargo test

current="Frontend types"; step "$current"
pnpm check

current="Frontend tests (components, sanitiser, links, EPUB limits)"; step "$current"
pnpm test

if [ "$quick" = 0 ]; then
  current="Performance on three years of use"; step "$current"
  cargo test --release -p study-web --features trace --test perf

  current="Kill -9 while saving and while upgrading"; step "$current"
  cargo build --release -p study-web
  python3 scripts/kill-tests.py

  current="Known vulnerabilities in dependencies"; step "$current"
  if command -v cargo-audit >/dev/null; then cargo audit; else echo "skipped: cargo install cargo-audit"; fi
  if command -v cargo-deny >/dev/null; then cargo deny check advisories bans sources; else echo "skipped: cargo install cargo-deny"; fi
  pnpm audit --prod
fi

trap - ERR
printf '\n\033[1mAll passed.\033[0m\n'

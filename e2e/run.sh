#!/usr/bin/env bash
# Needs tauri-driver (cargo install tauri-driver) and WebKitWebDriver (webkitgtk).
set -euo pipefail
cd "$(dirname "$0")/.."
[ -n "${APP:-}" ] || pnpm tauri build --debug --no-bundle
tauri-driver & driver=$!
trap 'kill $driver' EXIT
sleep 2
python3 e2e/slides_test.py

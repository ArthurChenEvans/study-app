#!/usr/bin/env bash
# Needs tauri-driver (cargo install tauri-driver) and WebKitWebDriver (webkitgtk).
set -euo pipefail
cd "$(dirname "$0")/.."
[ -n "${APP:-}" ] || pnpm tauri build --debug --no-bundle
# A fresh, throwaway data folder so tests never touch real study data. It has to be
# set before the driver starts, since the app inherits the driver's environment.
export XDG_DATA_HOME="$(mktemp -d)"
tauri-driver & driver=$!
trap 'kill $driver; rm -rf "$XDG_DATA_HOME"' EXIT
sleep 2
for test in e2e/*_test.py; do python3 "$test"; done

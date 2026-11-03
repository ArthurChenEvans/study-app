#!/usr/bin/env bash
# Needs tauri-driver (cargo install tauri-driver), WebKitWebDriver (webkitgtk) and
# python selenium. With no display (CI, SSH) it starts Xvfb. Each run gets a fresh,
# throwaway database so the tests never touch your real questions.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -n "${APP:-}" ] || pnpm tauri build --debug --no-bundle
if [ -z "${DISPLAY:-}" ]; then
  Xvfb :99 -screen 0 1440x900x24 >/dev/null 2>&1 & xvfb=$!
  export DISPLAY=:99
fi
export XDG_DATA_HOME="$(mktemp -d)"
tauri-driver & driver=$!
trap 'kill $driver ${xvfb:-} 2>/dev/null; rm -rf "$XDG_DATA_HOME"' EXIT
sleep 2
status=0
for test in e2e/slides_test.py e2e/rich_content_test.py e2e/recall_test.py e2e/flashcards_test.py e2e/short_answer_test.py; do
  echo "== $test"
  python3 "$test" || status=1
done
exit $status

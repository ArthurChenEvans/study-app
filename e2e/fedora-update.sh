#!/usr/bin/env bash
# updates.feature end to end on Fedora (R165 to R168). Runs as root in a fresh Fedora container.
#
#   e2e/fedora-update.sh FOLDER
#
# FOLDER holds old.rpm, new.rpm and new.rpm.sig: two builds of the app signed with a throwaway
# key that both trust, old.rpm at version $OLD and new.rpm at $NEW. The script installs the old
# one, adds some study data, then serves update feeds from this machine and checks that:
#   the same or an older version is not an update, offline fails quietly, a tampered package is
#   refused, and a newer release installs, keeps the data and backs it up first.
set -euo pipefail

ART=$(realpath "$1")
OLD=${OLD:?set OLD to the old version}
NEW=${NEW:?set NEW to the new version}
HERE=$(dirname "$(realpath "$0")")
FEED=/srv/feed
PORT=8765
DATA=/home/chen/.local/share/com.arthurchenevans.studyapp
PASS=0

say() { printf '\n== %s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; exit 1; }
ok() { printf 'ok: %s\n' "$*"; PASS=$((PASS + 1)); }
version() { rpm -qa --qf '%{NAME} %{VERSION}\n' | awk '/study|shudhana/ {print $2}'; }
packages() { rpm -qa --qf '%{NAME}\n' | grep -E 'study|shudhana' | sort | tr '\n' ' '; }

say "Fedora $(rpm -E %fedora), a user with a password prompt they can't see (sudo)"
dnf -y -q install sqlite python3 sudo xorg-x11-server-Xvfb xvfb-run dbus-daemon --skip-unavailable >/dev/null
id chen >/dev/null 2>&1 || useradd -m chen
# pkexec and zenity aren't installed, so the updater falls back to sudo, as on a terminal.
echo 'chen ALL=(ALL) NOPASSWD: /usr/bin/rpm' > /etc/sudoers.d/chen

say "Install the current version by hand"
dnf -y -q install "$ART/old.rpm" >/dev/null
[ "$(version)" = "$OLD" ] || fail "installed $(version), expected $OLD"
ok "installed $OLD"

say "Publish feeds"
mkdir -p "$FEED"
cp "$ART/new.rpm" "$ART/new.rpm.sig" "$FEED/"
# Tampered: same signature, one byte changed near the end, still a valid-looking .rpm.
cp "$ART/new.rpm" "$FEED/tampered.rpm"
size=$(stat -c %s "$FEED/tampered.rpm")
printf '\xff' | dd of="$FEED/tampered.rpm" bs=1 seek=$((size - 100)) conv=notrunc status=none
cp "$ART/new.rpm.sig" "$FEED/tampered.rpm.sig"
echo "Notes for $NEW." > /tmp/notes
url=http://127.0.0.1:$PORT
python3 "$HERE/../scripts/latest-json.py" "$NEW" /tmp/notes "$FEED/newer.json" "linux-x86_64-rpm=$FEED/new.rpm=$url/new.rpm" >/dev/null
python3 "$HERE/../scripts/latest-json.py" "$OLD" /tmp/notes "$FEED/same.json" "linux-x86_64-rpm=$FEED/new.rpm=$url/new.rpm" >/dev/null
python3 "$HERE/../scripts/latest-json.py" "0.0.1" /tmp/notes "$FEED/older.json" "linux-x86_64-rpm=$FEED/new.rpm=$url/new.rpm" >/dev/null
python3 "$HERE/../scripts/latest-json.py" "$NEW" /tmp/notes "$FEED/tampered.json" "linux-x86_64-rpm=$FEED/tampered.rpm=$url/tampered.rpm" >/dev/null
python3 -m http.server "$PORT" -d "$FEED" >/tmp/http.log 2>&1 &
sleep 1

# Runs `study-app --update-now` as chen against a feed. Sets CODE and keeps stderr in /tmp/run.log.
run() {
  set +e
  sudo -u chen -H env STUDY_UPDATE_URL="$1" WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    timeout 120 dbus-run-session -- xvfb-run -a study-app --update-now >/tmp/run.log 2>&1
  CODE=$?
  set -e
  grep '^update:' /tmp/run.log || true
}

say "Same version: not an update (also creates the database)"
run "$url/same.json"
[ "$CODE" = 3 ] || { cat /tmp/run.log; fail "exit $CODE, expected 3"; }
[ -f "$DATA/study.sqlite" ] || fail "no database at $DATA"
ok "same version left alone"

say "Add study data"
# The app's triggers call functions it registers itself, so stand in for them here.
sudo -u chen python3 - "$DATA/study.sqlite" <<'PY'
import sqlite3, sys, time, uuid
db = sqlite3.connect(sys.argv[1])
db.create_function("new_uid", 0, lambda: str(uuid.uuid4()))
device = db.execute("SELECT device FROM this_device").fetchone()[0]
db.create_function("stamp_now", 0, lambda: f"{int(time.time() * 1000):016d}-0000-{device}")
db.create_function("device_id", 0, lambda: device)
db.execute("INSERT INTO modules (name) VALUES ('Kept through the update')")
db.commit()
PY
ok "data added"

say "Older version: not an update"
run "$url/older.json"
[ "$CODE" = 3 ] || fail "exit $CODE, expected 3"
[ "$(version)" = "$OLD" ] || fail "version changed to $(version)"
ok "older release ignored"

say "Offline: fails quietly"
run "http://127.0.0.1:9/latest.json"
[ "$CODE" = 4 ] || fail "exit $CODE, expected 4"
[ "$(version)" = "$OLD" ] || fail "version changed to $(version)"
ok "offline failed without changing anything"

say "Tampered package: refused"
run "$url/tampered.json"
[ "$CODE" = 4 ] || fail "exit $CODE, expected 4"
grep -qi signature /tmp/run.log || { cat /tmp/run.log; fail "the refusal didn't mention the signature"; }
[ "$(version)" = "$OLD" ] || fail "a tampered package got installed: $(version)"
ok "tampered package refused, still $OLD"

say "Newer release: installs"
run "$url/newer.json"
[ "$CODE" = 0 ] || { cat /tmp/run.log; fail "exit $CODE, expected 0"; }
[ "$(version)" = "$NEW" ] || fail "installed $(version), expected $NEW"
ok "updated $OLD -> $NEW"
# A rename (study-app to shudhana) must replace the old package, not sit beside it.
[ "$(rpm -qa --qf '%{NAME}\n' | grep -cE 'study|shudhana')" = 1 ] || fail "two copies installed: $(packages)"
ok "one copy installed: $(packages)"

kept=$(sudo -u chen sqlite3 "$DATA/study.sqlite" "SELECT count(*) FROM modules WHERE name = 'Kept through the update';")
[ "$kept" = 1 ] || fail "the study data is gone"
ok "study data still there"
ls "$DATA/backups" | grep -q before-update || { ls -la "$DATA/backups"; fail "no before-update backup"; }
ok "backed up before installing: $(ls "$DATA/backups" | grep before-update)"

say "The new version starts and has nothing newer"
run "$url/newer.json"
[ "$CODE" = 3 ] || fail "exit $CODE, expected 3"
ok "new version runs"

say "All $PASS checks passed"

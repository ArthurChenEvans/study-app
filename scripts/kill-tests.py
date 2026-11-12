#!/usr/bin/env python3
"""Kills the running app (kill -9) while it saves and while it upgrades an older database,
then checks the database with SQLite's own checks and a few invariants of the app's.

  scripts/kill-tests.py [--rounds 40] [--skip-upgrade]

Needs a built server (cargo build --release -p study-web). Everything happens in a
throwaway folder under the temp folder, never in your own data.
"""
import argparse, json, os, random, shutil, signal, sqlite3, subprocess, sys, tempfile, threading, time, urllib.error, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("STUDY_WEB", os.path.join(ROOT, "target", "release", "study-web"))
PORT = int(os.environ.get("KILL_TEST_PORT", "7394"))


def call(command, args=None, timeout=5):
    request = urllib.request.Request(f"http://127.0.0.1:{PORT}/api/{command}", data=json.dumps(args or {}).encode(), headers={"content-type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return True, json.loads(response.read() or b"null")
    except urllib.error.HTTPError as error:
        return False, error.read().decode()[:200]


def start(folder, seed):
    try:
        call("today_day", timeout=1)
        raise SystemExit(f"something is already answering on port {PORT}. Stop it, or set KILL_TEST_PORT")
    except (urllib.error.URLError, ConnectionError, OSError):
        pass
    env = dict(os.environ, STUDY_DB=os.path.join(folder, "study.sqlite"), STUDY_DIST=folder, STUDY_ADDRESS=f"127.0.0.1:{PORT}", STUDY_SEED="1" if seed else "0")
    process = subprocess.Popen([BIN], env=env, stdout=open(os.path.join(folder, "server.log"), "a"), stderr=subprocess.STDOUT)
    for _ in range(600):
        if process.poll() is not None:
            break
        try:
            if call("today_day")[0]:
                return process
        except Exception:
            time.sleep(0.05)
    raise SystemExit("the server didn't start:\n" + open(os.path.join(folder, "server.log")).read()[-800:])


def checks(database):
    """SQLite's own checks, as a list of problems."""
    connection = sqlite3.connect(database)
    problems = []
    integrity = connection.execute("PRAGMA integrity_check").fetchall()
    if integrity != [("ok",)]:
        problems.append(("integrity_check", integrity[:3]))
    links = connection.execute("PRAGMA foreign_key_check").fetchall()
    if links:
        problems.append(("foreign_key_check", links[:3]))
    connection.close()
    return problems


INVARIANTS = {
    "a question without all 4 options": "SELECT q.id FROM questions q WHERE q.text LIKE 'Q r%' AND (SELECT COUNT(*) FROM options o WHERE o.question_id = q.id) <> 4",
    "a question without both tags": "SELECT q.id FROM questions q WHERE q.text LIKE 'Q r%' AND (SELECT COUNT(*) FROM question_tags o WHERE o.question_id = q.id) <> 2",
    "a question without its citation": "SELECT q.id FROM questions q WHERE q.text LIKE 'Q r%' AND (SELECT COUNT(*) FROM citations o WHERE o.question_id = q.id) <> 1",
    "a cloze note without its 3 cards": "SELECT n.id FROM notes n JOIN topics t ON t.id = n.topic_id WHERE t.name LIKE 'r%' AND (SELECT COUNT(*) FROM cards c WHERE c.note_id = n.id) <> 3",
    "a topic with only some of its tags": "SELECT t.id FROM topics t WHERE t.name LIKE 'r%' AND (SELECT COUNT(*) FROM topic_tags g WHERE g.topic_id = t.id) IN (1, 2)",
    "a checklist with only some of its points": "SELECT c.id FROM recall_checklists c JOIN topics t ON t.id = c.topic_id WHERE t.name LIKE 'r%' AND (SELECT COUNT(*) FROM recall_points p WHERE p.checklist_id = c.id) BETWEEN 1 AND 5",
    "a card review without the card being updated": "SELECT r.id FROM card_reviews r JOIN cards c ON c.id = r.card_id WHERE c.last_review IS NULL",
    "a row without its ID or change stamp": "SELECT id FROM topics WHERE uid IS NULL OR changed IS NULL",
}


def kill_while_saving(rounds, seed):
    folder = tempfile.mkdtemp(prefix="study-kill-")
    random.seed(seed)
    acknowledged, problems = [], []
    process = start(folder, True)
    for round_number in range(rounds):
        stop, local = threading.Event(), []

        def writer():
            n = 0
            while not stop.is_set():
                n += 1
                name = f"r{round_number}-t{n}"
                try:
                    ok, topic = call("create_topic", {"name": name, "module": None})
                    if not ok:
                        continue
                    ok1, _ = call("save_question", {"input": {"topic": topic, "text": f"Q {name}", "multiple": True, "options": [{"text": "a", "correct": True}, {"text": "b", "correct": True}, {"text": "c", "correct": False}, {"text": "d", "correct": False}], "explanation": "e", "tags": ["k1", "k2"], "citations": [{"source": 1, "location": "p1"}], "short": None}})
                    ok2, _ = call("save_note", {"id": None, "input": {"note": {"topic": topic, "kind": "Cloze", "front": "", "back": "", "cues": [], "text": "a {{c1::b}} {{c2::c}} {{c3::d}}", "extra": ""}, "suggestion": None}})
                    ok3, _ = call("set_topic_tags", {"topic": topic, "tags": ["x", "y", "z"]})
                    if ok1 and ok2 and ok3:
                        local.append(name)
                    ok, checklist = call("create_checklist", {"subject": {"Topic": topic}})
                    if ok:
                        call("save_points", {"id": checklist, "points": [{"text": f"p{i}", "question": None, "note": None} for i in range(6)]})
                        call("finish_check", {"id": checklist, "dump": "d", "recalled": [0, 1, 2]})
                    ok, _ = call("start_quiz", {"setup": {"topics": [topic], "count": None, "even": False, "shuffle_questions": False, "shuffle_options": False, "exam": False, "total_minutes": None, "per_question_seconds": None, "easy": False, "types": "Both"}})
                    if ok:
                        call("answer", {"picked": [0, 1]})
                        call("quit_quiz")
                    call("start_review", {"scope": {"Topic": topic}})
                    for _ in range(3):
                        call("grade_card", {"rating": "Good", "seconds": 3})
                    if n % 11 == 0:
                        call("back_up_now")
                except Exception:
                    pass

        thread = threading.Thread(target=writer)
        thread.start()
        time.sleep(random.uniform(0.05, 0.6))
        process.send_signal(signal.SIGKILL)
        process.wait()
        stop.set()
        thread.join()
        acknowledged += local
        process = start(folder, False)
        database = os.path.join(folder, "study.sqlite")
        problems += [(round_number, *problem) for problem in checks(database)]
        connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True)
        names = {row[0] for row in connection.execute("SELECT name FROM topics")}
        lost = [name for name in acknowledged if name not in names]
        if lost:
            problems.append((round_number, "saves the app said were done are gone", lost[:5]))
            acknowledged = [name for name in acknowledged if name in names]
        for what, sql in INVARIANTS.items():
            rows = connection.execute(sql).fetchall()
            if rows:
                problems.append((round_number, what, rows[:3]))
        connection.close()
    # Half-written backups the kills left behind are cleared by a later backup, once they
    # are over an hour old. Age them, back up once more, and none may be left.
    backups = os.path.join(folder, "backups")
    long_ago = time.time() - 3 * 60 * 60
    for name in os.listdir(backups) if os.path.isdir(backups) else []:
        if ".partial" in name:
            os.utime(os.path.join(backups, name), (long_ago, long_ago))
    call("back_up_now")
    process.kill()
    process.wait()
    leftovers = [name for name in (os.listdir(backups) if os.path.isdir(backups) else []) if ".partial" in name]
    if leftovers:
        problems.append((rounds, "half-written backup files left behind", leftovers[:3]))
    shutil.rmtree(folder, ignore_errors=True)
    print(f"kill while saving: {rounds} kills, {len(acknowledged)} acknowledged saves checked")
    return problems


def kill_while_upgrading(attempts):
    old = tempfile.mkdtemp(prefix="study-old-")
    made = subprocess.run(["cargo", "test", "--release", "-p", "study-store", "write_old_databases", "--", "--ignored"], cwd=ROOT, env=dict(os.environ, OLD_DATABASES=old, OLD_ROWS="1500"), capture_output=True, text=True)
    if made.returncode != 0:
        raise SystemExit("couldn't make the older databases:\n" + made.stdout[-600:] + made.stderr[-600:])
    random.seed(3)
    problems, kills, mid = [], 0, 0
    for name in sorted(os.listdir(old)):
        for _ in range(attempts):
            folder = tempfile.mkdtemp(prefix="study-upgrade-")
            database = os.path.join(folder, "study.sqlite")
            shutil.copy(os.path.join(old, name), database)
            connection = sqlite3.connect(database)
            before = {table: connection.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0] for (table,) in connection.execute("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'").fetchall()}
            connection.close()
            env = dict(os.environ, STUDY_DB=database, STUDY_DIST=folder, STUDY_ADDRESS=f"127.0.0.1:{PORT}", STUDY_SEED="0")
            for _ in range(5):
                process = subprocess.Popen([BIN], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                time.sleep(random.uniform(0.0, 0.1))
                process.send_signal(signal.SIGKILL)
                process.wait()
                kills += 1
                mid += os.path.exists(database + "-journal")
                problems += [(name, *problem) for problem in checks(database)]
            process = start(folder, False)
            process.kill()
            process.wait()
            connection = sqlite3.connect(database)
            for table, count in before.items():
                try:
                    now = connection.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0]
                except sqlite3.OperationalError:
                    continue  # a table a later version replaced
                if now < count:
                    problems.append((name, f"{table} lost rows in the upgrade", (count, now)))
            connection.close()
            backups = os.path.join(folder, "backups")
            copies = [file for file in (os.listdir(backups) if os.path.isdir(backups) else []) if file.endswith(".sqlite")]
            if not any("before-update" in file for file in copies):
                problems.append((name, "no backup was made before the upgrade", copies[:3]))
            for file in copies:
                problems += [(name, f"backup {file}: {problem[0]}", problem[1]) for problem in checks(os.path.join(backups, file))]
            shutil.rmtree(folder, ignore_errors=True)
    shutil.rmtree(old, ignore_errors=True)
    print(f"kill while upgrading: {kills} kills, {mid} landed with a change half-written")
    return problems


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--rounds", type=int, default=40)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--upgrade-attempts", type=int, default=2)
    parser.add_argument("--skip-upgrade", action="store_true")
    options = parser.parse_args()
    if not os.path.exists(BIN):
        raise SystemExit(f"build the server first: cargo build --release -p study-web ({BIN} isn't there)")
    found = kill_while_saving(options.rounds, options.seed)
    if not options.skip_upgrade:
        found += kill_while_upgrading(options.upgrade_attempts)
    seen = {}
    for problem in found:
        seen.setdefault(problem[1], []).append(problem)
    for what, examples in seen.items():
        print(f"PROBLEM [{len(examples)}x] {what}: {examples[0]}")
    sys.exit(1 if found else 0)

"""End-to-end: sync in the real app window (sync.feature, R119 to R128).

The app in the window is Desktop. A browser-version server (study-web) on its own database
is Laptop, and both use the same folder, the way two computers share a Syncthing folder.
Sets up sync in the window, makes changes on each side, syncs, and checks they arrive,
then edits the same card on both and settles it from the things-to-check panel.
Screenshots go to $SHOTS if it is set. Run with e2e/run.sh."""
import json, os, pathlib, shutil, subprocess, tempfile, time, urllib.request
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
web = os.environ.get("WEB", str(root / "target/debug/study-web"))
shots = os.environ.get("SHOTS")
audit = os.environ.get("AUDIT")
port = 7431

work = pathlib.Path(tempfile.mkdtemp(prefix="study-sync-e2e-"))
folder = work / "Sync"
folder.mkdir()
laptop = subprocess.Popen([web], env={**os.environ, "STUDY_DB": str(work / "laptop.sqlite"), "STUDY_ADDRESS": f"127.0.0.1:{port}", "STUDY_SEED": "0", "STUDY_DIST": str(root / "dist")}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

options = ArgOptions()
options.set_capability("tauri:options", {"application": app})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
driver.set_script_timeout(30)
wait = WebDriverWait(driver, 15)
checks = 0
findings = []


def ok(name: str) -> None:
    global checks
    checks += 1
    print(f"  ok  {name}")


def shot(name: str) -> None:
    time.sleep(0.9)
    if shots:
        driver.save_screenshot(os.path.join(shots, f"sync-{name}.png"))
    if audit:
        for theme in ["studyapp-dark", "studyapp"]:
            driver.execute_script("document.documentElement.dataset.theme = arguments[0]", theme)
            time.sleep(0.2)
            result = driver.execute_script(open(audit).read())
            if result["fails"] or result["small"] or result["primaries"] > 1:
                findings.append(f"{name} ({theme}): {result}")
        driver.execute_script("document.documentElement.dataset.theme = 'studyapp-dark'")


def find(xpath: str):
    return wait.until(lambda _: driver.find_element(By.XPATH, xpath))


def ipc(command: str, args: dict):
    result = driver.execute_async_script(
        "const [command, args, done] = arguments;"
        "window.__TAURI_INTERNALS__.invoke(command, args).then((out) => done({ ok: out }), (error) => done({ error }));",
        command, args,
    )
    assert "ok" in result, result
    return result["ok"]


def on_laptop(command: str, args: dict | None = None):
    request = urllib.request.Request(f"http://127.0.0.1:{port}/api/{command}", data=json.dumps(args or {}).encode(), headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.loads(response.read() or b"null")


def nav(name: str) -> None:
    find(f"//nav//a[.//span[normalize-space()='{name}'] or normalize-space()='{name}']").click()


try:
    driver.set_window_size(1440, 900)
    for _ in range(50):
        try:
            on_laptop("topics")
            break
        except OSError:
            time.sleep(0.2)
    module = ipc("create_module", {"name": "Computer Architecture"})
    topic = ipc("create_topic", {"name": "Caches", "module": module})
    ipc("save_note", {"id": None, "input": {"note": {"topic": topic, "kind": "Basic", "front": "Cache latencies", "back": "About 3 cycles at L1, 10 at L2", "cues": [], "text": "", "extra": ""}, "suggestion": None}})

    # ---------- Set up sync on Desktop ----------
    driver.execute_script("location.hash = '#data'")  # Your data is under Settings now (R158)
    find("//*[@role='status'][contains(., 'Sync is off')]")
    find("//section[@aria-labelledby='sync-h']//button[normalize-space()='Set up sync']").click()
    dialog = find("//dialog[@open]")
    shot("setup")
    dialog.find_element(By.XPATH, ".//input[@placeholder]").send_keys(str(folder))
    name = dialog.find_element(By.XPATH, ".//label[.//span[normalize-space()='Name this computer']]//input")
    name.send_keys(Keys.CONTROL, "a")
    name.send_keys("Desktop")
    dialog.find_element(By.XPATH, ".//button[normalize-space()='Start syncing']").click()
    find("//table[@aria-label='Computers']//td[contains(., 'Desktop')]")
    assert any(entry.name.endswith(".jsonl") for entry in (folder / "changes").rglob("*")), list(folder.rglob("*"))
    assert not list(folder.rglob("*.sqlite")), "no database in the folder"
    ok("sync is set up in three steps, and only change files go in the folder (R119, R120)")

    # ---------- Laptop joins and gets everything ----------
    on_laptop("sync_setup", {"setup": {"folder": str(folder), "name": "Laptop"}})
    names = [row["name"] for row in on_laptop("topics")]
    assert "Caches" in names and len(names) == len(ipc("topics", {})), names
    ok("the other computer picks the same folder and gets everything (R120)")

    # ---------- A change on Laptop arrives on Desktop ----------
    laptop_topic = next(row["id"] for row in on_laptop("topics") if row["name"] == "Caches")
    on_laptop("save_question", {"input": {"topic": laptop_topic, "text": "Which cache is fastest?", "multiple": False,
        "options": [{"text": "L1", "correct": True}, {"text": "L2", "correct": False}, {"text": "L3", "correct": False}], "explanation": "", "tags": [], "citations": [], "short": None}})
    on_laptop("sync_now", {"auto": False})
    find("//button[normalize-space()='Sync now']").click()
    find("//*[@role='status'][contains(., 'Synced just now')]")
    rows = driver.find_elements(By.XPATH, "//table[@aria-label='Computers']//tbody/tr")
    assert "Laptop" in rows[1].text, [row.text for row in rows]
    assert [text for _, text in ipc("questions_in", {"topic": topic})] == ["Which cache is fastest?"]
    side = find("//nav//a[@href='#data'][contains(., 'Synced')]")
    ok(f"Sync now brings Laptop's question over, and the sidebar says '{side.text}' (R121, R122)")
    shot("on")

    # ---------- The same card edited on both ----------
    note = ipc("topic_cards", {"topic": topic})["cards"][0]["note"]
    ipc("save_note", {"id": note, "input": {"note": {"topic": topic, "kind": "Basic", "front": "Cache latencies", "back": "About 3 cycles at L1, 11 at L2", "cues": [], "text": "", "extra": ""}, "suggestion": None}})
    time.sleep(0.05)
    laptop_note = on_laptop("topic_cards", {"topic": laptop_topic})["cards"][0]["note"]
    on_laptop("save_note", {"id": laptop_note, "input": {"note": {"topic": laptop_topic, "kind": "Basic", "front": "Cache latencies", "back": "About 4 cycles at L1, 12 at L2", "cues": [], "text": "", "extra": ""}, "suggestion": None}})
    find("//button[normalize-space()='Sync now']").click()
    time.sleep(0.5)
    on_laptop("sync_now", {"auto": False})
    find("//button[normalize-space()='Sync now']").click()
    panel = find("//*[@role='region'][@aria-labelledby='checks-h']")
    assert "1 thing to check" in panel.text and "Card in Cache latencies · Caches · Computer Architecture" in panel.text, panel.text
    assert panel.find_elements(By.XPATH, ".//ins[normalize-space()='4']") and panel.find_elements(By.XPATH, ".//del[normalize-space()='3']"), panel.get_attribute("innerHTML")
    ok("the same field changed on both is listed once, newer in use, with what differs marked (R123)")
    driver.execute_script("arguments[0].scrollIntoView({ block: 'center' })", panel)
    shot("check")
    panel.find_element(By.XPATH, ".//button[normalize-space()=\"Use Desktop's instead\"]").click()
    wait.until(lambda _: not driver.find_elements(By.XPATH, "//*[@aria-labelledby='checks-h']"))
    on_laptop("sync_now", {"auto": False})
    back = on_laptop("note_view", {"id": laptop_note})
    assert "11 at L2" in json.dumps(back), back
    assert not on_laptop("sync_to_check"), "settled on Laptop too"
    ok("putting Desktop's version back reaches Laptop and settles it there too (R123)")

    # ---------- The folder goes missing ----------
    shutil.move(str(folder), str(work / "unplugged"))
    ipc("create_topic", {"name": "Made while unplugged", "module": None})
    find("//button[normalize-space()='Sync now']").click()
    status = find("//*[@role='status'][contains(., \"can't find the sync folder\")]")
    primaries = driver.find_elements(By.XPATH, "//main//button[contains(@class, 'btn-primary')]")
    assert [button.text.lower() for button in primaries] == ["change folder"], [button.text for button in primaries]
    find("//nav//a[@href='#data'][normalize-space()='Sync paused']")
    ok("a missing folder is said in words (here and in the sidebar), changes are counted, and Change folder is the one primary (R127)")
    shot("missing")
    shutil.move(str(work / "unplugged"), str(folder))
    find("//button[normalize-space()='Sync now']").click()
    find("//*[@role='status'][contains(., 'Synced just now')]")
    on_laptop("sync_now", {"auto": False})
    assert "Made while unplugged" in [row["name"] for row in on_laptop("topics")]
    ok("once the folder is back, what was waiting goes out (R127)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on sync in the running app")
finally:
    driver.quit()
    laptop.terminate()
    laptop.wait(timeout=10)
    shutil.rmtree(work, ignore_errors=True)

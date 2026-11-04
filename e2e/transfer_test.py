"""End-to-end: export and import in the real app window (transfer.feature, R112 to R118).

Shares a topic to a file in Downloads, deletes the topic, imports the file back and checks
the topic and its question return, then imports it again and is told there's nothing new.
Screenshots go to $SHOTS if it is set. Run with e2e/run.sh."""
import os, pathlib, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
shots = os.environ.get("SHOTS")
audit = os.environ.get("AUDIT")

options = ArgOptions()
options.set_capability("tauri:options", {"application": app})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
# The app runs on this machine, so a file input takes a local path as it is.
from selenium.webdriver.remote.file_detector import UselessFileDetector
driver.file_detector = UselessFileDetector()
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
        driver.save_screenshot(os.path.join(shots, f"transfer-{name}.png"))
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


def nav(name: str) -> None:
    find(f"//nav//a[.//span[normalize-space()='{name}'] or normalize-space()='{name}']").click()


saved = None
try:
    driver.set_window_size(1440, 900)
    topic = ipc("create_topic", {"name": "Shared on purpose", "module": None})
    ipc("save_question", {"input": {"topic": topic, "text": "Which layer adds port numbers?", "multiple": False,
        "options": [{"text": "Transport", "correct": True}, {"text": "Network", "correct": False}, {"text": "Session", "correct": False}],
        "explanation": "", "tags": [], "citations": [], "short": None}})

    # ---------- Export ----------
    nav("Your data")
    find("//h1[normalize-space()='Your data']")
    find("//button[starts-with(normalize-space(), 'Export')]").click()
    dialog = find("//dialog[@open]")
    dialog.find_element(By.XPATH, ".//label[.//span[normalize-space()='Some topics, to share']]//input").click()
    find("//dialog[@open]//input[@type='search']").send_keys("Shared on")
    find("//dialog[@open]//input[@aria-label='Shared on purpose']").click()
    wait.until(lambda _: "Saves Shared on purpose.studyapp: 1 topic and 1 question." in dialog.text)
    assert not find("//dialog[@open]//label[.//span[normalize-space()='Include my history']]//input").is_selected()
    ok("sharing a topic says what goes in, with history left out (R113)")
    shot("export")
    find("//dialog[@open]//button[normalize-space()='Export 1 topic']").click()
    notice = find("//*[@role='alert' and starts-with(normalize-space(), 'Saved Shared on purpose')]")
    saved = notice.text.split(" to ", 1)[1].rstrip(".")
    assert os.path.isfile(saved) and saved.endswith(".studyapp"), saved
    ok("the file is saved to Downloads and the app says where (R112)")

    # ---------- Import it back ----------
    ipc("delete_topic", {"topic": topic})
    driver.find_element(By.XPATH, "//input[@aria-label='Import a .studyapp file']").send_keys(saved)
    preview = find("//dialog[@open]")
    wait.until(lambda _: "Import Shared on purpose" in preview.text)
    assert "from this computer" in preview.text, preview.text
    cells = [[cell.text for cell in row.find_elements(By.TAG_NAME, "td")] for row in preview.find_elements(By.XPATH, ".//table/tbody/tr")]
    assert ["Topics", "1", "0", "0"] in cells and ["Questions", "1", "0", "0"] in cells, cells
    ok("the preview says what's new before anything changes (R114)")
    shot("import")
    preview.find_element(By.XPATH, ".//button[normalize-space()='Import']").click()
    done = find("//*[@role='alert' and starts-with(normalize-space(), 'Imported')]")
    assert "1 topic and 1 question" in done.text and "restore the backup from today" in done.text, done.text
    names = [row["name"] for row in ipc("topics", {})]
    assert "Shared on purpose" in names, names
    ok("importing brings the topic and its question back, with a backup first (R114)")

    # ---------- Again ----------
    driver.find_element(By.XPATH, "//input[@aria-label='Import a .studyapp file']").send_keys(saved)
    find("//dialog[@open]//*[normalize-space()='Everything in this file is here already.']")
    find("//dialog[@open]//button[normalize-space()='Close']").click()
    assert [row["name"] for row in ipc("topics", {})].count("Shared on purpose") == 1
    ok("the same file again adds nothing and makes no copies (R115)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on export and import in the running app")
finally:
    if saved and os.path.isfile(saved):
        os.remove(saved)
    driver.quit()

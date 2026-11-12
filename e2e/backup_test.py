"""End-to-end: backups in the real app window (backup.feature, R101 to R108).

Adds a topic, checks the day's backup is made when moving between screens and the sidebar
says so, backs up by hand, restores that backup over a later change, and checks the
backup files on disk. Screenshots go to $SHOTS if it is set. Run with e2e/run.sh."""
import os, pathlib, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
shots = os.environ.get("SHOTS")
audit = os.environ.get("AUDIT")
folder = pathlib.Path(os.environ["XDG_DATA_HOME"]) / "com.arthurchenevans.studyapp" / "backups"

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
        driver.save_screenshot(os.path.join(shots, f"backup-{name}.png"))
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


def rows():
    return driver.find_elements(By.XPATH, "//table[@aria-label='Backups']/tbody/tr[td[3]]")


try:
    driver.set_window_size(1440, 900)
    ipc("create_topic", {"name": "Kept by the backup", "module": None})

    # ---------- The day's backup, and the sidebar line ----------
    nav("Topics")
    line = find("//nav//a[@href='#data' and normalize-space()='Backed up today']")
    ok("moving between screens makes the day's backup, and the sidebar says so (R102, R108)")
    line.click()
    find("//h1[normalize-space()='Your data']")
    status = find("//*[@role='status']")
    assert "Backed up today at" in status.text, status.text
    assert any("DAILY" in row.text.upper() for row in rows()), [row.text for row in rows()]
    ok("Your data leads with when the last backup was, and lists it (R104)")
    shot("your-data")

    # ---------- By hand ----------
    find("//button[contains(normalize-space(), 'Back up now')]").click()
    wait.until(lambda _: "BY HAND" in rows()[0].text.upper())
    names = sorted(path.name for path in folder.glob("study *.sqlite"))
    assert any(name.endswith("by-hand.sqlite") for name in names) and any(name.endswith("daily.sqlite") for name in names), names
    assert not list(folder.glob("*.partial")), "nothing half-written"
    ok("Back up now adds a backup by hand, and both files are on disk (R101, R104)")

    # ---------- Restore ----------
    ipc("create_topic", {"name": "Made after the backup", "module": None})
    rows()[0].find_element(By.XPATH, ".//button[contains(., 'Restore')]").click()
    dialog = find("//dialog[@open]")
    wait.until(lambda _: "topic added" in dialog.text)
    assert "1 topic added" in dialog.text and "topics added" not in dialog.text, dialog.text
    assert driver.switch_to.active_element.text.strip().lower() == "cancel", "Cancel has the focus"
    ok("restoring says what it undoes and starts on Cancel (R105)")
    shot("restore")
    dialog.find_element(By.XPATH, ".//button[contains(., 'Restore today')]").click()
    notice = find("//*[@role='alert' and starts-with(normalize-space(), 'Restored the backup')]")
    assert "restore that to undo this" in notice.text, notice.text
    wait.until(lambda _: "BEFORE RESTORE" in rows()[0].text.upper())
    names = [topic["name"] for topic in ipc("topics", {})]
    assert "Kept by the backup" in names and "Made after the backup" not in names, names
    ok("the restore backs up first, then puts the data back (R105)")
    shot("restored")

    # ---------- Settings ----------
    find("//button[@aria-label='Change how many are kept']").click()
    days = find("//dialog[@open]//label[.//span[normalize-space()='Days']]//input")
    days.clear()
    days.send_keys("14")
    find("//dialog[@open]//button[normalize-space()='Save']").click()
    find("//*[normalize-space()='The last 14 days, 4 weeks and 12 months']")
    assert '"keep_days": 14' in (folder / "settings.json").read_text()
    ok("how many are kept is saved beside the backups, not in the database (R103, R107)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on backups in the running app")
finally:
    driver.quit()

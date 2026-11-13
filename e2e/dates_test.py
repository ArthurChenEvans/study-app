"""End-to-end: dates on the calendar in the real app window (dates.feature).

Adds an exam covering a module from the calendar, checks the preview, the chip and
"Coming up", opens the exam's day and its panel, edits it, and deletes it after asking.
Screenshots go to $SHOTS if it is set. Run with e2e/run.sh."""
import os, pathlib, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
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
        driver.save_screenshot(os.path.join(shots, f"dates-{name}.png"))
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


def button(name: str):
    return find(f"//button[starts-with(normalize-space(), '{name}') or @aria-label='{name}']")


def labelled(name: str):
    return find(f"//*[@aria-label='{name}' or @id=//label[starts-with(normalize-space(), '{name}')]/@for]")


def ipc(command: str, args: dict):
    result = driver.execute_async_script(
        "const [command, args, done] = arguments;"
        "window.__TAURI_INTERNALS__.invoke(command, args).then((out) => done({ ok: out }), (error) => done({ error }));",
        command, args,
    )
    assert "ok" in result, result
    return result["ok"]


def text() -> str:
    return driver.find_element(By.TAG_NAME, "main").text


def nav(name: str) -> None:
    find(f"//nav//a[.//span[normalize-space()='{name}'] or normalize-space()='{name}']").click()


from selenium.webdriver.support.ui import Select


def content(selector: str) -> str:
    """The text of the first match, read in the page (WebKitWebDriver can't read a dialog's text)."""
    return driver.execute_script("const node = document.querySelector(arguments[0]); return node ? node.textContent.replace(/\\s+/g, ' ') : ''", selector)


try:
    driver.set_window_size(1440, 900)
    module = ipc("create_module", {"name": "Algorithms"})
    ipc("create_topic", {"name": "Sorting", "module": module})
    ipc("create_topic", {"name": "Graphs", "module": module})

    # ---------- Adding a date ----------
    nav("Calendar")
    find("//h1[normalize-space()='Calendar']")
    button("Add a date").click()
    find("//dialog//label[starts-with(normalize-space(), 'Name')]//input").send_keys("Algorithms exam")
    find("//dialog//label[starts-with(normalize-space(), 'Kind')]//input").send_keys("Exam")
    find("//dialog//button[@role='radio' and normalize-space()='A module']").click()
    Select(find("//dialog//select")).select_by_visible_text("Algorithms (2 topics)")
    tick = find("//dialog//label[.//span[normalize-space()='Tighten reviews before this']]//input")
    assert tick.is_selected(), "an exam tightens reviews unless told otherwise"
    wait.until(lambda _: "Graphs hasn't been quizzed yet" in content("dialog"))
    ok("an exam covering a module ticks tightening and previews what moves (R132, R136)")
    shot("add")
    button("Add date").click()
    coming = lambda: content("section[aria-label='Coming up']")
    wait.until(lambda _: "Algorithms exam" in coming() and "in 14 days" in coming() and "Reviews tightened" in coming())
    find("//*[contains(@class, 'date-chip') and contains(normalize-space(), 'Algorithms exam')]")
    ok("the date shows as a chip on its day and in Coming up (R130, R134)")
    shot("month")

    # ---------- Its day ----------
    find("//section[@aria-label='Coming up']//button[contains(normalize-space(), 'Algorithms exam')]").click()
    find("//section[.//h2[normalize-space()='Algorithms exam']]")
    panel = lambda: content("section[aria-labelledby^='h-date-']")
    wait.until(lambda _: "Algorithms module" in panel() and "in 14 days" in panel() and "Not quizzed yet, so nothing to tighten" in panel())
    ok("its day has its own panel with the countdown and the never-quizzed topics (R135)")
    shot("day")

    # ---------- Editing and deleting ----------
    find("//section[.//h2[normalize-space()='Algorithms exam']]//button[normalize-space()='Edit']").click()
    name = find("//dialog//label[starts-with(normalize-space(), 'Name')]//input")
    name.clear()
    name.send_keys("Algorithms final")
    button("Save").click()
    find("//section//h2[normalize-space()='Algorithms final']")
    find("//section[.//h2[normalize-space()='Algorithms final']]//button[normalize-space()='Edit']").click()
    find("//dialog//button[normalize-space()='Delete']").click()
    find("//dialog//*[contains(normalize-space(), 'Anything it tightened goes back to its normal schedule.')]")
    button("Delete date").click()
    wait.until(lambda _: not driver.find_elements(By.XPATH, "//section//h2[normalize-space()='Algorithms final']"))
    # toasts.feature (R157): it goes from the calendar at once, and for real once Undo has passed
    find("//*[@role='status']//*[contains(normalize-space(), 'Date deleted')]")
    WebDriverWait(driver, 15).until(lambda _: ipc("dates", {}) == [])
    ok("a date is edited, and deleted only after asking, with Undo (R130, R157)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on dates on the calendar in the running app")
finally:
    driver.quit()

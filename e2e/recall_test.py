"""End-to-end: free recall in the real app window (free-recall.feature).

Creates a module and topic, makes a checklist about the module through the UI, writes
points and adds one from an MCQ, runs a typed recall, ticks under the pass mark, and checks
the result, the schedule and the history. Screenshots go to $SHOTS if it is set.
Run with e2e/run.sh."""
import os, pathlib, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
shots = os.environ.get("SHOTS")

options = ArgOptions()
options.set_capability("tauri:options", {"application": app})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
driver.set_script_timeout(30)
wait = WebDriverWait(driver, 15)
checks = 0


def ok(name: str) -> None:
    global checks
    checks += 1
    print(f"  ok  {name}")


audit = os.environ.get("AUDIT")  # path to a JS file whose body returns { fails, small, primaries }
findings = []


def shot(name: str) -> None:
    time.sleep(0.9)  # let entry animations finish before measuring or capturing
    if shots:
        driver.save_screenshot(os.path.join(shots, f"app-{name}.png"))
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


try:
    driver.set_window_size(1440, 900)
    module = ipc("create_module", {"name": "Computer Architecture"})
    topic = ipc("create_topic", {"name": "CPU", "module": module})
    ipc("save_question", {"input": {"topic": topic, "text": "Which stage reads registers?", "multiple": False,
        "options": [{"text": "Fetch", "correct": False}, {"text": "Decode", "correct": True}, {"text": "Execute", "correct": False}],
        "explanation": "", "tags": [], "citations": []}})

    driver.execute_script("location.hash = '#recall'")
    button("Create your first checklist").click()
    find("//select").send_keys("Computer Architecture")
    button("Create checklist").click()
    find("//h1[normalize-space()='Computer Architecture']")
    ok("checklist made about a module, opened in the editor (R52)")

    for point in ["Five pipeline stages", "Cache levels L1 to L3", "Amdahl's law: $S = \\frac{1}{(1-p)+p/n}$"]:
        box = find("//input[@aria-label='New point']")
        box.send_keys(point)
        box.send_keys(Keys.ENTER)
        wait.until(lambda _: point.split(":")[0] in text())
    button("Add from questions").click()
    find("//dialog//input[@type='checkbox']").click()
    button("Add 1 point").click()
    wait.until(lambda _: "Which stage reads registers? → Decode" in text())
    assert "From MCQ" in text()
    assert driver.find_elements(By.CSS_SELECTOR, "[data-point] .katex"), "maths rendered in a point"
    ok("points written, maths rendered, one added from an MCQ (R53, R54)")
    shot("checklist")

    button("Start recall").click()
    dump = find("//textarea[@aria-label='Your recall']")
    assert "Five pipeline stages" not in text(), "checklist hidden during the dump"
    dump.send_keys("Fetch decode execute memory write back. Registers are read in decode.")
    ok("dump typed with the checklist hidden (R55)")
    shot("dump")
    dump.send_keys(Keys.CONTROL, Keys.ENTER)

    boxes = wait.until(lambda _: driver.find_elements(By.XPATH, "//ol[@aria-label='Checklist']//*[@role='checkbox']"))
    assert len(boxes) == 4
    boxes[0].click()
    boxes[3].click()
    wait.until(lambda _: "50%" in find("//*[@aria-label='Score so far']").text)
    summary = find("//*[@aria-label='Score so far']").text
    assert "Under the pass mark" in summary and "Starts on Day" in summary, summary
    ok("live score and outcome while ticking (R56)")
    shot("check")
    driver.find_element(By.TAG_NAME, "body").send_keys(Keys.CONTROL, Keys.ENTER)

    wait.until(lambda _: "Started on Day" in text())
    assert "2 of 4 recalled" in text() and "Next check tomorrow" in text(), text()
    assert "Cache levels L1 to L3" in find("//ul[@aria-label='Missed points']").text
    ok("first check puts it on Day, due tomorrow, missed points listed (R57)")
    shot("result")

    button("Back to Recall").click()
    find("//*[normalize-space()='All caught up']")
    assert not driver.find_elements(By.XPATH, "//nav//*[contains(@aria-label, 'due today')]"), "no due count once done"
    ok("nothing due afterwards, sidebar count gone (R58)")
    shot("list")

    find("//table[@aria-label='Checklists']//button").click()
    find("//button[@role='tab' and normalize-space()='History']").click()
    row = find("//table[@aria-label='Checks']//tbody/tr[1]").text
    assert "50%" in row and "Started on Day" in row, row
    find("//table[@aria-label='Checks']//button[normalize-space()='View dump']").click()
    assert "Registers are read in decode." in text()
    ok("history shows the check and its typed dump (R59)")
    shot("history")
    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on free recall in the running app")
finally:
    driver.quit()

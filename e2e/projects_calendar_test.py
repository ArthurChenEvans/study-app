"""End-to-end: projects, drills, quiz scheduling and the calendar in the real app window
(projects.feature, calendar.feature).

Makes a project, runs a session logging two struggles, reviews them into a card suggestion
and a drill, checks the issue is shared, does the drill from the calendar, and checks a
quiz starts its topic's schedule. Screenshots go to $SHOTS if it is set. Run with e2e/run.sh."""
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
        driver.save_screenshot(os.path.join(shots, f"practice-{name}.png"))
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


try:
    driver.set_window_size(1440, 900)
    today = ipc("today_day", {})

    # ---------- A project and a session ----------
    nav("Projects")
    find("//h1[normalize-space()='Projects']")
    shot("first-time")
    button("New project").click()
    find("//dialog//label[.//span[normalize-space()='Name']]//input").send_keys("Study app")
    find("//dialog//input[@list='project-kinds']").send_keys("Coding")
    button("Create project").click()
    find("//h1[normalize-space()='Study app']")
    button("Start a session").click()
    timer = find("//*[@role='timer' and @aria-label='Session time']")
    what = labelled("What tripped you up")
    assert driver.switch_to.active_element == what, "focus goes to the log"
    ok("a project with a running session and its timer (R87, R88)")

    what.send_keys("Forgot how to slice a string from the end")
    find("//*[@id='struggle-why']").send_keys("I was thinking in Python's negative indexes")
    find("//button[@role='radio' and normalize-space()=\"Couldn't recall it\"]").click()
    labelled("What tripped you up").send_keys(Keys.CONTROL, Keys.ENTER)
    wait.until(lambda _: "Forgot how to slice" in find("//ul[starts-with(@aria-label, 'Struggles on')]").text)
    driver.find_element(By.TAG_NAME, "body").send_keys(Keys.CONTROL, "l")
    assert driver.switch_to.active_element == labelled("What tripped you up"), "Ctrl+L focuses the log"
    labelled("What tripped you up").send_keys("Loop ran one too many times reading the file")
    find("//*[@id='struggle-why']").send_keys("Used <= out of habit")
    find("//button[@role='radio' and normalize-space()=\"Couldn't apply it\"]").click()
    button("Log it").click()
    wait.until(lambda _: "Loop ran one too many times" in text())
    assert text().count("To review at the end of this session") == 2, text()
    ok("two struggles logged with Ctrl+Enter and the button, Ctrl+L focuses the log (R89)")
    shot("session")

    # ---------- Session review ----------
    button("End session and review").click()
    find("//*[contains(., 'Struggle') and contains(., 'of 2')]")
    assert find("//input[@type='radio' and @name='action']/ancestor::label[contains(., 'Suggest a card')]//input").is_selected()
    shot("review-card")
    button("Next struggle").click()
    wait.until(lambda _: "Loop ran one too many times" in text())
    labelled("Issue").send_keys("Off-by-one in loops")
    labelled("The drill").send_keys("Write a loop over a slice without an off-by-one")
    labelled("Hints").send_keys("0..len")
    shot("review-drill")
    button("Schedule the drill").click()
    wait.until(lambda _: "Tomorrow" in find("//dialog[@open]").text)
    shot("review-when")
    button("Schedule for tomorrow").click()
    done = find("//ul[@aria-label='What happens next']")
    assert "1 card suggestion" in done.text and 'Drill for "Off-by-one in loops" tomorrow' in done.text, done.text
    ok("the review makes a card suggestion and schedules a drill for tomorrow (R90, R92, R95)")
    shot("review-done")
    button("Back to Study app").click()
    wait.until(lambda _: "Card suggested" in text() and "Issue: Off-by-one in loops · drill tomorrow" in text())
    ok("the project shows what became of each struggle")

    nav("Projects")
    find("//button[@role='tab' and contains(., 'Issues')]").click()
    issue = find("//table[@aria-label='Issues']//tr[contains(., 'Off-by-one in loops')]")
    assert "1 time in Study app" in issue.text and "Due Tomorrow" in issue.text, issue.text
    ok("the issue is listed with its drill (R94)")
    shot("issues")

    # ---------- The drill, from the calendar ----------
    drill = ipc("known_drill", {"issue": "off-by-one IN loops"})
    assert drill and drill["task"].startswith("Write a loop"), drill
    ipc("move_drill", {"id": drill["id"], "day": today})
    nav("Calendar")
    grid = find("//div[@role='grid']")
    today_cell = grid.find_element(By.XPATH, ".//button[@aria-current='date']")
    assert "1 drill" in today_cell.text, today_cell.text
    assert find("//nav//a[contains(., 'Calendar')]").text.strip().endswith("1") or "1" in find("//nav//a[contains(., 'Projects')]").text
    ok("the calendar shows today's drill, and the sidebar counts it (R98, R100)")
    shot("month")
    today_cell.click()
    find("//h1[starts-with(normalize-space(), 'Today,')]")
    shot("today")
    button("Start with the drills").click()
    find("//*[@role='heading' and @aria-level='1' and contains(., 'Write a loop over a slice')]")
    assert not driver.find_elements(By.XPATH, "//nav[@aria-label='Main']"), "the sidebar is hidden while drilling"
    shot("drill")
    driver.find_element(By.TAG_NAME, "body").send_keys("2")
    find("//h1[contains(., 'Again tomorrow')]")
    ok("a Shaky drill comes back tomorrow (R93)")
    shot("drill-shaky")
    button("Done").click()
    find("//h1[starts-with(normalize-space(), 'Today,')]")

    # ---------- Quiz scheduling ----------
    topic = ipc("create_topic", {"name": "Pipeline hazards", "module": None})
    for n in range(2):
        ipc("save_question", {"input": {"topic": topic, "text": f"Question {n}", "multiple": False,
            "options": [{"text": "Right", "correct": True}, {"text": "Wrong", "correct": False}, {"text": "Also wrong", "correct": False}],
            "explanation": "", "tags": [], "citations": [], "short": None}})
    nav("Topics")
    find("//tr[.//td[normalize-space()='Pipeline hazards']]//*[normalize-space()='Quiz it to start']")
    ipc("start_quiz", {"setup": {"topics": [topic], "count": None, "even": False, "shuffle_questions": False, "shuffle_options": False, "exam": True, "total_minutes": None, "per_question_seconds": None, "easy": False, "types": "Both"}})
    ipc("answer", {"picked": [0]})
    attempt = ipc("answer", {"picked": [0]})["next"]["Finished"]
    results = ipc("results", {"attempt": attempt})
    assert [(m["name"], m["passed"], m["to"]) for m in results["moves"]] == [("Pipeline hazards", True, "Day")], results["moves"]
    nav("Calendar")
    nav("Topics")
    row = find("//tr[.//td[normalize-space()='Pipeline hazards']]")
    wait.until(lambda _: "Tomorrow" in row.text and "Day step" in row.text)
    ok("a finished quiz starts the topic's quiz schedule, due tomorrow (R96, R97)")
    shot("topics")

    nav("Cards")
    find("//button[contains(normalize-space(), 'suggestion') or starts-with(normalize-space(), 'Look through them')]").click()
    find("//button[@role='radio' and starts-with(normalize-space(), 'Project struggles')]")
    assert "Forgot how to slice a string from the end" in text()
    ok("the struggle waits in Suggestions as a card suggestion (R95)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on projects, drills, quiz scheduling and the calendar in the running app")
finally:
    driver.quit()

"""End-to-end: short answer questions in the real app window (short-answer.feature).

Writes one question checked by accepted answers and one by keywords, tries answers in the
editor, takes a short-answer-only study quiz, overrules a verdict, then undoes it on the
results page, and checks the checklist import and card suggestions. Screenshots go to
$SHOTS if it is set. Run with e2e/run.sh."""
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
        driver.save_screenshot(os.path.join(shots, f"short-{name}.png"))
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


def radio(name: str):
    return find(f"//button[@role='radio' and normalize-space()='{name}']")


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


def status() -> str:
    return find("//*[@role='status']").text


try:
    driver.set_window_size(1440, 900)
    driver.execute_script("location.hash = '#topics'")
    button("New topic").click()
    find("//dialog//input").send_keys("OSI layers")
    button("Create topic").click()
    (find("//table//button[normalize-space()='OSI layers']").click(), find("//button[@aria-label='Edit OSI layers']"))[1].click()

    # ---------- Accepted answers ----------
    radio("Short answer").click()
    labelled("Question").send_keys("Which OSI layer gives each application its own port number?")
    labelled("Accepted answer 1").send_keys("Transport layer")
    button("Add another accepted answer").click()
    labelled("Accepted answer 2").send_keys("Layer 4")
    labelled("Model answer").send_keys("The **transport** layer (layer 4)")
    labelled("Explanation").send_keys("Ports let several apps share one address.")
    labelled("A test answer").send_keys("the transprot layer")
    tried = find("//section[@aria-label='Try an answer']")
    wait.until(lambda _: "Right, with a typo" in tried.text)
    assert 'matches "Transport layer"' in tried.text, tried.text
    ok("try an answer: a typo is allowed and says which answer it matched (R80, R82)")
    shot("editor-accepted")
    button("Save question").click()
    wait.until(lambda _: "Question saved" in text())

    # ---------- Keywords ----------
    assert find("//button[@role='radio' and normalize-space()='Short answer']").get_attribute("aria-checked") == "true", "stays on short answer"
    labelled("Question").send_keys("What three messages open a TCP connection, in order?")
    radio("Keywords").click()
    labelled("Keyword 1").send_keys("SYN")
    for number, word, also in [(2, "SYN-ACK", "synack"), (3, "ACK", "")]:
        button("Add keyword").click()
        labelled(f"Keyword {number}").send_keys(word)
        if also:
            labelled(f"Other ways to write keyword {number}").send_keys(also)
    labelled("A test answer").send_keys("syn then synack")
    tried = find("//section[@aria-label='Try an answer']")
    wait.until(lambda _: "Wrong" in tried.text and "2 of 3 keywords" in tried.text)
    ok("keywords: SYN-ACK isn't also counted as SYN and ACK (R81)")
    shot("editor-keywords")
    button("Save question").click()
    wait.until(lambda _: "Question saved" in text())
    topic = next(row for row in ipc("topics", {}) if row["name"] == "OSI layers")
    assert (topic["question_count"], topic["short_count"]) == (2, 2), topic
    ipc("save_question", {"input": {"topic": topic["id"], "text": "Which layer routes packets?", "multiple": False, "options": [{"text": "Network", "correct": True}, {"text": "Session", "correct": False}, {"text": "Physical", "correct": False}], "explanation": "", "tags": [], "citations": [], "short": None}})
    ok("both short answer questions saved, plus an MCQ (R78, R79)")

    # ---------- Quiz ----------
    find("//nav//*[self::a or self::button][normalize-space()='Topics']").click()
    find("//table//button[normalize-space()='OSI layers']").click()
    find("//button[@aria-label='Quiz OSI layers']").click()
    find("//label[contains(., 'OSI layers')][contains(., '1 MCQ · 2 short answer')]")
    button("Change settings").click()
    radio("Short answer").click()
    summary = find("//*[@aria-label='Quiz summary']")
    wait.until(lambda _: "2 questions" in summary.text and "Short answer only" in summary.text)
    ok("setup counts each type and picks short answers only (R85)")
    shot("setup")
    button("Start quiz").click()

    box = labelled("Your answer")
    assert "short answer" in text().lower() and not driver.find_elements(By.XPATH, "//*[@aria-label='Answers']"), "a typed answer box, no options"
    box.send_keys("network layer")
    box.send_keys(Keys.ENTER)
    wait.until(lambda _: "Wrong" in status())
    feedback = status()
    assert "Model answer" in feedback and "Also accepted" in feedback and "Ports let several apps" in feedback, feedback
    assert driver.find_elements(By.XPATH, "//*[@role='status']//strong[normalize-space()='transport']"), "model answer is Markdown"
    ok("a typed answer is marked, with the model answer and explanation (R83)")
    shot("quiz-wrong")
    driver.find_element(By.TAG_NAME, "body").send_keys("o")
    wait.until(lambda _: "COUNTED AS RIGHT" in status().upper() and "YOU OVERRULED THIS" in status().upper())
    ok("O overrules the verdict (R84)")
    shot("quiz-overruled")
    driver.find_element(By.TAG_NAME, "body").send_keys(Keys.ENTER)

    box = wait.until(lambda _: (lambda b: b if b.get_attribute("value") == "" and not b.get_attribute("readonly") else None)(labelled("Your answer")))
    box.send_keys("SYN, SYN-ACK, ACK")
    box.send_keys(Keys.ENTER)
    wait.until(lambda _: "Correct" in status())
    assert driver.find_element(By.XPATH, "//*[@aria-label='Keywords']").text.count("SYN") == 2
    ok("keywords all hit, shown as chips")
    shot("quiz-keywords")
    driver.find_element(By.TAG_NAME, "body").send_keys(Keys.ENTER)

    # ---------- Results ----------
    find("//h1[normalize-space()='Results' or normalize-space()='Quiz finished']")
    wait.until(lambda _: "2 / 2" in text().replace("\n", " ") or "2/ 2" in text())
    first = find("//li[@aria-label='Question 1' or .//*[contains(., 'own port number')]][.//*[contains(., 'You overruled this')]]")
    assert "Your answer: network layer" in first.text and "Didn't match any accepted answer" in first.text, first.text
    ok("results count the overruled answer as right and say so (R84)")
    shot("results-overruled")
    first.find_element(By.XPATH, ".//button[starts-with(normalize-space(), 'Undo')]").click()
    wait.until(lambda _: "1 / 2" in text().replace("\n", " "))
    find("//button[normalize-space()='Count it as right']")
    ok("undoing on the results page counts it as wrong again")
    shot("results")

    attempt = ipc("history", {})[0]["id"]
    suggestion = next(s for s in ipc("card_suggestions", {}) if "own port number" in s["text"])
    assert suggestion["note"] == "Wrong, you wrote “network layer”" and suggestion["answer"] == "The **transport** layer (layer 4)", suggestion
    checklist = ipc("create_checklist", {"subject": {"Topic": topic["id"]}})
    kinds = [c["kind"] for c in ipc("recall_candidates", {"id": checklist})]
    assert kinds.count("Short answer") == 2 and kinds.count("MCQ") == 1, kinds
    ok("wrong short answers are card suggestions and can be checklist points (R86)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on short answer in the running app")
finally:
    driver.quit()

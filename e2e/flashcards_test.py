"""End-to-end: flashcards in the real app window (flashcards.feature).

Makes a topic, writes a basic card with an extra cue, a reversed card and a cloze card
through the editor, reviews them all with the keyboard (one Again, which comes back),
undoes a grade, checks the summary, then turns a recall miss into a card from
Suggestions. Screenshots go to $SHOTS if it is set. Run with e2e/run.sh."""
import os, pathlib, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import Select, WebDriverWait

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
        driver.save_screenshot(os.path.join(shots, f"cards-{name}.png"))
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


def press(*keys) -> None:
    driver.find_element(By.TAG_NAME, "body").send_keys(*keys)


def labelled(label: str):
    return find(f"//*[@id=//label[normalize-space()='{label}']/@for]")


try:
    driver.set_window_size(1440, 900)
    module = ipc("create_module", {"name": "Operating Systems"})
    topic = ipc("create_topic", {"name": "Memory", "module": module})

    driver.execute_script("location.hash = '#cards'")
    find("//h1[normalize-space()='Cards']")
    shot("first-time")
    button("Write your first card").click()
    find("//*[@role='radiogroup' and @aria-label='Card type']")
    Select(labelled("Topic")).select_by_visible_text("Memory")
    labelled("Front").send_keys("I keep translating the same pages. What caches the translations?")
    labelled("Back").send_keys("The **TLB**, a cache of page table entries")
    button("Add another cue").click()
    find("//input[@aria-label='Extra cue 1']").send_keys("What makes a page walk unnecessary most of the time?")
    assert driver.find_element(By.CSS_SELECTOR, "section[aria-label='Preview'] strong").text == "TLB"
    shot("editor-basic")
    button("Save 2 cards").click()
    wait.until(lambda _: "2 cards added to Memory" in text())
    find("//h1[normalize-space()='Memory']")
    ok("basic card with an extra cue saves as 2 cards, into the topic picked (R62)")

    button("New card").click()
    find("//button[@role='radio' and normalize-space()='Basic and reversed']").click()
    labelled("Front").send_keys("Thrashing")
    labelled("Back").send_keys("Spending more time paging than running")
    button("Save 2 cards").click()
    wait.until(lambda _: "2 cards added" in text() and "Thrashing" in text())
    ok("reversed card makes two (R63)")

    button("New card").click()
    find("//button[@role='radio' and normalize-space()='Cloze']").click()
    box = labelled("Text")
    box.send_keys("A page fault means the page isn't in RAM, so the OS loads it from disk")
    driver.execute_script("const b = arguments[0], at = b.value.indexOf('RAM'); b.setSelectionRange(at, at + 3)", box)
    box.send_keys(Keys.CONTROL, Keys.SHIFT, "c")
    driver.execute_script("const b = arguments[0]; b.setSelectionRange(b.value.length - 4, b.value.length)", box)
    button("Make a gap").click()
    assert box.get_attribute("value") == "A page fault means the page isn't in {{c1::RAM}}, so the OS loads it from {{c2::disk}}", box.get_attribute("value")
    preview = find("//section[@aria-label='Preview']")
    assert "Makes 2 cards" in preview.text and len(preview.find_elements(By.CSS_SELECTOR, ".cloze-gap")) == 2
    shot("editor-cloze")
    box.send_keys(Keys.CONTROL, "s")
    wait.until(lambda _: "2 cards added" in text() and "Cloze 2 of 2" in text())
    rows = driver.execute_script("return document.querySelector('table[aria-label=\"Cards in Memory\"]').tBodies[0].rows.length")
    assert rows == 6, rows
    ok("cloze gaps from the selection and the shortcut, previewed, saved as 2 cards (R64)")
    shot("topic")

    find("//nav//a[@href='#cards']").click()
    find("//*[contains(@aria-label, '6 new, 0 learning, 0 to review')]")
    assert find("//nav//a[@href='#cards']").text.endswith("6"), "sidebar shows 6 due"
    ok("home and sidebar show 6 new due (R68)")
    shot("home")
    press(Keys.ENTER)

    find("//*[@role='heading' and @aria-level='1']")
    assert not driver.find_elements(By.XPATH, "//nav[@aria-label='Main']"), "sidebar hidden in review"
    seen = []
    again_card = None
    graded = 0
    while True:
        heading = driver.find_elements(By.XPATH, "//*[@role='heading' and @aria-level='1']")
        if not heading:
            break
        front = heading[0].text
        seen.append(front)
        if graded == 0:
            shot("review-front")
        press(" ")
        find("//button[starts-with(normalize-space(), 'Good')]")
        if graded == 0:
            assert "Later today" in button("Again").text
            shot("review-answer")
        if graded == 1:
            again_card = front
            press("1")
        elif graded == 2:
            press("3")
            wait.until(lambda _: driver.find_element(By.XPATH, "//*[@role='heading' and @aria-level='1']").text != front)
            button("Undo").click()
            wait.until(lambda _: driver.find_element(By.XPATH, "//*[@role='heading' and @aria-level='1']").text == front)
            ok("undo brings the last card back (R67)")
            press(" ")
            find("//button[starts-with(normalize-space(), 'Good')]")
            press(" ")
        else:
            press(" ")
        graded += 1
        wait.until(lambda _: (lambda h: not h or h[0].text != front)(driver.find_elements(By.XPATH, "//*[@role='heading' and @aria-level='1']")))
        assert graded < 20, "session never ended"
    assert again_card and seen.count(again_card) == 2, seen
    assert any("[...]" in front for front in seen), "a cloze front was shown"
    assert "Spending more time paging than running" in seen and "Thrashing" in seen, "both directions shown"
    ok(f"reviewed {len(seen)} cards with the keyboard, the Again card came back (R65, R66)")

    find("//h1[normalize-space()='Done for today']")
    assert "1 time Again" in text(), text()
    ok("the summary shows reviewed, remembered and time (R69)")
    shot("done")
    button("Back to Cards").click()
    find("//*[normalize-space()='Done for today']")
    assert not driver.find_elements(By.XPATH, "//nav//a[@href='#cards']//*[contains(@aria-label, 'due today')]")
    ok("nothing due afterwards")

    checklist = ipc("create_checklist", {"subject": {"Topic": topic}})
    ipc("save_points", {"id": checklist, "points": [{"text": "Belady's anomaly: more frames, more faults with FIFO", "question": None}, {"text": "Working set", "question": None}]})
    ipc("finish_check", {"id": checklist, "dump": "Working set", "recalled": [1]})
    driver.execute_script("location.hash = '#topics'")
    driver.execute_script("location.hash = '#cards'")
    button("Look through them").click()
    item = find("//ul[@aria-label='Suggestions']/li[contains(., \"Belady\")]")
    item.find_element(By.XPATH, ".//button[starts-with(normalize-space(), 'Make a card')]").click()
    front = labelled("Front")
    assert front.get_attribute("value") == ""
    front.send_keys("Why might adding frames make FIFO worse?")
    Select(labelled("Topic")).select_by_visible_text("Memory")
    shot("suggestion")
    button("Save card").click()
    wait.until(lambda _: "Card saved in Memory" in text())
    assert "Belady" not in text()
    ok("a recall miss becomes a card only when accepted (R72, R73)")

    if audit:
        print("audit: " + ("no contrast, target or primary-button problems on any screen, both themes" if not findings else "\n  ".join(findings)))
    print(f"PASS: {checks} checks on flashcards in the running app")
finally:
    driver.quit()

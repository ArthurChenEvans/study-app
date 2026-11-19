"""End-to-end: nested modules, Default, query search and the graph in the real app window
(nested-modules.feature and search-query.feature, R217 to R220).

Test IDs E2E-01 to E2E-07 in specs/tests/nested-modules-and-search.md. Screenshots go to
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


def shot(name: str) -> None:
    time.sleep(0.5)
    if shots:
        driver.save_screenshot(os.path.join(shots, f"tree-{name}.png"))


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


def question(topic: int, words: str) -> None:
    ipc("save_question", {"input": {"topic": topic, "text": words, "multiple": False,
        "options": [{"text": "A", "correct": True}, {"text": "B", "correct": False}, {"text": "C", "correct": False}],
        "explanation": "", "tags": [], "citations": []}})


def search(query: str) -> None:
    box = find("//*[@role='search']//input[@aria-label='Search']")
    box.clear()
    for character in query:
        box.send_keys(character)
    box.send_keys(Keys.ENTER)


try:
    driver.set_window_size(1440, 900)

    # E2E-01: namesakes stay separate
    ca = ipc("create_submodule", {"name": "Computer Architecture", "parent": None})
    n2t = ipc("create_submodule", {"name": "N2T", "parent": ca})
    csapp = ipc("create_submodule", {"name": "CS:APP", "parent": ca})
    ucm = ipc("create_submodule", {"name": "UCM CySec CA Module", "parent": ca})
    n2t_cpu = ipc("create_topic", {"name": "CPU", "module": n2t})
    csapp_cpu = ipc("create_topic", {"name": "CPU", "module": csapp})
    alu = ipc("create_topic", {"name": "ALU", "module": n2t})
    question(n2t_cpu, "What does the program counter hold?")
    question(alu, "The ALU zero flag is used by the CPU to jump")
    driver.execute_script("location.hash = '#topics'")
    find("//*[normalize-space()='Computer Architecture']")
    assert len(ipc("questions_in", {"topic": csapp_cpu})) == 0
    rows = driver.find_elements(By.XPATH, "//tr[.//*[normalize-space()='CPU']]")
    assert len(rows) == 2, "two CPU rows, one under each module"
    ok("E2E-01: two CPU topics under different modules, a question in one only (R217)")
    shot("tree")

    # E2E-02: Default warning
    button("New topic").click()
    find("//input[@aria-label='Topic name']").send_keys("Misc")
    button("Create topic").click()
    find("//*[@role='dialog' and contains(., 'will put this under Default')]")
    button("No").click()
    assert find("//input[@aria-label='Topic name']").get_attribute("value") == "Misc"
    button("Create topic").click()
    button("Yes").click()
    find("//*[normalize-space()='Default']")
    ok("E2E-02: no module asks first, No keeps the typing, Yes puts it in Default (R217)")

    # E2E-03: scopes
    driver.execute_script("location.hash = '#search'")
    search("module:N2T CPU")
    find("//*[contains(normalize-space(), 'zero flag')]")
    search("module:N2T topic:CPU")
    wait.until(lambda _: "zero flag" not in text())
    assert "program counter" in text()
    ok("E2E-03: module: finds mentions anywhere inside, topic: keeps to that topic (R220)")

    # E2E-04: keyboard-only pills read as English
    search("CPU & (ALU $ (RAM ^ ROM))")
    pills = find("//*[@role='search']").text.replace("\n", " ")
    assert "CPU AND (ALU OR (RAM XOR ROM))" in " ".join(pills.split()), pills
    groups = driver.find_elements(By.XPATH, "//*[@role='search']//*[@data-outline='solid']//*[@data-outline='dashed']")
    assert groups, "the XOR group sits inside the OR group"
    ok("E2E-04: symbols show as words, groups nest (R220)")
    shot("pills")

    # E2E-05: quiz scope
    for topic in [csapp_cpu]:
        question(topic, "Which CPU stage reads memory?")
    driver.execute_script("location.hash = '#quizzes'")
    button("New quiz").click()
    find("//label[contains(., 'Computer Architecture')]//input[@type='checkbox']").click()
    find("//label[contains(., 'CS:APP')]//input[@type='checkbox']").click()
    button("Start quiz").click()
    for _ in range(3):
        assert "reads memory" not in text(), "no CS:APP question"
        if not driver.find_elements(By.XPATH, "//button[normalize-space()='Next']"):
            break
        button("Next").click()
    ok("E2E-05: a quiz on a module with a submodule unticked leaves it out (R217)")

    # E2E-06: graph search and open by keyboard
    driver.execute_script("location.hash = '#graph'")
    graph_box = find("//input[@aria-label='Search the graph']")
    graph_box.send_keys("CPU", Keys.ENTER)
    wait.until(lambda _: len(driver.find_elements(By.CSS_SELECTOR, "[data-graph-node][data-match='true']")) == 2)
    find("//*[@data-graph-node and @aria-label='Computer Architecture']").send_keys(Keys.ARROW_DOWN)
    driver.switch_to.active_element.send_keys(Keys.ENTER)
    find("//h1[normalize-space()='N2T']")
    ok("E2E-06: graph search highlights both CPUs, arrow keys and Enter open a node (R219)")
    shot("graph")

    # E2E-07: delete with Move contents up and a clash
    ipc("create_topic", {"name": "ALU", "module": ca})
    driver.execute_script("location.hash = '#topics'")
    find("//tr[.//*[normalize-space()='N2T']]//button[@aria-label='More']").click()
    button("Delete").click()
    find("//*[@role='alert' and contains(., 'ALU')]")
    find("//input[@aria-label='Type N2T to confirm']").send_keys("N2T")
    assert not button("Delete N2T").is_enabled(), "blocked while ALU clashes"
    button("Cancel").click()
    ipc("rename_topic", {"topic": alu, "name": "Hack ALU"})
    find("//tr[.//*[normalize-space()='N2T']]//button[@aria-label='More']").click()
    button("Delete").click()
    find("//input[@aria-label='Type N2T to confirm']").send_keys("N2T")
    button("Delete N2T").click()
    wait.until(lambda _: not driver.find_elements(By.XPATH, "//tr[.//*[normalize-space()='N2T']]"))
    assert "Hack ALU" in text()
    ok("E2E-07: delete with Move contents up after resolving a clash (R217)")

    print(f"PASS: {checks} checks on nested modules, query search and the graph in the running app")
finally:
    driver.quit()

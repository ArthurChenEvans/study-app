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
    ipc("create_submodule", {"name": "UCM CySec CA Module", "parent": ca})
    n2t_cpu = ipc("create_topic", {"name": "CPU", "module": n2t})
    csapp_cpu = ipc("create_topic", {"name": "CPU", "module": csapp})
    alu = ipc("create_topic", {"name": "ALU", "module": n2t})
    question(n2t_cpu, "What does the program counter hold?")
    question(alu, "The ALU zero flag is used by the CPU to jump")
    question(csapp_cpu, "Which CPU stage reads memory?")
    driver.execute_script("location.hash = '#topics'")
    tree = "//nav[@aria-label='Topics by module']"
    find(f"{tree}//h2[normalize-space()='N2T']")
    cpus = driver.find_elements(By.XPATH, f"{tree}//button[.//span[normalize-space()='CPU']]")
    assert len(cpus) == 2, "two CPU topics, one under each module"
    assert len(ipc("questions_in", {"topic": csapp_cpu})) == 1 and len(ipc("questions_in", {"topic": n2t_cpu})) == 1
    ok("E2E-01: two CPU topics under different modules, each with only its own question (R217)")
    shot("tree")

    # E2E-02: Default warning
    button("New topic").click()
    form = "//dialog[.//h3[normalize-space()='New topic']]"
    find(f"{form}//input").send_keys("Misc")
    find(f"{form}//button[normalize-space()='Create topic']").click()
    find("//dialog[contains(., 'will put this under Default')]")
    find("//dialog[contains(., 'will put this under Default')]//button[normalize-space()='No']").click()
    assert find(f"{form}//input").get_attribute("value") == "Misc"
    find(f"{form}//button[normalize-space()='Create topic']").click()
    find("//dialog[contains(., 'will put this under Default')]//button[normalize-space()='Yes']").click()
    find(f"{tree}//h2[normalize-space()='Default']")
    ok("E2E-02: no module asks first, No keeps the typing, Yes puts it in Default (R217)")

    # E2E-03: scopes
    driver.execute_script("location.hash = '#search'")
    search("module:N2T CPU")
    find("//*[@aria-label='Results']//*[contains(normalize-space(), 'zero flag')]")
    search("module:N2T topic:CPU")
    wait.until(lambda _: "zero flag" not in text())
    assert "program counter" in text()
    ok("E2E-03: module: finds mentions anywhere inside, topic: keeps to that topic (R220)")

    # E2E-04: keyboard-only pills read as English
    search("CPU & (ALU $ (RAM ^ ROM))")
    wait.until(lambda _: driver.find_elements(By.CSS_SELECTOR, "[role='search'] [data-outline='solid'] [data-outline='dashed']"))
    words = " ".join(find("//*[@role='search']//*[@role='list']").text.split()).replace("( ", "(").replace(" )", ")")
    assert "CPU AND (ALU OR (RAM XOR ROM))" in words, words
    ok("E2E-04: symbols show as words, groups nest (R220)")
    shot("pills")

    # E2E-05: quiz scope with a submodule left out
    driver.execute_script("location.hash = '#setup'")
    find("//button[starts-with(@aria-label, 'Computer Architecture ') and contains(@aria-label, 'topics')]").click()
    button("Tick all").click()
    button("Leave out CS:APP").click()
    button("Start quiz").click()
    wait.until(lambda _: "program counter" in text() or "zero flag" in text())
    assert "reads memory" not in text(), "no CS:APP question"
    ok("E2E-05: a quiz on a module with a submodule left out skips it (R217)")

    # E2E-06: graph search and open by keyboard
    driver.execute_script("location.hash = '#topics'")
    button("Graph").click()
    graph_box = find("//input[@aria-label='Search the graph']")
    graph_box.send_keys("CPU", Keys.ENTER)
    wait.until(lambda _: len(driver.find_elements(By.CSS_SELECTOR, "[data-graph-node][data-match='true']")) >= 2)
    node = find("//*[@data-graph-node and @aria-label='Computer Architecture']")
    driver.execute_script("arguments[0].focus()", node)
    node.send_keys(Keys.ARROW_DOWN)
    driver.switch_to.active_element.send_keys(Keys.ARROW_DOWN)
    driver.switch_to.active_element.send_keys(Keys.ENTER)
    find("//section[@aria-labelledby='picked-h']//*[contains(normalize-space(), 'Topic in Computer Architecture › CS:APP')]")
    ok("E2E-06: graph search highlights both CPUs, arrow keys and Enter open a topic (R219)")
    shot("graph")

    # E2E-07: delete with Move contents up after resolving a clash
    ipc("create_topic", {"name": "ALU", "module": ca})
    driver.execute_script("location.reload()")
    find("//button[@aria-label='N2T module actions']").click()
    find("//*[@role='menuitem' and normalize-space()='Delete module…']").click()
    find("//dialog//*[@role='alert' and contains(., 'ALU')]")
    find("//dialog//input[@autocomplete='off']").send_keys("N2T")
    assert not button("Delete N2T").is_enabled(), "blocked while ALU clashes"
    button("Cancel").click()
    ipc("rename_topic", {"topic": alu, "name": "Hack ALU"})
    driver.execute_script("location.reload()")
    find("//button[@aria-label='N2T module actions']").click()
    find("//*[@role='menuitem' and normalize-space()='Delete module…']").click()
    find("//dialog//input[@autocomplete='off']").send_keys("N2T")
    button("Delete N2T").click()
    WebDriverWait(driver, 30).until(lambda _: all(row["name"] != "N2T" for row in ipc("modules", {})))
    moved = [topic for topic in ipc("topics", {}) if topic["name"] == "Hack ALU"]
    assert moved and moved[0]["module"] == ca, moved
    ok("E2E-07: delete with Move contents up after resolving a clash (R217)")

    print(f"PASS: {checks} checks on nested modules, query search and the graph in the running app")
finally:
    driver.quit()

"""End-to-end: clicks through the main flow in the real app window.
Create a topic, write a question with code and an image, quiz on it, see the results.
Run with e2e/run.sh."""
import base64, os, pathlib, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
# A real 1x1 PNG, so the image protocol has to serve working bytes.
PNG = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="

options = ArgOptions()
options.set_capability("tauri:options", {"application": app})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
wait = WebDriverWait(driver, 20)


def find(css):
    return wait.until(lambda d: d.find_element(By.CSS_SELECTOR, css))


def button(text):
    return wait.until(lambda d: next(b for b in d.find_elements(By.TAG_NAME, "button") if b.text.strip().startswith(text) and b.is_enabled()))


def go(screen):
    driver.execute_script(f"location.hash = '#{screen}'")
    time.sleep(0.5)


try:
    # Topics: create one
    go("topics")
    button("New topic").click()
    find('input[placeholder^="e.g. CPU"]').send_keys("CPU")
    button("Create topic").click()
    button("Edit").click()

    # Editor: a question with highlighted code and a stored image
    image = driver.execute_async_script(
        "const done = arguments[1]; window.__TAURI_INTERNALS__.invoke('save_image', { data: arguments[0], mime: 'image/png' }).then(done, (e) => done('ERR ' + JSON.stringify(e)))",
        PNG,
    )
    assert len(image) == 64, image
    # WebKitWebDriver drops typed newlines, so set the multi-line text directly.
    driver.execute_script(
        "const box = arguments[0]; box.value = arguments[1]; box.dispatchEvent(new Event('input', { bubbles: true }))",
        find("#question-text"),
        f"Which stage reads registers? ![pipe](image:{image})\n\n```c\nint index = 0; // start\n```",
    )
    for index, text in enumerate(["Fetch", "Decode", "Execute"], start=1):
        find(f'input[aria-label="Option {index}"]').send_keys(text)
    find('input[aria-label="Option 2 is correct"]').click()
    find("#question-explanation").send_keys("Operands are read in decode.")
    button("Save question").click()
    wait.until(lambda d: "Question saved" in d.page_source)

    # Setup and quiz
    go("setup")
    wait.until(lambda d: d.find_element(By.XPATH, "//label[.//span[text()='CPU']]//input")).click()
    button("Start quiz").click()
    heading = find('[role="heading"][aria-level="1"]')
    wait.until(lambda d: d.execute_script("const img = document.querySelector('[role=heading] img'); return img && img.complete && img.naturalWidth > 0"))
    assert heading.find_elements(By.CSS_SELECTOR, ".hljs-type"), "code is coloured"
    print("image served and code coloured in the quiz")

    driver.find_element(By.TAG_NAME, "body").send_keys("2")
    button("Submit").click()
    wait.until(lambda d: "Operands are read in decode." in d.page_source)
    button(next(b.text.strip() for b in driver.find_elements(By.TAG_NAME, "button") if b.text.strip().startswith(("Next", "See results", "Finish")))).click()

    # Results
    wait.until(lambda d: "1 right and sure" in d.page_source)
    print("PASS: topic, question with image and code, quiz and results all work in the running app")
finally:
    driver.quit()

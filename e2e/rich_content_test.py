"""End-to-end: images and highlighted code in questions (R7, questions.feature).

Drives the real app window through tauri-driver: writes a question with a pasted image,
a picked image and highlighted code, saves it, then takes a quiz on it and checks the
images really decode and the code is coloured in both themes. Run with e2e/run.sh."""
import base64, os, pathlib, struct, tempfile, zlib
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))


def png(width: int, height: int, rgb: tuple[int, int, int]) -> bytes:
    """A real, decodable PNG of one colour."""
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    rows = b"".join(b"\x00" + bytes(rgb) * width for _ in range(height))
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


options = ArgOptions()
options.set_capability("tauri:options", {"application": app})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
driver.set_script_timeout(30)
wait = WebDriverWait(driver, 15)
steps = []


def step(name: str) -> None:
    steps.append(name)
    print(f"  ok  {name}")


def find(xpath: str):
    return wait.until(lambda _: driver.find_element(By.XPATH, xpath))


def button(name: str):
    return find(f"//button[normalize-space()='{name}' or @aria-label='{name}']")


def labelled(name: str):
    return find(f"//*[@aria-label='{name}' or @id=//label[normalize-space()='{name}']/@for]")


def ipc(command: str, args: dict):
    return driver.execute_async_script(
        "const [command, args, done] = arguments;"
        "window.__TAURI_INTERNALS__.invoke(command, args).then((out) => done({ ok: out }), (error) => done({ error }));",
        command, args,
    )


def type_lines(field, text: str) -> None:
    """WebKitWebDriver drops newlines from send_keys, so multiline text goes in at the
    cursor with an input event, the same thing typing does."""
    driver.execute_script(
        "const [field, text] = arguments; field.focus();"
        "field.setRangeText(text, field.selectionStart, field.selectionEnd, 'end');"
        "field.dispatchEvent(new Event('input', { bubbles: true }));",
        field, text,
    )


def pick_file(picker, path: pathlib.Path, mime: str) -> None:
    """WebKitWebDriver can't send files to an <input type=file>, so the file goes in
    through a DataTransfer and a change event, which runs the app's own pick handler."""
    driver.execute_script(
        "const [picker, name, mime, b64] = arguments;"
        "const bytes = Uint8Array.from(atob(b64), (character) => character.charCodeAt(0));"
        "const data = new DataTransfer(); data.items.add(new File([bytes], name, { type: mime }));"
        "picker.files = data.files; picker.dispatchEvent(new Event('change', { bubbles: true }));",
        picker, path.name, mime, base64.b64encode(path.read_bytes()).decode(),
    )


def image_loaded(scope: str) -> bool:
    """True when every stored image in `scope` has a blob: URL and real decoded pixels."""
    return driver.execute_script(
        "const images = [...document.querySelectorAll(arguments[0] + ' img[data-image-id]')];"
        "return images.length > 0 && images.every((image) => image.src.startsWith('blob:') && image.complete && image.naturalWidth > 0);",
        scope,
    )


def token_colours(scope: str) -> list[str]:
    return driver.execute_script(
        "return [...document.querySelectorAll(arguments[0] + ' pre.shiki span[style]')].map((span) => getComputedStyle(span).color);",
        scope,
    )


try:
    # ---------- Write the question ----------
    driver.execute_script("location.hash = '#topics'")
    button("New topic").click()
    find("//dialog//input").send_keys("CPU pipeline")
    button("Create topic").click()
    (find("//table//button[normalize-space()='CPU pipeline']").click(), find("//button[@aria-label='Edit CPU pipeline']"))[1].click()
    question = labelled("Question")
    step("opened the editor for a new topic")

    type_lines(question, "Which stage does this diagram show?\n\n```python\n# fetch the next word\ndef fetch(pc): return memory[pc]\n```\n\n")
    pasted = png(48, 24, (2, 88, 132))
    driver.execute_script(
        """
        const [field, b64] = arguments;
        const bytes = Uint8Array.from(atob(b64), (character) => character.charCodeAt(0));
        const data = new DataTransfer();
        data.items.add(new File([bytes], "pipeline.png", { type: "image/png" }));
        field.focus();
        field.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
        """,
        question, base64.b64encode(pasted).decode(),
    )
    wait.until(lambda _: "![pipeline](image:" in question.get_attribute("value"))
    wait.until(lambda _: image_loaded("[aria-label='Question preview']"))
    step("pasted an image into the question, it shows in the preview")

    wait.until(lambda _: len(token_colours("[aria-label='Question preview']")) > 2)
    dark = token_colours("[aria-label='Question preview']")
    assert len(set(dark)) > 1, f"code isn't coloured: {dark}"
    # The theme switch lives in Settings now (R158). Flip it the same way it does.
    driver.execute_script("document.documentElement.dataset.theme = 'studyapp'")
    light = token_colours("[aria-label='Question preview']")
    assert light != dark, "the code colours didn't change with the theme"
    driver.execute_script("document.documentElement.dataset.theme = 'studyapp-dark'")
    step(f"code is highlighted ({len(set(dark))} colours) and follows the Dark / Light switch")

    picked_file = pathlib.Path(tempfile.mkdtemp()) / "alu.png"
    picked_file.write_bytes(png(30, 30, (204, 108, 19)))
    option_one = labelled("Option 1")
    option_one.click()
    picker = labelled("Image file")
    pick_file(picker, picked_file, "image/png")
    wait.until(lambda _: option_one.get_attribute("value").startswith("![alu](image:"))
    find("//*[@role='status' and normalize-space()='Image added to option 1']")
    step("picked an image from a file into option 1")

    not_an_image = picked_file.with_name("fake.png")
    not_an_image.write_text("<svg onload='alert(1)'/>")
    labelled("Explanation").click()
    pick_file(picker, not_an_image, "image/png")
    find("//*[@role='alert' and contains(., \"isn't a PNG, JPEG, GIF or WebP image\")]")
    assert "image:" not in labelled("Explanation").get_attribute("value")
    step("a fake image is refused with a message saying why")

    type_lines(labelled("Explanation"), "Decode reads the registers.\n\n```rust\nlet operands = registers[source];\n```")
    labelled("Option 2").send_keys("Execute")
    labelled("Option 3").send_keys("Write back")
    labelled("Option 1 is correct").click()
    button("Save question").click()
    find("//*[@role='status' and normalize-space()='Question saved']")

    topics = ipc("topics", {})["ok"]
    topic = next(row for row in topics if row["name"] == "CPU pipeline")
    [[question_id, _]] = ipc("questions_in", {"topic": topic["id"]})["ok"]
    stored = ipc("question", {"id": question_id})["ok"]
    assert "![pipeline](image:" in stored["text"] and "```python" in stored["text"], stored["text"]
    assert stored["options"][0]["text"].startswith("![") and "(image:" in stored["options"][0]["text"], stored["options"]
    step("saved, the images are kept with the question")

    # ---------- Take the quiz ----------
    find("//nav//*[self::a or self::button][normalize-space()='Topics']").click()
    find("//table//button[normalize-space()='CPU pipeline']").click()
    find("//button[@aria-label='Quiz CPU pipeline']").click()
    button("Start quiz").click()
    find("//*[@aria-label='Answers']")
    wait.until(lambda _: image_loaded("main [role='heading']"))
    wait.until(lambda _: image_loaded("[aria-label='Answers']"))
    step("the quiz shows the question's image and the option's image")

    wait.until(lambda _: len(set(token_colours("main [role='heading']"))) > 1)
    step("the question's code is highlighted in the quiz")

    find("//*[@aria-label='Answers']//button[1]").click()
    find("//button[starts-with(normalize-space(), 'Submit')]").click()
    wait.until(lambda _: any("rust" == pre.get_attribute("data-language") for pre in driver.find_elements(By.CSS_SELECTOR, "pre.shiki")))
    step("the explanation's Rust code is highlighted in the feedback")
    if os.environ.get("SCREENSHOTS"):
        pathlib.Path(os.environ["SCREENSHOTS"], "quiz-feedback.png").write_bytes(driver.get_screenshot_as_png())

    print(f"PASS: {len(steps)} checks on images and highlighted code in the running app")
except Exception:
    pathlib.Path(root / "e2e/failure.png").write_bytes(driver.get_screenshot_as_png())
    print("FAIL after:", steps, "(screenshot in e2e/failure.png)")
    raise
finally:
    driver.quit()

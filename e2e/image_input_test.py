"""End-to-end: images into card fields the ways people actually do it (R7), in WebKitGTK.

WebKitGTK gives the page an empty clipboard for a pasted image and no files for a drop, so the
app reads the system clipboard and takes dropped paths from Tauri. This puts a real image and a
real copied file on the X clipboard (xclip) and drags a real file from a GTK window (xdotool),
then checks each lands in the field as an image. Needs xclip, xdotool and python3.12 with GTK.
Run with e2e/run.sh; the drag wants a screen at least 2200px wide."""
import os, pathlib, shutil, subprocess, sys, tempfile, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
if not all(shutil.which(tool) for tool in ("xclip", "xdotool", "python3.12")):
    print("  skip  needs xclip, xdotool and python3.12 with GTK")
    sys.exit(0)

folder = pathlib.Path(tempfile.mkdtemp())
picture = folder / "red.png"
from PIL import Image
Image.new("RGB", (40, 30), (200, 60, 20)).save(picture)

options = ArgOptions()
options.set_capability("tauri:options", {"application": os.environ.get("APP", str(root / "target/debug/study-app"))})
options.set_capability("browserName", "wry")
driver = webdriver.Remote("http://127.0.0.1:4444", options=options)
driver.set_script_timeout(30)
wait = WebDriverWait(driver, 15)


def ipc(command, args):
    return driver.execute_async_script("const [c,a,done]=arguments; window.__TAURI_INTERNALS__.invoke(c,a).then(o=>done(o))", command, args)


def image_lands_in(field):
    wait.until(lambda _: "](image:" in field.get_attribute("value"))


source = None
try:
    driver.set_window_size(1280, 840)
    module = ipc("create_module", {"name": "OS"})
    ipc("create_topic", {"name": "Memory", "module": module})
    driver.execute_script("location.hash = '#cards'")
    wait.until(lambda _: driver.find_elements(By.XPATH, "//button[contains(normalize-space(), 'Write your first card')]"))[0].click()
    front = wait.until(lambda _: driver.find_element(By.ID, "card-front"))
    back = driver.find_element(By.ID, "card-back")
    extra = None

    subprocess.run(["xclip", "-selection", "clipboard", "-t", "image/png", "-i", str(picture)], check=True)
    time.sleep(0.5)
    front.click()
    front.send_keys(Keys.CONTROL, "v")
    image_lands_in(front)
    print("  ok  a copied image pastes into the front")

    copy = subprocess.Popen(["xclip", "-selection", "clipboard", "-t", "text/uri-list", "-i"], stdin=subprocess.PIPE)
    copy.communicate(f"file://{picture}\r\n".encode())
    time.sleep(0.5)
    back.click()
    back.send_keys(Keys.CONTROL, "v")
    image_lands_in(back)
    print("  ok  a copied image file pastes into the back as an image, not its path")

    driver.execute_script("arguments[0].value = ''; arguments[0].dispatchEvent(new Event('input', { bubbles: true }))", back)
    window = driver.get_window_rect()
    middle = driver.execute_script("const r = arguments[0].getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]", back)
    to_x, to_y = int(window["x"] + middle[0]), int(window["y"] + middle[1])
    source = subprocess.Popen(["python3.12", str(root / "e2e/drag_source.py"), str(picture)])
    time.sleep(2)
    xdo = lambda *args: subprocess.run(["xdotool", *map(str, args)], check=True)
    xdo("mousemove", 1800, 200)
    time.sleep(0.3)
    xdo("mousedown", 1)
    for step in range(1, 31):
        xdo("mousemove", int(1800 + (to_x - 1800) * step / 30), int(200 + (to_y - 200) * step / 30))
        time.sleep(0.05)
    time.sleep(0.5)
    xdo("mouseup", 1)
    image_lands_in(back)
    assert "file://" not in back.get_attribute("value")
    print("  ok  a file dragged from a file manager drops into the back as an image")
finally:
    if source:
        source.terminate()
    driver.quit()
    shutil.rmtree(folder, ignore_errors=True)

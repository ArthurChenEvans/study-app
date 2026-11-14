"""UI sweep in the real app window (WebKitGTK through tauri-driver), not a stand-in browser.

Walks every screen in each state (empty, normal at scale with long names, an item picked,
a dialog open, a form error, the calendar's week) in dark and light at desktop, tablet and
phone widths, and checks each view three ways:

  spill   anything whose content runs out of its box (a button out of its card, text out of
          a key hint), unless a box between them scrolls or clips on purpose
  audit   contrast, 24px targets and one primary button per view (e2e/audit.js)
  axe     axe-core's WCAG 2.2 A and AA rules

Writes findings.json and a screenshot of every view to $SWEEP (default /tmp/sweep).
Run with: TESTS=e2e/sweep.py bash e2e/run.sh   (it seeds its own throwaway databases)"""
import json, os, pathlib, shutil, subprocess, sys, tempfile, time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.options import ArgOptions
from selenium.webdriver.support.ui import WebDriverWait

root = pathlib.Path(__file__).resolve().parent.parent
app = os.environ.get("APP", str(root / "target/debug/study-app"))
web = str(root / "target/debug/study-web")
out = pathlib.Path(os.environ.get("SWEEP", "/tmp/sweep"))
only = os.environ.get("ONLY", "")  # e.g. "large:desk:dark" to narrow a rerun
out.mkdir(parents=True, exist_ok=True)
audit_js = (root / "e2e/audit.js").read_text()
axe_js = (root / "node_modules/axe-core/axe.min.js").read_text()

SPILL_JS = r"""
const boxed = (el) => {
  if (el.matches('.panel, .itemcard > .panel, .modal-box, .note-box, .tile, .seg, .btn, .chip, kbd, .kbd, .toast, .note-toast, .pop, .stat, .cele-stat, .week-day, .cal-day, .nav-sheet, .selbar, .dock-tab, .rail-item, .settings-row, input, select, textarea')) return true;
  const cs = getComputedStyle(el);
  return parseFloat(cs.borderLeftWidth) >= 1 && cs.borderLeftStyle !== 'none' && parseFloat(cs.borderRightWidth) >= 1;
};
const clips = (el) => { const cs = getComputedStyle(el); return /(auto|scroll|hidden|clip)/.test(cs.overflowX + cs.overflow) || cs.position === 'fixed'; };
const visible = (el) => { const r = el.getBoundingClientRect(); const cs = getComputedStyle(el); return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.opacity !== '0' && !el.closest('[hidden], dialog:not([open]), .sr-only, svg') && !(el.closest('details:not([open])') && !el.matches('details') && !el.closest('summary')); };
const label = (el) => `${el.tagName.toLowerCase()}${el.className && typeof el.className === 'string' ? '.' + el.className.trim().split(/\s+/).slice(0, 3).join('.') : ''} "${(el.textContent || el.getAttribute('aria-label') || '').trim().replace(/\s+/g, ' ').slice(0, 40)}"`;
const found = [];
const scope = document.querySelector('dialog[open]') || document.body;
for (const el of scope.querySelectorAll('*')) {
  if (!visible(el)) continue;
  const r = el.getBoundingClientRect();
  // Out of its box: walk up to the nearest box, stopping at anything that scrolls or clips.
  for (let p = el.parentElement; p && p !== document.body; p = p.parentElement) {
    if (clips(p)) {
      // A box that hides what doesn't fit (not one that scrolls) and cuts this off.
      const cs = getComputedStyle(p);
      if (/(hidden|clip)/.test(cs.overflowX) && (!el.children.length || el.matches('button, a, input, select')) && getComputedStyle(el).textOverflow !== 'ellipsis' && cs.textOverflow !== 'ellipsis') {
        const b = p.getBoundingClientRect();
        const over = Math.max(b.left - r.left, r.right - b.right);
        if (over > 1.5) found.push(`${label(el)} is cut off ${Math.round(over)}px by ${label(p)}`);
      }
      break;
    }
    if (boxed(p)) {
      const b = p.getBoundingClientRect();
      const over = Math.max(b.left - r.left, r.right - b.right, b.top - r.top, r.bottom - b.bottom);
      if (over > 1.5) found.push(`${label(el)} spills ${Math.round(over)}px out of ${label(p)}`);
      break;
    }
  }
  // Content bigger than a small box that doesn't scroll (a key hint, a chip, a button).
  if (el.matches('kbd, .kbd, .chip, .btn, .due-pill, .date-chip, th, td') && !clips(el)) {
    if (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1) found.push(`${label(el)} content ${el.scrollWidth}x${el.scrollHeight} in a ${el.clientWidth}x${el.clientHeight} box`);
  }
}
// A key hint's glyph should sit inside its box with room around it. Layout can't see ink, so
// measure the glyph itself.
const canvas = document.createElement('canvas'); const ink = canvas.getContext('2d');
for (const el of scope.querySelectorAll('kbd, .kbd')) {
  if (!visible(el) || !el.textContent.trim()) continue;
  const cs = getComputedStyle(el);
  ink.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
  const m = ink.measureText(el.textContent.trim());
  const tall = m.actualBoundingBoxAscent + m.actualBoundingBoxDescent;
  const room = el.clientHeight - parseFloat(cs.paddingTop) - parseFloat(cs.paddingBottom);
  if (tall > room * 0.72) found.push(`${label(el)} glyph is ${tall.toFixed(1)}px tall in ${room.toFixed(1)}px of room`);
}
// Cut off at the window's edge: hidden by a clip rather than reachable by scrolling sideways.
const scrollsX = (el) => /(auto|scroll)/.test(getComputedStyle(el).overflowX);
for (const el of scope.querySelectorAll('*')) {
  if (!visible(el) || el.children.length) continue;
  const r = el.getBoundingClientRect();
  if (r.right <= innerWidth + 1 && r.left >= -1) continue;
  let scroller = false, fixed = false;
  for (let p = el.parentElement; p; p = p.parentElement) { if (scrollsX(p) && p !== document.documentElement && p !== document.body) { scroller = true; break; } if (getComputedStyle(p).position === 'fixed') fixed = true; }
  if (!scroller && !fixed) found.push(`${label(el)} is cut off at the window edge (${Math.round(r.left)} to ${Math.round(r.right)} of ${innerWidth})`);
}
// A table that has to be scrolled sideways on a phone (R161, R162: tables stack instead).
if (innerWidth < 600) for (const t of scope.querySelectorAll('table')) {
  if (!visible(t)) continue;
  for (let p = t.parentElement; p && p !== document.body; p = p.parentElement) {
    if (scrollsX(p) && p.scrollWidth > p.clientWidth + 1) { found.push(`${label(t)} scrolls sideways on a phone (${p.scrollWidth} in ${p.clientWidth})`); break; }
  }
}
const page = document.documentElement;
if (page.scrollWidth > innerWidth + 1) found.push(`page is ${page.scrollWidth}px wide in a ${innerWidth}px window`);
return [...new Set(found)];
"""

AXE_RUN = """
const done = arguments[arguments.length - 1];
const scope = document.querySelector('dialog[open]') || document;
axe.run(scope, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'] }, resultTypes: ['violations'] })
  .then((r) => done(r.violations.map((v) => `${v.id} (${v.impact}): ${v.nodes.length}x, e.g. ${v.nodes[0].target.join(' ')} :: ${v.help}`)))
  .catch((e) => done(['axe failed: ' + e]));
"""

WIDTHS = {"desk": (1440, 900), "tablet": (1000, 800), "phone": (390, 844)}
THEMES = ["studyapp-dark", "studyapp"]
SCREENS = ["overview", "calendar", "topics", "cards", "recall", "projects", "bibliography", "history", "search", "setup", "data", "settings"]
LONG_TOPIC = "Instruction-level parallelism, superscalar pipelines and out-of-order execution in modern CPUs"
LONG_MODULE = "Computer Systems Architecture and Operating Systems Fundamentals (Year 2)"

findings: dict[str, dict] = {}
server = None
views = 0


def start(profile: str) -> tuple[webdriver.Remote, str]:
    data = tempfile.mkdtemp(prefix=f"sweep-{profile}-")
    folder = pathlib.Path(data) / "com.arthurchenevans.studyapp"
    folder.mkdir(parents=True)
    if profile == "large":
        subprocess.run([web, "seed", str(folder / "study.sqlite"), "--large"], check=True, stdout=subprocess.DEVNULL)
    # The app inherits its data folder from the driver, so each profile gets its own driver.
    global server
    server = subprocess.Popen(["tauri-driver", "--port", "4544", "--native-port", "4545"], env={**os.environ, "XDG_DATA_HOME": data}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(2)
    options = ArgOptions()
    options.set_capability("tauri:options", {"application": app})
    options.set_capability("browserName", "wry")
    driver = webdriver.Remote("http://127.0.0.1:4544", options=options)
    driver.set_script_timeout(60)
    try:
        WebDriverWait(driver, 60).until(lambda d: d.execute_script("return !!document.querySelector('main')"))
    except Exception:
        driver.save_screenshot(str(out / f"start-failed-{profile}.png"))
        print("  page at start: " + driver.execute_script("return document.documentElement.outerHTML.slice(0, 400)"))
        driver.quit()
        server.terminate()
        raise
    return driver, data


def ipc(driver, command: str, args: dict):
    result = driver.execute_async_script(
        "const [command, args, done] = arguments;"
        "window.__TAURI_INTERNALS__.invoke(command, args).then((out) => done({ ok: out }), (error) => done({ error }));",
        command, args,
    )
    return result.get("ok")


def settle(driver, seconds=0.8):
    time.sleep(seconds)


def go(driver, screen: str):
    driver.execute_script("location.hash = '#overview'; location.hash = '#' + arguments[0];", screen)
    settle(driver, 1.0)


def click(driver, xpath: str) -> bool:
    found = driver.find_elements(By.XPATH, xpath)
    found = [el for el in found if el.is_displayed()]
    if not found:
        return False
    driver.execute_script("arguments[0].scrollIntoView({block: 'center'}); arguments[0].click();", found[0])
    settle(driver, 0.7)
    return True


def close_dialogs(driver):
    driver.execute_script("document.activeElement && document.activeElement.blur && document.activeElement.blur()")
    for _ in range(3):
        if not driver.find_elements(By.CSS_SELECTOR, "dialog[open], [role='dialog'][aria-modal='true']"):
            return
        if not click(driver, "//dialog[@open]//button[normalize-space()='Cancel' or normalize-space()='Done' or normalize-space()='Close' or @aria-label='Close']"):
            driver.find_element(By.TAG_NAME, "body").send_keys("")  # Escape
            settle(driver, 0.4)


def check(driver, key: str):
    global views
    views += 1
    name = key.replace(":", "-").replace("/", "_")
    driver.save_screenshot(str(out / f"{name}.png"))
    if os.environ.get("PROBE"):
        print("  probe", key, driver.execute_script(os.environ["PROBE"]))
    spill = driver.execute_script(SPILL_JS)
    audit = driver.execute_script(audit_js)
    if not driver.execute_script("return !!window.axe"):
        driver.execute_script(axe_js + "\n;window.axe = window.axe || axe;")
    axe = driver.execute_async_script(AXE_RUN)
    problems = {}
    if spill: problems["spill"] = spill
    if audit["fails"]: problems["contrast"] = audit["fails"]
    if audit["small"]: problems["targets"] = audit["small"]
    if audit["primaries"] > 1: problems["primaries"] = [f"{audit['primaries']} primary buttons"]
    if axe: problems["axe"] = axe
    if problems:
        findings[key] = problems
        print(f"  !! {key}: " + "; ".join(f"{kind} {len(items)}" for kind, items in problems.items()))


STATES = {
    # screen: [(state, action)] where the action opens that state from the screen's normal view
    "topics": [("picked", "//table[@aria-label='Topics']//tbody//tr[1]//button"),
               ("dialog", "//button[contains(normalize-space(), 'New topic')]"),
               ("error", "NEW_TOPIC_EMPTY")],
    "bibliography": [("picked", "//table//tbody//tr[1]//button"), ("dialog", "//button[contains(normalize-space(), 'Add a source') or contains(normalize-space(), 'Add source')]")],
    "calendar": [("week", "//button[@role='radio' and normalize-space()='Week']"), ("dialog", "//button[contains(normalize-space(), 'Add a date')]"), ("day", "TODAY_CELL")],
    "recall": [("dialog", "//button[contains(normalize-space(), 'New checklist')]")],
    "projects": [("dialog", "//button[contains(normalize-space(), 'New project')]")],
    "settings": [("dialog", "//button[contains(normalize-space(), 'Celebrations')]")],
    "overview": [("table", "//details[contains(@class,'table-view')]/summary")],
    "history": [("table", "//details[contains(@class,'table-view')]/summary")],
    "data": [("export", "//button[contains(normalize-space(), 'Export')]"),
             ("export-anki", "EXPORT_ANKI"),
             ("import-anki", "UPLOAD:crates/study-api/tests/fixtures/anki/networks-modern.apkg"),
             ("import-csv", "UPLOAD:e2e/fixtures/cpu-questions.csv"),
             ("import-problems", "CSV_PROBLEMS"),
             ("import-unreadable", "UPLOAD:e2e/fixtures/lecture-notes.zip")],
    "cards": [("table", "//details[contains(@class,'table-view')]/summary"), ("dialog", "//button[contains(normalize-space(), 'Settings') or contains(normalize-space(), 'Card settings')]")],
}


def run_state(driver, screen: str, state: str, action: str) -> bool:
    if action == "NEW_TOPIC_EMPTY":
        if not click(driver, "//button[contains(normalize-space(), 'New topic')]"):
            return False
        return click(driver, "//dialog[@open]//button[@type='submit']")
    if action == "TODAY_CELL":
        return click(driver, "//*[@role='grid']//button[@aria-current='date'] | //ol[contains(@aria-label, 'Week of')]//button[@aria-current='date']")
    if action.startswith("UPLOAD:"):
        # WebKitWebDriver can't upload files, so the page is handed one built from its bytes.
        import base64
        path = root / action[7:]
        driver.execute_script(
            "const [name, data] = arguments; const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));"
            "const input = document.querySelector(\"input[aria-label='Import a file']\"); const list = new DataTransfer();"
            "list.items.add(new File([bytes], name)); input.files = list.files; input.dispatchEvent(new Event('change', { bubbles: true }));",
            path.name, base64.b64encode(path.read_bytes()).decode())
        settle(driver, 1.5)
        return bool(driver.find_elements(By.CSS_SELECTOR, "dialog[open]"))
    if action == "EXPORT_ANKI":
        if not click(driver, "//button[contains(normalize-space(), 'Export')]"):
            return False
        driver.execute_script("const s = document.querySelector('dialog[open] select[aria-label=\"Kind of file\"]'); s.value = 'Anki'; s.dispatchEvent(new Event('change', { bubbles: true }));")
        settle(driver, 0.6)
        return True
    if action == "CSV_PROBLEMS":
        if not run_state(driver, screen, state, "UPLOAD:e2e/fixtures/cpu-questions.csv"):
            return False
        return click(driver, "//dialog[@open]//button[normalize-space()='See which']")
    return click(driver, action)


def sweep(profile: str):
    driver, data = start(profile)
    try:
        if profile == "large":
            module = ipc(driver, "create_module", {"name": LONG_MODULE})
            ipc(driver, "create_topic", {"name": LONG_TOPIC, "module": module})
        for width, (w, h) in WIDTHS.items():
            driver.set_window_size(w, h)
            settle(driver, 0.6)
            for theme in THEMES:
                driver.execute_script("localStorage.setItem('studyapp-theme', arguments[0]); document.documentElement.dataset.theme = arguments[0]; localStorage.removeItem('studyapp-calendar-view');", theme)
                for screen in SCREENS:
                    key = f"{profile}:{width}:{'dark' if theme.endswith('dark') else 'light'}:{screen}"
                    if only and not key.startswith(only):
                        continue
                    go(driver, screen)
                    check(driver, f"{key}:normal")
                    for state, action in STATES.get(screen, []):
                        go(driver, screen)
                        if run_state(driver, screen, state, action):
                            check(driver, f"{key}:{state}")
                        close_dialogs(driver)
    finally:
        driver.quit()
        if server:
            server.terminate()
            server.wait()
        shutil.rmtree(data, ignore_errors=True)


failed = None
try:
    for profile in ["empty", "large"]:
        if only and not only.startswith(profile) and ":" in only:
            continue
        print(f"== {profile}")
        sweep(profile)
except Exception:
    import traceback
    failed = traceback.format_exc()
    print(failed)
finally:
    (out / "findings.json").write_text(json.dumps(findings, indent=1))
    kinds: dict[str, int] = {}
    for problems in findings.values():
        for kind, items in problems.items():
            kinds[kind] = kinds.get(kind, 0) + len(items)
    print(f"SWEEP: {views} views, {len(findings)} with findings " + json.dumps(kinds))
    sys.exit(2 if failed else 1 if findings else 0)

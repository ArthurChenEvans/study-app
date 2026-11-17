// Layout check (R214): opens screens at phone, tablet and desktop widths against a seeded
// study-web and fails when text is squeezed into a narrow column (a few letters per line),
// runs off the right edge, or the page scrolls sideways. Usage:
//   STUDY_DB=/tmp/x.sqlite STUDY_SEED=1 STUDY_DIST=dist target/debug/study-web &
//   node scripts/layout-check.mjs [screen ...]
import { createRequire } from "module";
import { execSync } from "child_process";

const require = createRequire(import.meta.url);
let playwright;
// The pre-installed browser matches the global Playwright, so that one is tried first.
try { playwright = require(execSync("npm root -g").toString().trim() + "/playwright"); } catch { playwright = require("playwright"); }

const base = process.env.STUDY_URL ?? "http://127.0.0.1:7392/";
const screens = process.argv.slice(2).length ? process.argv.slice(2) : ["overview", "calendar", "quizzes", "cards", "recall", "projects", "history", "setup", "data", "settings", "topics", "search"];
const sizes = { phone: [390, 844], tablet: [1024, 768], desktop: [1440, 900] };

const probe = () => {
  const problems = [];
  const width = document.documentElement.clientWidth;
  if (document.documentElement.scrollWidth > width + 1) problems.push(`page scrolls sideways (${document.documentElement.scrollWidth}px)`);
  // R216: a page-tall box that scrolls on its own makes iPhone bounce and stop short of the
  // top and bottom, so on a phone the page itself scrolls.
  if (width < 600) for (const element of document.querySelectorAll("body *")) {
    if (element.closest(".modal-box, nav, aside")) continue;
    const style = getComputedStyle(element);
    if (/(auto|scroll)/.test(style.overflowY) && element.clientHeight >= window.innerHeight * 0.6 && element.scrollHeight > element.clientHeight + 4) problems.push(`scrolls inside a page-tall box: <${element.tagName.toLowerCase()} class="${String(element.className).slice(0, 40)}">`);
  }
  // Two or three choices side by side (a .seg) shouldn't fall into a stack.
  for (const seg of document.querySelectorAll(".seg")) {
    const tops = new Set([...seg.children].filter((child) => child.getBoundingClientRect().height).map((child) => Math.round(child.getBoundingClientRect().top)));
    if (tops.size > 1) problems.push(`choices stacked: "${seg.textContent.trim().slice(0, 40)}"`);
  }
  for (const element of document.querySelectorAll("main *, dialog[open] *, [role=dialog] *")) {
    const style = getComputedStyle(element);
    if (style.display === "none" || style.visibility === "hidden" || element.closest("[aria-hidden=true], .sr-only, svg, canvas") || (element.closest("details:not([open])") && !element.closest("summary"))) continue;
    const rect = element.getBoundingClientRect();
    if (!rect.width || !rect.height) continue;
    const text = [...element.childNodes].filter((node) => node.nodeType === 3).map((node) => node.textContent).join("").replace(/\s+/g, " ").trim();
    if (text.length >= 12) {
      const line = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.5;
      const inner = rect.height - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom) - parseFloat(style.borderTopWidth) - parseFloat(style.borderBottomWidth);
      const lines = Math.round(inner / line);
      if ((lines >= 3 && text.length / lines < 12) || (lines >= 4 && rect.width < Math.min(220, width * 0.5))) problems.push(`squeezed: "${text.slice(0, 40)}" (${lines} lines in ${Math.round(rect.width)}px)`);
    }
    const scroller = element.closest(".overflow-x-auto, .overflow-auto, .rail, .seg, [data-scrolls]");
    if (!scroller && style.position !== "fixed" && rect.right > width + 1) problems.push(`off the right edge: <${element.tagName.toLowerCase()}> "${(element.textContent ?? "").trim().slice(0, 30)}"`);
    if (!scroller && style.position !== "fixed" && rect.left < -1) problems.push(`off the left edge: <${element.tagName.toLowerCase()}> "${(element.textContent ?? "").trim().slice(0, 30)}"`);
  }
  return [...new Set(problems)];
};

// Screens you reach by clicking: dialogs and the deeper pages (R216).
const flows = {
  "new quiz": ["quizzes", (page) => page.getByRole("button", { name: /Make a quiz/ }).first().click()],
  "quiz schedule": ["quizzes", (page) => page.getByRole("button", { name: /^Schedule/ }).first().click()],
  "question editor": ["quizzes", async (page) => { await page.getByRole("button", { name: "CPU" }).first().click(); await page.getByRole("complementary", { name: "Picked topic" }).getByRole("button", { name: /^Edit/ }).click(); }],
  "new card": ["cards", (page) => page.getByRole("button", { name: /New card/ }).first().click()],
  "card review": ["cards", (page) => page.getByRole("button", { name: /Start review/ }).first().click()],
  "new log": ["projects", (page) => page.getByRole("button", { name: /New log/ }).first().click()],
  "practice log": ["projects", (page) => page.getByRole("button", { name: "Study app" }).first().click()],
  "checklist": ["recall", (page) => page.getByRole("button", { name: /OSI model/ }).first().click()],
  "add a date": ["calendar", (page) => page.getByRole("button", { name: /Add a date/ }).first().click()],
  "new topic": ["topics", (page) => page.getByRole("button", { name: /New topic/ }).first().click()],
  "new module": ["topics", (page) => page.getByRole("button", { name: /New module/ }).first().click()],
};
const runFlows = !process.argv.slice(2).length || process.env.FLOWS;

const browser = await playwright.chromium.launch();
let failed = 0;
const report = (name, size, problems) => {
  if (problems.length) {
    failed += problems.length;
    console.log(`✗ ${name} at ${size}\n` + problems.slice(0, 12).map((problem) => "    " + problem).join("\n"));
  } else console.log(`✓ ${name} at ${size}`);
};
for (const [size, [width, height]] of Object.entries(sizes)) {
  const page = await browser.newPage({ viewport: { width, height }, hasTouch: size === "phone" });
  for (const screen of screens) {
    await page.goto(base + "#" + screen);
    await page.waitForTimeout(600);
    report(screen, size, await page.evaluate(probe));
  }
  if (runFlows) for (const [name, [screen, open]] of Object.entries(flows)) {
    // A fresh load each time, so a dialog left open by the last one doesn't get in the way.
    await page.goto(base + "#" + screen);
    await page.reload();
    await page.waitForTimeout(700);
    try {
      await open(page);
      await page.waitForTimeout(600);
      report(name, size, await page.evaluate(probe));
    } catch (error) {
      report(name, size, [`couldn't open it: ${String(error.message).split("\n")[0]}`]);
    }
  }
  await page.close();
}
await browser.close();
process.exit(failed ? 1 : 0);

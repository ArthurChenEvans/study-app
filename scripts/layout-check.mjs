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
  for (const element of document.querySelectorAll("main *")) {
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
    const scroller = element.closest(".overflow-x-auto, .overflow-auto, .rail, [data-scrolls]");
    if (!scroller && style.position !== "fixed" && rect.right > width + 1) problems.push(`off the right edge: <${element.tagName.toLowerCase()}> "${(element.textContent ?? "").trim().slice(0, 30)}"`);
  }
  return [...new Set(problems)];
};

const browser = await playwright.chromium.launch();
let failed = 0;
for (const [size, [width, height]] of Object.entries(sizes)) {
  const page = await browser.newPage({ viewport: { width, height } });
  for (const screen of screens) {
    await page.goto(base + "#" + screen);
    await page.waitForTimeout(600);
    const problems = await page.evaluate(probe);
    if (problems.length) {
      failed += problems.length;
      console.log(`✗ ${screen} at ${size}\n` + problems.slice(0, 12).map((problem) => "    " + problem).join("\n"));
    } else console.log(`✓ ${screen} at ${size}`);
  }
  await page.close();
}
await browser.close();
process.exit(failed ? 1 : 0);

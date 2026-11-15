// R211: opens the built phone site in WebKit (Safari's engine) at iPhone size and checks it
// starts: the study core loads in its worker and the sign-in screen shows.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { devices, webkit } from "playwright";

const root = process.argv[2] ?? "dist-phone";
const base = process.argv[3] ?? "/study-app/";
const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css", ".png": "image/png", ".webmanifest": "application/manifest+json", ".svg": "image/svg+xml", ".woff2": "font/woff2" };
const server = createServer(async (request, response) => {
  const path = decodeURIComponent(new URL(request.url, "http://x").pathname);
  const inside = path.startsWith(base) ? path.slice(base.length) : null;
  const file = normalize(join(root, inside === null || inside === "" ? "index.html" : inside));
  try {
    const body = await readFile(file);
    response.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
    response.end(body);
  } catch {
    response.writeHead(404).end();
  }
}).listen(0);
const port = server.address().port;

const browser = await webkit.launch();
const page = await browser.newPage({ ...devices["iPhone 13"] });
const problems = [];
page.on("pageerror", (error) => problems.push(`page error: ${error.message}`));
page.on("console", (message) => { if (message.type() === "error") problems.push(`console: ${message.text()}`); });
await page.goto(`http://127.0.0.1:${port}${base}`);
await page.getByText("Sign in to study on your phone").waitFor({ timeout: 30_000 });
// The account box asks the worker for the sign-in status, so this shows the core is running.
await page.getByRole("button", { name: "Sign in to sync" }).waitFor({ timeout: 30_000 });
await page.getByRole("button", { name: "Sign in to sync" }).click();
await page.getByLabel("Email").waitFor();
const manifest = await page.locator('link[rel="manifest"]').getAttribute("href");
const width = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
await page.screenshot({ path: "phone-smoke.png", fullPage: true });
await browser.close();
server.close();
const real = problems.filter((problem) => !/supabase|Failed to load resource/i.test(problem));
console.log(`manifest: ${manifest}, fits the width: ${width}`);
if (real.length) {
  console.log(real.join("\n"));
  process.exit(1);
}
if (!manifest || !width) process.exit(1);
console.log("phone smoke test passed");

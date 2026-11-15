import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// R186, R212: screens switch without an animation. A fade and slide on the whole screen made
// WebKitGTK (the Linux desktop app's webview) flicker on and off, and a filled transform once
// made dialog backdrops cover only the screen's height.
describe("screen entrance", () => {
  const css = readFileSync("src/app.css", "utf8");

  it("doesn't animate the whole screen", () => {
    expect(css).not.toMatch(/\.screen-enter\s*\{[^}]*animation/);
    expect(css).not.toMatch(/@keyframes enter\b/);
  });
});

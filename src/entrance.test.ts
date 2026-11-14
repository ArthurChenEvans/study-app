import { describe, expect, it } from "vitest";
import css from "./app.css?raw";

// R186: the screen entrance must not leave a transform behind once it ends. A filled transform
// animation makes the screen the containing block for position: fixed, so dialog backdrops
// only covered the screen's height instead of the whole window.
describe("screen entrance", () => {
  const rule = css.match(/\.screen-enter\s*\{([^}]*)\}/)?.[1] ?? "";

  it("only applies while it runs", () => {
    expect(rule).toMatch(/animation:[^;]*\bbackwards\b/);
    expect(rule).not.toMatch(/\b(both|forwards)\b/);
  });

  it("never fades the whole screen out to nothing, so switching screens doesn't flash", () => {
    const frames = css.match(/@keyframes enter\s*\{([^}]*\}[^}]*)\}/)?.[1] ?? "";
    const start = Number(frames.match(/from\s*\{[^}]*opacity:\s*([\d.]+)/)?.[1] ?? 0);
    expect(start).toBeGreaterThanOrEqual(0.6);
  });
});

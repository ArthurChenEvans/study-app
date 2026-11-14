import { beforeEach, describe, expect, it } from "vitest";
import { confettiColours } from "./confettiColours";

// R193: confetti matches the theme, or is a rainbow
describe("confetti colours", () => {
  beforeEach(() => {
    const root = document.documentElement.style;
    root.setProperty("--color-primary", "#db9068");
    root.setProperty("--chart-right", "#7ebd83");
    root.setProperty("--color-info", "#6e9ad9");
    root.setProperty("--accent-ink", "#e2b66d");
    root.setProperty("--primary-ink", "#f4b393");
    root.setProperty("--color-base-content", "#f4e9e1");
  });

  it("takes the theme's own colours", () => {
    expect(confettiColours("theme")).toEqual(["#db9068", "#7ebd83", "#6e9ad9", "#e2b66d", "#f4b393", "#f4e9e1"]);
  });

  it("is a rainbow when asked, the same in every theme", () => {
    const rainbow = confettiColours("rainbow");
    expect(rainbow).toHaveLength(7);
    document.documentElement.style.setProperty("--color-primary", "#000000");
    expect(confettiColours("rainbow")).toEqual(rainbow);
  });

  it("leaves out a colour the theme hasn't set", () => {
    document.documentElement.style.removeProperty("--color-info");
    expect(confettiColours("theme")).not.toContain("");
  });
});

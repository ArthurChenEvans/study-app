import { beforeEach, describe, expect, it } from "vitest";
import { DARK, LIGHT, applyPalette, applyTheme, initialPalette, initialTheme } from "./theme";

// appearance.feature
describe("theme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("data-palette");
  });

  it("is dark by default", () => {
    expect(initialTheme()).toBe(DARK);
  });

  it("switches to light and remembers the choice", () => {
    applyTheme(LIGHT);
    expect(document.documentElement.dataset.theme).toBe(LIGHT);
    expect(initialTheme()).toBe(LIGHT);
  });

  it("ignores an unknown saved value", () => {
    localStorage.setItem("studyapp-theme", "neon");
    expect(initialTheme()).toBe(DARK);
  });

  it("still works when storage is unavailable", () => {
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = () => {
      throw new Error("blocked");
    };
    try {
      expect(initialTheme()).toBe(DARK);
    } finally {
      Storage.prototype.getItem = original;
    }
  });

  // R129: right and wrong in the standard green and red, or the colour-blind safe blue and orange
  it("uses the standard colours unless the colour-blind safe ones were picked, and remembers the choice", () => {
    expect(initialPalette()).toBe("standard");
    applyPalette("colour-blind");
    expect(document.documentElement.dataset.palette).toBe("cb");
    expect(initialPalette()).toBe("colour-blind");
    applyPalette("standard");
    expect(document.documentElement.dataset.palette).toBeUndefined();
    expect(initialPalette()).toBe("standard");
  });
});

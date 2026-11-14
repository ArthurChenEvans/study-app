import { beforeEach, describe, expect, it } from "vitest";
import { DARK, LIGHT, applyPalette, applyTheme, followTheme, initialPalette, initialTheme } from "./theme";
import { buildColours } from "./colours";

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

  // appearance.feature: Colour themes (R183). However light or dark gets set, the colours follow.
  it("keeps the colours in step when the mode is set some other way", async () => {
    const stop = followTheme();
    document.documentElement.dataset.theme = LIGHT;
    await Promise.resolve();
    expect(document.documentElement.style.getPropertyValue("--color-base-100")).toBe(buildColours("coffee", false, false)["--color-base-100"]);
    document.documentElement.dataset.theme = DARK;
    await Promise.resolve();
    expect(document.documentElement.style.getPropertyValue("--color-base-100")).toBe(buildColours("coffee", true, false)["--color-base-100"]);
    stop();
  });
});

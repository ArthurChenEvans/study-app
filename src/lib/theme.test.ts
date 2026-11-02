import { beforeEach, describe, expect, it } from "vitest";
import { DARK, LIGHT, applyTheme, initialTheme } from "./theme";

// appearance.feature
describe("theme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
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
});

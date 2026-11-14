import { beforeEach, describe, expect, it } from "vitest";
import { COLOURS, applyColours, buildColours, initialColours } from "./colours";
import { apca, deltaE, simulate, wcag } from "./oklch";

// appearance.feature: Colour themes (R183, R184)
describe("colour themes", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("style");
  });

  it("offers nine, round the colour wheel, with Coffee by default", () => {
    expect(COLOURS.map((theme) => theme.id)).toEqual(["mono", "coffee", "honey", "brick", "rose", "lavender", "ocean", "miku", "sage"]);
    expect(initialColours()).toBe("coffee");
    localStorage.setItem("studyapp-colours", "indigo");
    expect(initialColours()).toBe("coffee");
  });

  it("applies a theme's colours to the page and remembers it", () => {
    applyColours("sage", false, false);
    const sage = buildColours("sage", false, false);
    expect(document.documentElement.style.getPropertyValue("--color-primary")).toBe(sage["--color-primary"]);
    expect(document.documentElement.style.getPropertyValue("--chart-right")).toBe(sage["--chart-right"]);
    expect(initialColours()).toBe("sage");
  });

  it("swaps right and wrong for the colour-blind safe pair", () => {
    const standard = buildColours("coffee", false, false);
    const safe = buildColours("coffee", false, true);
    expect(safe["--chart-right"]).not.toBe(standard["--chart-right"]);
    expect(safe["--color-primary"]).toBe(standard["--color-primary"]);
  });

  // Every theme, light and dark, standard and colour-blind safe.
  for (const theme of COLOURS) for (const dark of [false, true]) for (const safe of [false, true]) {
    const name = `${theme.id} ${dark ? "dark" : "light"}${safe ? " colour-blind" : ""}`;
    it(`${name}: everything readable`, () => {
      const c = buildColours(theme.id, dark, safe);
      const text = (fg: string, bg: string, lc = 60) => { expect(wcag(fg, bg), `${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5); expect(Math.abs(apca(fg, bg)), `${fg} on ${bg} Lc`).toBeGreaterThanOrEqual(lc); };
      const fill = (fg: string, bg: string) => { expect(wcag(fg, bg), `${fg} on ${bg}`).toBeGreaterThanOrEqual(3); expect(Math.abs(apca(fg, bg)), `${fg} on ${bg} Lc`).toBeGreaterThanOrEqual(30); };
      for (const ground of [c["--color-base-100"], c["--surface"]]) {
        text(c["--color-base-content"], ground);
        text(c["--muted"], ground, 45);
        text(c["--primary-ink"], ground);
        for (const ink of ["--right-ink", "--wrong-ink", "--info-ink", "--accent-ink", "--error-ink"] as const) text(c[ink], ground);
        for (const solid of ["--chart-right", "--chart-wrong", "--color-info", "--color-warning", "--chart-ink", "--field-border"] as const) fill(c[solid], ground);
        fill(c["--primary-edge"], ground);
      }
      text(c["--color-primary-content"], c["--color-primary"], 45);
      text(c["--on-right"], c["--chart-right"], 45);
      text(c["--on-wrong"], c["--chart-wrong"], 45);
    });
  }

  it("keeps colour-blind safe right and wrong apart for every kind of colour blindness", () => {
    for (const theme of COLOURS) for (const dark of [false, true]) {
      const c = buildColours(theme.id, dark, true);
      for (const kind of ["protanopia", "deuteranopia", "tritanopia"] as const) {
        expect(deltaE(simulate(c["--chart-right"], kind), simulate(c["--chart-wrong"], kind)), `${theme.id} ${kind}`).toBeGreaterThan(0.15);
      }
    }
  });
});

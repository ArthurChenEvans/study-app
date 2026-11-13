import { describe, expect, it } from "vitest";
import { DEFAULTS, Pacer, bigMode, parsePrefs, popGap, popsFor } from "./celebrate";

// celebrations.feature (R151): small pops land every 15 to 40 cards whatever the queue size.
describe("popGap", () => {
  it("is a quarter of the queue, kept between 15 and 40 and rounded to a friendly number", () => {
    expect(popGap(30, "normal")).toBe(15);
    expect(popGap(64, "normal")).toBe(15);
    expect(popGap(120, "normal")).toBe(30);
    expect(popGap(400, "normal")).toBe(40);
  });
  it("moves the range for fewer or more", () => {
    expect(popGap(400, "fewer")).toBe(60);
    expect(popGap(64, "fewer")).toBe(25);
    expect(popGap(400, "more")).toBe(25);
    expect(popGap(30, "more")).toBe(10);
  });
});

describe("popsFor", () => {
  const at = (queue: number) => popsFor(queue, "normal", true).map((pop) => `${pop.at}${pop.kind === "count" ? "" : pop.kind === "halfway" ? "h" : "s"}`);

  it("gives small queues nothing, the finish screen is enough", () => {
    expect(at(8)).toEqual([]);
    expect(at(14)).toEqual([]);
  });
  it("matches the agreed table", () => {
    expect(at(30)).toEqual(["15h"]);
    expect(at(64)).toEqual(["15", "32h", "48s"]);
    expect(at(120)).toEqual(["30", "60h", "90s"]);
    expect(at(400)).toEqual(["40", "80", "120", "160", "200h", "240", "300s", "320", "360"]);
  });
  it("leaves out halfway and last stretch when they're turned off", () => {
    expect(popsFor(64, "normal", false).map((pop) => pop.at)).toEqual([15, 30, 45]);
  });
  it("never puts two pops closer than 8 cards", () => {
    for (const queue of [15, 22, 37, 64, 99, 150, 401, 1000]) {
      const pops = popsFor(queue, "normal", true);
      pops.slice(1).forEach((pop, index) => expect(pop.at - pops[index].at).toBeGreaterThanOrEqual(8));
      pops.forEach((pop) => expect(pop.at).toBeLessThan(queue));
    }
  });
});

// celebrations.feature (R152): settings merge over defaults and junk is ignored.
describe("parsePrefs", () => {
  it("starts with everything on", () => {
    expect(parsePrefs(null)).toEqual(DEFAULTS);
    expect(DEFAULTS.on && DEFAULTS.counts && DEFAULTS.markers && DEFAULTS.streaks && DEFAULTS.toasts).toBe(true);
    expect(DEFAULTS.frequency).toBe("normal");
  });
  it("keeps what's valid and drops the rest", () => {
    const prefs = parsePrefs(JSON.stringify({ on: false, frequency: "loads", streakLength: 10, moments: { quiz: "pop", recall: "nope" }, extra: 1 }));
    expect(prefs.on).toBe(false);
    expect(prefs.frequency).toBe("normal");
    expect(prefs.streakLength).toBe(10);
    expect(prefs.moments).toEqual({ reviews: "full", quiz: "pop", recall: "full" });
    expect("extra" in prefs).toBe(false);
  });
  it("survives broken JSON", () => {
    expect(parsePrefs("{oops")).toEqual(DEFAULTS);
  });
});

describe("bigMode", () => {
  it("follows each moment's choice, and the master switch turns them all off", () => {
    const prefs = { ...DEFAULTS, moments: { reviews: "full", quiz: "pop", recall: "off" } as const };
    expect(bigMode("reviews", prefs)).toBe("full");
    expect(bigMode("quiz", prefs)).toBe("pop");
    expect(bigMode("recall", prefs)).toBe("off");
    expect(bigMode("reviews", { ...prefs, on: false })).toBe("off");
  });
});

// celebrations.feature (R151, R153): one session's small moments, at most one per answer.
describe("Pacer", () => {
  const minute = 60_000;
  const run = (pacer: Pacer, answers: boolean[], start = 0, step = 1000) => answers.map((right, index) => pacer.answered(right, start + index * step)).filter(Boolean);

  it("fires the count pops and markers for the queue it started with", () => {
    const pacer = new Pacer(64, { ...DEFAULTS, streaks: false }, 0);
    const events = run(pacer, Array(64).fill(true));
    expect(events).toEqual([{ kind: "count", done: 15 }, { kind: "halfway", done: 32 }, { kind: "stretch", done: 48 }]);
  });
  it("counts streaks of right answers, and a wrong answer resets it", () => {
    const pacer = new Pacer(10, DEFAULTS, 0);
    const events = run(pacer, [true, true, true, true, true, false, true, true, true, true, true]);
    expect(events).toEqual([{ kind: "streak", streak: 5 }, { kind: "streak", streak: 5 }]);
  });
  it("lets a streak win when two land on the same card", () => {
    const pacer = new Pacer(30, DEFAULTS, 0);
    const events = run(pacer, Array(15).fill(true));
    expect(events.at(-1)).toEqual({ kind: "streak", streak: 15 });
    expect(events.filter((event) => event!.kind === "streak")).toHaveLength(3);
  });
  it("nudges once after a quiet stretch, on the next right answer", () => {
    const pacer = new Pacer(10, { ...DEFAULTS, streaks: false, nudge: true, nudgeMinutes: 5 }, 0);
    expect(pacer.answered(true, 4 * minute)).toBeNull();
    expect(pacer.answered(false, 6 * minute)).toBeNull();
    expect(pacer.answered(true, 6 * minute)).toEqual({ kind: "nudge" });
    expect(pacer.answered(true, 7 * minute)).toBeNull();
  });
  it("is silent when celebrations are off", () => {
    const pacer = new Pacer(64, { ...DEFAULTS, on: false }, 0);
    expect(run(pacer, Array(64).fill(true))).toEqual([]);
  });
});

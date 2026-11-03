import { describe, expect, it } from "vitest";
import { dayLabel, dueLabel, onDay , dateOf, dayFromIso, dayOf, isoOf, monthGrid, monthLabel } from "./dates";

// Days are numbered from 1970-01-01 in local time, as the backend stores them.
const NOV_1_2026 = 20758;

describe("dayLabel", () => {
  it("says Today and Yesterday", () => {
    expect(dayLabel(NOV_1_2026, NOV_1_2026)).toBe("Today");
    expect(dayLabel(NOV_1_2026 - 1, NOV_1_2026)).toBe("Yesterday");
  });

  it("shows day and month this year", () => {
    expect(dayLabel(NOV_1_2026 - 3, NOV_1_2026)).toBe("29 Oct");
  });

  it("adds the year for other years", () => {
    expect(dayLabel(NOV_1_2026 - 365, NOV_1_2026)).toBe("1 Nov 2025");
  });
});

describe("onDay", () => {
  it("reads naturally mid-sentence", () => {
    expect(onDay(20_760, 20_760)).toBe("today");
    expect(onDay(20_761, 20_760)).toBe("tomorrow");
    expect(onDay(20_762, 20_760)).toBe("on Thu 5 Nov");
  });
});

describe("dueLabel", () => {
  const tue3Nov = 20_760;
  it("says Today for due and late, and Tomorrow", () => {
    expect(dueLabel(tue3Nov, tue3Nov)).toBe("Today");
    expect(dueLabel(tue3Nov - 2, tue3Nov)).toBe("Today");
    expect(dueLabel(tue3Nov + 1, tue3Nov)).toBe("Tomorrow");
  });
  it("adds the weekday this year and the year after", () => {
    expect(dueLabel(tue3Nov + 2, tue3Nov)).toBe("Thu 5 Nov");
    expect(dueLabel(tue3Nov + 92, tue3Nov)).toBe("3 Feb 2027");
  });
});

describe("calendar helpers", () => {
  // calendar.feature: See what's when (weeks start on Monday)
  it("lays a month out in Monday-to-Sunday weeks", () => {
    const grid = monthGrid(2026, 10); // November 2026
    expect(grid.length).toBe(42);
    expect(dateOf(grid[0])).toEqual({ year: 2026, month: 9, date: 26, weekday: 0 });
    expect(dateOf(grid[41])).toEqual({ year: 2026, month: 11, date: 6, weekday: 6 });
    expect(grid).toContain(20_760);
    expect(monthGrid(2027, 1).length).toBe(28); // February 2027 starts on a Monday and ends on a Sunday
  });

  it("converts between day numbers, dates and date inputs", () => {
    expect(dayOf(2026, 10, 3)).toBe(20_760);
    expect(dateOf(20_760)).toEqual({ year: 2026, month: 10, date: 3, weekday: 1 });
    expect(monthLabel(2026, 10)).toBe("November 2026");
    expect(isoOf(20_762)).toBe("2026-11-05");
    expect(dayFromIso("2026-11-05")).toBe(20_762);
  });
});

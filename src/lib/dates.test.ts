import { describe, expect, it } from "vitest";
import { dayLabel, dueLabel, onDay, pastOn, dateOf, dayFromIso, dayOf, isoOf, monthGrid, monthLabel, weekLabel, weekStart } from "./dates";

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

describe("pastOn", () => {
  it("reads mid-sentence for past days", () => {
    expect(pastOn(20_760, 20_760)).toBe("today");
    expect(pastOn(20_759, 20_760)).toBe("yesterday");
    expect(pastOn(20_754, 20_760)).toBe("on 28 Oct");
  });
});

// navigation.feature (R163): the week view starts on a Monday and says which week it is
describe("weeks", () => {
  it("finds the Monday of any day's week", () => {
    expect(dateOf(weekStart(20_760)).weekday).toBe(0);
    expect(weekStart(20_760)).toBe(20_759);
    expect(weekStart(20_759)).toBe(20_759);
    expect(weekStart(20_765)).toBe(20_759);
  });
  it("names the week, across months and years too", () => {
    expect(weekLabel(20_759)).toBe("2 to 8 Nov 2026");
    expect(weekLabel(dayOf(2026, 10, 30))).toBe("30 Nov to 6 Dec 2026");
    expect(weekLabel(dayOf(2026, 11, 28))).toBe("28 Dec 2026 to 3 Jan 2027");
  });
});

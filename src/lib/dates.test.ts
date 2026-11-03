import { describe, expect, it } from "vitest";
import { dayLabel, dueLabel } from "./dates";

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

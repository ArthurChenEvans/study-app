import { describe, expect, it } from "vitest";
import { dayLabel } from "./dates";

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

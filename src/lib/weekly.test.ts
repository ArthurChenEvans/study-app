import { describe, expect, it } from "vitest";
import { weekStart, weekly } from "./weekly";

// Day 0 (1 Jan 1970) was a Thursday, so day 20_759 is a Monday.
const TODAY = 20_760;

describe("weekly scores", () => {
  it("starts weeks on Monday", () => {
    expect(weekStart(TODAY)).toBe(20_759);
    expect(weekStart(20_759)).toBe(20_759);
    expect(weekStart(20_758)).toBe(20_752);
  });

  it("averages each week's quizzes, leaves out retries, and marks this week as partial", () => {
    const weeks = weekly([
      { day: 20_753, percent: 60, retry: false },
      { day: 20_755, percent: 80, retry: false },
      { day: 20_755, percent: 100, retry: true },
      { day: 20_760, percent: 91, retry: false },
    ], TODAY);
    expect(weeks).toEqual([
      { start: 20_752, percent: 70, quizzes: 2, partial: false },
      { start: 20_759, percent: 91, quizzes: 1, partial: true },
    ]);
  });
});

import { describe, expect, it } from "vitest";
import { repeatsLabel } from "./words";

// R214: a ladder step said the way you'd say it out loud
describe("repeatsLabel", () => {
  it("says how often for the usual steps", () => {
    expect(repeatsLabel("Day")).toBe("Every day");
    expect(repeatsLabel("Week")).toBe("Every week");
    expect(repeatsLabel("Fortnight")).toBe("Every 2 weeks");
    expect(repeatsLabel("Month")).toBe("Every month");
  });
  it("keeps where you are on a repeated step", () => {
    expect(repeatsLabel("Monthly 1 of 6")).toBe("Every month, 1 of 6");
    expect(repeatsLabel("Weekly 2 of 3")).toBe("Every week, 2 of 3");
  });
  it("leaves a step you named yourself as it is", () => {
    expect(repeatsLabel("Before the exam")).toBe("Before the exam");
  });
});

import { describe, expect, it } from "vitest";
import { keptLabel, madeDay, madeInline, madeLabel, sizeLabel, statusLine } from "./backup";
import type { BackupStatus } from "./api";

const TODAY = 20_761; // Wed 4 Nov 2026
const status = (overrides: Partial<BackupStatus> = {}): BackupStatus => ({
  latest: { name: "x", day: TODAY, minute: 552, reason: "Daily", bytes: 6_100_000 }, count: 14, bytes: 61_000_000, folder: "/b",
  settings: { keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: null }, copy_problem: null, has_data: true, ...overrides,
});

describe("backup words", () => {
  it("says when a backup was made", () => {
    expect(madeLabel({ day: TODAY, minute: 552 }, TODAY)).toBe("Today, 09:12");
    expect(madeLabel({ day: TODAY - 1, minute: 527 }, TODAY)).toBe("Yesterday, 08:47");
    expect(madeLabel({ day: TODAY - 2, minute: 1170 }, TODAY)).toBe("Mon 2 Nov, 19:30");
    expect(madeDay({ day: TODAY - 2 }, TODAY)).toBe("Mon 2 Nov");
    expect(madeInline({ day: TODAY, minute: 870 }, TODAY)).toBe("today, 14:30");
    expect(madeInline({ day: TODAY - 1, minute: 527 }, TODAY)).toBe("yesterday, 08:47");
    expect(madeInline({ day: TODAY - 2, minute: 1170 }, TODAY)).toBe("Mon 2 Nov, 19:30");
  });

  it("gives sizes in MB or KB", () => {
    expect(sizeLabel(6_100_000)).toBe("6.1 MB");
    expect(sizeLabel(820_400)).toBe("820 KB");
    expect(sizeLabel(12)).toBe("1 KB");
  });

  // R104, R108
  it("sums up the status in words", () => {
    expect(statusLine(status(), TODAY)).toEqual({ safe: true, short: "Backed up today", title: "Backed up today at 09:12" });
    expect(statusLine(status({ latest: { name: "x", day: TODAY - 3, minute: 0, reason: "Daily", bytes: 1 } }), TODAY)).toMatchObject({ safe: false, short: "Backed up 3 days ago" });
    expect(statusLine(status({ latest: null }), TODAY)).toMatchObject({ safe: false, short: "No backup yet" });
    expect(statusLine(status({ latest: null, has_data: false }), TODAY)).toMatchObject({ safe: true, short: "Nothing to back up yet" });
    expect(statusLine(status({ copy_problem: "Can't reach /media/usb" }), TODAY)).toMatchObject({ safe: false, short: "Backed up today, copy failed" });
  });

  it("says how many are kept", () => {
    expect(keptLabel({ keep_days: 7, keep_weeks: 4, keep_months: 12 })).toBe("The last 7 days, 4 weeks and 12 months");
    expect(keptLabel({ keep_days: 1, keep_weeks: 0, keep_months: 1 })).toBe("The last 1 day and 1 month");
    expect(keptLabel({ keep_days: 14, keep_weeks: 0, keep_months: 0 })).toBe("The last 14 days");
  });
});

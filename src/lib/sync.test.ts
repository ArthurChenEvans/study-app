import { describe, expect, it } from "vitest";
import { agoLabel, fieldLabel, groupChecks, syncLine, wordDiff } from "./sync";
import type { SyncStatus, ToCheck } from "./api";

// Wed 4 Nov 2026 at 20:00 UTC.
const NOW = 20_761 * 86_400 + 20 * 3600;
const status = (overrides: Partial<SyncStatus> = {}): SyncStatus => ({
  on: true, paused: null, folder: "/home/chen/Sync/StudyApp", device: "aaaa0001", name: "Desktop",
  devices: [
    { id: "aaaa0001", name: "Desktop", this_one: true, last_seen: NOW, waiting: 0, newer_app: false },
    { id: "bbbb0002", name: "Laptop", this_one: false, last_seen: NOW - 120, waiting: 0, newer_app: false },
  ],
  last_synced: NOW - 120, problem: null, waiting: 0, held: 0, to_check: 0, ...overrides,
});

describe("sync words", () => {
  it("says how long ago, in words", () => {
    expect(agoLabel(NOW - 20, NOW)).toBe("just now");
    expect(agoLabel(NOW - 60, NOW)).toBe("a minute ago");
    expect(agoLabel(NOW - 125, NOW)).toBe("2 minutes ago");
    expect(agoLabel(NOW - 3 * 3600, NOW)).toBe("3 hours ago");
    expect(agoLabel(NOW - 22 * 3600 + 600, NOW)).toBe("yesterday at 22:10");
    expect(agoLabel(NOW - 2 * 86_400, NOW)).toBe("Mon 2 Nov");
  });

  // R128: in words, never only a colour
  it("sums up sync in one line for each state", () => {
    expect(syncLine(status(), NOW)).toEqual({ ok: true, short: "Synced 2 minutes ago", title: "Synced 2 minutes ago", detail: "With Laptop.", icon: "on" });
    expect(syncLine(status({ on: false }), NOW)).toMatchObject({ ok: true, short: "Sync is off", detail: "Only this computer has your data." });
    expect(syncLine(status({ paused: "Paused after restoring a backup" }), NOW)).toMatchObject({ ok: false, short: "Sync paused", title: "Paused after restoring a backup" });
    expect(syncLine(status({ problem: "Can't find the sync folder at /x. Changes here are kept and go out once it's back.", waiting: 37 }), NOW)).toEqual({
      ok: false, short: "Sync paused", title: "Sync paused: can't find the sync folder", detail: "Last synced 2 minutes ago. 37 changes here are kept and go out once it's back.", icon: "missing",
    });
    expect(syncLine(status({ to_check: 1 }), NOW)).toMatchObject({ ok: false, short: "Synced, 1 to check", title: "Synced 2 minutes ago, 1 thing to check", icon: "check" });
    expect(syncLine(status({ problem: "Laptop has a newer version of the app." }), NOW)).toMatchObject({ ok: false, short: "Synced, with a problem", detail: "Laptop has a newer version of the app." });
    expect(syncLine(status({ last_synced: null, devices: [status().devices[0]] }), NOW)).toMatchObject({ short: "Waiting for the other computer", detail: "Nothing else has joined this folder yet." });
  });

  // R123: what differs, marked by more than colour (underlined and struck through)
  it("finds the words that differ between two versions", () => {
    expect(wordDiff("About 3 cycles at L1, 10 at L2", "About 4 cycles at L1, 12 at L2")).toEqual({
      older: [{ text: "About ", kind: "same" }, { text: "3", kind: "removed" }, { text: " cycles at L1, ", kind: "same" }, { text: "10", kind: "removed" }, { text: " at L2", kind: "same" }],
      newer: [{ text: "About ", kind: "same" }, { text: "4", kind: "added" }, { text: " cycles at L1, ", kind: "same" }, { text: "12", kind: "added" }, { text: " at L2", kind: "same" }],
    });
    expect(wordDiff("", "New")).toEqual({ older: [], newer: [{ text: "New", kind: "added" }] });
  });

  it("names fields the way the app does", () => {
    expect(fieldLabel("note.back")).toBe("Back");
    expect(fieldLabel("text")).toBe("Question");
    expect(fieldLabel("subject_name")).toBe("What it covers");
    expect(fieldLabel("something_new")).toBe("Something new");
  });

  it("puts fields of the same thing together", () => {
    const check = (n: number, record: string, field: string): ToCheck => ({ n, record, what: "Card", label: "L1", context: null, field, deleted: false, kept: "a", other: "b", kept_by: "Laptop", kept_when: NOW, other_by: "Desktop", other_when: NOW - 60 });
    const groups = groupChecks([check(1, "notes:x", "note.back"), check(2, "questions:y", "text"), check(3, "notes:x", "note.front")]);
    expect(groups.map((group) => [group.record, group.fields.map((field) => field.n)])).toEqual([["notes:x", [1, 3]], ["questions:y", [2]]]);
  });
});

import { beforeEach, describe, expect, it } from "vitest";
import { matchesFilter, savedColumns, saveColumns, sortBy } from "./lists";

describe("lists", () => {
  beforeEach(() => localStorage.clear());

  it("filters on any field, ignoring case", () => {
    expect(matchesFilter("", "CPU")).toBe(true);
    expect(matchesFilter("cach", "Caches", null)).toBe(true);
    expect(matchesFilter("net", "CPU", "Networks")).toBe(true);
    expect(matchesFilter("kerberos", "CPU", "Networks")).toBe(false);
  });

  it("sorts both ways with nothing last", () => {
    const items = [{ n: "b", v: 2 }, { n: "a", v: null }, { n: "c", v: 1 }];
    expect(sortBy(items, (i) => i.v, true, (i) => i.n).map((i) => i.n)).toEqual(["c", "b", "a"]);
    expect(sortBy(items, (i) => i.v, false, (i) => i.n).map((i) => i.n)).toEqual(["b", "c", "a"]);
    expect(sortBy(items, (i) => i.n, true).map((i) => i.n)).toEqual(["a", "b", "c"]);
  });

  // lists.feature: Sort and pick columns (still on next time)
  it("remembers columns", () => {
    expect(savedColumns("cols", { tags: false })).toEqual({ tags: false });
    saveColumns("cols", { tags: true });
    expect(savedColumns("cols", { tags: false, step: true })).toEqual({ tags: true, step: true });
  });
});

import { describe, expect, it } from "vitest";
import { closeTab, kindOf, numberLines, openTab, type Tab } from "./preview";

// sources.feature: Open a file beside the editor (outline)
describe("kindOf", () => {
  it.each([
    ["lecture-3.pdf", "pdf"],
    ["Lecture 3.PPTX", "slides"],
    ["old.ppt", "slides"],
    ["deck.odp", "slides"],
    ["book.epub", "epub"],
    ["notes.md", "markdown"],
    ["notes.markdown", "markdown"],
    ["notes.txt", "text"],
  ])("%s is %s", (name, kind) => {
    expect(kindOf(name)).toBe(kind);
  });

  it("rejects anything else", () => {
    expect(kindOf("photo.png")).toBeNull();
    expect(kindOf("README")).toBeNull();
  });
});

// sources.feature: Markdown and txt show line numbers
describe("numberLines", () => {
  it("numbers every line from 1", () => {
    expect(numberLines("first\nsecond\r\n\nfourth")).toEqual([
      { number: 1, text: "first" },
      { number: 2, text: "second" },
      { number: 3, text: "" },
      { number: 4, text: "fourth" },
    ]);
  });
});

// sources.feature: Several files in tabs
describe("tabs", () => {
  const tab = (id: number, name: string): Tab => ({ id, name, kind: kindOf(name)! });

  it("opens a file as a new active tab", () => {
    const first = openTab([], tab(1, "a.pdf"));
    const second = openTab(first.tabs, tab(2, "b.pptx"));
    expect(second.tabs.map((t) => t.name)).toEqual(["a.pdf", "b.pptx"]);
    expect(second.active).toBe(2);
  });

  it("closing the active tab selects its neighbour", () => {
    const tabs = [tab(1, "a.pdf"), tab(2, "b.md"), tab(3, "c.txt")];
    expect(closeTab(tabs, 2, 2)).toEqual({ tabs: [tabs[0], tabs[2]], active: 3 });
    expect(closeTab(tabs, 3, 3)).toEqual({ tabs: [tabs[0], tabs[1]], active: 2 });
    expect(closeTab([tabs[0]], 1, 1)).toEqual({ tabs: [], active: null });
  });

  it("closing another tab keeps the active one", () => {
    const tabs = [tab(1, "a.pdf"), tab(2, "b.md")];
    expect(closeTab(tabs, 2, 1)).toEqual({ tabs: [tabs[1]], active: 2 });
  });
});

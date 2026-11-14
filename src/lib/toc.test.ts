import { beforeEach, describe, expect, it } from "vitest";
import { locationTarget, markdownHeadings, pageHeading, pageHeadings } from "./toc";
import { fileFor, forget, remember } from "./sourceFiles.svelte";

// sources.feature: A table of contents (R179)
describe("Markdown headings", () => {
  it("lists headings with their depth and line, and skips code blocks", () => {
    const text = "# Networks\nintro\n## DNS\n```\n# not a heading\n```\n### Records\n#hashtag\n## TLS";
    expect(markdownHeadings(text)).toEqual([
      { title: "Networks", depth: 1, line: 1 },
      { title: "DNS", depth: 2, line: 3 },
      { title: "Records", depth: 3, line: 7 },
      { title: "TLS", depth: 2, line: 9 },
    ]);
  });
});

// sources.feature: Showing the page a question cites (R181)
describe("where a citation points", () => {
  it.each([
    ["p. 42", { page: 42 }],
    ["pp. 42-45", { page: 42 }],
    ["page 7", { page: 7 }],
    ["42", { page: 42 }],
    ["slide 3", { page: 3 }],
    ["Slides 3–5", { page: 3 }],
    ["line 120", { line: 120 }],
    ["l. 12", { line: 12 }],
    ["ch. 2", null],
    ["", null],
  ])("%s", (location, target) => expect(locationTarget(location)).toEqual(target));
});

// sources.feature: Remembering which file goes with a source (R182)
describe("which file goes with a source", () => {
  beforeEach(() => forget(5));
  it("remembers by file name only", () => {
    expect(fileFor(5)).toBeNull();
    remember(5, "Lecture 3.pdf");
    expect(fileFor(5)).toBe("Lecture 3.pdf");
    expect(JSON.parse(localStorage.getItem("source-files") ?? "{}")).toEqual({ 5: "Lecture 3.pdf" });
  });
});

// R197: a PDF with no outline gets contents from each page's heading, its largest text
describe("page headings", () => {
  const item = (str: string, height: number) => ({ str, height });
  it("takes the page's biggest text when it stands out", () => {
    expect(pageHeading([item("Lecture 5", 12), item("Virtual memory", 28), item("and", 12), item("paging", 28), item("body text", 12), item("more body", 12)])).toBe("Virtual memory paging");
    expect(pageHeading([item("all the same size", 12), item("still the same", 12)])).toBeNull();
    expect(pageHeading([])).toBeNull();
  });

  it("drops a heading that repeats on every page, like a running title", () => {
    const pages = [
      [item("Course header", 20), item("Intro", 30), item("body", 11)],
      [item("Course header", 20), item("Paging", 30), item("body", 11)],
      [item("Course header", 20), item("Paging", 30), item("body", 11)],
      [item("Course header", 20), item("body only here", 11), item("x", 11)],
    ];
    expect(pageHeadings(pages)).toEqual([{ title: "Intro", depth: 1, at: 1 }, { title: "Paging", depth: 1, at: 2 }]);
  });
});

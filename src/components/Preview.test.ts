import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { slidesToPdf: vi.fn() } };
});

import { api } from "../lib/api";
import { fileFor, forget, remember, showCitation } from "../lib/sourceFiles.svelte";
import Preview from "./Preview.svelte";

const mocked = vi.mocked(api);
const file = (name: string, text = "first line\nsecond line") => new File([text], name);

async function open(...files: File[]) {
  const u = userEvent.setup({ applyAccept: false });
  await u.upload(screen.getByLabelText("Open file"), files);
  return u;
}

beforeEach(() => {
  vi.resetAllMocks();
  forget(5);
  Element.prototype.scrollIntoView = vi.fn();
});
const NOTES = "# Networks\nintro\n## DNS\nnames\n## TLS\nhandshakes";
const current = (list: HTMLElement) => within(list).getAllByRole("listitem").find((line) => line.getAttribute("aria-current") === "location")?.textContent;

describe("Source preview", () => {
  it("starts empty with a way to open a file", () => {
    render(Preview);
    expect(screen.getByText("Read alongside as you write")).toBeInTheDocument();
    expect(screen.getByLabelText("Open file")).toHaveAttribute("accept", ".pdf,.pptx,.ppt,.odp,.epub,.md,.markdown,.txt");
  });

  // sources.feature: Markdown and txt show line numbers
  it("shows Markdown and text with line numbers", async () => {
    render(Preview);
    await open(file("notes.md"));
    const lines = await screen.findByRole("list", { name: "notes.md" });
    expect(within(lines).getAllByRole("listitem").map((line) => line.textContent)).toEqual(["1first line", "2second line"]);
  });

  it("refuses files it can't show", async () => {
    render(Preview);
    await open(file("photo.png"));
    expect(await screen.findByRole("alert")).toHaveTextContent("photo.png can't be previewed");
  });

  // sources.feature: PowerPoint is converted to PDF first
  it("converts slides and explains when it can't", async () => {
    mocked.slidesToPdf.mockRejectedValue({ message: "install LibreOffice to open slides, or export them as PDF" });
    render(Preview);
    await open(file("Lecture 3.pptx"));
    expect(mocked.slidesToPdf).toHaveBeenCalled();
    expect(await screen.findByRole("alert")).toHaveTextContent("install LibreOffice to open slides");
  });

  // sources.feature: Several files in tabs
  it("opens several files as tabs and closes them", async () => {
    render(Preview);
    const u = await open(file("a.md", "alpha"), file("b.txt", "bravo"));
    expect(screen.getByRole("tab", { name: "b.txt", selected: true })).toBeInTheDocument();
    await u.click(screen.getByRole("tab", { name: "a.md" }));
    expect(await screen.findByRole("list", { name: "a.md" })).toHaveTextContent("alpha");
    await u.click(screen.getByRole("button", { name: "Close a.md" }));
    expect(screen.queryByRole("tab", { name: "a.md" })).not.toBeInTheDocument();
    expect(await screen.findByRole("list", { name: "b.txt" })).toHaveTextContent("bravo");
  });

  // sources.feature: A table of contents (R179)
  it("lists a Markdown file's headings and jumps to one", async () => {
    render(Preview);
    const u = await open(file("notes.md", NOTES));
    await u.click(await screen.findByRole("button", { name: "Contents" }));
    const contents = screen.getByRole("navigation", { name: "Contents of notes.md" });
    await u.click(within(contents).getByRole("button", { name: "TLS" }));
    expect(current(screen.getByRole("list", { name: "notes.md" }))).toBe("5## TLS");
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  });

  // sources.feature: Going straight to a page (R180)
  it("goes to a line with Ctrl+G", async () => {
    render(Preview);
    const u = await open(file("notes.md", NOTES));
    await screen.findByRole("list", { name: "notes.md" });
    await u.keyboard("{Control>}g{/Control}");
    const box = screen.getByRole("spinbutton", { name: "Go to line" });
    expect(box).toHaveFocus();
    await u.type(box, "4{Enter}");
    expect(current(screen.getByRole("list", { name: "notes.md" }))).toBe("4names");
  });

  // sources.feature: Showing where a question cites (R181, R182)
  it("asks once which open file is the cited source, then shows the line", async () => {
    render(Preview);
    const u = await open(file("notes.md", NOTES), file("other.txt", "x"));
    showCitation(5, "Smith (2024)", "line 3");
    const ask = await screen.findByRole("group", { name: "Which open file is Smith (2024)?" });
    await u.selectOptions(within(ask).getByRole("combobox", { name: "File" }), "notes.md");
    await u.click(within(ask).getByRole("button", { name: "Show line 3" }));
    expect(fileFor(5)).toBe("notes.md");
    expect(screen.getByRole("tab", { name: "notes.md", selected: true })).toBeInTheDocument();
    expect(current(await screen.findByRole("list", { name: "notes.md" }))).toBe("3## DNS");
    await u.click(screen.getByRole("tab", { name: "other.txt" }));
    showCitation(5, "Smith (2024)", "line 5");
    expect(current(await screen.findByRole("list", { name: "notes.md" }))).toBe("5## TLS");
  });

  it("says which file to open when the cited one isn't open", async () => {
    remember(5, "Lecture 3.pdf");
    render(Preview);
    showCitation(5, "Smith (2024)", "p. 42");
    expect(await screen.findByRole("status")).toHaveTextContent("Open Lecture 3.pdf to see page 42 of Smith (2024).");
    expect(screen.getByRole("button", { name: "Open Lecture 3.pdf…" })).toBeInTheDocument();
  });
});

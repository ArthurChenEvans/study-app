import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { slidesToPdf: vi.fn(), fetchPage: vi.fn(), sources: vi.fn(), addSource: vi.fn(), updateSource: vi.fn(), deleteSource: vi.fn() } };
});

import { api } from "../lib/api";
import { fileFor, forget, remember, showCitation } from "../lib/sourceFiles.svelte";
import { readerDrop } from "../lib/readerDrop.svelte";
import { fireEvent } from "@testing-library/svelte";
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
  mocked.sources.mockResolvedValue([]);
  mocked.addSource.mockResolvedValue(77);
  forget(5);
  Element.prototype.scrollIntoView = vi.fn();
});
const NOTES = "# Networks\nintro\n## DNS\nnames\n## TLS\nhandshakes";
const current = (list: HTMLElement) => within(list).getAllByRole("listitem").find((line) => line.getAttribute("aria-current") === "location")?.textContent;

describe("Source preview", () => {
  // R201: a web page opens as a reading view, with its headings as contents
  it("opens a link as a reading view with contents and a way to the browser", async () => {
    const u = userEvent.setup();
    mocked.fetchPage.mockResolvedValue({ url: "https://blog.example.com/dns", html: "<title>DNS | Blog</title><body><nav>Menu</nav><article><h1>How DNS works</h1><p>Resolvers ask.</p><h2>Caching</h2><p>TTL.</p></article></body>" });
    render(Preview);
    await u.click(screen.getByRole("button", { name: "Open a link" }));
    const box = screen.getByRole("textbox", { name: "Web address" });
    await u.type(box, "blog.example.com/dns{Enter}");
    expect(mocked.fetchPage).toHaveBeenCalledWith("blog.example.com/dns");
    const article = await screen.findByRole("article", { name: "How DNS works" });
    expect(article).toHaveTextContent("Resolvers ask.");
    expect(article).not.toHaveTextContent("Menu");
    expect(screen.getByRole("tab", { name: /How DNS works/ })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Open in browser/ })).toHaveAttribute("href", "https://blog.example.com/dns");
    const contents = screen.getByRole("navigation", { name: "Contents of How DNS works" });
    await u.click(within(contents).getByRole("button", { name: "Caching" }));
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  });

  // R202: anything opened becomes a source, filled in from its details, to check when you like
  it("adds what you open to your sources and lets you check it", async () => {
    const u = userEvent.setup();
    mocked.fetchPage.mockResolvedValue({ url: "https://blog.example.com/dns", html: '<head><meta property="og:site_name" content="Example Blog"></head><article><h1>How DNS works</h1></article>' });
    render(Preview);
    await u.click(screen.getByRole("button", { name: "Open a link" }));
    await u.type(screen.getByRole("textbox", { name: "Web address" }), "blog.example.com/dns{Enter}");
    const note = await screen.findByRole("status", { name: "New source" });
    expect(mocked.addSource).toHaveBeenCalledWith({ authors: "", year: null, title: "How DNS works", edition: null, description: null, publisher: "Example Blog", url: "https://blog.example.com/dns" });
    expect(note).toHaveTextContent("Added to your sources");
    await u.click(within(note).getByRole("button", { name: "Check it" }));
    const dialog = screen.getByRole("dialog", { name: "Check this source" });
    expect(within(dialog).getByText(/Filled in from https:\/\/blog.example.com\/dns/)).toBeInTheDocument();
    expect(within(dialog).getByLabelText(/^Title/)).toHaveValue("How DNS works");
    expect(within(dialog).getByText("From the file", { selector: "[data-field=title] *" })).toBeInTheDocument();
    expect(within(dialog).getByText("Not in the file", { selector: "[data-field=authors] *" })).toBeInTheDocument();
    await u.type(within(dialog).getByLabelText(/^Authors/), "Doe, J.");
    expect(within(dialog).getByText(/Doe, J\. \(n\.d\.\)/)).toBeInTheDocument();
    await u.click(within(dialog).getByRole("button", { name: "Save source" }));
    expect(mocked.updateSource).toHaveBeenCalledWith(77, expect.objectContaining({ authors: "Doe, J.", title: "How DNS works" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.queryByRole("status", { name: "New source" })).not.toBeInTheDocument();
  });

  it("takes the new source back if you don't keep it, and doesn't add one it already has", async () => {
    const u = userEvent.setup();
    mocked.deleteSource.mockResolvedValue(0);
    render(Preview);
    await open(file("networks.md", NOTES));
    await u.click(within(await screen.findByRole("status", { name: "New source" })).getByRole("button", { name: "Check it" }));
    await u.click(screen.getByRole("button", { name: "Don't keep it" }));
    expect(mocked.deleteSource).toHaveBeenCalledWith(77);
    mocked.addSource.mockClear();
    mocked.sources.mockResolvedValue([{ id: 3, fields: { authors: "", year: null, title: "networks", edition: null, description: null, publisher: null, url: null }, apa: "", used_by: [], cited_by_questions: 0 }]);
    await open(file("networks.md", NOTES));
    await screen.findAllByRole("tab", { name: /networks\.md/ });
    await new Promise((done) => setTimeout(done, 20));
    expect(mocked.addSource).not.toHaveBeenCalled();
  });

  it("says why a link couldn't open", async () => {
    const u = userEvent.setup();
    mocked.fetchPage.mockRejectedValue({ message: "that page wasn't found (404)" });
    render(Preview);
    await u.click(screen.getByRole("button", { name: "Open a link" }));
    await u.type(screen.getByRole("textbox", { name: "Web address" }), "example.com/gone{Enter}");
    expect(await screen.findByRole("alert")).toHaveTextContent("that page wasn't found (404)");
  });

  it("starts empty with a way to open a file", () => {
    render(Preview);
    expect(screen.getByText("Read alongside as you write")).toBeInTheDocument();
    expect(screen.getByLabelText("Open file")).toHaveAttribute("accept", ".pdf,.pptx,.ppt,.odp,.epub,.md,.markdown,.txt");
  });

  // R195: a file dropped on the reader opens there instead of starting an import
  it("opens a file dropped on it, in the browser and from the desktop app's drop", async () => {
    render(Preview);
    const reader = document.querySelector("[data-reader-drop]")!;
    expect(reader).not.toBeNull();
    const dataTransfer = { files: [file("notes.md", "# Dropped")], types: ["Files"] };
    await fireEvent.drop(reader, { dataTransfer });
    expect(await screen.findByRole("tab", { name: /notes\.md/ })).toBeInTheDocument();
    readerDrop.files = [file("more.txt")];
    expect(await screen.findByRole("tab", { name: /more\.txt/ })).toBeInTheDocument();
    expect(readerDrop.files).toEqual([]);
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
  // R197: the contents sit beside the file and stay open, with a search box
  it("lists a Markdown file's headings and jumps to one", async () => {
    localStorage.clear();
    render(Preview);
    const u = await open(file("notes.md", NOTES));
    const contents = await screen.findByRole("navigation", { name: "Contents of notes.md" });
    expect(screen.getByRole("button", { name: "Contents" })).toHaveAttribute("aria-expanded", "true");
    await u.type(within(contents).getByRole("searchbox", { name: "Find a heading" }), "tl");
    expect(within(contents).queryByRole("button", { name: "DNS" })).not.toBeInTheDocument();
    await u.click(within(contents).getByRole("button", { name: "TLS" }));
    expect(current(screen.getByRole("list", { name: "notes.md" }))).toBe("5## TLS");
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
    await u.click(screen.getByRole("button", { name: "Contents" }));
    expect(screen.queryByRole("navigation", { name: "Contents of notes.md" })).not.toBeInTheDocument();
    expect(localStorage.getItem("studyapp-contents")).toBe("shut");
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

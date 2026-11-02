import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { slidesToPdf: vi.fn() } };
});

import { api } from "../lib/api";
import Preview from "./Preview.svelte";

const mocked = vi.mocked(api);
const file = (name: string, text = "first line\nsecond line") => new File([text], name);

async function open(...files: File[]) {
  const u = userEvent.setup({ applyAccept: false });
  await u.upload(screen.getByLabelText("Open file"), files);
  return u;
}

beforeEach(() => vi.resetAllMocks());

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
});

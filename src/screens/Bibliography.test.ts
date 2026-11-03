import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { sources: vi.fn(), topics: vi.fn(), addSource: vi.fn(), updateSource: vi.fn(), deleteSource: vi.fn() } };
});

import { api, type SourceRow } from "../lib/api";
import Bibliography from "./Bibliography.svelte";

const mocked = vi.mocked(api);
const kirk: SourceRow = {
  id: 1,
  fields: { authors: "Kirk, D. B., & Hwu, W. W.", year: "2022", title: "Programming massively parallel processors", edition: "4th", description: null, publisher: "Morgan Kaufmann", url: null },
  apa: "Kirk, D. B., & Hwu, W. W. (2022). *Programming massively parallel processors* (4th ed.). Morgan Kaufmann.",
  used_by: [1, 2],
  cited_by_questions: 7,
};

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topics.mockResolvedValue([
    { id: 1, name: "GPU", module: null, tags: [], question_count: 5, short_count: 0 },
    { id: 2, name: "CPU", module: null, tags: [], question_count: 5, short_count: 0 },
  ]);
});

describe("Bibliography", () => {
  // sources.feature: Add a source to the bibliography / See which topics use a source
  it("lists sources in APA with where they're used", async () => {
    mocked.sources.mockResolvedValue([kirk]);
    render(Bibliography);
    const entry = await screen.findByRole("listitem", { name: /Kirk/ });
    expect(within(entry).getByText("Programming massively parallel processors").tagName).toBe("EM");
    expect(entry).toHaveTextContent("Cited by 7 questions");
    expect(within(entry).getByText("GPU")).toBeInTheDocument();
    expect(within(entry).getByText("CPU")).toBeInTheDocument();
  });

  it("has an empty state", async () => {
    mocked.sources.mockResolvedValue([]);
    render(Bibliography);
    expect(await screen.findByText("No sources yet")).toBeInTheDocument();
  });

  it("adds a source, leaving blank fields out", async () => {
    const u = userEvent.setup();
    mocked.sources.mockResolvedValue([]);
    mocked.addSource.mockResolvedValue(2);
    render(Bibliography);
    await u.click(await screen.findByRole("button", { name: "Add your first source" }));
    const dialog = screen.getByRole("dialog", { name: "Add source" });
    await u.type(within(dialog).getByLabelText("Author(s)"), "University College Isle of Man");
    await u.type(within(dialog).getByLabelText("Year"), "2026");
    await u.type(within(dialog).getByLabelText("Title"), "Lecture 3: The processor pipeline");
    await u.type(within(dialog).getByLabelText("Description"), "Lecture slides");
    mocked.sources.mockResolvedValue([kirk]);
    await u.click(within(dialog).getByRole("button", { name: "Add source" }));
    expect(mocked.addSource).toHaveBeenCalledWith({
      authors: "University College Isle of Man", year: "2026", title: "Lecture 3: The processor pipeline",
      edition: null, description: "Lecture slides", publisher: null, url: null,
    });
    expect(await screen.findByRole("listitem", { name: /Kirk/ })).toBeInTheDocument();
  });

  it("shows why a source couldn't be added", async () => {
    const u = userEvent.setup();
    mocked.sources.mockResolvedValue([]);
    mocked.addSource.mockRejectedValue({ message: "give the source a title" });
    render(Bibliography);
    await u.click(await screen.findByRole("button", { name: "Add your first source" }));
    await u.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Add source" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("give the source a title");
  });

  // R45
  it("edits a source", async () => {
    mocked.sources.mockResolvedValue([kirk]);
    const u = userEvent.setup();
    render(Bibliography);
    await u.click(await screen.findByRole("button", { name: "Edit source" }));
    const year = screen.getByLabelText("Year");
    expect(year).toHaveValue("2022");
    await u.clear(year);
    await u.type(year, "2023");
    await u.click(screen.getByRole("button", { name: "Save source" }));
    expect(mocked.updateSource).toHaveBeenCalledWith(1, { ...kirk.fields, year: "2023" });
  });

  // R44
  it("deletes a source after saying how many questions cite it", async () => {
    mocked.sources.mockResolvedValue([kirk]);
    mocked.deleteSource.mockResolvedValue(7);
    const u = userEvent.setup();
    render(Bibliography);
    await u.click(await screen.findByRole("button", { name: "Delete source" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent("7 questions cite it");
    mocked.sources.mockResolvedValue([]);
    await u.click(within(dialog).getByRole("button", { name: "Delete source" }));
    expect(mocked.deleteSource).toHaveBeenCalledWith(1);
    expect(await screen.findByText("No sources yet")).toBeInTheDocument();
  });
});

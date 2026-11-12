import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { search: vi.fn(), modules: vi.fn(), today: vi.fn() } };
});

import { api, type SearchQuery } from "../lib/api";
import Search from "./Search.svelte";

const mocked = vi.mocked(api);
const lastQuery = (): SearchQuery => mocked.search.mock.calls.at(-1)![0];

beforeEach(() => {
  vi.resetAllMocks();
  mocked.modules.mockResolvedValue([{ id: 1, name: "Computer Architecture" }]);
  mocked.today.mockResolvedValue(20759); // 2 Nov 2026
  mocked.search.mockResolvedValue([
    { Question: { id: 10, text: "What is a shader core?", topic: 2, topic_name: "GPU", tags: ["hardware"], edited: "2026-11-02 10:15:00" } },
    { Tag: { name: "hardware" } },
  ]);
});

describe("Search", () => {
  // search.feature: Search as you type
  it("searches as you type", async () => {
    const u = userEvent.setup();
    render(Search, { onOpenTopic: vi.fn() });
    await u.type(await screen.findByRole("searchbox", { name: "Search" }), "shad");
    expect(lastQuery()).toEqual({ text: "shad", tab: "All", module: null, tag: null, sort: "Edited", descending: true });
    const results = await screen.findByRole("list", { name: "Questions" });
    expect(within(results).getByRole("button", { name: "What is a shader core?" })).toBeInTheDocument();
    expect(within(results).getByText("Today")).toBeInTheDocument();
    expect(within(results).getByText("shad").tagName).toBe("MARK");
    expect(screen.getByRole("list", { name: "Other matches" })).toHaveTextContent("hardware");
  });

  // search.feature: Search across categories
  it("switches category tabs", async () => {
    const u = userEvent.setup();
    render(Search, { onOpenTopic: vi.fn() });
    await u.click(await screen.findByRole("tab", { name: "Topics" }));
    expect(lastQuery().tab).toBe("Topics");
    expect(screen.getByRole("tab", { name: "Topics" })).toHaveAttribute("aria-selected", "true");
    await u.click(screen.getByRole("tab", { name: "Questions" }));
    expect(lastQuery().tab).toBe("Questions");
  });

  // search.feature: Combine filters
  it("combines a module and a tag filter", async () => {
    const u = userEvent.setup();
    render(Search, { onOpenTopic: vi.fn() });
    await u.selectOptions(await screen.findByLabelText("Module"), "Computer Architecture");
    await u.type(screen.getByLabelText("Tag"), "hardware");
    expect(lastQuery()).toMatchObject({ module: 1, tag: "hardware" });
  });

  // search.feature: Sort results
  it("sorts by a column, then flips the direction", async () => {
    const u = userEvent.setup();
    render(Search, { onOpenTopic: vi.fn() });
    await u.click(await screen.findByRole("radio", { name: "Topic" }));
    expect(lastQuery()).toMatchObject({ sort: "Topic", descending: false });
    await u.click(screen.getByRole("button", { name: /switch to descending/ }));
    expect(lastQuery()).toMatchObject({ sort: "Topic", descending: true });
  });

  it("shows where a question matched, and moves through results with the arrow keys", async () => {
    const u = userEvent.setup();
    mocked.search.mockResolvedValue([
      { Question: { id: 10, text: "What does the TLB cache?", topic: 2, topic_name: "Memory", tags: [], edited: "2026-11-02 10:15:00", matched: null } },
      { Question: { id: 11, text: "What does a branch predictor guess?", topic: 3, topic_name: "CPU", tags: [], edited: "2026-11-02 10:15:00", matched: { Option: "Which cache line to evict" } } },
    ]);
    render(Search, { onOpenTopic: vi.fn() });
    const box = await screen.findByRole("searchbox", { name: "Search" });
    await u.type(box, "cache");
    const second = await screen.findByRole("button", { name: "What does a branch predictor guess? Option: Which cache line to evict" });
    expect(screen.getByText("2 questions match \"cache\"")).toBeInTheDocument();
    await u.keyboard("{ArrowDown}");
    expect(screen.getByRole("button", { name: "What does the TLB cache?" })).toHaveFocus();
    await u.keyboard("{ArrowDown}");
    expect(second).toHaveFocus();
    await u.keyboard("{ArrowUp}{ArrowUp}");
    expect(box).toHaveFocus();
  });

  it("opens a question's topic", async () => {
    const onOpenTopic = vi.fn();
    render(Search, { onOpenTopic });
    await userEvent.click(await screen.findByRole("button", { name: "What is a shader core?" }));
    expect(onOpenTopic).toHaveBeenCalledWith(2);
  });

  it("says when nothing matches", async () => {
    const u = userEvent.setup();
    mocked.search.mockResolvedValue([]);
    render(Search, { onOpenTopic: vi.fn() });
    await u.type(await screen.findByRole("searchbox"), "tensor core");
    expect(await screen.findByText('No matches for "tensor core"')).toBeInTheDocument();
  });
});

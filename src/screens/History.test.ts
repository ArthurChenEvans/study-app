import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { history: vi.fn(), topics: vi.fn(), today: vi.fn() } };
});

import { api, type HistoryRow } from "../lib/api";
import History from "./History.svelte";

const mocked = vi.mocked(api);
const TODAY = 20758;
const tally = (right_sure: number, right_guessed: number, wrong: number, unanswered = 0) => ({ right_sure, right_guessed, wrong, wrong_flagged: 0, unanswered });
const row = (id: number, day: number, overall: ReturnType<typeof tally>, extra: Partial<HistoryRow> = {}): HistoryRow => ({ id, day, status: "Finished", retry_of: null, topics: [1, 2], summary: { overall, per_topic: {} }, ...extra });

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.topics.mockResolvedValue([
    { id: 1, name: "CPU", module: null, tags: [], question_count: 10, short_count: 0 },
    { id: 2, name: "GPU", module: null, tags: [], question_count: 10, short_count: 0 },
  ]);
});

describe("History", () => {
  it("has an empty state", async () => {
    mocked.history.mockResolvedValue([]);
    render(History, { onView: vi.fn() });
    expect(await screen.findByText("No quizzes yet")).toBeInTheDocument();
  });

  // history.feature: Attempts listed in history
  it("lists attempts newest first with retries and incomplete ones marked", async () => {
    mocked.history.mockResolvedValue([
      row(3, TODAY, tally(5, 1, 1), { retry_of: 2 }),
      row(2, TODAY - 1, tally(13, 3, 4)),
      row(1, TODAY - 5, tally(3, 1, 2, 9), { status: "Incomplete", topics: [1] }),
    ]);
    render(History, { onView: vi.fn() });
    const rows = await screen.findAllByRole("row", { name: /^Quiz/ });
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Today");
    expect(rows[0]).toHaveTextContent("CPU, GPU");
    expect(rows[0]).toHaveTextContent("6 / 7");
    expect(within(rows[0]).getByText("Retry")).toBeInTheDocument();
    expect(rows[1]).toHaveTextContent("Yesterday");
    expect(rows[2]).toHaveTextContent("4 / 15");
    expect(within(rows[2]).getByText("Incomplete")).toBeInTheDocument();
  });

  it("charts every attempt oldest first", async () => {
    mocked.history.mockResolvedValue([row(2, TODAY, tally(8, 0, 2)), row(1, TODAY - 1, tally(5, 0, 5))]);
    render(History, { onView: vi.fn() });
    expect(await screen.findByRole("img", { name: "Score per quiz, from 50% to 80%" })).toBeInTheDocument();
  });

  it("opens an attempt's results", async () => {
    const onView = vi.fn();
    mocked.history.mockResolvedValue([row(7, TODAY, tally(1, 0, 0))]);
    render(History, { onView });
    await userEvent.click(await screen.findByRole("button", { name: /^View results from / }));
    expect(onView).toHaveBeenCalledWith(7);
  });

  // lists.feature: History by month, the last 30 charted (R148)
  it("groups by month, folds older months, charts the last 30 and filters", async () => {
    const many = Array.from({ length: 40 }, (_, index) => row(100 - index, TODAY - index * 3, tally(5, 0, 5), { topics: index % 2 ? [1] : [2] }));
    mocked.history.mockResolvedValue(many);
    const u = userEvent.setup();
    render(History, { onView: vi.fn() });
    const months = await screen.findAllByRole("button", { expanded: true });
    expect(months).toHaveLength(2);
    expect(screen.getAllByRole("button", { expanded: false }).length).toBeGreaterThan(0);
    expect(screen.getByRole("img", { name: /Score per quiz/ }).querySelectorAll("circle").length).toBeLessThanOrEqual(31);
    await u.click(screen.getByRole("radio", { name: "All" }));
    expect(screen.getByRole("img", { name: /Score per quiz/ }).querySelectorAll("circle").length).toBeGreaterThanOrEqual(40);
    await u.type(screen.getByRole("searchbox", { name: "Filter" }), "gpu");
    expect(screen.getByText(/20 of 40/)).toBeInTheDocument();
    expect(screen.getAllByRole("row", { name: /^Quiz/ })).toHaveLength(20);
  });
});


import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { overview: vi.fn(), topics: vi.fn(), today: vi.fn(), dueToday: vi.fn(), topicQuizzes: vi.fn() } };
});

import { api, type Progress } from "../lib/api";
import Overview from "./Overview.svelte";

const mocked = vi.mocked(api);
const TODAY = 20758;
const tally = (right_sure: number, right_guessed: number, wrong: number) => ({ right_sure, right_guessed, wrong, wrong_flagged: 0, unanswered: 0 });

function progress(overrides: Partial<Progress> = {}): Progress {
  return {
    accuracy: 62, change: 2, answered_last_7_days: 42, quizzes_last_7_days: 3, guessed_share: 19,
    scores: [{ day: TODAY - 4, percent: 27, retry: false }, { day: TODAY, percent: 80, retry: false }, { day: TODAY, percent: 86, retry: true }],
    weakest_topics: [[3, tally(3, 1, 11)], [1, tally(14, 3, 3)]],
    never_quizzed: [4],
    reasons: [["Blank", 3], ["Misremembered", 1]],
    ...overrides,
  };
}

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.topics.mockResolvedValue([
    { id: 1, name: "CPU", module: null, tags: [], question_count: 10, short_count: 0 },
    { id: 3, name: "OSI model", module: null, tags: [], question_count: 15, short_count: 0 },
    { id: 4, name: "Misc", module: null, tags: [], question_count: 4, short_count: 0 },
  ]);
});

describe("Overview", () => {
  // overview.feature: First run
  it("shows first steps before any quiz", async () => {
    const onNavigate = vi.fn();
    mocked.overview.mockResolvedValue("FirstRun");
    render(Overview, { onNavigate });
    expect(await screen.findByRole("heading", { name: "Welcome" })).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: /Score per quiz/ })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Create topic" }));
    expect(onNavigate).toHaveBeenCalledWith("topics");
  });

  // overview.feature: What's due leads (design/playful-redesign: Overview)
  it("leads with what's due today, and starts the due quizzes from it", async () => {
    const u = userEvent.setup();
    const onNavigate = vi.fn();
    const onQuizTopics = vi.fn();
    mocked.overview.mockResolvedValue({ Progress: progress() });
    mocked.dueToday.mockResolvedValue({ cards: 4, checklists: 1, quizzes: 2, drills: 0, total: 7 });
    mocked.topicQuizzes.mockResolvedValue([
      { topic: 1, step: "Week", position: 1, steps: 7, due: TODAY, due_today: true, normal_due: TODAY, tightened_for: null },
      { topic: 3, step: "Day", position: 0, steps: 7, due: TODAY - 1, due_today: true, normal_due: TODAY - 1, tightened_for: null },
      { topic: 4, step: "Day", position: 0, steps: 7, due: TODAY + 3, due_today: false, normal_due: TODAY + 3, tightened_for: null },
    ]);
    render(Overview, { onNavigate, onQuizTopics });
    const hero = await screen.findByRole("region", { name: "2 quizzes due today" });
    expect(hero).toHaveTextContent("CPU and OSI model are ready");
    // The pink banner is the one primary, so nothing else is pink
    expect(document.querySelectorAll(".btn-primary, .hero")).toHaveLength(1);
    await u.click(within(hero).getByRole("button", { name: "Start quiz" }));
    expect(onQuizTopics).toHaveBeenCalledWith([1, 3]);
    const due = screen.getByRole("list", { name: "Due today" });
    expect(within(due).getAllByRole("listitem").map((item) => item.textContent?.trim())).toEqual(["Quiz 2 topics", "Review 4 cards", "Recall 1 checklist"]);
    await u.click(within(due).getByRole("link", { name: "Review 4 cards" }));
    expect(onNavigate).toHaveBeenCalledWith("cards");
  });

  it("offers a new quiz when nothing is due", async () => {
    const u = userEvent.setup();
    const onNavigate = vi.fn();
    mocked.overview.mockResolvedValue({ Progress: progress() });
    mocked.dueToday.mockResolvedValue({ cards: 0, checklists: 0, quizzes: 0, drills: 0, total: 0 });
    mocked.topicQuizzes.mockResolvedValue([]);
    render(Overview, { onNavigate });
    const hero = await screen.findByRole("region", { name: "Nothing due today" });
    await u.click(within(hero).getByRole("button", { name: "New quiz" }));
    expect(onNavigate).toHaveBeenCalledWith("setup");
    expect(screen.getByRole("list", { name: "Due today" })).toHaveTextContent("All done for today");
  });

  // overview.feature: Progress summary / Change from the previous week
  it("summarises the last 7 days", async () => {
    mocked.overview.mockResolvedValue({ Progress: progress() });
    render(Overview, { onNavigate: vi.fn() });
    const accuracy = await screen.findByRole("region", { name: "Accuracy" });
    expect(accuracy).toHaveTextContent("62%");
    expect(accuracy).toHaveTextContent("+2 points");
    expect(screen.getByRole("region", { name: "Questions answered" })).toHaveTextContent("42");
    expect(screen.getByRole("region", { name: "Right answers that were guesses" })).toHaveTextContent("19%");
  });

  it("shows a drop and a quiet week plainly", async () => {
    mocked.overview.mockResolvedValue({ Progress: progress({ change: -5 }) });
    const { unmount } = render(Overview, { onNavigate: vi.fn() });
    expect(await screen.findByRole("region", { name: "Accuracy" })).toHaveTextContent("-5 points");
    unmount();
    mocked.overview.mockResolvedValue({ Progress: progress({ accuracy: null, change: null, quizzes_last_7_days: 0, answered_last_7_days: 0, guessed_share: null }) });
    render(Overview, { onNavigate: vi.fn() });
    expect(await screen.findByRole("region", { name: "Accuracy" })).toHaveTextContent("No quizzes in the last 7 days");
  });

  // overview.feature: Score per quiz
  it("charts every quiz", async () => {
    mocked.overview.mockResolvedValue({ Progress: progress() });
    render(Overview, { onNavigate: vi.fn() });
    expect(await screen.findByRole("img", { name: "Score per quiz, from 27% to 86%" })).toBeInTheDocument();
  });

  // overview.feature: Weakest topics first / Topics never quizzed
  it("lists topics weakest first and the ones never quizzed", async () => {
    mocked.overview.mockResolvedValue({ Progress: progress() });
    render(Overview, { onNavigate: vi.fn() });
    const list = await screen.findByRole("list", { name: "Topics, weakest first" });
    const items = within(list).getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("OSI model");
    expect(items[0]).toHaveTextContent("27% right");
    expect(items[1]).toHaveTextContent("CPU");
    expect(items[1]).toHaveTextContent("85% right");
    expect(screen.getByText("Not quizzed yet: Misc")).toBeInTheDocument();
  });

  // overview.feature: Reasons ranked
  it("ranks the reasons for wrong answers", async () => {
    mocked.overview.mockResolvedValue({ Progress: progress() });
    render(Overview, { onNavigate: vi.fn() });
    const reasons = await screen.findByRole("list", { name: "Why you got things wrong" });
    expect(within(reasons).getAllByRole("listitem").map((item) => item.textContent?.replace(/\s+/g, " ").trim())).toEqual(["Blank 3", "Misremembered 1"]);
  });
  it("switches to one point per week once there are many quizzes", async () => {
    const scores = Array.from({ length: 45 }, (_, n) => ({ day: TODAY - 44 + n, percent: 70, retry: false }));
    mocked.overview.mockResolvedValue({ Progress: progress({ scores }) });
    render(Overview, { onNavigate: vi.fn() });
    expect(await screen.findByRole("heading", { name: "Weekly average score" })).toBeInTheDocument();
    expect(screen.getByText(/45 quizzes since/)).toBeInTheDocument();
    const points = screen.getAllByTestId("score-point");
    expect(points.length).toBeLessThan(10);
    expect(points.at(-1)).toHaveAttribute("data-hollow", "true");
  });
});

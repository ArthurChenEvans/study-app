import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { topics: vi.fn(), modules: vi.fn(), startQuiz: vi.fn(), topicQuizzes: vi.fn() } };
});

import { api } from "../lib/api";
import Setup from "./Setup.svelte";

const mocked = vi.mocked(api);
const view = { position: 1, total: 8, exam: false, topic: 1, text: "Q", options: [], multiple: false, short: false, answers_to_pick: null, flagged: false, total_seconds: null, per_question_seconds: null, track: [] };

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topics.mockResolvedValue([
    { id: 1, name: "CPU", module: 1, tags: [], question_count: 10, short_count: 0 },
    { id: 2, name: "GPU", module: 1, tags: [], question_count: 10, short_count: 0 },
  ]);
  mocked.modules.mockResolvedValue([{ id: 1, name: "Computer Architecture" }]);
  mocked.startQuiz.mockResolvedValue(view);
});

describe("Quiz setup", () => {
  // quiz-setup.feature: Quiz on several topics, with the default settings
  it("starts with the default settings", async () => {
    const onStarted = vi.fn();
    const u = userEvent.setup();
    render(Setup, { onStarted });
    await u.click(await screen.findByRole("checkbox", { name: /CPU/ }));
    await u.click(screen.getByRole("checkbox", { name: /GPU/ }));
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("20 questions");
    await u.click(screen.getByRole("button", { name: "Start quiz" }));
    expect(mocked.startQuiz).toHaveBeenCalledWith({
      topics: [1, 2], count: null, even: false, shuffle_questions: false, shuffle_options: false,
      exam: false, total_minutes: null, per_question_seconds: null, easy: false, types: "Both",
    });
    expect(onStarted).toHaveBeenCalledWith(view);
  });

  it("sends every setting that was changed", async () => {
    const u = userEvent.setup();
    render(Setup, { preselected: [1, 2], onStarted: vi.fn() });
    await screen.findByRole("checkbox", { name: /CPU/ });
    await u.click(screen.getByRole("button", { name: "Change settings" }));
    await u.click(screen.getByRole("radio", { name: "Exam" }));
    await u.click(screen.getByRole("radio", { name: "Set number" }));
    await u.clear(screen.getByLabelText("Number of questions"));
    await u.type(screen.getByLabelText("Number of questions"), "8");
    await u.click(screen.getByRole("radio", { name: "Even" }));
    await u.click(screen.getByRole("radio", { name: "Shuffle questions" }));
    await u.click(screen.getByRole("radio", { name: "Shuffle options" }));
    await u.click(screen.getByLabelText("Use total timer"));
    await u.click(screen.getByLabelText("Use per-question timer"));
    await u.click(screen.getByRole("radio", { name: "Easy" }));
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("8 questions");
    await u.click(screen.getByRole("button", { name: "Start quiz" }));
    expect(mocked.startQuiz).toHaveBeenCalledWith({
      topics: [1, 2], count: 8, even: true, shuffle_questions: true, shuffle_options: true,
      exam: true, total_minutes: 20, per_question_seconds: 45, easy: true, types: "Both",
    });
  });

  it("asks for a topic before starting", async () => {
    const u = userEvent.setup();
    render(Setup, { onStarted: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Start quiz" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Pick at least one topic to start.");
    expect(mocked.startQuiz).not.toHaveBeenCalled();
  });

  it("shows why a quiz couldn't start", async () => {
    const u = userEvent.setup();
    mocked.startQuiz.mockRejectedValue({ message: "not decided yet in the specs: MoreThanAvailable" });
    render(Setup, { preselected: [1], onStarted: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Start quiz" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("not decided yet");
  });

  // R38
  it("greys out topics with no questions", async () => {
    mocked.topics.mockResolvedValue([
      { id: 1, name: "CPU", module: 1, tags: [], question_count: 10, short_count: 0 },
      { id: 3, name: "Misc", module: null, tags: [], question_count: 0, short_count: 0 },
    ]);
    render(Setup, { onStarted: vi.fn() });
    expect(await screen.findByRole("checkbox", { name: /Misc/ })).toBeDisabled();
    expect(screen.getByText("no questions yet")).toBeInTheDocument();
  });

  // R36
  it("says how many are available when asking for more", async () => {
    const u = userEvent.setup();
    render(Setup, { preselected: [1, 2], onStarted: vi.fn() });
    await screen.findByRole("checkbox", { name: /CPU/ });
    await u.click(screen.getByRole("button", { name: "Change settings" }));
    await u.click(screen.getByRole("radio", { name: "Set number" }));
    const input = screen.getByLabelText("Number of questions");
    await u.clear(input);
    await u.type(input, "25");
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("20 questions");
    expect(screen.getByText("Only 20 available")).toBeInTheDocument();
  });

  // short-answer.feature: Choose the question types for a quiz
  it("picks the question types, leaving out topics with none of that type", async () => {
    const u = userEvent.setup();
    mocked.topics.mockResolvedValue([
      { id: 1, name: "CPU", module: 1, tags: [], question_count: 7, short_count: 2 },
      { id: 2, name: "GPU", module: 1, tags: [], question_count: 10, short_count: 0 },
    ]);
    render(Setup, { preselected: [1, 2], onStarted: vi.fn() });
    const cpu = await screen.findByRole("checkbox", { name: /CPU/ });
    expect(cpu.closest("label")).toHaveTextContent("5 MCQ · 2 short answer");
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("17 questions");
    await u.click(screen.getByRole("button", { name: "Change settings" }));
    await u.click(screen.getByRole("radio", { name: "Short answer" }));
    expect(screen.getByRole("checkbox", { name: /GPU/ })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /GPU/ }).closest("label")).toHaveTextContent("no short answer yet");
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("2 questions");
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("Short answer only");
    await u.click(screen.getByRole("button", { name: "Start quiz" }));
    expect(mocked.startQuiz).toHaveBeenCalledWith(expect.objectContaining({ topics: [1], types: "Short" }));
  });

  it("only offers question types once there are short answers", async () => {
    const u = userEvent.setup();
    render(Setup, { onStarted: vi.fn() });
    await screen.findByRole("checkbox", { name: /CPU/ });
    await u.click(screen.getByRole("button", { name: "Change settings" }));
    expect(screen.queryByRole("radiogroup", { name: "Question types" })).toBeNull();
  });
  it("folds the settings into a summary until you ask to change them", async () => {
    const u = userEvent.setup();
    render(Setup, { onStarted: vi.fn() });
    await screen.findByRole("checkbox", { name: /CPU/ });
    expect(screen.getByRole("list", { name: "Current settings" })).toHaveTextContent("Study mode");
    expect(screen.queryByRole("radio", { name: "Exam" })).toBeNull();
    const toggle = screen.getByRole("button", { name: "Change settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await u.click(toggle);
    await u.click(screen.getByRole("radio", { name: "Exam" }));
    expect(screen.getByRole("list", { name: "Current settings" })).toHaveTextContent("Exam mode");
  });

  it("says why it can't start yet, and keeps the button reachable", async () => {
    render(Setup, { onStarted: vi.fn() });
    const start = await screen.findByRole("button", { name: "Start quiz" });
    expect(start).toHaveAttribute("aria-disabled", "true");
    expect(start).toHaveAccessibleDescription("Pick at least one topic to start.");
  });

  it("marks topics due today and picks them in one go", async () => {
    const u = userEvent.setup();
    mocked.topicQuizzes.mockResolvedValue([
      { topic: 1, step: "Week", position: 2, steps: 6, due: 0, due_today: false, normal_due: 0, tightened_for: null, waiting: false },
      { topic: 2, step: "Day", position: 1, steps: 6, due: 0, due_today: true, normal_due: 0, tightened_for: null, waiting: false },
    ]);
    render(Setup, { onStarted: vi.fn() });
    const gpu = await screen.findByRole("checkbox", { name: /GPU/ });
    expect(gpu.closest("label")).toHaveTextContent("Due today");
    expect(screen.getByRole("checkbox", { name: /CPU/ }).closest("label")).not.toHaveTextContent("Due today");
    await u.click(screen.getByRole("button", { name: "Pick the 1 due" }));
    expect(gpu).toBeChecked();
    expect(screen.queryByRole("button", { name: "Pick the 1 due" })).toBeNull();
  });

  // lists.feature: Pick a whole module for a quiz
  it("ticks a whole module from the rail and unticks from the chips", async () => {
    const u = userEvent.setup();
    mocked.topicQuizzes.mockResolvedValue([]);
    render(Setup, { onStarted: vi.fn() });
    await u.click(await screen.findByRole("button", { name: /^Computer Architecture 2 topics/ }));
    await u.click(screen.getByRole("button", { name: "Tick all 2 in Computer Architecture" }));
    const picked = screen.getByRole("list", { name: "Picked topics" });
    expect(picked).toHaveTextContent("CPU");
    expect(picked).toHaveTextContent("GPU");
    expect(screen.getByLabelText("Quiz summary")).toHaveTextContent("from 2 topics");
    await u.click(screen.getByRole("button", { name: "Untick GPU" }));
    expect(screen.getByRole("checkbox", { name: /GPU/ })).not.toBeChecked();
    await u.type(screen.getByRole("searchbox", { name: "Filter" }), "gp");
    expect(screen.queryByRole("checkbox", { name: /CPU/ })).toBeNull();
    expect(screen.getByText(/1 of 2/)).toBeInTheDocument();
  });
});

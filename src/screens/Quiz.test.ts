import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { answer: vi.fn(), toggleFlag: vi.fn(), questionTimedOut: vi.fn(), totalTimedOut: vi.fn(), quitQuiz: vi.fn() } };
});

import { api, type QuizView } from "../lib/api";
import Quiz from "./Quiz.svelte";

const mocked = vi.mocked(api);

function view(overrides: Partial<QuizView> = {}): QuizView {
  return {
    position: 1, total: 3, exam: false, topic: 1,
    text: "Which stage reads the **register file**?",
    options: [{ index: 2, text: "Execute" }, { index: 0, text: "Fetch" }, { index: 1, text: "Decode" }],
    multiple: false, answers_to_pick: null, flagged: false, total_seconds: null, per_question_seconds: null,
    track: ["Current", "Upcoming", "Upcoming"],
    ...overrides,
  };
}

const second = view({ position: 2, text: "Second question", track: ["RightSure", "Current", "Upcoming"] });

beforeEach(() => vi.resetAllMocks());
afterEach(() => vi.useRealTimers());

describe("Quiz", () => {
  it("shows the question and its options in the shuffled order", () => {
    render(Quiz, { initial: view(), onFinished: vi.fn() });
    expect(screen.getByRole("heading", { level: 1 }).innerHTML).toContain("<strong>register file</strong>");
    expect(screen.getAllByRole("button", { name: /^\d / }).map((button) => button.textContent?.trim())).toEqual(["1 Execute", "2 Fetch", "3 Decode"]);
    expect(screen.getByText("of 3")).toBeInTheDocument();
  });

  // taking-quiz.feature: Study mode feedback
  it("study mode: submits the original option index and shows feedback", async () => {
    const u = userEvent.setup();
    mocked.answer.mockResolvedValue({ feedback: { right: false, auto_reason: null, correct: [1], explanation: "Operands are read in decode.", citations: [] }, next: { Question: second } });
    render(Quiz, { initial: view(), onFinished: vi.fn() });
    expect(screen.getByRole("button", { name: /Submit/ })).toBeDisabled();
    await u.click(screen.getByRole("button", { name: "2 Fetch" }));
    await u.click(screen.getByRole("button", { name: /Submit/ }));
    expect(mocked.answer).toHaveBeenCalledWith([0]);
    expect(await screen.findByRole("status")).toHaveTextContent("Wrong");
    expect(screen.getByRole("status")).toHaveTextContent("Operands are read in decode.");
    expect(screen.getByRole("button", { name: /3 Decode/ })).toHaveTextContent("Correct");
    expect(screen.getByRole("button", { name: /2 Fetch/ })).toHaveTextContent("Your pick");
    await u.click(screen.getByRole("button", { name: /Next question/ }));
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Second question");
  });

  // taking-quiz.feature: Exam mode feedback
  it("exam mode: goes straight to the next question", async () => {
    const u = userEvent.setup();
    mocked.answer.mockResolvedValue({ feedback: null, next: { Question: { ...second, exam: true } } });
    render(Quiz, { initial: view({ exam: true }), onFinished: vi.fn() });
    expect(screen.getByText("No going back in exam mode")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "1 Execute" }));
    await u.click(screen.getByRole("button", { name: /Submit/ }));
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Second question");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  // taking-quiz.feature: Keyboard shortcuts
  it("answers and flags from the keyboard", async () => {
    const u = userEvent.setup();
    mocked.toggleFlag.mockResolvedValue(view({ flagged: true }));
    mocked.answer.mockResolvedValue({ feedback: null, next: { Question: second } });
    render(Quiz, { initial: view(), onFinished: vi.fn() });
    await u.keyboard("3");
    expect(screen.getByRole("button", { name: "3 Decode" })).toHaveAttribute("aria-pressed", "true");
    await u.keyboard("f");
    expect(mocked.toggleFlag).toHaveBeenCalled();
    expect(await screen.findByRole("button", { name: /Flagged as guessed/ })).toBeInTheDocument();
    await u.keyboard("{Enter}");
    expect(mocked.answer).toHaveBeenCalledWith([1]);
  });

  // taking-quiz.feature: Easy and hard mode, multiple answers
  it("multiple answer: picks several and shows the count in easy mode", async () => {
    const u = userEvent.setup();
    mocked.answer.mockResolvedValue({ feedback: null, next: { Question: second } });
    render(Quiz, { initial: view({ multiple: true, answers_to_pick: 2 }), onFinished: vi.fn() });
    expect(screen.getByText("Pick 2")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "3 Decode" }));
    await u.click(screen.getByRole("button", { name: "1 Execute" }));
    await u.click(screen.getByRole("button", { name: /Submit/ }));
    expect(mocked.answer).toHaveBeenCalledWith([1, 2]);
  });

  it("hands over to results when the quiz finishes", async () => {
    const u = userEvent.setup();
    const onFinished = vi.fn();
    mocked.answer.mockResolvedValue({ feedback: null, next: { Finished: 42 } });
    render(Quiz, { initial: view({ exam: true }), onFinished });
    await u.click(screen.getByRole("button", { name: "1 Execute" }));
    await u.click(screen.getByRole("button", { name: /Submit/ }));
    expect(onFinished).toHaveBeenCalledWith(42);
  });

  // taking-quiz.feature: Quit halfway
  it("quitting saves and hands over to results", async () => {
    const u = userEvent.setup();
    const onFinished = vi.fn();
    mocked.quitQuiz.mockResolvedValue(7);
    render(Quiz, { initial: view(), onFinished });
    await u.click(screen.getByRole("button", { name: /Quit/ }));
    expect(onFinished).toHaveBeenCalledWith(7);
  });

  // taking-quiz.feature: Per-question timeout
  it("runs out the per-question timer", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mocked.questionTimedOut.mockResolvedValue({ feedback: null, next: { Question: second } });
    render(Quiz, { initial: view({ per_question_seconds: 5 }), onFinished: vi.fn() });
    expect(screen.getByRole("timer")).toHaveTextContent("0:05");
    await vi.advanceTimersByTimeAsync(5_100);
    expect(mocked.questionTimedOut).toHaveBeenCalled();
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Second question");
  });

  // taking-quiz.feature: Total timeout
  it("ends the quiz when the total timer runs out", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const onFinished = vi.fn();
    mocked.totalTimedOut.mockResolvedValue(9);
    render(Quiz, { initial: view({ total_seconds: 3 }), onFinished });
    await vi.advanceTimersByTimeAsync(3_100);
    expect(onFinished).toHaveBeenCalledWith(9);
  });
});

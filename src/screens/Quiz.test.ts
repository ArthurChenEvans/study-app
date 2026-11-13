import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { answer: vi.fn(), answerText: vi.fn(), overruleAnswer: vi.fn(), toggleFlag: vi.fn(), questionTimedOut: vi.fn(), totalTimedOut: vi.fn(), quitQuiz: vi.fn() } };
});

vi.mock("../lib/sound", () => ({ play: vi.fn() }));

const cele = vi.hoisted(() => ({ answered: vi.fn(), celebrate: vi.fn(), startSession: vi.fn() }));
vi.mock("../lib/celebrations.svelte", () => ({
  celebrate: (...args: unknown[]) => cele.celebrate(...args),
  praise: () => "Well done!",
  startSession: (queue: number) => cele.startSession(queue) ?? { answered: cele.answered },
}));

import { play } from "../lib/sound";
import { api, type Feedback, type QuizView, type ShortFeedback } from "../lib/api";
import Quiz from "./Quiz.svelte";

const mocked = vi.mocked(api);

function view(overrides: Partial<QuizView> = {}): QuizView {
  return {
    position: 1, total: 3, exam: false, topic: 1,
    text: "Which stage reads the **register file**?",
    options: [{ index: 2, text: "Execute" }, { index: 0, text: "Fetch" }, { index: 1, text: "Decode" }],
    multiple: false, short: false, answers_to_pick: null, flagged: false, total_seconds: null, per_question_seconds: null,
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
    mocked.answer.mockResolvedValue({ feedback: { index: 0, short: null, right: false, auto_reason: null, correct: [1], explanation: "Operands are read in decode.", citations: [] }, next: { Question: second } });
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

  // UI polish: the whole row answers, other options stay readable, and a cue plays
  it("tints the picked and correct rows, keeps the others readable, and plays a cue", async () => {
    const u = userEvent.setup();
    mocked.answer.mockResolvedValue({ feedback: { index: 0, short: null, right: false, auto_reason: null, correct: [1], explanation: "", citations: [] }, next: { Question: second } });
    render(Quiz, { initial: view(), onFinished: vi.fn() });
    await u.click(screen.getByRole("button", { name: "2 Fetch" }));
    await u.click(screen.getByRole("button", { name: /Submit/ }));
    await screen.findByRole("status");
    expect(screen.getByRole("button", { name: /2 Fetch/ })).toHaveClass("option-wrong");
    expect(screen.getByRole("button", { name: /3 Decode/ })).toHaveClass("option-right");
    const other = screen.getByRole("button", { name: /1 Execute/ });
    expect(other).toHaveClass("text-muted");
    expect(other.className).not.toMatch(/opacity/);
    expect(vi.mocked(play)).toHaveBeenCalledWith("wrong");
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

  describe("short answer", () => {
    const shortView = view({ text: "Which layer adds port numbers?", options: [], short: true });
    const marked = (right: boolean, short: Partial<ShortFeedback> = {}): Feedback => ({
      index: 0, right, auto_reason: null, correct: [], explanation: "Ports let apps share an address.", citations: [],
      short: { typed: "network layer", marking: { right, detail: "NoMatch" }, shown_answer: "The **transport** layer", also_accepted: ["Layer 4"], overruled: false, ...short },
    });

    // short-answer.feature: Study mode shows the verdict
    it("types the answer, submits with Enter and shows the verdict", async () => {
      const u = userEvent.setup();
      mocked.answerText.mockResolvedValue({ feedback: marked(false), next: { Question: second } });
      render(Quiz, { initial: shortView, onFinished: vi.fn() });
      expect(screen.getByText("Short answer")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^\d / })).toBeNull();
      const box = screen.getByLabelText("Your answer");
      expect(screen.getByRole("button", { name: /Submit/ })).toBeDisabled();
      await u.type(box, "network{Shift>}{Enter}{/Shift}layer");
      expect(box).toHaveValue("network\nlayer");
      await u.keyboard("{Enter}");
      expect(mocked.answerText).toHaveBeenCalledWith("network\nlayer");
      const status = await screen.findByRole("status");
      expect(status).toHaveTextContent("Wrong");
      expect(status).toHaveTextContent("Model answer");
      expect(status.querySelector("strong")).toHaveTextContent("transport");
      expect(status).toHaveTextContent("Also accepted: Layer 4");
      expect(status).toHaveTextContent("Ports let apps share an address.");
      expect(box).toHaveAttribute("readonly");
      expect(box).toHaveClass("is-wrong");
    });

    it("shows a typo and the keywords hit and missed", async () => {
      const u = userEvent.setup();
      mocked.answerText
        .mockResolvedValueOnce({ feedback: marked(true, { typed: "the transprot layer", marking: { right: true, detail: { Matched: { answer: "Transport layer", typo: "transprot" } } } }), next: { Question: shortView } })
        .mockResolvedValueOnce({ feedback: marked(false, { marking: { right: false, detail: { Keywords: { hit: ["SYN", "SYN-ACK"], missed: ["ACK"], needed: 3 } } } }), next: { Finished: 4 } });
      render(Quiz, { initial: shortView, onFinished: vi.fn() });
      await u.type(screen.getByLabelText("Your answer"), "the transprot layer{Enter}");
      const first = await screen.findByRole("status");
      expect(first).toHaveTextContent("Correct, with a typo");
      expect(first).toHaveTextContent('you wrote "transprot"');
      expect(screen.getByRole("status")).not.toHaveTextContent("Also accepted");
      await u.keyboard("{Enter}");
      expect(screen.getByLabelText("Your answer")).toHaveValue("");
      await u.type(screen.getByLabelText("Your answer"), "SYN then SYN-ACK{Enter}");
      const status = await screen.findByRole("status");
      expect(status).toHaveTextContent("Wrong");
      expect(status).toHaveTextContent("needs all 3 keywords, you had 2");
      expect(screen.getByLabelText("Keywords")).toHaveTextContent(/SYN, found\s*SYN-ACK, found\s*ACK, missed/);
    });

    // short-answer.feature: Overrule a verdict
    it("overrules with O and undoes, using the saved attempt once the quiz has finished", async () => {
      const u = userEvent.setup();
      const onFinished = vi.fn();
      mocked.answerText.mockResolvedValue({ feedback: marked(false), next: { Finished: 9 } });
      mocked.overruleAnswer.mockResolvedValueOnce(marked(true, { overruled: true, marking: { right: false, detail: "NoMatch" } })).mockResolvedValueOnce(marked(false));
      render(Quiz, { initial: shortView, onFinished });
      await u.type(screen.getByLabelText("Your answer"), "network layer{Enter}");
      await screen.findByRole("status");
      await u.keyboard("o");
      expect(mocked.overruleAnswer).toHaveBeenCalledWith(9, 0);
      const status = await screen.findByText("Counted as right");
      expect(status.closest("[role=status]")).toHaveTextContent("You overruled this");
      expect(screen.getByLabelText("Your answer")).not.toHaveClass("is-right");
      await u.click(screen.getByRole("button", { name: /Undo, count it as wrong/ }));
      expect(await screen.findByRole("button", { name: /Count it as right/ })).toBeInTheDocument();
      await u.keyboard("{Enter}");
      expect(onFinished).toHaveBeenCalledWith(9);
    });

    it("overrules in a running quiz and updates the track", async () => {
      const u = userEvent.setup();
      const next = view({ position: 2, short: true, options: [], text: "Next one", track: ["Wrong", "Current", "Upcoming"] });
      mocked.answerText.mockResolvedValue({ feedback: marked(false), next: { Question: next } });
      mocked.overruleAnswer.mockResolvedValue(marked(true, { overruled: true, marking: { right: false, detail: "NoMatch" } }));
      render(Quiz, { initial: shortView, onFinished: vi.fn() });
      await u.type(screen.getByLabelText("Your answer"), "network layer{Enter}");
      await u.click(await screen.findByRole("button", { name: /Count it as right/ }));
      expect(mocked.overruleAnswer).toHaveBeenCalledWith(null, 0);
      await screen.findByText("Counted as right");
      await u.keyboard("{Enter}");
      expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Next one");
      const cells = screen.getByLabelText(/Progress/).querySelectorAll("span");
      expect(cells[0].getAttribute("style")).toContain("chart-right");
    });

    it("flags a guess with Ctrl+G while typing", async () => {
      const u = userEvent.setup();
      mocked.toggleFlag.mockResolvedValue({ ...shortView, flagged: true });
      render(Quiz, { initial: shortView, onFinished: vi.fn() });
      await u.type(screen.getByLabelText("Your answer"), "maybe");
      await u.keyboard("{Control>}g{/Control}");
      expect(mocked.toggleFlag).toHaveBeenCalled();
      expect(await screen.findByRole("button", { name: /Flagged as guessed/ })).toBeInTheDocument();
      await u.keyboard("f");
      expect(mocked.toggleFlag).toHaveBeenCalledTimes(1);
      expect(screen.getByLabelText("Your answer")).toHaveValue("maybef");
    });
  });

  // celebrations.feature (R151): right answers count towards streaks
  it("tells the session pacer about each answer", async () => {
    cele.startSession.mockReturnValue({ answered: cele.answered });
    const u = userEvent.setup();
    mocked.answer.mockResolvedValue({ feedback: { index: 0, short: null, right: true, auto_reason: null, correct: [1], explanation: "", citations: [] }, next: { Question: second } });
    render(Quiz, { initial: view(), onFinished: vi.fn() });
    expect(cele.startSession).toHaveBeenCalledWith(3);
    await u.click(screen.getByRole("button", { name: "3 Decode" }));
    await u.click(screen.getByRole("button", { name: /Submit/ }));
    await screen.findByRole("status");
    expect(cele.answered).toHaveBeenCalledWith(true);
  });
});

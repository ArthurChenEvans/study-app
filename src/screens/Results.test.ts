import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { results: vi.fn(), topics: vi.fn(), today: vi.fn(), giveReason: vi.fn(), retry: vi.fn(), overruleResult: vi.fn() } };
});

vi.mock("../lib/sound", () => ({ play: vi.fn() }));

import * as soundModule from "../lib/sound";
import { api, type ResultsView, type ReviewItem, type ShortFeedback } from "../lib/api";
import Results from "./Results.svelte";

const mocked = vi.mocked(api);
const sound = vi.mocked(soundModule);
const tally = (right_sure: number, right_guessed: number, wrong: number) => ({ right_sure, right_guessed, wrong, wrong_flagged: 0, unanswered: 0 });

function results(overrides: Partial<ResultsView> = {}): ResultsView {
  return {
    attempt: 3, status: "Finished", exam: false,
    summary: { overall: tally(1, 1, 2), per_topic: { "1": tally(1, 1, 1), "2": tally(0, 0, 1) } },
    review: [
      { index: 1, topic: 1, text: "Which describe a warp?", options: ["Lockstep", "Own PC", "Serialised"], picked: [0], correct: [0, 2], explanation: "SIMT.", status: "Wrong", auto_reason: "Incomplete", reason: null, short: null },
      { index: 2, topic: 2, text: "CPU time?", options: ["2 s", "3 s", "4.5 s"], picked: [2], correct: [1], explanation: "T = N CPI / f", status: "Wrong", auto_reason: null, reason: null, short: null },
      { index: 0, topic: 1, text: "Which stage reads registers?", options: ["Fetch", "Decode", "Execute"], picked: [1], correct: [1], explanation: "Decode.", status: "RightGuessed", auto_reason: null, reason: null, short: null },
      { index: 3, topic: 1, text: "ALU?", options: ["Maths", "Storage", "IO"], picked: [0], correct: [0], explanation: "", status: "Right", auto_reason: null, reason: null, short: null },
    ],
    to_prompt: [2],
    retry_count: 3,
    moves: [],
    ...overrides,
  };
}

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topics.mockResolvedValue([
    { id: 1, name: "GPU", module: null, tags: [], question_count: 3, short_count: 0 },
    { id: 2, name: "CPU", module: null, tags: [], question_count: 1, short_count: 0 },
  ]);
});

describe("Results", () => {
  // results.feature: Results summary
  it("shows the score, guesses and each topic", async () => {
    mocked.results.mockResolvedValue(results());
    render(Results, { attempt: 3, onRetry: vi.fn() });
    const score = await screen.findByRole("group", { name: "Score" });
    expect(await within(score).findByText("2")).toBeInTheDocument();
    expect(within(score).getByText("/ 4")).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Right, guessed" })).toHaveTextContent("1");
    expect(screen.getByText("1 of those guessed")).toBeInTheDocument();
    const byTopic = screen.getByRole("list", { name: "By topic" });
    expect(within(byTopic).getByText("GPU")).toBeInTheDocument();
    expect(within(byTopic).getByText("2 / 3")).toBeInTheDocument();
  });

  // results.feature: Review each question
  it("reviews wrong and guessed questions first with answers and explanations", async () => {
    mocked.results.mockResolvedValue(results());
    render(Results, { attempt: 3, onRetry: vi.fn() });
    const items = await screen.findAllByRole("listitem", { name: /^Question/ });
    expect(items.map((item) => item.getAttribute("aria-label"))).toEqual(["Question 2", "Question 3", "Question 1", "Question 4"]);
    expect(items[0]).toHaveTextContent("Incomplete (recorded automatically)");
    expect(items[1]).toHaveTextContent("Your answer: 4.5 s");
    expect(items[1]).toHaveTextContent("Correct: 3 s");
    expect(items[2]).toHaveTextContent("Right, guessed");
  });

  // results.feature: Give a reason / Give a free text reason
  it("saves a preset or free-text reason", async () => {
    const u = userEvent.setup();
    mocked.results.mockResolvedValue(results());
    mocked.giveReason.mockResolvedValue(results({ to_prompt: [] }));
    render(Results, { attempt: 3, onRetry: vi.fn() });
    const item = (await screen.findAllByRole("listitem", { name: "Question 3" }))[0];
    await u.click(within(within(item).getByRole("radiogroup", { name: /Why did you get it wrong/ })).getByRole("radio", { name: "Misremembered" }));
    expect(mocked.giveReason).toHaveBeenCalledWith(3, 2, "Misremembered");
    await u.type(within(item).getByLabelText("Your own reason"), "mixed up units{Enter}");
    expect(mocked.giveReason).toHaveBeenCalledWith(3, 2, { Other: "mixed up units" });
  });

  // retry.feature: Retry wrong and guessed
  it("retries the wrong and guessed questions", async () => {
    const u = userEvent.setup();
    const onRetry = vi.fn();
    const view = { position: 1, total: 3 } as never;
    mocked.results.mockResolvedValue(results());
    mocked.retry.mockResolvedValue(view);
    render(Results, { attempt: 3, onRetry });
    await u.click(await screen.findByRole("button", { name: "Retry the 3 wrong and guessed" }));
    expect(mocked.retry).toHaveBeenCalledWith(3);
    expect(onRetry).toHaveBeenCalledWith(view);
  });

  // retry.feature: Nothing to retry
  it("says when there is nothing to retry", async () => {
    mocked.results.mockResolvedValue(results({ retry_count: 0 }));
    render(Results, { attempt: 3, onRetry: vi.fn() });
    expect(await screen.findByText(/Nothing to retry/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Retry/ })).not.toBeInTheDocument();
  });

  it("marks an incomplete attempt", async () => {
    mocked.results.mockResolvedValue(results({ status: "Incomplete" }));
    render(Results, { attempt: 3, onRetry: vi.fn() });
    expect(await screen.findByText(/Quit early/)).toBeInTheDocument();
  });

  describe("short answers", () => {
    const short = (index: number, status: ReviewItem["status"], feedback: Partial<ShortFeedback>): ReviewItem => ({
      index, topic: 1, text: `Short question ${index}`, options: [], picked: [], correct: [], explanation: "", status, auto_reason: null, reason: null,
      short: { typed: "network layer", marking: { right: false, detail: "NoMatch" }, shown_answer: "The **transport** layer", also_accepted: [], overruled: false, ...feedback },
    });
    const review = [
      short(0, "Wrong", {}),
      short(1, "Right", { typed: "a routr", marking: { right: true, detail: { Matched: { answer: "Router", typo: "routr" } } } }),
      short(2, "Wrong", { marking: { right: false, detail: { Keywords: { hit: ["SYN", "SYN-ACK"], missed: ["ACK"], needed: 3 } } } }),
    ];

    // short-answer.feature: Overrule a verdict
    it("shows what was written, how it was checked, and overrules", async () => {
      const u = userEvent.setup();
      mocked.results.mockResolvedValue(results({ review, to_prompt: [0, 2] }));
      mocked.overruleResult.mockResolvedValue(results({ review: [short(0, "Right", { overruled: true }), ...review.slice(1)], to_prompt: [2] }));
      render(Results, { attempt: 3, onRetry: vi.fn() });
      const first = await screen.findByRole("listitem", { name: "Question 1" });
      expect(first).toHaveTextContent("GPU · Short answer");
      expect(first).toHaveTextContent("Your answer: network layer");
      expect(first.querySelector("strong")).toHaveTextContent("transport");
      expect(first).toHaveTextContent("Checked: Didn't match any accepted answer");
      const typo = screen.getByRole("listitem", { name: "Question 2" });
      expect(typo).toHaveTextContent("Right, with a typo");
      expect(typo).toHaveTextContent('Checked: Matched "Router" with a typo');
      expect(within(typo).getByRole("button", { name: "Count it as wrong" })).toBeInTheDocument();
      expect(screen.getByRole("listitem", { name: "Question 3" })).toHaveTextContent("Checked: 2 of 3 keywords, missed ACK");

      await u.click(within(first).getByRole("button", { name: "Count it as right" }));
      expect(mocked.overruleResult).toHaveBeenCalledWith(3, 0);
      const overruled = await screen.findByRole("listitem", { name: "Question 1" });
      expect(overruled).toHaveTextContent("You overruled this");
      expect(within(overruled).getByRole("button", { name: /Undo, count it as wrong/ })).toBeInTheDocument();
      expect(within(overruled).queryByLabelText("Your own reason")).toBeNull();
    });
  });

  // calendar.feature: A topic's quiz schedule moves with its score (R97)
  it("shows where the quiz moved each topic's schedule", async () => {
    mocked.today.mockResolvedValue(20_760);
    mocked.results.mockResolvedValue(results({ moves: [
      { topic: 1, name: "CPU", score: 86, passed: true, from: "Week", to: "Fortnight", next_due: 20_774, tightened_due: 20_765, tightened_for: "Architecture mock" },
      { topic: 2, name: "OSI model", score: 50, passed: false, from: "Week", to: "Day", next_due: 20_761, tightened_due: null, tightened_for: null },
      { topic: 3, name: "Memory", score: 100, passed: true, from: null, to: "Day", next_due: 20_761, tightened_due: null, tightened_for: null },
    ] }));
    render(Results, { attempt: 3, onRetry: vi.fn() });
    const panel = await screen.findByRole("region", { name: "Quiz schedule" });
    const items = within(panel).getAllByRole("listitem");
    // dates.feature: results show the tightened next quiz and what it would have been
    expect(items[0]).toHaveTextContent("CPU Passed 86% Moves up to Fortnight · Next quiz on Sun 8 Nov, tightened for the Architecture mock (would be Tue 17 Nov) Tightened");
    expect(items[1]).toHaveTextContent("Under pass mark 50% Back to Day");
    expect(items[1]).toHaveTextContent("Next quiz tomorrow");
    expect(items[2]).toHaveTextContent("Passed 100% Starts on Day");
  });

  it("plays the finish cue and says Quiz finished only straight after a quiz", async () => {
    mocked.results.mockResolvedValue(results());
    const { unmount } = render(Results, { attempt: 3, onRetry: vi.fn(), fresh: true });
    expect(await screen.findByRole("heading", { name: "Quiz finished" })).toBeInTheDocument();
    expect(sound.play).toHaveBeenCalledWith("finished");
    unmount();
    sound.play.mockClear();
    render(Results, { attempt: 3, onRetry: vi.fn() });
    expect(await screen.findByRole("heading", { name: "Results" })).toBeInTheDocument();
    expect(sound.play).not.toHaveBeenCalled();
  });

  it("has no schedule panel for retries", async () => {
    mocked.results.mockResolvedValue(results());
    render(Results, { attempt: 3, onRetry: vi.fn() });
    await screen.findByText("Review");
    expect(screen.queryByRole("region", { name: "Quiz schedule" })).toBeNull();
  });
});

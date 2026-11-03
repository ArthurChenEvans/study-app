import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      checklist: vi.fn(), recallSchedule: vi.fn(), today: vi.fn(), savePoints: vi.fn(), recallCandidates: vi.fn(),
      addFromQuestions: vi.fn(), recallHistory: vi.fn(), deleteChecklist: vi.fn(),
    },
  };
});

import { api, type ChecklistView, type RecallPoint } from "../lib/api";
import Checklist from "./Checklist.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const p = (text: string, question: number | null = null): RecallPoint => ({ text, question });
const view = (points: RecallPoint[]): ChecklistView => ({
  row: { id: 4, subject_name: "Computer Architecture", module: true, subject_exists: true, points: points.length, step: "Month", next_step: "Monthly 1 of 6", position: [3, 1], due: TODAY, due_today: true, days_late: 0, last_score: 79 },
  points,
  topics: ["CPU", "Memory"],
});

beforeEach(() => {
  vi.resetAllMocks();
  mocked.recallSchedule.mockResolvedValue({ steps: [{ name: "Day", every: { count: 1, unit: "Days" }, repeats: 1 }, { name: "Yearly", every: { count: 1, unit: "Years" }, repeats: 0 }], pass_mark: 70, under_pass: "BackOneStep" });
  mocked.today.mockResolvedValue(TODAY);
  mocked.savePoints.mockImplementation(async (_, points) => points.map((point) => ({ ...point, text: point.text.trim() })));
});

const pointTexts = () => within(screen.getByRole("list", { name: "Points to recall" })).getAllByRole("listitem").map((item) => item.querySelector("[data-point]")?.textContent?.trim());

describe("Checklist editor", () => {
  // free-recall.feature: Write, reorder and remove points
  it("adds, edits, moves and removes points, saving each change", async () => {
    mocked.checklist.mockResolvedValue(view([p("Pipeline stages"), p("Cache levels")]));
    const u = userEvent.setup();
    render(Checklist, { id: 4, onBack: vi.fn(), onStart: vi.fn() });
    await screen.findByText("Pipeline stages");

    await u.type(screen.getByLabelText("New point"), "Amdahl's law{Enter}");
    expect(mocked.savePoints).toHaveBeenLastCalledWith(4, [p("Pipeline stages"), p("Cache levels"), p("Amdahl's law")]);
    expect(await screen.findByRole("status")).toHaveTextContent("Point added");

    screen.getByRole("button", { name: /Edit point 3/ }).focus();
    await u.keyboard("{Alt>}{ArrowUp}{ArrowUp}{/Alt}");
    expect(pointTexts()).toEqual(["Amdahl's law", "Pipeline stages", "Cache levels"]);

    await u.click(screen.getByRole("button", { name: "Remove point 3" }));
    expect(pointTexts()).toEqual(["Amdahl's law", "Pipeline stages"]);

    await u.click(screen.getByRole("button", { name: /Edit point 2/ }));
    const input = screen.getByLabelText("Point 2");
    await u.clear(input);
    await u.type(input, "Five pipeline stages{Enter}");
    expect(mocked.savePoints).toHaveBeenLastCalledWith(4, [p("Amdahl's law"), p("Five pipeline stages")]);
  });

  // free-recall.feature: Add and remove points from questions
  it("adds questions as points and removes ones that are unticked", async () => {
    mocked.checklist.mockResolvedValue(view([p("Which stage reads registers? → Decode", 10)]));
    mocked.recallCandidates.mockResolvedValue([
      { question: 10, topic_name: "CPU", text: "Which stage reads registers?", answer: "Decode", in_checklist: true },
      { question: 11, topic_name: "Memory", text: "What does the TLB cache?", answer: "Page translations", in_checklist: false },
    ]);
    mocked.addFromQuestions.mockResolvedValue([p("What does the TLB cache? → Page translations", 11)]);
    const u = userEvent.setup();
    render(Checklist, { id: 4, onBack: vi.fn(), onStart: vi.fn() });
    expect(await screen.findByText("From MCQ")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Add from questions" }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("button", { name: "No changes yet" })).toBeDisabled();
    await u.click(within(dialog).getByRole("checkbox", { name: /TLB/ }));
    await u.click(within(dialog).getByRole("checkbox", { name: /reads registers/ }));
    expect(within(dialog).getByText("Will be removed")).toBeInTheDocument();
    await u.click(within(dialog).getByRole("button", { name: "Add 1, remove 1" }));
    expect(mocked.addFromQuestions).toHaveBeenCalledWith(4, [11], [10]);
    expect(await screen.findByRole("status")).toHaveTextContent("Added 1, removed 1");
  });

  // free-recall.feature: A checklist needs a point before it can start
  it("can't start a recall with no points", async () => {
    mocked.checklist.mockResolvedValue(view([]));
    render(Checklist, { id: 4, onBack: vi.fn(), onStart: vi.fn() });
    expect(await screen.findByText("No points yet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start recall" })).toBeDisabled();
    expect(screen.getByText("Needs at least one point to start.")).toBeInTheDocument();
  });

  // free-recall.feature: History of a checklist
  it("shows each check with its step change and dump, and the most missed points", async () => {
    mocked.checklist.mockResolvedValue(view([p("A")]));
    mocked.recallHistory.mockResolvedValue({
      checks: [
        { day: TODAY, score: 79, passed: true, from: "Fortnight", to: "Month", dump: "Pipelines, caches" },
        { day: TODAY - 14, score: 58, passed: false, from: "Week", to: "Day", dump: null },
      ],
      most_missed: [{ text: "Memory-mapped I/O", missed: 2, checks: 2 }],
      pass_mark: 70,
    });
    const u = userEvent.setup();
    render(Checklist, { id: 4, onBack: vi.fn(), onStart: vi.fn() });
    await u.click(await screen.findByRole("tab", { name: "History" }));
    const rows = within(await screen.findByRole("table", { name: "Checks" })).getAllByRole("row");
    expect(rows[1]).toHaveTextContent("79% passed");
    expect(rows[1]).toHaveTextContent("Fortnight → Month");
    expect(rows[2]).toHaveTextContent("58% under pass mark");
    expect(rows[2]).toHaveTextContent("Outside the app");
    await u.click(within(rows[1]).getByRole("button", { name: "View dump" }));
    expect(screen.getByText("Pipelines, caches")).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "Missed most often" })).toHaveTextContent("Missed 2 of 2 checks");
  });

  it("deletes the checklist after asking", async () => {
    const onBack = vi.fn();
    mocked.checklist.mockResolvedValue(view([p("A")]));
    const u = userEvent.setup();
    render(Checklist, { id: 4, onBack, onStart: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Delete checklist" }));
    await u.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Delete checklist" }));
    expect(mocked.deleteChecklist).toHaveBeenCalledWith(4);
    expect(onBack).toHaveBeenCalled();
  });
});

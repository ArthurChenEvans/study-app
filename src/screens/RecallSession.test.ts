import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { checklist: vi.fn(), recallSchedule: vi.fn(), today: vi.fn(), previewCheck: vi.fn(), finishCheck: vi.fn(), recallList: vi.fn(), savePoints: vi.fn() },
  };
});

import { api, type ChecklistView, type RecallRow } from "../lib/api";
import RecallSession from "./RecallSession.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const row: RecallRow = { id: 4, subject_name: "Computer Architecture", module: true, subject_exists: true, points: 3, step: "Month", next_step: "Monthly 1 of 6", position: [3, 1], due: TODAY, due_today: true, days_late: 0, last_score: 79 };
const view: ChecklistView = { row, points: [{ text: "Pipeline stages", question: null, note: null }, { text: "Cache levels", question: null, note: null }, { text: "Amdahl's law", question: null, note: null }], topics: ["CPU"] };

beforeEach(() => {
  vi.resetAllMocks();
  mocked.checklist.mockResolvedValue(structuredClone(view));
  mocked.recallSchedule.mockResolvedValue({ steps: [{ name: "Month", every: { count: 1, unit: "Months" }, repeats: 1 }], pass_mark: 70, under_pass: "BackOneStep" });
  mocked.today.mockResolvedValue(TODAY);
  mocked.previewCheck.mockImplementation(async (_, recalled) => {
    const score = Math.round((recalled / 3) * 100);
    return score >= 70 ? { score, passed: true, to: "Monthly 1 of 6", next_due: TODAY + 30 } : { score, passed: false, to: "Fortnight", next_due: TODAY + 14 };
  });
  mocked.recallList.mockResolvedValue([row, { ...row, id: 5, subject_name: "OSI model" }]);
});

const props = () => ({ id: 4, onLeave: vi.fn(), onEdit: vi.fn(), onNext: vi.fn() });

describe("Recall session", () => {
  // free-recall.feature: Dump in the app or outside it (typed), The step moves up at the pass mark
  it("hides the checklist during the dump, then ticks, previews and finishes", async () => {
    mocked.finishCheck.mockResolvedValue({
      score: 67, recalled: 2, total: 3, passed: false, previous_score: 79, from: "Month", to: "Fortnight",
      position: [2, 1], next_step: "Month", next_due: TODAY + 14, missed: [{ text: "Amdahl's law", missed: 3, checks: 4 }],
    });
    const u = userEvent.setup();
    const handlers = props();
    render(RecallSession, handlers);
    const box = await screen.findByLabelText("Your recall");
    expect(screen.queryByText("Pipeline stages")).toBeNull();
    expect(screen.queryByRole("timer")).toBeNull();
    await u.type(box, "Fetch decode execute, caches");
    expect(screen.getByText("4 words")).toBeInTheDocument();
    await u.keyboard("{Control>}{Enter}{/Control}");

    expect(await screen.findByText("Fetch decode execute, caches")).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "Checklist" });
    const [first, second] = within(list).getAllByRole("checkbox");
    await u.click(first);
    second.focus();
    await u.keyboard(" ");
    expect(second).toHaveAttribute("aria-checked", "true");
    const score = screen.getByLabelText("Score so far");
    expect(score).toHaveTextContent("67%");
    expect(score).toHaveTextContent("Under the pass mark");
    expect(score).toHaveTextContent("Goes back to Fortnight, next check on Tue 17 Nov");

    await u.keyboard("{Control>}{Enter}{/Control}");
    expect(mocked.finishCheck).toHaveBeenCalledWith(4, "Fetch decode execute, caches", [0, 1]);
    expect(await screen.findByRole("heading", { name: /67%/ })).toBeInTheDocument();
    expect(screen.getByText("Back a step")).toBeInTheDocument();
    expect(screen.getByText(/Last check 79% · pass mark 70%/)).toHaveTextContent(/Recalled in 0:\d\d/);
    expect(screen.getByRole("list", { name: "Missed points" })).toHaveTextContent("Missed 3 of 4 checks");
    await u.click(screen.getByRole("button", { name: "Next: OSI model" }));
    expect(handlers.onNext).toHaveBeenCalledWith(5);
  });

  // free-recall.feature: Dump in the app or outside it (outside the app)
  it("saves no dump when done outside the app", async () => {
    mocked.finishCheck.mockResolvedValue({ score: 100, recalled: 3, total: 3, passed: true, previous_score: 79, from: "Month", to: "Monthly 1 of 6", position: [4, 1], next_step: "Monthly 2 of 6", next_due: TODAY + 30, missed: [] });
    const u = userEvent.setup();
    render(RecallSession, props());
    await u.click(await screen.findByRole("radio", { name: "Outside the app" }));
    await u.click(screen.getByRole("button", { name: /I'm done, show the checklist/ }));
    expect(screen.getByText("Done outside the app.")).toBeInTheDocument();
    for (const box of within(screen.getByRole("list", { name: "Checklist" })).getAllByRole("checkbox")) await u.click(box);
    await u.click(screen.getByRole("button", { name: /Finish check/ }));
    expect(mocked.finishCheck).toHaveBeenCalledWith(4, null, [0, 1, 2]);
    expect(await screen.findByText("Up a step")).toBeInTheDocument();
    expect(screen.getByText("Everything on the checklist, recalled.")).toBeInTheDocument();
  });

  it("adds a point the checklist was missing during the check", async () => {
    mocked.savePoints.mockImplementation(async (_, points) => points);
    const u = userEvent.setup();
    render(RecallSession, props());
    await u.click(await screen.findByRole("button", { name: /Done, check it/ }));
    await u.click(screen.getByRole("button", { name: "Add a point the list is missing" }));
    await u.type(screen.getByLabelText("Missing point"), "TLB{Enter}");
    expect(mocked.savePoints).toHaveBeenCalledWith(4, [...view.points, { text: "TLB", question: null, note: null }]);
    expect(within(screen.getByRole("list", { name: "Checklist" })).getAllByRole("checkbox")).toHaveLength(4);
  });

  it("asks before leaving with an unsaved dump", async () => {
    const handlers = props();
    const u = userEvent.setup();
    render(RecallSession, handlers);
    await u.type(await screen.findByLabelText("Your recall"), "Some notes");
    await u.click(screen.getByRole("button", { name: "Leave" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("Leave without saving this recall?");
    await u.click(screen.getByRole("button", { name: "Keep going" }));
    expect(handlers.onLeave).not.toHaveBeenCalled();
  });
});

import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      recallList: vi.fn(), recallSchedule: vi.fn(), today: vi.fn(), modules: vi.fn(), topics: vi.fn(),
      createChecklist: vi.fn(), setRecallSchedule: vi.fn(), resetRecallSchedule: vi.fn(), deleteChecklist: vi.fn(),
    },
  };
});

import { api, type RecallRow, type Schedule } from "../lib/api";
import Recall from "./Recall.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760; // Tue 3 Nov 2026
const ladder: Schedule = {
  steps: [
    { name: "Day", every: { count: 1, unit: "Days" }, repeats: 1 },
    { name: "Week", every: { count: 1, unit: "Weeks" }, repeats: 1 },
    { name: "Monthly", every: { count: 1, unit: "Months" }, repeats: 6 },
    { name: "Yearly", every: { count: 1, unit: "Years" }, repeats: 0 },
  ],
  pass_mark: 70,
  under_pass: "BackOneStep",
};
const row = (over: Partial<RecallRow>): RecallRow => ({
  id: 1, subject_name: "CPU", module: false, subject_exists: true, points: 8, step: "Week", next_step: "Monthly 1 of 6",
  position: [1, 1], due: TODAY, due_today: true, days_late: 0, last_score: 74, ...over,
});

beforeEach(() => {
  vi.resetAllMocks();
  mocked.recallSchedule.mockResolvedValue(ladder);
  mocked.today.mockResolvedValue(TODAY);
  mocked.modules.mockResolvedValue([{ id: 1, name: "Computer Architecture" }]);
  mocked.topics.mockResolvedValue([{ id: 2, name: "CPU", module: 1, tags: [], question_count: 3, short_count: 0 }]);
});

describe("Recall list", () => {
  // free-recall.feature: Due checklists come first
  it("puts due checklists first, then the rest by date", async () => {
    const onStart = vi.fn();
    mocked.recallList.mockResolvedValue([
      row({ id: 1, subject_name: "OSI model", days_late: 1, due: TODAY - 1 }),
      row({ id: 2, subject_name: "Cryptography", module: true, due: TODAY + 2, due_today: false, step: "Week" }),
      row({ id: 3, subject_name: "GPU", due: TODAY + 72, due_today: false, step: "Monthly 3 of 6", position: [2, 3] }),
    ]);
    render(Recall, { onOpen: vi.fn(), onStart });
    const card = await screen.findByRole("article", { name: "OSI model" });
    expect(card).toHaveTextContent("Due 1 day ago");
    expect(card).toHaveTextContent(/74%\s*Last check/);
    expect(screen.getByText("1 due today")).toBeInTheDocument();
    const table = screen.getByRole("table", { name: "Checklists" });
    const names = within(table).getAllByRole("row").slice(1).map((r) => r.textContent);
    expect(names[0]).toContain("Cryptography");
    expect(names[0]).toContain("Thu 5 Nov");
    expect(names[1]).toContain("GPU");
    await userEvent.click(within(card).getByRole("button", { name: "Start recall" }));
    expect(onStart).toHaveBeenCalledWith(1);
  });

  // R185: a checklist can be deleted from the list, after asking
  it("deletes a checklist from the list after asking", async () => {
    const u = userEvent.setup();
    mocked.recallList.mockResolvedValue([row({ id: 1, subject_name: "CPU" }), row({ id: 2, subject_name: "Memory", due: TODAY + 9, due_today: false })]);
    render(Recall, { onOpen: vi.fn(), onStart: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Memory actions" }));
    await u.click(screen.getByRole("menuitem", { name: "Delete checklist…" }));
    const dialog = screen.getByRole("dialog", { name: "Delete the Memory checklist?" });
    expect(dialog).toHaveTextContent("Its points and past checks go too.");
    mocked.recallList.mockResolvedValue([row({ id: 1, subject_name: "CPU" })]);
    await u.click(within(dialog).getByRole("button", { name: "Delete checklist" }));
    expect(mocked.deleteChecklist).toHaveBeenCalledWith(2);
    expect(await screen.findByRole("button", { name: "CPU actions" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Memory actions" })).toBeNull();
  });

  it("says when nothing is due", async () => {
    mocked.recallList.mockResolvedValue([row({ due: TODAY + 1, due_today: false })]);
    render(Recall, { onOpen: vi.fn(), onStart: vi.fn() });
    expect(await screen.findByText("All caught up")).toBeInTheDocument();
    expect(screen.getByText("Next up is CPU, tomorrow.")).toBeInTheDocument();
  });

  it("offers to add points instead of starting an empty checklist", async () => {
    mocked.recallList.mockResolvedValue([row({ points: 0, step: null, position: null, last_score: null })]);
    render(Recall, { onOpen: vi.fn(), onStart: vi.fn() });
    const card = await screen.findByRole("article", { name: "CPU" });
    expect(within(card).queryByRole("button", { name: "Start recall" })).toBeNull();
    expect(within(card).getByRole("button", { name: "Add points" })).toBeInTheDocument();
    expect(card).toHaveTextContent("First check");
  });

  // free-recall.feature: Make a checklist about a module or a topic
  it("creates a checklist about a module and opens it", async () => {
    const onOpen = vi.fn();
    mocked.recallList.mockResolvedValue([]);
    mocked.createChecklist.mockResolvedValue(9);
    const u = userEvent.setup();
    render(Recall, { onOpen, onStart: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Create your first checklist" }));
    await u.click(screen.getByRole("button", { name: "Create checklist" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Pick a module or topic to recall.");
    expect(screen.getByRole("radio", { name: /^Computer Architecture/ })).toHaveAccessibleName(/^Computer Architecture\s*1 topic$/);
    await u.type(screen.getByRole("searchbox", { name: "Find a module or topic" }), "comp");
    expect(screen.queryByRole("radio", { name: /^CPU/ })).toBeNull();
    await u.click(screen.getByRole("radio", { name: /^Computer Architecture/ }));
    await u.click(screen.getByRole("button", { name: "Create checklist" }));
    expect(mocked.createChecklist).toHaveBeenCalledWith({ Module: 1 });
    expect(onOpen).toHaveBeenCalledWith(9);
  });

  // free-recall.feature: Edit and reset the schedule
  it("edits and resets the schedule", async () => {
    mocked.recallList.mockResolvedValue([row({})]);
    mocked.resetRecallSchedule.mockResolvedValue(ladder);
    const u = userEvent.setup();
    render(Recall, { onOpen: vi.fn(), onStart: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Schedule" }));
    const passMark = screen.getByRole("spinbutton", { name: "Pass mark" });
    await u.clear(passMark);
    await u.type(passMark, "80");
    const monthly = screen.getByRole("spinbutton", { name: "Monthly times" });
    await u.clear(monthly);
    await u.type(monthly, "3");
    expect(screen.getByRole("row", { name: /^Monthly/ })).toHaveTextContent("3 months");
    expect(screen.getByRole("row", { name: /^Yearly/ })).toHaveTextContent("Then every year");
    await u.click(screen.getByRole("button", { name: "Save schedule" }));
    const saved = mocked.setRecallSchedule.mock.calls[0][0];
    expect(saved.pass_mark).toBe(80);
    expect(saved.steps[2].repeats).toBe(3);

    await u.click(screen.getByRole("button", { name: "Schedule" }));
    await u.click(screen.getByRole("button", { name: "Reset to default" }));
    expect(mocked.resetRecallSchedule).toHaveBeenCalled();
  });

  it("shows why a schedule wasn't saved", async () => {
    mocked.recallList.mockResolvedValue([row({})]);
    mocked.setRecallSchedule.mockRejectedValue({ message: "the pass mark has to be between 1% and 100%" });
    const u = userEvent.setup();
    render(Recall, { onOpen: vi.fn(), onStart: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Schedule" }));
    await u.click(screen.getByRole("button", { name: "Save schedule" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("the pass mark has to be between 1% and 100%");
  });
});

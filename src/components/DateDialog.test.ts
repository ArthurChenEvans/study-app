import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { addDate: vi.fn(), updateDate: vi.fn(), deleteDate: vi.fn(), previewDate: vi.fn() } };
});

import { api, type DateView, type ModuleRow, type TopicRow } from "../lib/api";
import { live } from "../lib/celebrations.svelte";
import DateDialog from "./DateDialog.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760; // Tue 3 Nov 2026
const modules: ModuleRow[] = [{ id: 1, name: "Networks" }, { id: 2, name: "Computer Architecture" }];
const topics: TopicRow[] = [
  { id: 10, name: "OSI model", module: 1, tags: [], question_count: 6, short_count: 0 },
  { id: 11, name: "TCP/IP", module: 1, tags: [], question_count: 4, short_count: 0 },
  { id: 12, name: "CPU", module: 2, tags: [], question_count: 9, short_count: 0 },
];

beforeEach(() => {
  vi.resetAllMocks();
  mocked.previewDate.mockResolvedValue({ quizzes: [{ topic: 11, name: "TCP/IP", step: "Month", from: TODAY + 25, to: TODAY + 10 }], unquizzed: [{ topic: 10, name: "OSI model", questions: 6 }], cards: 9, drills: 0 });
});

describe("DateDialog", () => {
  // dates.feature: Add a date (covering nothing)
  it("adds a plain date", async () => {
    const u = userEvent.setup();
    const onSaved = vi.fn();
    render(DateDialog, { today: TODAY, modules, topics, onSaved, onCancel: vi.fn() });
    await u.type(screen.getByLabelText("Name"), "Study app write-up");
    await u.type(screen.getByLabelText(/Kind/), "Deadline");
    const tick = screen.getByRole("checkbox", { name: /Tighten reviews before this/ });
    expect(tick).toBeDisabled();
    expect(screen.getByText("Pick a module or topics for this to do anything.")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Add date" }));
    expect(mocked.addDate).toHaveBeenCalledWith({ name: "Study app write-up", day: TODAY + 14, kind: "Deadline", module: null, topics: [], tighten: false });
    expect(onSaved).toHaveBeenCalled();
    // toasts.feature (R155)
    expect(live.toasts.at(-1)).toMatchObject({ text: "Date added", sub: "Study app write-up, Tue 17 Nov" });
  });

  it("needs a name", async () => {
    const u = userEvent.setup();
    render(DateDialog, { today: TODAY, modules, topics, onSaved: vi.fn(), onCancel: vi.fn() });
    await u.click(screen.getByRole("button", { name: "Add date" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Give the date a name.");
    expect(mocked.addDate).not.toHaveBeenCalled();
  });

  // dates.feature: Tightening is on by default for exams, Preview before saving
  it("ticks tightening for an exam covering a module and previews what moves", async () => {
    const u = userEvent.setup();
    render(DateDialog, { today: TODAY, modules, topics, onSaved: vi.fn(), onCancel: vi.fn() });
    await u.type(screen.getByLabelText("Name"), "Networks exam");
    await u.type(screen.getByLabelText(/Kind/), "Exam");
    await u.click(screen.getByRole("radio", { name: "A module" }));
    expect(screen.getByRole("combobox", { name: "Module" })).toHaveDisplayValue("Networks (2 topics)");
    expect(screen.getByRole("checkbox", { name: /Tighten reviews before this/ })).toBeChecked();
    const preview = await screen.findByText(/If you add this today/);
    const box = preview.closest("div")!;
    expect(box).toHaveTextContent("TCP/IP quiz moves from Sat 28 Nov to Fri 13 Nov");
    expect(box).toHaveTextContent("OSI model hasn't been quizzed yet, so it has no schedule to tighten");
    expect(box).toHaveTextContent("9 cards come in sooner");
    expect(mocked.previewDate).toHaveBeenLastCalledWith(null, expect.objectContaining({ module: 1, tighten: true }));

    await u.click(screen.getByRole("checkbox", { name: /Tighten reviews before this/ }));
    expect(screen.queryByText(/If you add this today/)).not.toBeInTheDocument();
  });

  // dates.feature: covers topics
  it("covers picked topics", async () => {
    const u = userEvent.setup();
    render(DateDialog, { today: TODAY, modules, topics, onSaved: vi.fn(), onCancel: vi.fn() });
    await u.type(screen.getByLabelText("Name"), "Architecture mock");
    await u.click(screen.getByRole("radio", { name: "Topics" }));
    const list = screen.getByRole("list", { name: "Topics it covers" });
    await u.click(within(list).getByRole("checkbox", { name: /CPU/ }));
    expect(screen.getByText("1 topic picked")).toBeInTheDocument();
    await u.click(screen.getByRole("checkbox", { name: /Tighten reviews before this/ }));
    await u.click(screen.getByRole("button", { name: "Add date" }));
    expect(mocked.addDate).toHaveBeenCalledWith(expect.objectContaining({ module: null, topics: [12], tighten: true }));
  });

  // dates.feature: Edit and delete a date
  it("edits and deletes a date after asking", async () => {
    const u = userEvent.setup();
    const onSaved = vi.fn();
    const date: DateView = { id: 4, name: "Study app write-up", day: TODAY + 10, kind: "Deadline", covers: "Nothing", covers_label: "", topics: [], tighten: false, days_left: 10 };
    render(DateDialog, { today: TODAY, date, modules, topics, onSaved, onCancel: vi.fn() });
    expect(screen.getByRole("heading", { name: "Edit date" })).toBeInTheDocument();
    expect(screen.getByLabelText("Name")).toHaveValue("Study app write-up");
    expect(screen.getByLabelText("Date")).toHaveValue("2026-11-13");
    await u.click(screen.getByRole("button", { name: "Save" }));
    expect(mocked.updateDate).toHaveBeenCalledWith(4, { name: "Study app write-up", day: TODAY + 10, kind: "Deadline", module: null, topics: [], tighten: false });

    await u.click(screen.getByRole("button", { name: "Delete" }));
    expect(screen.getByText(/It's only taken off the calendar/)).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Keep it" }));
    await u.click(screen.getByRole("button", { name: "Delete" }));
    await u.click(screen.getByRole("button", { name: "Delete date" }));
    expect(mocked.deleteDate).toHaveBeenCalledWith(4);
  });
});

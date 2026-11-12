import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { today: vi.fn(), calendarMonth: vi.fn(), calendarDay: vi.fn(), moveDrill: vi.fn(), dates: vi.fn(), modules: vi.fn(), topics: vi.fn(), topicQuizzes: vi.fn(), previewDate: vi.fn(), addDate: vi.fn(), updateDate: vi.fn(), deleteDate: vi.fn() } };
});

import { api, type CalendarDay, type DateView, type DayView, type DrillView } from "../lib/api";
import Calendar from "./Calendar.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760; // Tue 3 Nov 2026
const blank = (day: number, when: CalendarDay["when"]): CalendarDay => ({ day, when, cards: 0, checklists: 0, quizzes: 0, drills: 0, struggles: 0 });
const drill: DrillView = { id: 5, task: "Write a loop over a slice without an off-by-one", hints: "", issue: "Off-by-one in loops", topic: null, topic_name: null, created: TODAY - 1, due: TODAY, last: null, runs: 0, from: [{ day: TODAY - 1, project_name: "Study app", what: "Loop ran once too often" }] };
const exam: DateView = { id: 2, name: "Networks exam", day: TODAY + 20, kind: "Exam", covers: { Module: 1 }, covers_label: "Networks module", topics: [10, 11, 12], tighten: true, days_left: 20 };
const writeUp: DateView = { id: 1, name: "Study app write-up", day: TODAY + 10, kind: "Deadline", covers: "Nothing", covers_label: "", topics: [], tighten: false, days_left: 10 };
const emptyDay = (day: number, when: DayView["when"]): DayView => ({
  day, when, cards: { due: { new: 0, learning: 0, review: 0 }, minutes: 0, reviewed: 0, again: 0, seconds: 0 },
  checklists: [], checks: [], quizzes: [], attempts: [], drills: [], drill_runs: [], struggles: [], dates: [], soon: [], waiting: [], clears_by: null, daily: 5,
});

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.dates.mockResolvedValue([]);
  mocked.calendarMonth.mockImplementation(async (from, to) => Array.from({ length: to - from + 1 }, (_, index) => {
    const day = from + index;
    const entry = blank(day, day < TODAY ? "Past" : day === TODAY ? "Today" : "Future");
    if (day === TODAY - 1) return { ...entry, cards: 14, quizzes: 1, struggles: 3 };
    if (day === TODAY) return { ...entry, cards: 12, checklists: 1, quizzes: 2, drills: 1 };
    if (day === TODAY + 1) return { ...entry, cards: 1, drills: 1 };
    return entry;
  }));
});

describe("Calendar", () => {
  // calendar.feature: See what's when
  it("shows the month in Monday weeks, with counts and words, today outlined", async () => {
    render(Calendar, {});
    const grid = await screen.findByRole("grid", { name: "November 2026" });
    const cells = within(grid).getAllByRole("button");
    expect(cells).toHaveLength(42);
    expect(cells[0]).toHaveTextContent("26");
    expect(mocked.calendarMonth).toHaveBeenCalledWith(20_752, 20_793);
    const today = within(grid).getByRole("button", { name: /^Today, Tue 3 Nov: 12 cards, 1 checklist, 2 quizzes, 1 drill due$/ });
    expect(today).toHaveAttribute("aria-current", "date");
    expect(today).toHaveTextContent("12 cards");
    expect(within(grid).getByRole("button", { name: /^Mon 2 Nov: 14 cards, 1 quiz, 3 struggles done$/ })).toBeInTheDocument();
    expect(within(grid).getByRole("button", { name: /^Wed 4 Nov: 1 card, 1 drill due$/ })).toHaveTextContent("1 card1 drill");
  });

  it("names the weekdays in full, keeps busy days to three lines, and moves by arrow key", async () => {
    const u = userEvent.setup();
    render(Calendar, {});
    const grid = await screen.findByRole("grid", { name: "November 2026" });
    expect(within(grid).getAllByRole("row")).toHaveLength(7);
    expect(within(grid).getAllByRole("columnheader")[0]).toHaveAccessibleName("Monday");
    const today = within(grid).getByRole("button", { name: /^Today, Tue 3 Nov/ });
    expect(today).toHaveTextContent("Today");
    expect(today).toHaveTextContent("+2 more");
    expect(today).not.toHaveTextContent("1 drill");
    today.focus();
    await u.keyboard("{ArrowRight}");
    expect(within(grid).getByRole("button", { name: /^Wed 4 Nov/ })).toHaveFocus();
    await u.keyboard("{ArrowUp}");
    expect(within(grid).getByRole("button", { name: /^Wed 28 Oct/ })).toHaveFocus();
  });

  it("moves between months", async () => {
    const u = userEvent.setup();
    render(Calendar, {});
    await screen.findByRole("grid", { name: "November 2026" });
    await u.click(screen.getByRole("button", { name: "Next month" }));
    expect(await screen.findByRole("grid", { name: "December 2026" })).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Today" }));
    expect(await screen.findByRole("grid", { name: "November 2026" })).toBeInTheDocument();
  });

  // calendar.feature: See what's when (a day's detail)
  it("opens today with start buttons and one primary", async () => {
    const u = userEvent.setup();
    const handlers = { onCards: vi.fn(), onRecall: vi.fn(), onQuiz: vi.fn(), onDrills: vi.fn() };
    mocked.calendarDay.mockResolvedValue({
      ...emptyDay(TODAY, "Today"),
      cards: { due: { new: 3, learning: 0, review: 9 }, minutes: 6, reviewed: 0, again: 0, seconds: 0 },
      checklists: [{ id: 4, name: "Computer Architecture", step: "Week", points: 8 }],
      quizzes: [{ topic: 1, name: "CPU", step: "Week", questions: 7, tightened_for: null }, { topic: 2, name: "OSI model", step: "Day", questions: 6, tightened_for: null }],
      drills: [drill],
    });
    render(Calendar, handlers);
    await u.click(await screen.findByRole("button", { name: /^Today, Tue 3 Nov/ }));
    expect(await screen.findByRole("heading", { name: "Today, Tue 3 Nov" })).toBeInTheDocument();
    expect(screen.getByText("16 things due · about 16 minutes")).toBeInTheDocument();
    expect(document.querySelectorAll(".btn-primary")).toHaveLength(1);
    await u.click(screen.getByRole("button", { name: /Start with the drills/ }));
    expect(handlers.onDrills).toHaveBeenCalledWith([5]);
    expect(screen.getByText(/Issue: Off-by-one in loops · from Study app/)).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Start review/ }));
    expect(handlers.onCards).toHaveBeenCalled();
    await u.click(screen.getByRole("button", { name: /Start recall/ }));
    expect(handlers.onRecall).toHaveBeenCalledWith(4);
    const quizzes = screen.getByRole("region", { name: "Quizzes" });
    await u.click(within(quizzes).getAllByRole("button", { name: /^Quiz$/ })[1]);
    expect(handlers.onQuiz).toHaveBeenLastCalledWith([2]);
    await u.click(within(quizzes).getByRole("button", { name: /Quiz all 2 together/ }));
    expect(handlers.onQuiz).toHaveBeenLastCalledWith([1, 2]);
  });

  it("shows what was done on a past day, and steps between days", async () => {
    const u = userEvent.setup();
    const handlers = { onResults: vi.fn(), onProject: vi.fn() };
    mocked.calendarDay.mockImplementation(async (day) => day === TODAY - 1 ? {
      ...emptyDay(day, "Past"),
      cards: { due: { new: 0, learning: 0, review: 0 }, minutes: 0, reviewed: 14, again: 3, seconds: 540 },
      attempts: [{ attempt: 9, topics: ["OSI model", "TCP/IP"], right: 7, total: 10, moves: ["OSI model moved up to Fortnight"] }],
      struggles: [{ id: 1, project: 2, project_name: "Study app", day, minute: 600, what: "Loop ran once too often", why: "habit", kind: "Apply", topic: null, topic_name: null, issue: "Off-by-one in loops", outcome: { Drill: { id: 5, due: TODAY } } }],
    } : emptyDay(day, day < TODAY ? "Past" : "Today"));
    render(Calendar, handlers);
    await u.click(await screen.findByRole("button", { name: /^Mon 2 Nov/ }));
    expect(await screen.findByRole("heading", { name: "Mon 2 Nov" })).toBeInTheDocument();
    expect(screen.getByText("What you did")).toBeInTheDocument();
    expect(screen.getByText("11 remembered · 3 times Again · 9 min")).toBeInTheDocument();
    expect(screen.getByText("7 of 10 right · OSI model moved up to Fortnight")).toBeInTheDocument();
    expect(screen.getByText("Study app · Couldn't apply · Issue: Off-by-one in loops · drill today")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Start/ })).toBeNull();
    await u.click(screen.getByRole("button", { name: "View results" }));
    expect(handlers.onResults).toHaveBeenCalledWith(9);
    await u.click(screen.getByRole("button", { name: "Open project" }));
    expect(handlers.onProject).toHaveBeenCalledWith(2);
    await u.click(screen.getByRole("button", { name: "Previous day" }));
    expect(await screen.findByText("Nothing was done on this day.")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /November 2026/ }));
    expect(await screen.findByRole("grid", { name: "November 2026" })).toBeInTheDocument();
  });

  // projects.feature: A drill can be moved to another day
  it("moves a drill to another day", async () => {
    const u = userEvent.setup();
    mocked.calendarDay.mockResolvedValue({ ...emptyDay(TODAY + 1, "Future"), drills: [{ ...drill, due: TODAY + 1 }] });
    mocked.moveDrill.mockResolvedValue({ ...drill, due: TODAY + 2 });
    render(Calendar, {});
    await u.click(await screen.findByRole("button", { name: /^Wed 4 Nov/ }));
    await screen.findByRole("heading", { name: "Wed 4 Nov" });
    expect(screen.queryByRole("button", { name: /Do drill/ })).toBeNull();
    await u.click(screen.getByRole("button", { name: /Move/ }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(drill.task)).toBeInTheDocument();
    await u.click(within(dialog).getByRole("radio", { name: /Another day/ }));
    await u.clear(within(dialog).getByLabelText("Pick a day"));
    await u.type(within(dialog).getByLabelText("Pick a day"), "2026-11-05");
    await u.click(within(dialog).getByRole("button", { name: "Move to Thu 5 Nov" }));
    expect(mocked.moveDrill).toHaveBeenCalledWith(5, TODAY + 2);
  });

  it("opens a given day straight away and says which day is open", async () => {
    const u = userEvent.setup();
    const onOpenDay = vi.fn();
    mocked.calendarDay.mockResolvedValue(emptyDay(TODAY, "Today"));
    render(Calendar, { initialDay: TODAY, onOpenDay });
    expect(await screen.findByRole("heading", { name: "Today, Tue 3 Nov" })).toBeInTheDocument();
    expect(onOpenDay).toHaveBeenLastCalledWith(TODAY);
    await u.click(screen.getByRole("button", { name: /November 2026/ }));
    expect(onOpenDay).toHaveBeenLastCalledWith(null);
  });

  // dates.feature: Dates on the month view
  it("shows dates as chips on their day and the next ones coming up", async () => {
    const u = userEvent.setup();
    mocked.dates.mockResolvedValue([writeUp, exam]);
    mocked.calendarDay.mockResolvedValue(emptyDay(TODAY + 10, "Future"));
    render(Calendar, {});
    const coming = await screen.findByRole("region", { name: "Coming up" });
    expect(coming).toHaveTextContent("Study app write-up Fri 13 Nov, in 10 days");
    expect(coming).toHaveTextContent("Networks exam Mon 23 Nov, in 20 days Reviews tightened");
    const grid = screen.getByRole("grid", { name: "November 2026" });
    expect(within(grid).getByRole("button", { name: /^Mon 23 Nov, Networks exam$/ })).toHaveTextContent("Networks exam");
    await u.click(within(coming).getByRole("button", { name: /Study app write-up/ }));
    expect(mocked.calendarDay).toHaveBeenCalledWith(TODAY + 10);
  });

  // dates.feature: Add a date
  it("adds a date from the primary button", async () => {
    const u = userEvent.setup();
    mocked.modules.mockResolvedValue([]);
    mocked.topics.mockResolvedValue([]);
    mocked.topicQuizzes.mockResolvedValue([]);
    mocked.addDate.mockResolvedValue(7);
    render(Calendar, {});
    await screen.findByRole("grid");
    expect(document.querySelectorAll(".btn-primary")).toHaveLength(1);
    await u.click(screen.getByRole("button", { name: "Add a date" }));
    const dialog = await screen.findByRole("dialog");
    await u.type(within(dialog).getByLabelText("Name"), "Study app write-up");
    mocked.dates.mockResolvedValue([writeUp]);
    await u.click(within(dialog).getByRole("button", { name: "Add date" }));
    expect(mocked.addDate).toHaveBeenCalled();
    expect(await screen.findByRole("region", { name: "Coming up" })).toHaveTextContent("Study app write-up");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  // dates.feature: The exam's day, The week before
  it("gives a date its own panel on its day, and counts down to it the days before", async () => {
    const u = userEvent.setup();
    mocked.dates.mockResolvedValue([exam]);
    mocked.calendarDay.mockImplementation(async (value) => value === TODAY + 20
      ? { ...emptyDay(value, "Future"), dates: [{ date: exam, quizzes: [{ topic: 11, name: "TCP/IP", step: "Month", from: TODAY + 25, to: TODAY + 10 }], unquizzed: [{ topic: 12, name: "Subnetting", questions: 5 }], cards_before: 21, cards_pulled: 9 }] }
      : { ...emptyDay(value, "Future"), soon: [exam], quizzes: [{ topic: 11, name: "TCP/IP", step: "Month", questions: 4, tightened_for: "Networks exam" }] });
    const onQuiz = vi.fn();
    render(Calendar, { onQuiz });
    await u.click(await screen.findByRole("button", { name: /^Mon 23 Nov, Networks exam/ }));
    const panel = await screen.findByRole("region", { name: "Networks exam" });
    expect(panel).toHaveTextContent("Exam · Networks module · in 20 days");
    expect(panel).toHaveTextContent("Before it, with reviews tightened");
    expect(panel).toHaveTextContent("Month step · next quiz Fri 13 Nov, pulled in from Sat 28 Nov");
    expect(panel).toHaveTextContent("Not quizzed yet, so nothing to tighten");
    expect(panel).toHaveTextContent("21 cards due before it9 of them pulled in from after it");
    await u.click(within(panel).getByRole("button", { name: /Quiz it/ }));
    expect(onQuiz).toHaveBeenCalledWith([12]);

    await u.click(screen.getByRole("button", { name: "Previous day" }));
    expect(await screen.findByRole("region", { name: "Networks exam coming up" })).toHaveTextContent("Networks exam in 1 day, on Mon 23 Nov. Reviews for it are tightened");
    expect(screen.getByText(/tightened for the Networks exam/)).toBeInTheDocument();
  });

  // lists.feature: Quizzes are capped a day
  it("says how many quizzes are waiting today and when the last comes in", async () => {
    mocked.calendarDay.mockResolvedValue({ ...emptyDay(TODAY, "Today"), quizzes: [{ topic: 1, name: "CPU", step: "Week", questions: 4, tightened_for: null }], waiting: [{ topic: 2, name: "GPU", step: "Week", questions: 3, tightened_for: null }, { topic: 3, name: "RAM", step: "Day", questions: 3, tightened_for: null }], clears_by: TODAY + 1, daily: 1 });
    const u = userEvent.setup();
    render(Calendar, {});
    await u.click(await screen.findByRole("button", { name: /^Today, Tue 3 Nov/ }));
    expect(await screen.findByText("2 more waiting")).toBeInTheDocument();
    expect(screen.getByText(/Only 1 topic quizzes come due a day, most overdue first. They keep their step, and the last comes in tomorrow./)).toBeInTheDocument();
  });
});


import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      topics: vi.fn(),
      topicQuizzes: vi.fn().mockResolvedValue([]),
      today: vi.fn().mockResolvedValue(20_760),
      quizSchedule: vi.fn(),
      setQuizSchedule: vi.fn(),
      resetQuizSchedule: vi.fn(),
      modules: vi.fn(),
      createTopic: vi.fn(),
      createModule: vi.fn(),
      deleteTopic: vi.fn(),
      deleteModule: vi.fn(),
      renameModule: vi.fn(),
      renameTopic: vi.fn(),
      setTopicTags: vi.fn(),
      moveTopic: vi.fn(),
      questionsIn: vi.fn(),
      deleteQuestion: vi.fn(),
      quizQueue: vi.fn().mockResolvedValue({ daily: 5, today: [], waiting: [], clears_by: null, next_four_weeks: 0 }),
      setQuizzesADay: vi.fn(),
    },
  };
});

import { api } from "../lib/api";
import { flushPendingDeletes, live } from "../lib/celebrations.svelte";
import Topics from "./Quizzes.svelte";

const mocked = vi.mocked(api);

function given(topics: Awaited<ReturnType<typeof api.topics>>, modules = [{ id: 1, name: "Computer Architecture" }]) {
  mocked.topics.mockResolvedValue(topics);
  mocked.modules.mockResolvedValue(modules);
  mocked.topicQuizzes.mockResolvedValue([]);
  mocked.today.mockResolvedValue(20_760);
  mocked.quizQueue.mockResolvedValue({ daily: 5, today: [], waiting: [], clears_by: null, next_four_weeks: 200 });
}

describe("Quizzes screen", () => {
  beforeEach(() => vi.resetAllMocks());

  // topics.feature: Create a topic / Topic outside any module. lists.feature: Narrow a long list by module
  it("lists topics with a module rail that counts and narrows them", async () => {
    given([
      { id: 1, name: "CPU", module: 1, tags: ["hardware"], question_count: 24, short_count: 0 },
      { id: 2, name: "Misc", module: null, tags: [], question_count: 4, short_count: 0 },
    ]);
    const u = userEvent.setup();
    render(Topics);
    const rail = await screen.findByRole("navigation", { name: "Modules" });
    expect(within(rail).getByRole("button", { name: "All modules 2 topics" })).toHaveAttribute("aria-pressed", "true");
    expect(within(rail).getByRole("button", { name: "No module 1 topic" })).toBeInTheDocument();
    const table = screen.getByRole("table", { name: "Quizzes" });
    expect(within(table).getByRole("button", { name: "CPU" }).closest("tr")).toHaveTextContent("Computer Architecture");
    expect(within(table).getByText("24")).toBeInTheDocument();
    await u.click(within(rail).getByRole("button", { name: /^Computer Architecture/ }));
    expect(within(table).queryByRole("button", { name: "Misc" })).toBeNull();
    expect(within(table).getByRole("button", { name: "CPU" }).closest("tr")).not.toHaveTextContent("Computer Architecture");
  });

  it("shows first steps when there are no quizzes", async () => {
    given([], []);
    render(Topics);
    expect(await screen.findByRole("heading", { level: 1, name: "Quizzes" })).toBeInTheDocument();
    expect(await screen.findByText("No quizzes yet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Make your first quiz" })).toHaveClass("btn-primary");
    // One primary button per view: the header's button steps back while the first steps show.
    expect(screen.getByRole("button", { name: /^New quiz/ })).not.toHaveClass("btn-primary");
  });

  // R189: a new quiz makes its topic and goes straight to writing the questions
  it("makes a quiz in a module and opens the question editor", async () => {
    given([{ id: 3, name: "CPU", module: 1, tags: [], question_count: 2, short_count: 0 }]);
    mocked.createTopic.mockResolvedValue(9);
    const onEdit = vi.fn();
    const user = userEvent.setup();
    render(Topics, { onEdit });
    await user.click(await screen.findByRole("button", { name: /^New quiz/ }));
    const dialog = screen.getByRole("dialog", { name: "New quiz" });
    await user.type(within(dialog).getByLabelText("Name"), "  GPU  ");
    await user.selectOptions(within(dialog).getByLabelText("Module"), "Computer Architecture");
    await user.click(within(dialog).getByRole("button", { name: "Create and add questions" }));
    expect(mocked.createTopic).toHaveBeenCalledWith("GPU", 1);
    expect(onEdit).toHaveBeenCalledWith(9);
  });

  it("shows the backend's message when making a quiz fails", async () => {
    given([]);
    mocked.createTopic.mockRejectedValue({ message: "give the topic a name" });
    const user = userEvent.setup();
    render(Topics);
    await user.click(await screen.findByRole("button", { name: "Make your first quiz" }));
    await user.click(screen.getByRole("button", { name: "Create and add questions" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("give the topic a name");
  });

  // R189: deleting a quiz takes its questions. The topic, with its cards and checklists, stays.
  it("deletes a quiz's questions after asking, keeping the topic", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 2, short_count: 1 }]);
    mocked.questionsIn.mockResolvedValue([[11, "a"], [12, "b"], [13, "c"]]);
    const user = userEvent.setup();
    render(Topics);
    await user.click(await screen.findByRole("button", { name: "CPU" }));
    await user.click(within(screen.getByRole("complementary", { name: "Picked topic" })).getByRole("button", { name: "Delete quiz" }));
    const dialog = screen.getByRole("dialog", { name: 'Delete the "CPU" quiz?' });
    expect(dialog).toHaveTextContent("Its 3 questions go. The topic, its cards and its checklists stay.");
    await user.click(within(dialog).getByRole("button", { name: "Delete quiz" }));
    expect(live.toasts.at(-1)).toMatchObject({ text: "Quiz deleted", sub: "CPU's 3 questions", action: { label: "Undo" } });
    expect(mocked.deleteQuestion).not.toHaveBeenCalled();
    await flushPendingDeletes();
    expect(mocked.deleteQuestion.mock.calls.map((call) => call[0])).toEqual([11, 12, 13]);
    expect(mocked.deleteTopic).not.toHaveBeenCalled();
  });

  it("renames a quiz from its card", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 2, short_count: 0 }]);
    const user = userEvent.setup();
    render(Topics);
    await user.click(await screen.findByRole("button", { name: "CPU" }));
    await user.click(within(screen.getByRole("complementary", { name: "Picked topic" })).getByRole("button", { name: "Rename CPU" }));
    const dialog = screen.getByRole("dialog", { name: "Rename quiz" });
    await user.clear(within(dialog).getByLabelText("Name"));
    await user.type(within(dialog).getByLabelText("Name"), "Processors");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(mocked.renameTopic).toHaveBeenCalledWith(1, "Processors");
  });

  // lists.feature: The item card. Rows carry no buttons of their own, Delete is never the first thing you see
  it("picks a topic into the item card, moves with the arrow keys and edits on Enter or double-click", async () => {
    given([
      { id: 1, name: "CPU", module: 1, tags: [], question_count: 3, short_count: 0 },
      { id: 2, name: "Memory", module: 1, tags: [], question_count: 2, short_count: 0 },
    ]);
    const u = userEvent.setup();
    const onEdit = vi.fn();
    const onQuiz = vi.fn();
    render(Topics, { onEdit, onQuiz });
    expect(await screen.findByText("Pick a quiz to see it here")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Delete/ })).toBeNull();
    await u.click(screen.getByRole("button", { name: "CPU" }));
    const card = screen.getByRole("complementary", { name: "Picked topic" });
    expect(within(card).getByRole("heading", { name: "CPU" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "CPU" }).closest("tr")).toHaveAttribute("aria-selected", "true");
    await u.keyboard("{ArrowDown}");
    expect(within(card).getByRole("heading", { name: "Memory" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Memory" })).toHaveFocus();
    await u.keyboard(" ");
    expect(screen.getByRole("checkbox", { name: "Tick Memory" })).toBeChecked();
    await u.keyboard("{Enter}");
    expect(onEdit).toHaveBeenCalledWith(2);
    await u.dblClick(screen.getByRole("button", { name: "CPU" }));
    expect(onEdit).toHaveBeenLastCalledWith(1);
    await u.click(within(screen.getByRole("complementary", { name: "Picked topic" })).getByRole("button", { name: "Quiz CPU" }));
    expect(onQuiz).toHaveBeenCalledWith(1);
  });

  // lists.feature: Filter as you type, Nothing matches
  it("filters on names, modules and tags, and says when nothing matches", async () => {
    given([
      { id: 1, name: "Caches", module: 1, tags: [], question_count: 3, short_count: 0 },
      { id: 2, name: "TLB", module: 1, tags: ["cache"], question_count: 2, short_count: 0 },
      { id: 3, name: "Routing", module: null, tags: [], question_count: 2, short_count: 0 },
    ]);
    const u = userEvent.setup();
    const onSearch = vi.fn();
    render(Topics, { onSearch });
    const filter = await screen.findByRole("searchbox", { name: "Filter" });
    await u.type(filter, "cache");
    expect(screen.getByText(/2 of 3/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Routing" })).toBeNull();
    expect(document.querySelector("mark")).toHaveTextContent("Cache");
    await u.keyboard("{Escape}");
    expect(filter).toHaveValue("");
    expect(screen.getByRole("button", { name: "Routing" })).toBeInTheDocument();
    await u.type(filter, "kerberos");
    expect(screen.getByText('Nothing matches "kerberos"')).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Search questions/ }));
    expect(onSearch).toHaveBeenCalledWith("kerberos");
    await u.click(screen.getByRole("button", { name: "Clear the filter" }));
    expect(screen.getAllByRole("row")).toHaveLength(4);
  });

  // lists.feature: Sort and pick columns
  it("sorts by a column both ways and remembers the columns shown", async () => {
    localStorage.clear();
    given([
      { id: 1, name: "B topic", module: 1, tags: ["x"], question_count: 3, short_count: 0 },
      { id: 2, name: "A topic", module: 1, tags: [], question_count: 9, short_count: 0 },
    ]);
    const u = userEvent.setup();
    const { unmount } = render(Topics);
    await screen.findByRole("table");
    const names = () => screen.getAllByRole("row").slice(1).map((row) => within(row).getAllByRole("button")[0].textContent);
    expect(names()).toEqual(["A topic", "B topic"]);
    const questions = screen.getByRole("columnheader", { name: /Questions/ });
    await u.click(within(questions).getByRole("button"));
    expect(questions).toHaveAttribute("aria-sort", "ascending");
    expect(names()).toEqual(["B topic", "A topic"]);
    await u.click(within(questions).getByRole("button"));
    expect(questions).toHaveAttribute("aria-sort", "descending");
    expect(names()).toEqual(["A topic", "B topic"]);
    await u.click(screen.getByRole("button", { name: "Columns" }));
    await u.click(screen.getByRole("checkbox", { name: "Tags" }));
    expect(screen.getByRole("columnheader", { name: "Tags" })).toBeInTheDocument();
    unmount();
    render(Topics);
    expect(await screen.findByRole("columnheader", { name: "Tags" })).toBeInTheDocument();
    localStorage.clear();
  });

  // lists.feature: Act on ticked topics
  it("ticks topics and moves them to a module together", async () => {
    given([
      { id: 1, name: "CPU", module: null, tags: [], question_count: 3, short_count: 0 },
      { id: 2, name: "Memory", module: null, tags: ["exam"], question_count: 2, short_count: 0 },
      { id: 3, name: "GDPR", module: null, tags: [], question_count: 2, short_count: 0 },
    ]);
    mocked.moveTopic = vi.fn().mockResolvedValue(undefined);
    const u = userEvent.setup();
    const onQuizTopics = vi.fn();
    render(Topics, { onQuizTopics });
    await u.click(await screen.findByRole("checkbox", { name: "Tick CPU" }));
    await u.click(screen.getByRole("checkbox", { name: "Tick Memory" }));
    const bar = screen.getByRole("region", { name: "Ticked topics" });
    expect(bar).toHaveTextContent("2 ticked");
    await u.click(within(bar).getByRole("button", { name: /Quiz them/ }));
    expect(onQuizTopics).toHaveBeenCalledWith([1, 2]);
    await u.click(within(bar).getByRole("button", { name: /Tag/ }));
    await u.type(screen.getByLabelText("Tag to add"), "exam");
    await u.click(screen.getByRole("button", { name: "Add tag" }));
    expect(mocked.setTopicTags).toHaveBeenCalledTimes(1);
    expect(mocked.setTopicTags).toHaveBeenCalledWith(1, ["exam"]);
    expect(live.toasts.at(-1)).toMatchObject({ text: 'Tagged 1 topic "exam"' });
    await u.click(within(screen.getByRole("region", { name: "Ticked topics" })).getByRole("button", { name: /Move to module/ }));
    await u.selectOptions(screen.getByLabelText("To module"), "Computer Architecture");
    await u.click(screen.getByRole("button", { name: "Move" }));
    expect(mocked.moveTopic).toHaveBeenCalledWith(1, 1);
    expect(mocked.moveTopic).toHaveBeenCalledWith(2, 1);
    // toasts.feature (R155): says what moved, and Undo moves them back
    expect(live.toasts.at(-1)).toMatchObject({ text: "2 topics moved to Computer Architecture" });
    live.toasts.at(-1)!.action!.run();
    await vi.waitFor(() => expect(mocked.moveTopic).toHaveBeenCalledWith(2, null));
    expect(mocked.moveTopic).toHaveBeenCalledWith(1, null);
    await u.click(screen.getByRole("checkbox", { name: "Tick all 3 shown" }));
    expect(screen.getByRole("region", { name: "Ticked topics" })).toHaveTextContent("3 ticked");
    await u.click(screen.getByRole("button", { name: "Untick all" }));
    expect(screen.queryByRole("region", { name: "Ticked topics" })).toBeNull();
  });

  // calendar.feature: A topic's quiz schedule moves with its score (R97)
  it("shows each topic's next quiz and quizzes the due ones together", async () => {
    const u = userEvent.setup();
    const onQuizTopics = vi.fn();
    given([
      { id: 1, name: "CPU", module: 1, tags: [], question_count: 7, short_count: 0 },
      { id: 2, name: "OSI model", module: 1, tags: [], question_count: 6, short_count: 2 },
      { id: 3, name: "Memory", module: 1, tags: [], question_count: 4, short_count: 0 },
      { id: 4, name: "TCP/IP", module: 1, tags: [], question_count: 5, short_count: 0 },
    ]);
    mocked.topicQuizzes.mockResolvedValue([
      { topic: 1, step: "Week", position: 1, steps: 7, due: 20_760, due_today: true, normal_due: 20_760, tightened_for: null, waiting: false },
      { topic: 2, step: "Day", position: 0, steps: 7, due: 20_755, due_today: true, normal_due: 20_755, tightened_for: null, waiting: false },
      { topic: 3, step: "Fortnight", position: 2, steps: 7, due: 20_766, due_today: false, normal_due: 20_780, tightened_for: "Networks exam", waiting: false },
    ]);
    render(Topics, { onQuizTopics });
    const row = (name: string) => screen.getByRole("button", { name }).closest("tr")!;
    await screen.findByText("Fortnight");
    expect(row("CPU")).toHaveTextContent(/Week\s*Due today/);
    expect(row("OSI model")).toHaveTextContent("Due today", { normalizeWhitespace: true });
    expect(row("Memory")).toHaveTextContent("Mon 9 Nov");
    // dates.feature: Tightened next quizzes are labelled
    expect(row("Memory")).toHaveTextContent("Tightened for Networks exam");
    expect(row("CPU")).not.toHaveTextContent("Tightened");
    expect(row("TCP/IP")).toHaveTextContent("Quiz it to start");
    expect(screen.getByText(/2 are due today/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /New quiz/ })).not.toHaveClass("btn-primary");
    await u.click(screen.getByRole("button", { name: /Quiz the 2 due topics/ }));
    expect(onQuizTopics).toHaveBeenCalledWith([1, 2]);
    // lists.feature: Only what's due
    await u.click(screen.getByRole("radio", { name: "Due 2" }));
    expect(screen.getAllByRole("row")).toHaveLength(3);
    await u.click(screen.getByRole("radio", { name: "Tightened 1" }));
    expect(row("Memory")).toBeInTheDocument();
  });

  it("edits the quiz schedule separately from recall", async () => {
    const u = userEvent.setup();
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 7, short_count: 0 }]);
    const schedule = { steps: [{ name: "Day", every: { count: 1, unit: "Days" as const }, repeats: 1 }, { name: "Yearly", every: { count: 1, unit: "Years" as const }, repeats: 0 }], pass_mark: 70, under_pass: "BackOneStep" as const };
    mocked.quizSchedule.mockResolvedValue(schedule);
    render(Topics, {});
    expect(screen.queryByRole("button", { name: /Quiz the/ })).toBeNull();
    await u.click(await screen.findByRole("button", { name: /Quiz schedule/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("heading", { name: "Quiz schedule" })).toBeInTheDocument();
    const pass = within(dialog).getByLabelText("Pass mark");
    await u.clear(pass);
    await u.type(pass, "80");
    // lists.feature: Quizzes are capped a day (the limit, and the warning)
    expect(within(dialog).getByText(/About 7 come due a day over the next four weeks, more than 5/)).toBeInTheDocument();
    const perDay = within(dialog).getByLabelText(/Topics to quiz a day/);
    await u.clear(perDay);
    await u.type(perDay, "8");
    expect(within(dialog).queryByText(/come due a day over the next four weeks/)).toBeNull();
    await u.click(within(dialog).getByRole("button", { name: "Save schedule" }));
    expect(mocked.setQuizSchedule).toHaveBeenCalledWith({ ...schedule, pass_mark: 80 });
    expect(mocked.setQuizzesADay).toHaveBeenCalledWith(8);
  });
});

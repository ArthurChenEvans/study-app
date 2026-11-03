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
      setTopicTags: vi.fn(),
    },
  };
});

import { api } from "../lib/api";
import Topics from "./Topics.svelte";

const mocked = vi.mocked(api);

function given(topics: Awaited<ReturnType<typeof api.topics>>, modules = [{ id: 1, name: "Computer Architecture" }]) {
  mocked.topics.mockResolvedValue(topics);
  mocked.modules.mockResolvedValue(modules);
  mocked.topicQuizzes.mockResolvedValue([]);
  mocked.today.mockResolvedValue(20_760);
}

describe("Topics screen", () => {
  beforeEach(() => vi.resetAllMocks());

  // topics.feature: Create a topic / Topic outside any module
  it("groups topics under their module, with a group for topics with no module", async () => {
    given([
      { id: 1, name: "CPU", module: 1, tags: ["hardware"], question_count: 24, short_count: 0 },
      { id: 2, name: "Misc", module: null, tags: [], question_count: 4, short_count: 0 },
    ]);
    render(Topics);
    const architecture = await screen.findByRole("region", { name: "Computer Architecture" });
    expect(within(architecture).getByText("CPU")).toBeInTheDocument();
    expect(within(architecture).getByText("hardware")).toBeInTheDocument();
    expect(within(architecture).getByText("24")).toBeInTheDocument();
    const none = screen.getByRole("region", { name: "No module" });
    expect(within(none).getByText("Misc")).toBeInTheDocument();
  });

  it("shows first steps when there are no topics", async () => {
    given([], []);
    render(Topics);
    expect(await screen.findByText("No topics yet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create your first topic" })).toBeInTheDocument();
  });

  it("creates a topic in a module", async () => {
    given([]);
    mocked.createTopic.mockResolvedValue(9);
    const user = userEvent.setup();
    render(Topics);
    await user.click(await screen.findByRole("button", { name: "Create your first topic" }));
    const dialog = screen.getByRole("dialog", { name: "New topic" });
    await user.type(within(dialog).getByLabelText("Name"), "  GPU  ");
    await user.selectOptions(within(dialog).getByLabelText("Module"), "Computer Architecture");
    mocked.topics.mockResolvedValue([{ id: 9, name: "GPU", module: 1, tags: [], question_count: 0, short_count: 0 }]);
    await user.click(within(dialog).getByRole("button", { name: "Create topic" }));
    expect(mocked.createTopic).toHaveBeenCalledWith("GPU", 1);
    expect(await screen.findByText("GPU")).toBeInTheDocument();
  });

  it("shows the backend's message when creating fails", async () => {
    given([]);
    mocked.createTopic.mockRejectedValue({ message: "give the topic a name" });
    const user = userEvent.setup();
    render(Topics);
    await user.click(await screen.findByRole("button", { name: "Create your first topic" }));
    await user.click(screen.getByRole("button", { name: "Create topic" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("give the topic a name");
  });

  // topics.feature: Delete a topic asks for confirmation
  it("asks before deleting, and says history is kept", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 24, short_count: 0 }]);
    const user = userEvent.setup();
    render(Topics);
    await user.click(await screen.findByRole("button", { name: "Delete CPU" }));
    const dialog = screen.getByRole("dialog", { name: 'Delete "CPU"?' });
    expect(dialog).toHaveTextContent("history isn't affected");
    expect(mocked.deleteTopic).not.toHaveBeenCalled();
    mocked.topics.mockResolvedValue([]);
    await user.click(within(dialog).getByRole("button", { name: "Delete topic" }));
    expect(mocked.deleteTopic).toHaveBeenCalledWith(1);
  });

  // R43
  it("deletes a module after asking, keeping its topics", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 3, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "Delete module Computer Architecture" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent('moved to "No module"');
    given([{ id: 1, name: "CPU", module: null, tags: [], question_count: 3, short_count: 0 }], []);
    await u.click(within(dialog).getByRole("button", { name: "Delete module" }));
    expect(mocked.deleteModule).toHaveBeenCalledWith(1);
    expect(await screen.findByRole("region", { name: "No module" })).toHaveTextContent("CPU");
  });

  // R51
  it("edits a topic's tags", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: ["hardware"], question_count: 3, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "Edit tags for CPU" }));
    const input = screen.getByLabelText("Tags");
    expect(input).toHaveValue("hardware");
    await u.clear(input);
    await u.type(input, "hardware, pipeline ,  ,exam");
    await u.click(screen.getByRole("button", { name: "Save tags" }));
    expect(mocked.setTopicTags).toHaveBeenCalledWith(1, ["hardware", "pipeline", "exam"]);
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
      { topic: 1, step: "Week", position: 1, steps: 7, due: 20_760, due_today: true },
      { topic: 2, step: "Day", position: 0, steps: 7, due: 20_755, due_today: true },
      { topic: 3, step: "Fortnight", position: 2, steps: 7, due: 20_766, due_today: false },
    ]);
    render(Topics, { onQuizTopics });
    const row = (name: string) => screen.getByRole("cell", { name }).closest("tr")!;
    await screen.findByText("Fortnight step");
    expect(row("CPU")).toHaveTextContent("TodayWeek step");
    expect(row("OSI model")).toHaveTextContent("Today", { normalizeWhitespace: true });
    expect(row("Memory")).toHaveTextContent("Mon 9 Nov");
    expect(row("TCP/IP")).toHaveTextContent("Quiz it to start");
    expect(screen.getByText(/2 quizzes due today/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /New topic/ })).not.toHaveClass("btn-primary");
    await u.click(screen.getByRole("button", { name: /Quiz the 2 due topics/ }));
    expect(onQuizTopics).toHaveBeenCalledWith([1, 2]);
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
    await u.click(within(dialog).getByRole("button", { name: "Save schedule" }));
    expect(mocked.setQuizSchedule).toHaveBeenCalledWith({ ...schedule, pass_mark: 80 });
  });
});

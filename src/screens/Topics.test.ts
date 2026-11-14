import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      topics: vi.fn(), modules: vi.fn(), cardsHome: vi.fn(), recallList: vi.fn(),
      createTopic: vi.fn(), createModule: vi.fn(), deleteTopic: vi.fn(), deleteModule: vi.fn(),
      renameModule: vi.fn(), renameTopic: vi.fn(), setTopicTags: vi.fn(), moveTopic: vi.fn(),
    },
  };
});

import { api, type CardsHome, type RecallRow, type TopicRow } from "../lib/api";
import { flushPendingDeletes, live } from "../lib/celebrations.svelte";
import Topics from "./Topics.svelte";

const mocked = vi.mocked(api);
const home = (per: [number, number][]): CardsHome => ({ due: { new: 0, learning: 0, review: 0 }, total_cards: 0, minutes: 0, forecast: [], retention: null, target: 90, suggestions: 0, per_topic: per.map(([topic, total]) => ({ topic, name: "", module: null, due: { new: 0, learning: 0, review: 0 }, total, retention: null })) });
const checklist = (name: string, module = false): RecallRow => ({ id: 1, subject_name: name, module, subject_exists: true, points: 4, step: null, next_step: "", position: null, due: 0, due_today: false, days_late: 0, last_score: null });

function given(topics: TopicRow[], modules = [{ id: 1, name: "Networks" }], cards: [number, number][] = [], checklists: RecallRow[] = []) {
  mocked.topics.mockResolvedValue(topics);
  mocked.modules.mockResolvedValue(modules);
  mocked.cardsHome.mockResolvedValue(home(cards));
  mocked.recallList.mockResolvedValue(checklists);
}

// R189: Topics & Modules is where topics and modules are made, renamed, moved and deleted.
describe("Topics & Modules", () => {
  beforeEach(() => vi.resetAllMocks());

  it("lists modules with their topics, and shows what's inside the picked topic", async () => {
    given(
      [{ id: 3, name: "TLS", module: 1, tags: ["exam"], question_count: 20, short_count: 2 }, { id: 4, name: "Misc", module: null, tags: [], question_count: 0, short_count: 0 }],
      [{ id: 1, name: "Networks" }],
      [[3, 96]],
      [checklist("TLS"), checklist("Networks", true)],
    );
    const onEdit = vi.fn();
    const onCards = vi.fn();
    const u = userEvent.setup();
    render(Topics, { onEdit, onCards });
    expect(await screen.findByRole("heading", { level: 1, name: "Topics & Modules" })).toBeInTheDocument();
    const tree = await screen.findByRole("navigation", { name: "Topics by module" });
    expect(within(tree).getByRole("heading", { name: "Networks" })).toBeInTheDocument();
    expect(within(tree).getByRole("heading", { name: "No module" })).toBeInTheDocument();
    await u.click(within(tree).getByRole("button", { name: "TLS" }));
    const picked = screen.getByRole("region", { name: "TLS" });
    expect(within(picked).getByLabelText("Module")).toHaveValue("1");
    // R189: each count is a big number with a short label, so nothing wraps in a narrow box
    expect(within(picked).getByRole("button", { name: "Open in Quizzes" }).parentElement).toHaveTextContent("Questions 22 Open");
    expect(within(picked).getByRole("button", { name: "Open in Cards" }).parentElement).toHaveTextContent("Cards 96 Open");
    expect(within(picked).getByRole("button", { name: "Open in Free Recall" }).parentElement).toHaveTextContent("Checklists 1 Open");
    await u.click(within(picked).getByRole("button", { name: "Open in Quizzes" }));
    expect(onEdit).toHaveBeenCalledWith(3);
    await u.click(within(picked).getByRole("button", { name: "Open in Cards" }));
    expect(onCards).toHaveBeenCalledWith(3);
  });

  it("moves the picked topic to another module", async () => {
    given([{ id: 3, name: "TLS", module: 1, tags: [], question_count: 2, short_count: 0 }], [{ id: 1, name: "Networks" }, { id: 2, name: "Security" }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "TLS" }));
    await u.selectOptions(within(screen.getByRole("region", { name: "TLS" })).getByLabelText("Module"), "Security");
    expect(mocked.moveTopic).toHaveBeenCalledWith(3, 2);
  });

  it("makes a topic in the picked module", async () => {
    given([]);
    mocked.createTopic.mockResolvedValue(9);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "New topic" }));
    const dialog = screen.getByRole("dialog", { name: "New topic" });
    await u.type(within(dialog).getByLabelText("Name"), "  DNS  ");
    await u.selectOptions(within(dialog).getByLabelText("Module"), "Networks");
    mocked.topics.mockResolvedValue([{ id: 9, name: "DNS", module: 1, tags: [], question_count: 0, short_count: 0 }]);
    await u.click(within(dialog).getByRole("button", { name: "Create topic" }));
    expect(mocked.createTopic).toHaveBeenCalledWith("DNS", 1);
    expect(live.toasts.at(-1)).toMatchObject({ text: "Topic added" });
    expect(await screen.findByRole("region", { name: "DNS" })).toBeInTheDocument();
  });

  it("makes a module", async () => {
    given([]);
    mocked.createModule.mockResolvedValue(5);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "New module" }));
    const dialog = screen.getByRole("dialog", { name: "New module" });
    await u.type(within(dialog).getByLabelText("Name"), "Security");
    await u.click(within(dialog).getByRole("button", { name: "Create module" }));
    expect(mocked.createModule).toHaveBeenCalledWith("Security");
  });

  it("renames a module and a topic", async () => {
    given([{ id: 3, name: "TLS", module: 1, tags: [], question_count: 2, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "Networks module actions" }));
    await u.click(screen.getByRole("menuitem", { name: "Rename module…" }));
    let dialog = screen.getByRole("dialog", { name: "Rename module" });
    await u.clear(within(dialog).getByLabelText("Name"));
    await u.type(within(dialog).getByLabelText("Name"), "Networking");
    await u.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(mocked.renameModule).toHaveBeenCalledWith(1, "Networking");
    await u.click(screen.getByRole("button", { name: "TLS" }));
    await u.click(within(screen.getByRole("region", { name: "TLS" })).getByRole("button", { name: "Rename" }));
    dialog = screen.getByRole("dialog", { name: "Rename topic" });
    await u.clear(within(dialog).getByLabelText("Name"));
    await u.type(within(dialog).getByLabelText("Name"), "TLS 1.3");
    await u.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(mocked.renameTopic).toHaveBeenCalledWith(3, "TLS 1.3");
  });

  // R51
  it("edits a topic's tags", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: ["hardware"], question_count: 3, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "CPU" }));
    const input = within(screen.getByRole("region", { name: "CPU" })).getByLabelText("Tags");
    expect(input).toHaveValue("hardware");
    await u.clear(input);
    await u.type(input, "hardware, pipeline ,  ,exam");
    await u.tab();
    expect(mocked.setTopicTags).toHaveBeenCalledWith(1, ["hardware", "pipeline", "exam"]);
  });

  // topics.feature: Delete a topic asks for confirmation, and says what goes with it
  it("asks before deleting a topic, and says what goes with it", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 24, short_count: 0 }], [{ id: 1, name: "Networks" }], [[1, 10]], [checklist("CPU")]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "CPU" }));
    await u.click(within(screen.getByRole("region", { name: "CPU" })).getByRole("button", { name: "Delete" }));
    const dialog = screen.getByRole("dialog", { name: 'Delete "CPU"?' });
    expect(dialog).toHaveTextContent("Its 24 questions, 10 cards and 1 checklist go with it.");
    expect(dialog).toHaveTextContent("history isn't affected");
    await u.click(within(dialog).getByRole("button", { name: "Delete topic" }));
    expect(screen.queryByRole("button", { name: "CPU" })).not.toBeInTheDocument();
    expect(live.toasts.at(-1)).toMatchObject({ text: "Topic deleted", action: { label: "Undo" } });
    expect(mocked.deleteTopic).not.toHaveBeenCalled();
    mocked.topics.mockResolvedValue([]);
    await flushPendingDeletes();
    expect(mocked.deleteTopic).toHaveBeenCalledWith(1);
  });

  // toasts.feature (R157): an empty topic skips the question, Undo brings it back
  it("deletes an empty topic straight away, and Undo puts it back", async () => {
    given([{ id: 1, name: "Scratch", module: null, tags: [], question_count: 0, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "Scratch" }));
    await u.click(within(screen.getByRole("region", { name: "Scratch" })).getByRole("button", { name: "Delete" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Scratch" })).not.toBeInTheDocument();
    live.toasts.at(-1)!.action!.run();
    expect(await screen.findByRole("button", { name: "Scratch" })).toBeInTheDocument();
    await flushPendingDeletes();
    expect(mocked.deleteTopic).not.toHaveBeenCalled();
  });

  // R43
  it("deletes a module after asking, keeping its topics", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 3, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.click(await screen.findByRole("button", { name: "Networks module actions" }));
    await u.click(screen.getByRole("menuitem", { name: "Delete module…" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent('moved to "No module"');
    given([{ id: 1, name: "CPU", module: null, tags: [], question_count: 3, short_count: 0 }], []);
    await u.click(within(dialog).getByRole("button", { name: "Delete module" }));
    expect(live.toasts.at(-1)).toMatchObject({ text: "Module deleted", action: { label: "Undo" } });
    await flushPendingDeletes();
    expect(mocked.deleteModule).toHaveBeenCalledWith(1);
    expect(await screen.findByRole("heading", { name: "No module" })).toBeInTheDocument();
  });

  it("filters the tree", async () => {
    given([{ id: 1, name: "CPU", module: 1, tags: [], question_count: 3, short_count: 0 }, { id: 2, name: "DNS", module: 1, tags: [], question_count: 3, short_count: 0 }]);
    const u = userEvent.setup();
    render(Topics);
    await u.type(await screen.findByRole("searchbox", { name: "Find a topic or module" }), "dn");
    expect(screen.queryByRole("button", { name: "CPU" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DNS" })).toBeInTheDocument();
  });
});

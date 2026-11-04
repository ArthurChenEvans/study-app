import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      projects: vi.fn(), issues: vi.fn(), today: vi.fn(), createProject: vi.fn(),
      project: vi.fn(), topics: vi.fn(), startProjectSession: vi.fn(), endProjectSession: vi.fn(), logStruggle: vi.fn(),
    },
  };
});

import { api, type DrillView, type ProjectRow, type ProjectView, type StruggleView } from "../lib/api";
import Projects from "./Projects.svelte";
import Project from "./Project.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const row = (overrides: Partial<ProjectRow> = {}): ProjectRow => ({ id: 1, name: "Study app", kind: "Coding", created: TODAY - 22, last_day: TODAY, struggles: 14, to_review: 2, drills_due: 1, session: null, ...overrides });
const drill: DrillView = { id: 5, task: "Loop over a slice", hints: "", issue: "Off-by-one in loops", topic: null, topic_name: null, created: TODAY - 1, due: TODAY, last: null, runs: 0, from: [] };
const struggle = (overrides: Partial<StruggleView> = {}): StruggleView => ({ id: 1, project: 1, project_name: "Study app", day: TODAY, minute: 860, what: "Lifetime error returning a reference", why: "didn't see the borrow", kind: "Apply", topic: 3, topic_name: "Rust", issue: null, outcome: null, ...overrides });

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.topics.mockResolvedValue([{ id: 3, name: "Rust", module: null, tags: [], question_count: 0, short_count: 0 }]);
});

describe("Projects", () => {
  // projects.feature (R87)
  it("lists projects with what's waiting, and makes new ones", async () => {
    const u = userEvent.setup();
    const onOpen = vi.fn();
    mocked.projects.mockResolvedValue([row(), row({ id: 2, name: "Blender mug", kind: "Blender", last_day: TODAY - 1, to_review: 0, drills_due: 0, session: { id: 9, project: 2, day: TODAY, started: 0, ended: null } })]);
    mocked.issues.mockResolvedValue([]);
    mocked.createProject.mockResolvedValue(7);
    render(Projects, { onOpen });
    const table = await screen.findByRole("table", { name: "Projects" });
    // A running session comes first, then the most recently worked on.
    const [second, first] = within(table).getAllByRole("row").slice(1);
    expect(first).toHaveTextContent("Study appCoding");
    expect(first).toHaveTextContent("Today");
    expect(first).toHaveTextContent("2 to review");
    expect(first).toHaveTextContent("1 today");
    expect(second).toHaveTextContent("Session running");
    expect(second).toHaveTextContent("None");
    await u.click(within(first).getByRole("button", { name: "Study app" }));
    expect(onOpen).toHaveBeenCalledWith(1);
    await u.click(screen.getByRole("button", { name: /New project/ }));
    const dialog = screen.getByRole("dialog");
    await u.type(within(dialog).getByLabelText("Name"), "Networks past paper");
    await u.click(within(dialog).getByRole("radio", { name: "Past paper" }));
    await u.click(within(dialog).getByRole("button", { name: "Create project" }));
    expect(mocked.createProject).toHaveBeenCalledWith("Networks past paper", "Past paper");
    expect(onOpen).toHaveBeenLastCalledWith(7);
  });

  it("takes a kind of its own under Something else", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue([row()]);
    mocked.issues.mockResolvedValue([]);
    mocked.createProject.mockResolvedValue(8);
    render(Projects, { onOpen: vi.fn() });
    await u.click(await screen.findByRole("button", { name: /New project/ }));
    const dialog = screen.getByRole("dialog");
    await u.type(within(dialog).getByLabelText("Name"), "Mug");
    await u.click(within(dialog).getByRole("radio", { name: "Something else" }));
    await u.type(within(dialog).getByLabelText("Kind"), "Pottery");
    await u.click(within(dialog).getByRole("button", { name: "Create project" }));
    expect(mocked.createProject).toHaveBeenCalledWith("Mug", "Pottery");
  });

  it("finds a project and filters by kind once there are many", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue(Array.from({ length: 7 }, (_, n) => row({ id: n + 1, name: `Project ${n}`, kind: n < 4 ? "Coding" : "Blender", last_day: TODAY - n })));
    mocked.issues.mockResolvedValue([]);
    render(Projects, {});
    const table = await screen.findByRole("table", { name: "Projects" });
    await u.click(screen.getByRole("radio", { name: "Blender 3" }));
    expect(within(table).getAllByRole("row")).toHaveLength(4);
    await u.type(screen.getByRole("searchbox", { name: "Find a project" }), "Project 6");
    expect(within(table).getAllByRole("row")).toHaveLength(2);
  });

  it("explains itself the first time", async () => {
    mocked.projects.mockResolvedValue([]);
    mocked.issues.mockResolvedValue([]);
    render(Projects, {});
    expect(await screen.findByRole("heading", { name: "Log what trips you up while you work" })).toBeInTheDocument();
  });

  // projects.feature (R94)
  it("lists the shared issues with their drills", async () => {
    const u = userEvent.setup();
    const onDrills = vi.fn();
    mocked.projects.mockResolvedValue([row()]);
    mocked.issues.mockResolvedValue([
      { id: 1, name: "Off-by-one in loops", seen: 4, projects: ["Networks past paper", "Study app"], last_seen: TODAY - 1, drill },
      { id: 2, name: "Mirror seams", seen: 1, projects: ["Blender mug"], last_seen: TODAY - 2, drill: { ...drill, id: 6, due: null, last: { drill: 6, day: TODAY - 6, rating: "Solid", seconds: 1, note: "" } } },
      { id: 3, name: "Borrowing in closures", seen: 1, projects: ["Study app"], last_seen: TODAY - 7, drill: null },
    ]);
    render(Projects, { onDrills });
    await u.click(await screen.findByRole("tab", { name: /Issues/ }));
    const rows = within(screen.getByRole("table", { name: "Issues" })).getAllByRole("row").slice(1);
    expect(rows[0]).toHaveTextContent("4 times in 2 projects");
    expect(rows[0]).toHaveTextContent("Due today");
    expect(rows[1]).toHaveTextContent("1 time in Blender mug");
    expect(rows[1]).toHaveTextContent("Solid on 28 Oct");
    expect(rows[2]).toHaveTextContent("None");
    await u.click(screen.getByRole("button", { name: "Do today's drill" }));
    expect(onDrills).toHaveBeenCalledWith([5]);
    await u.click(within(rows[0]).getByRole("button", { name: "Do the drill for Off-by-one in loops" }));
    expect(onDrills).toHaveBeenLastCalledWith([5]);
  });

  it("puts a due drill first, then the issues seen most", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue([row()]);
    mocked.issues.mockResolvedValue([
      { id: 1, name: "Seen once", seen: 1, projects: ["Study app"], last_seen: TODAY - 1, drill: null },
      { id: 2, name: "Seen a lot", seen: 6, projects: ["Study app", "Mug"], last_seen: TODAY - 30, drill: null },
      { id: 3, name: "Drill today", seen: 2, projects: ["Study app"], last_seen: TODAY - 9, drill },
    ]);
    render(Projects, {});
    await u.click(await screen.findByRole("tab", { name: /Issues/ }));
    const rows = within(screen.getByRole("table", { name: "Issues" })).getAllByRole("row").slice(1);
    expect(rows.map((r) => r.querySelector("td span")?.textContent)).toEqual(["Drill today", "Seen a lot", "Seen once"]);
  });
});

describe("A project", () => {
  const view = (overrides: Partial<ProjectRow> = {}, struggles: StruggleView[] = []): ProjectView => ({ row: row(overrides), struggles });

  // projects.feature: Work on a project in sessions, Log a struggle during a session
  it("starts a session, logs a struggle with Ctrl+Enter, and ends into the review", async () => {
    const u = userEvent.setup();
    const onReview = vi.fn();
    const running = { id: 9, project: 1, day: TODAY, started: Math.floor(Date.now() / 1000) - 2_520, ended: null };
    mocked.project.mockResolvedValueOnce(view({ to_review: 0 })).mockResolvedValue(view({ session: running, to_review: 1 }, [struggle()]));
    mocked.startProjectSession.mockResolvedValue(view({ session: running, to_review: 0 }));
    mocked.logStruggle.mockResolvedValue(struggle());
    render(Project, { id: 1, onReview });
    expect(screen.queryByRole("form", { name: "Log a struggle" })).toBeNull();
    await u.click(await screen.findByRole("button", { name: /Start a session/ }));
    expect(await screen.findByRole("timer", { name: "Session time" })).toHaveTextContent("0:42");
    expect(screen.getByLabelText("What tripped you up")).toHaveFocus();
    await u.type(screen.getByLabelText("What tripped you up"), "Lifetime error returning a reference");
    await u.type(screen.getByLabelText("Why wasn't it obvious?"), "didn't see the borrow");
    await u.click(screen.getByRole("radio", { name: "Couldn't apply it" }));
    await u.selectOptions(screen.getByLabelText("Topic (optional)"), "3");
    screen.getByLabelText("What tripped you up").focus();
    await u.keyboard("{Control>}{Enter}{/Control}");
    expect(mocked.logStruggle).toHaveBeenCalledWith(1, { what: "Lifetime error returning a reference", why: "didn't see the borrow", kind: "Apply", topic: 3 });
    expect(await screen.findByText(/To review at the end of this session/)).toBeInTheDocument();
    expect(screen.getByLabelText("What tripped you up")).toHaveValue("");
    await u.click(screen.getByRole("button", { name: /End session and review/ }));
    expect(mocked.endProjectSession).toHaveBeenCalledWith(1);
    expect(onReview).toHaveBeenCalledWith(1);
  });

  it("groups struggles by day, shows what became of them, and filters", async () => {
    const u = userEvent.setup();
    mocked.project.mockResolvedValue(view({ to_review: 1 }, [
      struggle(),
      struggle({ id: 2, day: TODAY - 1, minute: 600, what: "Forgot how to slice from the end", kind: "Recall", topic: null, topic_name: null, outcome: "Card" }),
      struggle({ id: 3, day: TODAY - 1, minute: 590, what: "Loop ran once too often", issue: "Off-by-one in loops", outcome: { Drill: { id: 5, due: TODAY } } }),
    ]));
    render(Project, { id: 1 });
    expect(await screen.findByText("Today · Tue 3 Nov")).toBeInTheDocument();
    const monday = screen.getByRole("list", { name: "Struggles on Mon 2 Nov" });
    expect(monday).toHaveTextContent("Card suggested");
    expect(monday).toHaveTextContent("Issue: Off-by-one in loops · drill today");
    expect(screen.getByRole("button", { name: /Review 1 struggle/ })).toBeInTheDocument();
    await u.click(screen.getByRole("radio", { name: /To review/ }));
    expect(screen.queryByRole("list", { name: "Struggles on Mon 2 Nov" })).toBeNull();
    await u.click(screen.getByRole("radio", { name: "All" }));
    await u.type(screen.getByRole("searchbox", { name: "Search struggles" }), "slice");
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });
});

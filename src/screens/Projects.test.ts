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
      updateProject: vi.fn(), deleteProject: vi.fn(), deleteStruggle: vi.fn(),
      updateStruggle: vi.fn(), renameIssue: vi.fn(), deleteIssue: vi.fn(), updateDrill: vi.fn(), deleteDrill: vi.fn(),
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
    const table = await screen.findByRole("table", { name: "Practice logs" });
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
    await u.click(screen.getByRole("button", { name: /New log/ }));
    const dialog = screen.getByRole("dialog", { name: "New practice log" });
    await u.type(within(dialog).getByLabelText("Name"), "Networks past paper");
    // R190: a quick pick fills the kind box, which is always there
    await u.click(within(dialog).getByRole("button", { name: "Past paper" }));
    expect(within(dialog).getByRole("button", { name: "Past paper" })).toHaveAttribute("aria-pressed", "true");
    expect(within(dialog).getByRole("textbox", { name: /^Kind/ })).toHaveValue("Past paper");
    await u.click(within(dialog).getByRole("button", { name: "Create log" }));
    expect(mocked.createProject).toHaveBeenCalledWith("Networks past paper", "Past paper");
    expect(onOpen).toHaveBeenLastCalledWith(7);
  });

  // R190: the kind box always shows, with a placeholder and how to nest a kind with ::
  it("takes a kind of its own, nested with ::", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue([row()]);
    mocked.issues.mockResolvedValue([]);
    mocked.createProject.mockResolvedValue(8);
    render(Projects, { onOpen: vi.fn() });
    await u.click(await screen.findByRole("button", { name: /New log/ }));
    const dialog = screen.getByRole("dialog", { name: "New practice log" });
    const kind = within(dialog).getByRole("textbox", { name: /^Kind/ });
    expect(kind).toHaveAttribute("placeholder", "Type any kind, e.g. Coding::Rust or Past paper::Maths");
    expect(kind).toHaveAccessibleDescription(/Put :: between levels to nest it, like Coding::Rust/);
    await u.type(within(dialog).getByLabelText("Name"), "Mug");
    await u.click(within(dialog).getByRole("button", { name: "Coding" }));
    await u.clear(kind);
    await u.type(kind, "Pottery::Glazing");
    expect(within(dialog).getByRole("button", { name: "Coding" })).toHaveAttribute("aria-pressed", "false");
    await u.click(within(dialog).getByRole("button", { name: "Create log" }));
    expect(mocked.createProject).toHaveBeenCalledWith("Mug", "Pottery::Glazing");
  });

  it("filters by the top of a nested kind", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue(Array.from({ length: 7 }, (_, n) => row({ id: n + 1, name: `Log ${n}`, kind: n < 2 ? "Coding::Rust" : n < 4 ? "Coding::Gleam" : "Blender", last_day: TODAY - n })));
    mocked.issues.mockResolvedValue([]);
    render(Projects, {});
    const kinds = await screen.findByRole("radiogroup", { name: "Kind" });
    expect(within(kinds).getAllByRole("radio").map((radio) => radio.textContent?.trim())).toEqual(["All 7", "Blender 3", "Coding 4"]);
    await u.click(within(kinds).getByRole("radio", { name: /Coding/ }));
    expect(within(screen.getByRole("table", { name: "Practice logs" })).getAllByRole("row")).toHaveLength(5);
  });

  it("finds a project and filters by kind once there are many", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue(Array.from({ length: 7 }, (_, n) => row({ id: n + 1, name: `Project ${n}`, kind: n < 4 ? "Coding" : "Blender", last_day: TODAY - n })));
    mocked.issues.mockResolvedValue([]);
    render(Projects, {});
    const table = await screen.findByRole("table", { name: "Practice logs" });
    await u.click(screen.getByRole("radio", { name: "Blender 3" }));
    expect(within(table).getAllByRole("row")).toHaveLength(4);
    await u.type(screen.getByRole("searchbox", { name: "Find a log" }), "Project 6");
    expect(within(table).getAllByRole("row")).toHaveLength(2);
  });

  it("explains itself the first time", async () => {
    mocked.projects.mockResolvedValue([]);
    mocked.issues.mockResolvedValue([]);
    render(Projects, {});
    expect(await screen.findByRole("heading", { name: "Log what trips you up while you work" })).toBeInTheDocument();
  });

  // R185: issues and drills can be edited and deleted from the issues list
  it("edits and deletes an issue's drill, renames and deletes the issue", async () => {
    const u = userEvent.setup();
    mocked.projects.mockResolvedValue([row()]);
    mocked.issues.mockResolvedValue([{ id: 1, name: "Off-by-one in loops", seen: 4, projects: ["Study app"], last_seen: TODAY - 1, drill }]);
    render(Projects, { onDrills: vi.fn() });
    await u.click(await screen.findByRole("tab", { name: /Issues/ }));
    const menu = () => screen.getByRole("button", { name: "Off-by-one in loops actions" });

    await u.click(menu());
    await u.click(screen.getByRole("menuitem", { name: "Edit drill…" }));
    let dialog = screen.getByRole("dialog", { name: "Edit drill" });
    expect(within(dialog).getByRole("textbox", { name: "Task" })).toHaveValue("Loop over a slice");
    await u.type(within(dialog).getByRole("textbox", { name: "Hints" }), "0..len");
    await u.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(mocked.updateDrill).toHaveBeenCalledWith(5, "Loop over a slice", "0..len");

    await u.click(menu());
    await u.click(screen.getByRole("menuitem", { name: "Rename issue…" }));
    dialog = screen.getByRole("dialog", { name: "Rename issue" });
    await u.clear(within(dialog).getByRole("textbox", { name: "Name" }));
    await u.type(within(dialog).getByRole("textbox", { name: "Name" }), "Off by one");
    await u.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(mocked.renameIssue).toHaveBeenCalledWith(1, "Off by one");

    await u.click(menu());
    await u.click(screen.getByRole("menuitem", { name: "Delete drill…" }));
    await u.click(within(screen.getByRole("dialog", { name: "Delete this drill?" })).getByRole("button", { name: "Delete drill" }));
    expect(mocked.deleteDrill).toHaveBeenCalledWith(5);

    await u.click(menu());
    await u.click(screen.getByRole("menuitem", { name: "Delete issue…" }));
    dialog = screen.getByRole("dialog", { name: "Delete the issue Off-by-one in loops?" });
    expect(dialog).toHaveTextContent("Struggles that named it keep going, without an issue.");
    await u.click(within(dialog).getByRole("button", { name: "Delete issue" }));
    expect(mocked.deleteIssue).toHaveBeenCalledWith(1);
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
    expect(rows[0]).toHaveTextContent("4 times in 2 logs");
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

  // R185: a project can be renamed and deleted, and a struggle deleted
  it("renames a project, deletes a struggle, then deletes the project", async () => {
    const u = userEvent.setup();
    const onBack = vi.fn();
    mocked.project.mockResolvedValue(view({}, [struggle()]));
    render(Project, { id: 1, onBack });
    await u.click(await screen.findByRole("button", { name: "Log actions" }));
    await u.click(screen.getByRole("menuitem", { name: "Rename log…" }));
    const rename = screen.getByRole("dialog", { name: "Rename log" });
    expect(within(rename).getByRole("textbox", { name: "Name" })).toHaveValue("Study app");
    expect(within(rename).getByRole("textbox", { name: "Kind" })).toHaveValue("Coding");
    await u.clear(within(rename).getByRole("textbox", { name: "Name" }));
    await u.type(within(rename).getByRole("textbox", { name: "Name" }), "Shudhana");
    await u.click(within(rename).getByRole("button", { name: "Save" }));
    expect(mocked.updateProject).toHaveBeenCalledWith(1, "Shudhana", "Coding");

    await u.click(screen.getByRole("button", { name: "Struggle actions: Lifetime error returning a reference" }));
    await u.click(screen.getByRole("menuitem", { name: "Edit struggle…" }));
    const edit = screen.getByRole("dialog", { name: "Edit struggle" });
    expect(within(edit).getByRole("textbox", { name: "Why it wasn't obvious" })).toHaveValue("didn't see the borrow");
    await u.type(within(edit).getByRole("textbox", { name: "What tripped you up" }), " again");
    await u.click(within(edit).getByRole("button", { name: "Save" }));
    expect(mocked.updateStruggle).toHaveBeenCalledWith(1, "Lifetime error returning a reference again", "didn't see the borrow");

    await u.click(screen.getByRole("button", { name: "Struggle actions: Lifetime error returning a reference" }));
    await u.click(screen.getByRole("menuitem", { name: "Delete struggle…" }));
    mocked.project.mockResolvedValue(view({ struggles: 0 }, []));
    await u.click(within(screen.getByRole("dialog", { name: "Delete this struggle?" })).getByRole("button", { name: "Delete struggle" }));
    expect(mocked.deleteStruggle).toHaveBeenCalledWith(1);

    await u.click(screen.getByRole("button", { name: "Log actions" }));
    await u.click(screen.getByRole("menuitem", { name: "Delete log…" }));
    const dialog = screen.getByRole("dialog", { name: /Delete .+\?/ });
    expect(dialog).toHaveTextContent("Its sessions and struggles go too.");
    await u.click(within(dialog).getByRole("button", { name: "Delete log" }));
    expect(mocked.deleteProject).toHaveBeenCalledWith(1);
    expect(onBack).toHaveBeenCalled();
  });
});

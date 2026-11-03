import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      strugglesToReview: vi.fn(), recallList: vi.fn(), topics: vi.fn(), issues: vi.fn(), today: vi.fn(), project: vi.fn(),
      knownDrill: vi.fn(), reviewStruggle: vi.fn(), drill: vi.fn(), rateDrill: vi.fn(), moveDrill: vi.fn(),
    },
  };
});

import { api, type DrillView, type RecallRow, type StruggleView } from "../lib/api";
import SessionReview from "./SessionReview.svelte";
import DrillSession from "./DrillSession.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const struggle = (id: number, overrides: Partial<StruggleView> = {}): StruggleView => ({ id, project: 1, project_name: "Study app", day: TODAY, minute: 860, what: `Struggle ${id}`, why: "a reason", kind: "Unsure", topic: null, topic_name: null, issue: null, outcome: null, ...overrides });
const drill: DrillView = { id: 5, task: "Write a loop over a slice without an off-by-one", hints: "Use 0..len", issue: "Off-by-one in loops", topic: null, topic_name: "Rust", created: TODAY - 6, due: null, last: { drill: 5, day: TODAY - 6, rating: "Solid", seconds: 300, note: "" }, runs: 1, from: [{ day: TODAY - 1, project_name: "Study app", what: "Loop ran once too often" }] };
const checklist = { id: 4, subject_name: "Rust" } as RecallRow;

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.recallList.mockResolvedValue([checklist]);
  mocked.topics.mockResolvedValue([]);
  mocked.issues.mockResolvedValue([{ id: 1, name: "Off-by-one in loops", seen: 4, projects: ["Networks past paper", "Study app"], last_seen: TODAY - 1, drill }]);
  mocked.project.mockResolvedValue({ row: { id: 1, name: "Study app", kind: "", created: 0, last_day: null, struggles: 0, to_review: 0, drills_due: 0, session: null }, struggles: [] });
  mocked.knownDrill.mockImplementation(async (issue) => (issue.toLowerCase().includes("off-by-one") ? drill : null));
  mocked.reviewStruggle.mockImplementation(async (id, choice) => struggle(id, { outcome: choice === "Card" ? "Card" : choice === "Nothing" ? "Nothing" : "Checklist" in choice ? { Checklist: { id: 4, name: "Rust" } } : { Drill: { id: 5, due: TODAY + 1 } }, issue: typeof choice === "object" && !("Checklist" in choice) ? "Off-by-one in loops" : null }));
});

describe("Session review", () => {
  // projects.feature: A recall miss becomes a card suggestion, A known issue offers its drill again
  it("steps through each struggle with the right default, and offers a known drill again", async () => {
    const u = userEvent.setup();
    const onSuggestions = vi.fn();
    mocked.strugglesToReview.mockResolvedValue([
      struggle(1, { kind: "Recall", what: "Forgot how to slice from the end" }),
      struggle(2, { kind: "Apply", what: "Wrote <= instead of <" }),
      struggle(3, { what: "Something else" }),
    ]);
    render(SessionReview, { project: 1, onSuggestions });
    expect(await screen.findByText("Forgot how to slice from the end")).toBeInTheDocument();
    expect(screen.getByText(/Struggle/).closest("p")).toHaveTextContent("Struggle 1 of 3");
    expect(screen.getByRole("radio", { name: /Suggest a card/ })).toBeChecked();
    await u.click(screen.getByRole("button", { name: /Next struggle/ }));
    expect(mocked.reviewStruggle).toHaveBeenLastCalledWith(1, "Card");

    expect(await screen.findByText("Wrote <= instead of <")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Schedule a drill/ })).toBeChecked();
    const issue = screen.getByLabelText("Issue");
    await u.type(issue, "off-by-one in loops");
    await u.tab();
    const note = await screen.findByRole("note");
    expect(note).toHaveTextContent("You have a drill for this issue");
    expect(note).toHaveTextContent("Last done on 28 Oct, rated Solid.");
    expect(screen.getByText("Seen 4 times before, in Networks past paper and Study app.")).toBeInTheDocument();
    expect(within(note).getByRole("button", { name: /Schedule this drill again/ })).toHaveAttribute("aria-pressed", "true");
    await u.click(screen.getByRole("button", { name: /Schedule the drill/ }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("radio", { name: /Tomorrow, Wed 4 Nov/ })).toBeChecked();
    await u.click(within(dialog).getByRole("button", { name: "Schedule for tomorrow" }));
    expect(mocked.reviewStruggle).toHaveBeenLastCalledWith(2, { DrillAgain: { issue: "off-by-one in loops", drill: 5, day: TODAY + 1 } });

    expect(await screen.findByText("Something else")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Finish review/ })).toBeDisabled();
    await u.click(screen.getByRole("radio", { name: /Nothing, just keep it logged/ }));
    await u.click(screen.getByRole("button", { name: /Finish review/ }));
    expect(mocked.reviewStruggle).toHaveBeenLastCalledWith(3, "Nothing");

    expect(await screen.findByRole("heading", { name: "Session reviewed" })).toBeInTheDocument();
    const next = screen.getByRole("list", { name: "What happens next" });
    expect(next).toHaveTextContent("1 card suggestion, waiting in Suggestions");
    expect(next).toHaveTextContent('Drill for "Off-by-one in loops" tomorrow');
    expect(next).toHaveTextContent("1 kept as notes");
    await u.click(screen.getByRole("button", { name: "Open Suggestions" }));
    expect(onSuggestions).toHaveBeenCalled();
  });

  // projects.feature: A struggle with a new issue gets a new drill, A struggle becomes a recall checklist point
  it("writes a new drill on another day, and adds a point to a checklist", async () => {
    const u = userEvent.setup();
    mocked.strugglesToReview.mockResolvedValue([struggle(1, { kind: "Apply" }), struggle(2, { kind: "Recall", topic: 3, topic_name: "Rust" })]);
    render(SessionReview, { project: 1 });
    await screen.findByText("Struggle 1", { selector: "p.text-lg" });
    await u.click(screen.getByRole("button", { name: /Schedule the drill/ }));
    expect(screen.getByRole("alert")).toHaveTextContent("Name the issue");
    await u.type(screen.getByLabelText("Issue"), "Borrowing in closures");
    await u.tab();
    await u.click(screen.getByRole("button", { name: /Schedule the drill/ }));
    expect(screen.getByRole("alert")).toHaveTextContent("Write the drill");
    await u.type(screen.getByLabelText("The drill"), "Spawn 3 threads using a Vec without cloning it");
    await u.type(screen.getByLabelText(/Hints/), "move");
    await u.click(screen.getByRole("button", { name: /Schedule the drill/ }));
    const dialog = screen.getByRole("dialog");
    await u.click(within(dialog).getByLabelText("Pick a day"));
    expect(within(dialog).getByRole("radio", { name: /Another day/ })).toBeChecked();
    await u.click(within(dialog).getByRole("button", { name: "Schedule for Thu 5 Nov" }));
    expect(mocked.reviewStruggle).toHaveBeenLastCalledWith(1, { NewDrill: { issue: "Borrowing in closures", task: "Spawn 3 threads using a Vec without cloning it", hints: "move", topic: null, day: TODAY + 2 } });

    await screen.findByText("Struggle 2", { selector: "p.text-lg" });
    await u.click(screen.getByRole("radio", { name: /Add to a recall checklist/ }));
    expect(screen.getByLabelText("Checklist")).toHaveValue("4");
    await u.click(screen.getByRole("button", { name: /Finish review/ }));
    expect(mocked.reviewStruggle).toHaveBeenLastCalledWith(2, { Checklist: { checklist: 4 } });
    expect(await screen.findByText("1 point added to recall checklists")).toBeInTheDocument();
  });

  // projects.feature: Finish a review later
  it("can finish later", async () => {
    const u = userEvent.setup();
    const onDone = vi.fn();
    mocked.strugglesToReview.mockResolvedValue([struggle(1), struggle(2)]);
    render(SessionReview, { project: 1, onDone });
    await u.click(await screen.findByRole("button", { name: /Finish later/ }));
    expect(onDone).toHaveBeenCalled();
    expect(mocked.reviewStruggle).not.toHaveBeenCalled();
  });
});

describe("Drills", () => {
  // projects.feature: Rating a drill
  it("rates with the keys and says what happens next", async () => {
    const u = userEvent.setup();
    const onLeave = vi.fn();
    mocked.drill.mockImplementation(async (id) => ({ ...drill, id, due: TODAY, task: id === 5 ? drill.task : "Mirror a mug with a clean seam" }));
    mocked.rateDrill.mockImplementation(async (id, rating, _seconds, note) => ({ ...drill, id, due: rating === "Solid" ? null : TODAY + 1, last: { drill: id, day: TODAY, rating, seconds: 30, note } }));
    render(DrillSession, { ids: [5, 6], onLeave });
    expect(await screen.findByRole("heading", { name: drill.task })).toBeInTheDocument();
    expect(screen.getByText(/Off-by-one in loops · Rust · from Study app/)).toBeInTheDocument();
    expect(screen.getByText('Mon 2 Nov, Study app: "Loop ran once too often"'.replace("Mon 2 Nov", "Yesterday"))).toBeInTheDocument();
    await u.click(screen.getByText("Show the hints"));
    expect(screen.getByText("Use 0..len")).toBeVisible();
    await u.type(screen.getByLabelText(/Note/), "second-guessed the bound");
    (document.activeElement as HTMLElement).blur();
    await u.keyboard("2");
    expect(mocked.rateDrill).toHaveBeenCalledWith(5, "Shaky", expect.any(Number), "second-guessed the bound");
    expect(await screen.findByRole("heading", { name: "Again tomorrow" })).toBeInTheDocument();
    expect(screen.getByText("second-guessed the bound")).toBeInTheDocument();
    await u.keyboard("{Enter}");
    expect(await screen.findByRole("heading", { name: "Mirror a mug with a clean seam" })).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Solid/ }));
    expect(await screen.findByRole("heading", { name: "Ticked off" })).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Done/ }));
    expect(onLeave).toHaveBeenCalled();
  });

  it("moves a drill that wasn't solid to another day", async () => {
    const u = userEvent.setup();
    mocked.drill.mockResolvedValue({ ...drill, due: TODAY });
    mocked.rateDrill.mockResolvedValue({ ...drill, due: TODAY + 1, last: { drill: 5, day: TODAY, rating: "Struggled", seconds: 1, note: "" } });
    mocked.moveDrill.mockResolvedValue({ ...drill, due: TODAY + 3, last: { drill: 5, day: TODAY, rating: "Struggled", seconds: 1, note: "" } });
    render(DrillSession, { ids: [5] });
    await u.click(await screen.findByRole("button", { name: /Struggled/ }));
    await u.click(await screen.findByRole("button", { name: /Move it to another day/ }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByLabelText("Do it on")).toHaveValue("2026-11-05");
    await u.click(within(dialog).getByRole("button", { name: /Move to Thu 5 Nov/ }));
    expect(mocked.moveDrill).toHaveBeenCalledWith(5, TODAY + 2);
    expect(await screen.findByRole("heading", { name: "Again on Fri 6 Nov" })).toBeInTheDocument();
  });
});

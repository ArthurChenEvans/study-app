import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { today: vi.fn(), startReview: vi.fn(), gradeCard: vi.fn(), undoCard: vi.fn(), leaveReview: vi.fn(), noteView: vi.fn() },
  };
});

import { api, type ReviewCard, type ReviewDone } from "../lib/api";
import CardReview from "./CardReview.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const card: ReviewCard = {
  card: 10, note: 100, front: "What stops the compiler caching a register?", back: "Declare it `volatile`", topic_name: "CPU", kind: "Basic", source: null,
  intervals: [0, 2, 6, 45], lapses: 0, leech: false, left: { new: 9, learning: 2, review: 6 }, queue: "review", done: 10, total: 27, can_undo: false,
};
const done: ReviewDone = { reviewed: 43, new: 10, again: 5, remembered: 88, target: 90, seconds: 1080, worth_rewriting: [{ note: 101, topic: 1, front: "Amdahl speedup?", agains: 2 }], due_tomorrow: 9, busiest: [TODAY + 6, 31] };

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.startReview.mockResolvedValue({ Card: structuredClone(card) });
  mocked.leaveReview.mockResolvedValue();
});

const props = (extra = {}) => ({ onLeave: vi.fn(), onEdit: vi.fn(), ...extra });

describe("Card review", () => {
  // flashcards.feature: Review a card, Grades show when the card comes back
  it("shows the front, reveals the answer with Space and grades with the keys", async () => {
    const u = userEvent.setup();
    mocked.gradeCard.mockResolvedValueOnce({ Card: { ...card, card: 11, front: "Next card", can_undo: true, queue: "new" } });
    render(CardReview, props({ scope: { Topic: 1 }, scopeName: "CPU" }));
    expect(await screen.findByRole("heading", { name: card.front })).toBeInTheDocument();
    expect(mocked.startReview).toHaveBeenCalledWith({ Topic: 1 });
    expect(screen.getByLabelText(/Left today: 9 new, 2 learning, 6 to review. This card is a review./)).toBeInTheDocument();
    expect(screen.queryByText("volatile")).toBeNull();
    expect(screen.getByRole("progressbar", { name: "Session progress" })).toHaveAttribute("aria-valuenow", "10");

    await u.keyboard(" ");
    expect(screen.getByText("volatile")).toBeInTheDocument();
    const again = screen.getByRole("button", { name: /Again/ });
    expect(again).toHaveTextContent("Later today");
    expect(screen.getByRole("button", { name: /Hard/ })).toHaveTextContent("2 days");
    expect(screen.getByRole("button", { name: /Easy/ })).toHaveTextContent("2 months");
    await u.keyboard("2");
    expect(mocked.gradeCard).toHaveBeenCalledWith("Hard", expect.any(Number));
    expect(await screen.findByRole("heading", { name: "Next card" })).toBeInTheDocument();
    expect(screen.getByLabelText(/This card is new./)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Hard/ })).toBeNull();
  });

  it("Space after the answer picks Good, and Undo goes back", async () => {
    const u = userEvent.setup();
    mocked.gradeCard.mockResolvedValueOnce({ Card: { ...card, card: 11, front: "Next card", can_undo: true } });
    mocked.undoCard.mockResolvedValueOnce({ Card: card });
    render(CardReview, props());
    await screen.findByRole("heading", { name: card.front });
    expect(screen.getByRole("button", { name: /Undo/ })).toBeDisabled();
    await u.click(screen.getByRole("button", { name: /Show answer/ }));
    await u.keyboard(" ");
    expect(mocked.gradeCard).toHaveBeenCalledWith("Good", expect.any(Number));
    await u.click(await screen.findByRole("button", { name: /Undo/ }));
    expect(mocked.undoCard).toHaveBeenCalled();
    expect(await screen.findByRole("heading", { name: card.front })).toBeInTheDocument();
  });

  // flashcards.feature: Cloze cards from gaps
  it("shows a cloze gap, then the whole text with the answer", async () => {
    mocked.startReview.mockResolvedValueOnce({ Card: { ...card, kind: "Cloze 2 of 2", front: 'L3 about <span class="cloze-gap">[...]</span>', back: 'L3 about <span class="cloze-answer">40</span>' } });
    const u = userEvent.setup();
    const { container } = render(CardReview, props());
    expect(await screen.findByText("[...]")).toHaveClass("cloze-gap");
    expect(screen.getByText(/CPU · Cloze 2 of 2/)).toBeInTheDocument();
    await u.keyboard(" ");
    expect(container.querySelector(".cloze-answer")).toHaveTextContent("40");
    expect(screen.queryByText("[...]")).toBeNull();
  });

  // flashcards.feature: A card that keeps slipping asks to be rewritten
  it("offers to rewrite a leech and leaves the session to edit", async () => {
    mocked.startReview.mockResolvedValueOnce({ Card: { ...card, leech: true, lapses: 9 } });
    mocked.noteView.mockResolvedValue({ id: 100, source: null, note: { topic: 3, kind: "Basic", front: "", back: "", cues: [], text: "", extra: "" }, cards: [] });
    const u = userEvent.setup();
    const handlers = props();
    render(CardReview, handlers);
    const note = await screen.findByRole("note");
    expect(note).toHaveTextContent("Again 9 times");
    await u.keyboard("e");
    expect(mocked.leaveReview).toHaveBeenCalled();
    expect(handlers.onEdit).toHaveBeenCalledWith(3, 100);
  });

  it("leaves the session", async () => {
    const u = userEvent.setup();
    const handlers = props();
    render(CardReview, handlers);
    await u.click(await screen.findByRole("button", { name: /Leave/ }));
    expect(mocked.leaveReview).toHaveBeenCalled();
    expect(handlers.onLeave).toHaveBeenCalled();
  });

  // flashcards.feature: The end of a session
  it("shows the summary with what's worth rewriting", async () => {
    mocked.startReview.mockResolvedValueOnce({ Done: done });
    const u = userEvent.setup();
    const handlers = props();
    render(CardReview, handlers);
    expect(await screen.findByRole("heading", { name: "Done for today" })).toBeInTheDocument();
    expect(screen.getByText("43")).toBeInTheDocument();
    expect(screen.getByText("Aiming for 90% · 5 times Again")).toBeInTheDocument();
    expect(screen.getByText("18 min")).toBeInTheDocument();
    expect(screen.getByText("25 s a card")).toBeInTheDocument();
    expect(screen.getByText(/9 due tomorrow. The busiest day this fortnight is Mon 9 Nov with 31./)).toBeInTheDocument();
    const list = screen.getByRole("region", { name: "Worth rewriting" });
    await u.click(within(list).getByRole("button", { name: /Rewrite/ }));
    expect(handlers.onEdit).toHaveBeenCalledWith(1, 101);
  });

  it("says when nothing is due", async () => {
    mocked.startReview.mockResolvedValueOnce({ Done: { ...done, reviewed: 0, worth_rewriting: [], busiest: null } });
    render(CardReview, props());
    expect(await screen.findByRole("heading", { name: "Nothing due" })).toBeInTheDocument();
  });
});

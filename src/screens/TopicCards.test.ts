import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { topicCards: vi.fn(), today: vi.fn(), topics: vi.fn(), noteView: vi.fn(), saveNote: vi.fn(), deleteNote: vi.fn(), archiveCard: vi.fn() },
  };
});

import { api, type NoteView, type TopicCards as TopicCardsView } from "../lib/api";
import TopicCards from "./TopicCards.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const data: TopicCardsView = {
  topic: 1, name: "CPU", module: "Computer Architecture", due: 3,
  cards: [
    { id: 10, note: 100, front: "Which design reads instructions and data at once?", kind: "Basic", source: "Written", due: TODAY, is_new: false, lapses: 0, leech: false, archived: false },
    { id: 11, note: 101, front: "Speedup of 60% sped up 4×?", kind: "Basic", source: "From quiz mistake", due: TODAY + 2, is_new: false, lapses: 9, leech: true, archived: false },
    { id: 12, note: 102, front: "Old card", kind: "Cloze 1 of 2", source: null, due: TODAY, is_new: true, lapses: 0, leech: false, archived: true },
  ],
};
const leechView: NoteView = {
  id: 101, source: "quiz:12",
  note: { topic: 1, kind: "Basic", front: "Speedup of 60% sped up 4×?", back: "Amdahl's law", cues: [], text: "", extra: "" },
  cards: [{ id: 11, key: "main", reps: 21, lapses: 9, leech: true, due: TODAY + 2, is_new: false, archived: false, remember_now: 62 }],
};

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topicCards.mockResolvedValue(structuredClone(data));
  mocked.today.mockResolvedValue(TODAY);
  mocked.topics.mockResolvedValue([{ id: 1, name: "CPU", module: 7, tags: [], question_count: 3, short_count: 0 }, { id: 2, name: "Memory", module: 7, tags: [], question_count: 0, short_count: 0 }]);
  mocked.noteView.mockResolvedValue(structuredClone(leechView));
  mocked.saveNote.mockResolvedValue(200);
});

const props = (extra = {}) => ({ topic: 1, onBack: vi.fn(), onReview: vi.fn(), onTopic: vi.fn(), ...extra });

describe("Topic cards", () => {
  // flashcards.feature: A topic's cards, Archive a card
  it("lists the cards with type, source, next date and leech flag", async () => {
    const u = userEvent.setup();
    const handlers = props();
    render(TopicCards, handlers);
    const table = await screen.findByRole("table", { name: "Cards in CPU" });
    const rows = within(table).getAllByRole("row").slice(1);
    expect(rows[0]).toHaveTextContent("Due today");
    expect(rows[1]).toHaveTextContent("From quiz mistake");
    expect(rows[1]).toHaveTextContent("Thu 5 Nov");
    expect(rows[1]).toHaveTextContent("9×");
    expect(rows[2]).toHaveTextContent("Cloze 1 of 2");
    expect(rows[2]).toHaveTextContent("Archived");
    await u.click(within(rows[0]).getByRole("button", { name: "Archive card" }));
    expect(mocked.archiveCard).toHaveBeenCalledWith(10, true);
    await u.click(within(rows[2]).getByRole("button", { name: "Bring back card" }));
    expect(mocked.archiveCard).toHaveBeenCalledWith(12, false);
    await u.click(screen.getByRole("button", { name: /Review the 3 due/ }));
    expect(handlers.onReview).toHaveBeenCalledWith({ Topic: 1 });
  });

  it("filters to what needs you, or by typing", async () => {
    const u = userEvent.setup();
    render(TopicCards, props());
    const table = await screen.findByRole("table", { name: "Cards in CPU" });
    const rows = () => within(table).getAllByRole("row").slice(1);
    expect(rows()[0]).toHaveTextContent("Due today");
    await u.click(screen.getByRole("radio", { name: "To review 1" }));
    expect(rows()).toHaveLength(1);
    await u.click(screen.getByRole("radio", { name: "New 0" }));
    expect(screen.getByText("No cards match.")).toBeInTheDocument();
    await u.click(screen.getByRole("radio", { name: "Tricky 1" }));
    expect(rows()[0]).toHaveTextContent("Speedup");
    await u.click(screen.getByRole("radio", { name: "Archived 1" }));
    expect(rows()[0]).toHaveTextContent("Old card");
    await u.click(screen.getByRole("radio", { name: "All 3" }));
    await u.type(screen.getByRole("searchbox", { name: "Filter cards" }), "design");
    expect(rows()).toHaveLength(1);
    await u.clear(screen.getByRole("searchbox", { name: "Filter cards" }));
    await u.type(screen.getByRole("searchbox", { name: "Filter cards" }), "nothing like it");
    expect(screen.getByText("No cards match.")).toBeInTheDocument();
  });

  it("shows a notice it was opened with", async () => {
    render(TopicCards, props({ notice: "2 cards added to CPU" }));
    expect(await screen.findByRole("status")).toHaveTextContent("2 cards added to CPU");
  });

  // flashcards.feature: Write a basic card with extra cues
  it("writes a basic card with an extra cue", async () => {
    const u = userEvent.setup();
    render(TopicCards, props());
    await u.click(await screen.findByRole("button", { name: /New card/ }));
    await u.type(screen.getByLabelText("Front"), "What stops caching a register?");
    await u.type(screen.getByLabelText("Back"), "volatile");
    await u.click(screen.getByRole("button", { name: /Add another cue/ }));
    await u.type(screen.getByLabelText("Extra cue 1"), "Why does my status read stale?");
    expect(screen.getByRole("button", { name: /Save 2 cards/ })).toBeInTheDocument();
    await u.keyboard("{Control>}s{/Control}");
    expect(mocked.saveNote).toHaveBeenCalledWith(null, { note: { topic: 1, kind: "Basic", front: "What stops caching a register?", back: "volatile", cues: ["Why does my status read stale?"], text: "", extra: "" }, suggestion: null });
    expect(await screen.findByText("2 cards added")).toBeInTheDocument();
  });

  // flashcards.feature: A reversed card makes two
  it("can save a reversed card into another topic, then opens that topic", async () => {
    const u = userEvent.setup();
    const handlers = props({ editNote: "new" });
    render(TopicCards, handlers);
    await u.click(await screen.findByRole("radio", { name: "Basic and reversed" }));
    expect(screen.getByText(/Makes two cards/)).toBeInTheDocument();
    await u.type(screen.getByLabelText("Front"), "TLB");
    await u.type(screen.getByLabelText("Back"), "A cache of page table entries");
    await u.selectOptions(screen.getByLabelText("Topic"), "2");
    await u.click(screen.getByRole("button", { name: /Save 2 cards/ }));
    expect(mocked.saveNote.mock.calls[0][1].note).toMatchObject({ topic: 2, kind: "Reversed" });
    await vi.waitFor(() => expect(handlers.onTopic).toHaveBeenCalledWith(2, "2 cards added to Memory"));
  });

  // flashcards.feature: Cloze cards from gaps
  it("makes cloze gaps from the selection and previews one card per number", async () => {
    const u = userEvent.setup();
    render(TopicCards, props({ editNote: "new" }));
    await u.click(await screen.findByRole("radio", { name: "Cloze" }));
    const box = screen.getByLabelText("Text") as HTMLTextAreaElement;
    await u.type(box, "L1 takes 4 cycles and L3 about 40");
    box.setSelectionRange(9, 10);
    await u.keyboard("{Control>}{Shift>}C{/Shift}{/Control}");
    expect(box.value).toBe("L1 takes {{c1::4}} cycles and L3 about 40");
    box.setSelectionRange(box.value.length - 2, box.value.length);
    await u.click(screen.getByRole("button", { name: /Make a gap/ }));
    expect(box.value).toBe("L1 takes {{c1::4}} cycles and L3 about {{c2::40}}");
    const preview = screen.getByRole("region", { name: "Preview" });
    expect(preview).toHaveTextContent("Makes 2 cards");
    expect(preview).toHaveTextContent("L1 takes [...] cycles and L3 about 40");
    expect(preview).toHaveTextContent("L1 takes 4 cycles and L3 about [...]");
    expect(screen.getByRole("button", { name: /Save 2 cards/ })).toBeInTheDocument();
  });

  it("shows the backend's error when saving fails", async () => {
    const u = userEvent.setup();
    mocked.saveNote.mockRejectedValueOnce({ message: "make at least one gap, like {{c1::answer}}" });
    render(TopicCards, props({ editNote: "new" }));
    await u.click(await screen.findByRole("radio", { name: "Cloze" }));
    await u.click(screen.getByRole("button", { name: /Save 0 cards/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("make at least one gap");
  });

  // flashcards.feature: A card that keeps slipping asks to be rewritten
  it("opens a leech note with the rewrite prompt and details, and deletes it", async () => {
    const u = userEvent.setup();
    render(TopicCards, props({ editNote: 101 }));
    expect(await screen.findByRole("note")).toHaveTextContent("Again 9 times.");
    const aside = screen.getByRole("complementary", { name: "Card details" });
    expect(aside).toHaveTextContent("Quiz mistake");
    expect(aside).toHaveTextContent("21 · 9");
    expect(aside).toHaveTextContent("62%");
    await u.click(screen.getByRole("button", { name: /Delete/ }));
    await u.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Delete" }));
    expect(mocked.deleteNote).toHaveBeenCalledWith(101);
    expect(await screen.findByText("Card deleted")).toBeInTheDocument();
  });
});

import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { cardsHome: vi.fn(), today: vi.fn(), modules: vi.fn(), topics: vi.fn(), browseCards: vi.fn(), noteView: vi.fn(), saveNote: vi.fn(), archiveCard: vi.fn(), deleteNote: vi.fn(), cardSettings: vi.fn(), setCardSettings: vi.fn(), resetCardSettings: vi.fn(), fitScheduler: vi.fn() },
  };
});

import { api, type BrowseRow, type CardsHome } from "../lib/api";
import { flushPendingDeletes, live } from "../lib/celebrations.svelte";
import Cards from "./Cards.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_760;
const home: CardsHome = {
  due: { new: 9, learning: 2, review: 16 }, total_cards: 120, minutes: 11, forecast: [27, 9, 4, 12, 31, 5, 3, 2, 1, 0, 6, 2, 3, 4], retention: 88, target: 90, suggestions: 4,
  per_topic: [{ topic: 1, name: "CPU", module: "Computer Architecture", due: { new: 4, learning: 2, review: 10 }, total: 80, retention: 91 }, { topic: 2, name: "OSI model", module: null, due: { new: 5, learning: 0, review: 6 }, total: 40, retention: null }],
};
const settings = { desired_retention: 0.9, new_per_day: 10, max_reviews: 200, leech_after: 8, parameters: [] };

beforeEach(() => {
  vi.resetAllMocks();
  mocked.cardsHome.mockResolvedValue(structuredClone(home));
  mocked.today.mockResolvedValue(TODAY);
  mocked.modules.mockResolvedValue([{ id: 7, name: "Computer Architecture" }]);
  mocked.topics.mockResolvedValue([]);
  mocked.cardSettings.mockResolvedValue({ ...settings });
});

const props = () => ({ onReview: vi.fn(), onTopic: vi.fn(), onSuggestions: vi.fn(), onNewCard: vi.fn(), onEditNote: vi.fn() });
const row = (over: Partial<BrowseRow>): BrowseRow => ({ id: 1, note: 1, topic: 1, topic_name: "CPU", module: 7, module_name: "Computer Architecture", tags: [], front: "Front", back: "Back", kind: "Basic", due: TODAY + 4, is_new: false, learning: false, archived: false, lapses: 0, ...over });

describe("Cards home", () => {
  // flashcards.feature: See what's due today, Review one topic or module
  it("shows today's counts and starts a review for the chosen scope", async () => {
    const u = userEvent.setup();
    const handlers = props();
    render(Cards, handlers);
    expect(await screen.findByText("27 due today")).toBeInTheDocument();
    expect(screen.getByLabelText("9 new, 2 learning, 16 to review")).toBeInTheDocument();
    expect(screen.getByText("About 11 minutes.")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Start review/ }));
    expect(handlers.onReview).toHaveBeenLastCalledWith(null);
    await u.selectOptions(screen.getByLabelText("Review which topics"), "module:7");
    await u.click(screen.getByRole("button", { name: /Start review/ }));
    expect(handlers.onReview).toHaveBeenLastCalledWith({ Module: 7 });
    await u.selectOptions(screen.getByLabelText("Review which topics"), "topic:2");
    await u.click(screen.getByRole("button", { name: /Start review/ }));
    expect(handlers.onReview).toHaveBeenLastCalledWith({ Topic: 2 });
  });

  // R191: Cards opens on Review, with no card fronts or answers in sight
  it("opens on Review with retention and the forecast, and no cards listed", async () => {
    const u = userEvent.setup();
    const handlers = props();
    render(Cards, handlers);
    expect(await screen.findByText("88%")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Review" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("aiming for 90%")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /next 14 days: 27, 9, 4, 12, 31/ })).toBeInTheDocument();
    expect(within(screen.getByRole("table", { name: "Cards due per day" })).getByRole("row", { name: "Sat 7 Nov 31" })).toBeInTheDocument();
    expect(screen.queryByRole("table", { name: "Cards by topic" })).not.toBeInTheDocument();
    expect(mocked.browseCards).not.toHaveBeenCalled();
    await u.click(screen.getByRole("button", { name: "Look through all 4" }));
    expect(handlers.onSuggestions).toHaveBeenCalled();
  });

  // R191: Browse all lists every card, filters fold away, and today's answers stay hidden
  it("browses every card, hiding the answers of cards due today", async () => {
    const u = userEvent.setup();
    const handlers = props();
    mocked.browseCards.mockResolvedValue([
      row({ id: 1, note: 10, front: "What is a TLB?", back: null, due: TODAY }),
      row({ id: 2, note: 11, front: "Cache line size", back: "64 bytes", topic: 2, topic_name: "OSI model", module: null, module_name: null, tags: ["exam"] }),
      row({ id: 3, note: 12, front: "Paused one", back: "Hidden? No", archived: true }),
    ]);
    render(Cards, handlers);
    await u.click(await screen.findByRole("tab", { name: "Browse all" }));
    const table = await screen.findByRole("table", { name: "All cards" });
    const first = within(table).getByRole("row", { name: /What is a TLB/ });
    expect(first).toHaveTextContent("Hidden until you review it");
    expect(within(table).getByRole("row", { name: /Cache line size/ })).toHaveTextContent("64 bytes");
    expect(screen.getByText("3 of 3 cards")).toBeInTheDocument();
    await u.click(within(first).getByRole("button", { name: "What is a TLB?" }));
    expect(handlers.onEditNote).toHaveBeenCalledWith(1, 10);
  });

  it("narrows with the search box and the filter panel, which folds away and remembers it", async () => {
    const u = userEvent.setup();
    localStorage.clear();
    mocked.browseCards.mockResolvedValue([
      row({ id: 1, front: "TLB", tags: ["exam"] }),
      row({ id: 2, front: "Subnet", topic: 2, topic_name: "OSI model", module: null, module_name: null, kind: "Cloze" }),
      row({ id: 3, front: "Paused", archived: true }),
    ]);
    const { unmount } = render(Cards, props());
    await u.click(await screen.findByRole("tab", { name: "Browse all" }));
    const search = await screen.findByRole("searchbox", { name: "Search cards" });
    await u.type(search, "type:cloze");
    expect(screen.getByText("1 of 3 cards")).toBeInTheDocument();
    await u.clear(search);
    const filters = screen.getByRole("region", { name: "Filters" });
    await u.click(within(filters).getByRole("checkbox", { name: /^Paused/ }));
    expect(screen.getByText("1 of 3 cards")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Remove filter: Paused" }));
    await u.click(within(filters).getByRole("checkbox", { name: /^OSI model/ }));
    expect(screen.getByText("1 of 3 cards")).toBeInTheDocument();
    const toggle = screen.getByRole("button", { name: /^Filters/ });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    await u.click(toggle);
    expect(screen.queryByRole("region", { name: "Filters" })).not.toBeInTheDocument();
    expect(toggle).toHaveTextContent("1");
    unmount();
    render(Cards, props());
    await u.click(await screen.findByRole("tab", { name: "Browse all" }));
    expect(await screen.findByRole("button", { name: /^Filters/ })).toHaveAttribute("aria-expanded", "false");
  });

  it("pauses, moves and deletes ticked cards", async () => {
    const u = userEvent.setup();
    mocked.browseCards.mockResolvedValue([row({ id: 1, note: 10, front: "TLB" }), row({ id: 2, note: 10, front: "TLB reversed" }), row({ id: 3, note: 11, front: "Cache" })]);
    mocked.topics.mockResolvedValue([{ id: 1, name: "CPU", module: 7, tags: [], question_count: 0, short_count: 0 }, { id: 2, name: "OSI model", module: null, tags: [], question_count: 0, short_count: 0 }]);
    mocked.noteView.mockResolvedValue({ id: 10, note: { topic: 1, kind: "Basic", front: "TLB", back: "x", cues: [], text: "", extra: "" }, source: null, cards: [] });
    render(Cards, props());
    await u.click(await screen.findByRole("tab", { name: "Browse all" }));
    await u.click(await screen.findByRole("checkbox", { name: "Tick TLB" }));
    await u.click(screen.getByRole("checkbox", { name: "Tick TLB reversed" }));
    const bar = screen.getByRole("region", { name: "Ticked cards" });
    expect(bar).toHaveTextContent("2 ticked");
    await u.click(within(bar).getByRole("button", { name: "Pause" }));
    expect(mocked.archiveCard.mock.calls).toEqual([[1, true], [2, true]]);
    await u.click(screen.getByRole("checkbox", { name: "Tick TLB" }));
    await u.click(screen.getByRole("checkbox", { name: "Tick TLB reversed" }));
    await u.click(within(screen.getByRole("region", { name: "Ticked cards" })).getByRole("button", { name: "Move to topic" }));
    const dialog = screen.getByRole("dialog", { name: "Move 2 cards" });
    await u.selectOptions(within(dialog).getByLabelText("To topic"), "OSI model");
    await u.click(within(dialog).getByRole("button", { name: "Move" }));
    expect(mocked.saveNote).toHaveBeenCalledTimes(1);
    expect(mocked.saveNote).toHaveBeenCalledWith(10, { note: expect.objectContaining({ topic: 2, front: "TLB" }), suggestion: null });
    await u.click(screen.getByRole("checkbox", { name: "Tick Cache" }));
    await u.click(within(screen.getByRole("region", { name: "Ticked cards" })).getByRole("button", { name: "Delete" }));
    expect(live.toasts.at(-1)).toMatchObject({ text: "Deleted 1 card", action: { label: "Undo" } });
    expect(mocked.deleteNote).not.toHaveBeenCalled();
    await flushPendingDeletes();
    expect(mocked.deleteNote).toHaveBeenCalledWith(11);
  });

  it("says done when nothing is due, and has a first-time state", async () => {
    mocked.cardsHome.mockResolvedValueOnce({ ...home, due: { new: 0, learning: 0, review: 0 } });
    const { unmount } = render(Cards, props());
    expect(await screen.findByText("Done for today")).toBeInTheDocument();
    expect(screen.getByText("9 due tomorrow.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Start review/ })).toBeNull();
    unmount();

    mocked.cardsHome.mockResolvedValueOnce({ ...home, total_cards: 0, per_topic: [], due: { new: 0, learning: 0, review: 0 } });
    const handlers = props();
    render(Cards, handlers);
    await userEvent.click(await screen.findByRole("button", { name: /Write your first card/ }));
    expect(handlers.onNewCard).toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /4 suggestions/ })).toBeInTheDocument();
  });

  // flashcards.feature: Change how much to remember, Fit the scheduler
  it("saves settings, including no limit on reviews", async () => {
    const u = userEvent.setup();
    render(Cards, props());
    await u.click(await screen.findByRole("button", { name: "Settings" }));
    const dialog = await screen.findByRole("dialog");
    const perDay = within(dialog).getByLabelText("New cards a day");
    await u.clear(perDay);
    await u.type(perDay, "15");
    await u.click(within(dialog).getByLabelText("No limit"));
    expect(within(dialog).getByLabelText("Most reviews a day")).toBeDisabled();
    await u.click(within(dialog).getByRole("button", { name: "Save settings" }));
    expect(mocked.setCardSettings).toHaveBeenCalledWith({ ...settings, new_per_day: 15, max_reviews: null });
  });

  it("reports fitting errors and success", async () => {
    const u = userEvent.setup();
    mocked.fitScheduler.mockRejectedValueOnce({ message: "fitting the scheduler needs at least 50 reviews, you have 12 so far" }).mockResolvedValueOnce(1840);
    render(Cards, props());
    await u.click(await screen.findByRole("button", { name: "Settings" }));
    await u.click(await screen.findByRole("button", { name: /Run now/ }));
    expect(await screen.findByText(/needs at least 50 reviews, you have 12/)).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Run now/ }));
    expect(await screen.findByText("Fitted to 1,840 reviews.")).toBeInTheDocument();
  });

  it("says 1 suggestion, not 1 suggestions, the first time", async () => {
    mocked.cardsHome.mockResolvedValueOnce({ ...home, total_cards: 0, per_topic: [], suggestions: 1, due: { new: 0, learning: 0, review: 0 } });
    render(Cards, props());
    expect(await screen.findByRole("button", { name: /^1 suggestion$/ })).toBeInTheDocument();
  });
});

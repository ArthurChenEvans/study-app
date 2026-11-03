import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { cardsHome: vi.fn(), today: vi.fn(), modules: vi.fn(), topics: vi.fn(), cardSettings: vi.fn(), setCardSettings: vi.fn(), resetCardSettings: vi.fn(), fitScheduler: vi.fn() },
  };
});

import { api, type CardsHome } from "../lib/api";
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

const props = () => ({ onReview: vi.fn(), onTopic: vi.fn(), onSuggestions: vi.fn(), onNewCard: vi.fn() });

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

  it("shows retention, the forecast peak and the topics table", async () => {
    const u = userEvent.setup();
    const handlers = props();
    render(Cards, handlers);
    expect(await screen.findByText("88%")).toBeInTheDocument();
    expect(screen.getByText("aiming for 90%")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /next 14 days: 27, 9, 4, 12, 31/ })).toBeInTheDocument();
    expect(screen.getByText("Sat 7 Nov · 31")).toBeInTheDocument();
    const table = screen.getByRole("table", { name: "Cards by topic" });
    expect(within(table).getByText("None yet")).toBeInTheDocument();
    await u.click(within(table).getByRole("button", { name: "OSI model" }));
    expect(handlers.onTopic).toHaveBeenCalledWith(2);
    await u.click(screen.getByRole("button", { name: "Look through them" }));
    expect(handlers.onSuggestions).toHaveBeenCalled();
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

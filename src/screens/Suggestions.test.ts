import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { cardSuggestions: vi.fn(), topics: vi.fn(), saveNote: vi.fn(), dismissSuggestion: vi.fn() } };
});

import { api, type Suggestion } from "../lib/api";
import Suggestions from "./Suggestions.svelte";

const mocked = vi.mocked(api);
const recall: Suggestion = { key: "recall:4:Amdahl's law", kind: "Recall", from: "Recall: Computer Architecture", day: 20_759, note: "missed 3 of the last 4 checks", text: "Amdahl's law", answer: null, topic: null };
const quiz: Suggestion = { key: "quiz:12", kind: "Quiz", from: "Quiz: OSI model", day: 20_758, note: "wrong", text: "Which layer adds port numbers?", answer: "Transport", topic: 2 };

beforeEach(() => {
  vi.resetAllMocks();
  mocked.cardSuggestions.mockResolvedValue([recall, quiz]);
  mocked.topics.mockResolvedValue([{ id: 1, name: "CPU", module: 7, tags: [], question_count: 3 }, { id: 2, name: "OSI model", module: null, tags: [], question_count: 5 }]);
  mocked.saveNote.mockResolvedValue(300);
});

describe("Suggestions", () => {
  // flashcards.feature: Suggestions wait for you to accept them
  it("filters by where they came from", async () => {
    const u = userEvent.setup();
    render(Suggestions, { onBack: vi.fn() });
    const list = await screen.findByRole("list", { name: "Suggestions" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(2);
    await u.click(screen.getByRole("radio", { name: "Quiz mistakes 1" }));
    expect(within(list).getAllByRole("listitem")).toHaveLength(1);
    expect(list).toHaveTextContent("→ Transport");
    await u.click(screen.getByRole("radio", { name: "Recall misses 1" }));
    expect(list).toHaveTextContent("Amdahl's law");
  });

  it("makes a card from a quiz mistake with the question and answer filled in", async () => {
    const u = userEvent.setup();
    const onChanged = vi.fn();
    mocked.cardSuggestions.mockResolvedValueOnce([recall, quiz]).mockResolvedValueOnce([recall]);
    render(Suggestions, { onBack: vi.fn(), onChanged });
    const item = (await screen.findAllByRole("listitem"))[1];
    await u.click(within(item).getByRole("button", { name: /Make a card/ }));
    expect(within(item).getByLabelText("Front")).toHaveValue("Which layer adds port numbers?");
    expect(within(item).getByLabelText("Back")).toHaveValue("Transport");
    await u.click(within(item).getByRole("button", { name: "Save card" }));
    expect(mocked.saveNote).toHaveBeenCalledWith(null, { note: { topic: 2, kind: "Basic", front: "Which layer adds port numbers?", back: "Transport", cues: [], text: "", extra: "" }, suggestion: "quiz:12" });
    expect(await screen.findByText("Card saved in OSI model")).toBeInTheDocument();
    expect(onChanged).toHaveBeenCalled();
  });

  it("leaves the front empty for a recall miss and asks for a topic", async () => {
    const u = userEvent.setup();
    render(Suggestions, { onBack: vi.fn() });
    const item = (await screen.findAllByRole("listitem"))[0];
    await u.click(within(item).getByRole("button", { name: /Make a card/ }));
    expect(within(item).getByLabelText("Front")).toHaveValue("");
    expect(within(item).getByLabelText("Back")).toHaveValue("Amdahl's law");
    await u.click(within(item).getByRole("button", { name: "Save card" }));
    expect(await within(item).findByRole("alert")).toHaveTextContent("Choose a topic");
    expect(mocked.saveNote).not.toHaveBeenCalled();
    await u.type(within(item).getByLabelText("Front"), "Overall speedup from a partial one?");
    await u.selectOptions(within(item).getByLabelText("Topic"), "1");
    await u.click(within(item).getByRole("button", { name: "Save card" }));
    expect(mocked.saveNote.mock.calls[0][1]).toMatchObject({ note: { topic: 1, front: "Overall speedup from a partial one?" }, suggestion: "recall:4:Amdahl's law" });
  });

  it("dismisses a suggestion and has an empty state", async () => {
    const u = userEvent.setup();
    mocked.cardSuggestions.mockResolvedValueOnce([recall, quiz]).mockResolvedValueOnce([]);
    render(Suggestions, { onBack: vi.fn() });
    await u.click(await screen.findByRole("button", { name: "Dismiss: Amdahl's law" }));
    expect(mocked.dismissSuggestion).toHaveBeenCalledWith("recall:4:Amdahl's law");
    expect(await screen.findByText("No suggestions right now")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Suggestion dismissed");
  });
});

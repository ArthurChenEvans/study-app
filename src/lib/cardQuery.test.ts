import { describe, expect, it } from "vitest";
import type { BrowseRow } from "./api";
import { filterCards, parseQuery, sortCards } from "./cardQuery";

const TODAY = 20_760;
const card = (over: Partial<BrowseRow>): BrowseRow => ({ id: 1, note: 1, topic: 1, topic_name: "DNS", module: 1, module_name: "Networks", tags: [], front: "Front", back: "Back", kind: "Basic", due: TODAY + 5, is_new: false, learning: false, archived: false, lapses: 0, ...over });

// R191: the Browse all search box
describe("card search", () => {
  it("reads words, left-out words and field filters, with quotes for spaces", () => {
    expect(parseQuery('handshake -tcp topic:"Karnaugh maps" module:net tag:exam is:due type:cloze')).toEqual({
      words: ["handshake"], not: ["tcp"], topic: ["karnaugh maps"], module: ["net"], tag: ["exam"], is: ["due"], type: ["cloze"],
    });
    expect(parseQuery("  ")).toEqual({ words: [], not: [], topic: [], module: [], tag: [], is: [], type: [] });
  });

  it("matches words in the front, a shown back or the topic, but never a hidden back", () => {
    const cards = [card({ id: 1, front: "What is TLS?", back: "Transport Layer Security" }), card({ id: 2, front: "Port for DNS", back: null }), card({ id: 3, front: "Handshake", back: "secret answer" })];
    const ids = (text: string) => filterCards(cards, parseQuery(text), {}, TODAY).map((row) => row.id);
    expect(ids("transport")).toEqual([1]);
    expect(ids("dns")).toEqual([1, 2, 3]);
    expect(ids("-tls")).toEqual([2, 3]);
    expect(ids("secret")).toEqual([3]);
    expect(filterCards([card({ id: 4, back: null })], parseQuery("back"), {}, TODAY)).toEqual([]);
  });

  it("filters by topic, module, tag, status and type, in the box or the panel", () => {
    const cards = [
      card({ id: 1, topic: 1, topic_name: "DNS", tags: ["exam"], due: TODAY }),
      card({ id: 2, topic: 2, topic_name: "Laws", module: 2, module_name: "Boolean Algebra", kind: "Cloze", is_new: true, due: TODAY }),
      card({ id: 3, topic: 1, topic_name: "DNS", archived: true }),
      card({ id: 4, topic: 1, topic_name: "DNS", learning: true, due: TODAY }),
    ];
    const ids = (text: string, panel = {}) => filterCards(cards, parseQuery(text), panel, TODAY).map((row) => row.id);
    expect(ids("topic:dns")).toEqual([1, 3, 4]);
    expect(ids("module:boolean")).toEqual([2]);
    expect(ids("tag:exam")).toEqual([1]);
    expect(ids("is:due")).toEqual([1, 2, 4]);
    expect(ids("is:new")).toEqual([2]);
    expect(ids("is:learning")).toEqual([4]);
    expect(ids("is:paused")).toEqual([3]);
    expect(ids("type:cloze")).toEqual([2]);
    expect(ids("", { topics: [2] })).toEqual([2]);
    expect(ids("", { status: ["paused", "new"] })).toEqual([2, 3]);
    expect(ids("", { kinds: ["Basic"], tags: ["exam"] })).toEqual([1]);
  });

  it("sorts by due date, newest, hardest or topic", () => {
    const cards = [card({ id: 1, due: TODAY + 3, lapses: 1, topic_name: "B" }), card({ id: 2, due: TODAY, lapses: 4, topic_name: "A" }), card({ id: 3, due: TODAY + 9, topic_name: "A", front: "Aa" })];
    expect(sortCards(cards, "due").map((row) => row.id)).toEqual([2, 1, 3]);
    // New cards come after the ones already in review, however early they were made.
    expect(sortCards([...cards, card({ id: 9, is_new: true, due: TODAY - 30 })], "due").map((row) => row.id)).toEqual([2, 1, 3, 9]);
    expect(sortCards(cards, "newest").map((row) => row.id)).toEqual([3, 2, 1]);
    expect(sortCards(cards, "hardest").map((row) => row.id)).toEqual([2, 1, 3]);
    expect(sortCards(cards, "topic").map((row) => row.id)).toEqual([3, 2, 1]);
  });
});

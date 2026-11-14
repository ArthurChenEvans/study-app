// R191: the Browse all search box and filter panel. Words search fronts, shown backs and topic
// names. A hidden back (a card due today) is never searched, so a match can't hint at it.
import type { BrowseRow } from "./api";

export interface CardQuery { words: string[]; not: string[]; topic: string[]; module: string[]; tag: string[]; is: string[]; type: string[] }
export type CardStatus = "due" | "new" | "learning" | "paused";
export interface CardPanel { topics?: number[]; status?: CardStatus[]; kinds?: BrowseRow["kind"][]; tags?: string[] }
export type CardSort = "due" | "newest" | "hardest" | "topic";

const FIELDS = ["topic", "module", "tag", "is", "type"] as const;

/** Splits the box into words, -left-out words and field:value filters. Quotes keep spaces. */
export function parseQuery(text: string): CardQuery {
  const query: CardQuery = { words: [], not: [], topic: [], module: [], tag: [], is: [], type: [] };
  for (const match of text.matchAll(/(-?)(?:(\w+):)?(?:"([^"]*)"|(\S+))/g)) {
    const [, minus, field, quoted, bare] = match;
    const value = (quoted ?? bare ?? "").trim().toLowerCase();
    if (!value) continue;
    const known = FIELDS.find((name) => name === field?.toLowerCase());
    if (known) query[known].push(value);
    else if (minus) query.not.push(field ? `${field.toLowerCase()}:${value}` : value);
    else query.words.push(field ? `${field.toLowerCase()}:${value}` : value);
  }
  return query;
}

const isDue = (row: BrowseRow, today: number) => !row.archived && row.due <= today;
const hasStatus = (row: BrowseRow, status: string, today: number) =>
  status === "due" ? isDue(row, today) : status === "new" ? row.is_new && !row.archived : status === "learning" ? row.learning && !row.archived : status === "paused" ? row.archived : status === "review" ? !row.is_new && !row.archived : false;

export function filterCards(rows: BrowseRow[], query: CardQuery, panel: CardPanel, today: number): BrowseRow[] {
  return rows.filter((row) => {
    const haystack = `${row.front}\n${row.back ?? ""}\n${row.topic_name}`.toLowerCase();
    const tags = row.tags.map((tag) => tag.toLowerCase());
    return query.words.every((word) => haystack.includes(word))
      && query.not.every((word) => !haystack.includes(word))
      && query.topic.every((value) => row.topic_name.toLowerCase().includes(value))
      && query.module.every((value) => (row.module_name ?? "").toLowerCase().includes(value))
      && query.tag.every((value) => tags.includes(value))
      && query.is.every((value) => hasStatus(row, value, today))
      && query.type.every((value) => row.kind.toLowerCase() === value)
      && (!panel.topics?.length || panel.topics.includes(row.topic))
      && (!panel.status?.length || panel.status.some((status) => hasStatus(row, status, today)))
      && (!panel.kinds?.length || panel.kinds.includes(row.kind))
      && (!panel.tags?.length || panel.tags.some((tag) => tags.includes(tag.toLowerCase())));
  });
}

export function sortCards(rows: BrowseRow[], by: CardSort): BrowseRow[] {
  const sorted = [...rows];
  const byTopic = (a: BrowseRow, b: BrowseRow) => a.topic_name.localeCompare(b.topic_name) || a.front.localeCompare(b.front);
  // New cards wait behind the ones already in review, however early they were made.
  if (by === "due") sorted.sort((a, b) => Number(a.is_new) - Number(b.is_new) || a.due - b.due || byTopic(a, b));
  else if (by === "newest") sorted.sort((a, b) => b.id - a.id);
  else if (by === "hardest") sorted.sort((a, b) => b.lapses - a.lapses || a.due - b.due);
  else sorted.sort(byTopic);
  return sorted;
}

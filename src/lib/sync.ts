// Words for sync (R119 to R128).
import type { SyncStatus, ToCheck } from "./api";
import { dayOf, dueLabel } from "./dates";

const clock = (date: Date) => `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
const localDay = (date: Date) => dayOf(date.getFullYear(), date.getMonth(), date.getDate());

/** "just now", "2 minutes ago", "3 hours ago", "yesterday at 22:10", "Mon 2 Nov". */
export function agoLabel(unix: number, now: number): string {
  const seconds = Math.max(0, now - unix);
  const then = new Date(unix * 1000);
  const today = localDay(new Date(now * 1000));
  const day = localDay(then);
  if (seconds < 60) return "just now";
  if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    return minutes === 1 ? "a minute ago" : `${minutes} minutes ago`;
  }
  if (day === today) {
    const hours = Math.floor(seconds / 3600);
    return hours === 1 ? "an hour ago" : `${hours} hours ago`;
  }
  if (day === today - 1) return `yesterday at ${clock(then)}`;
  // "Mon 2 Nov": the weekday and date, as for a day that's coming up.
  return dueLabel(day, day - 2);
}

/** When, for things to check: "Tue 3 Nov 21:40". */
export function whenLabel(unix: number, now: number): string {
  const then = new Date(unix * 1000);
  const day = localDay(then);
  const today = localDay(new Date(now * 1000));
  const date = day === today ? "Today" : day === today - 1 ? "Yesterday" : agoLabel(unix, now);
  return `${date} ${clock(then)}`;
}

const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;

/** The one-line status, for the sidebar and the top of Your data (R128). */
export type SyncIcon = "on" | "off" | "paused" | "missing" | "check";

export function syncLine(status: SyncStatus, now: number): { ok: boolean; short: string; title: string; detail: string; icon: SyncIcon } {
  if (!status.on) return { ok: true, short: "Sync is off", title: "Sync is off", detail: "Only this computer has your data.", icon: "off" };
  if (status.paused) return { ok: false, short: "Sync paused", title: status.paused, detail: "The other computers still have the newer changes. Turn sync back on when you're ready.", icon: "paused" };
  const ago = status.last_synced === null ? "not yet" : agoLabel(status.last_synced, now);
  if (status.problem?.startsWith("Can't find the sync folder")) {
    return { ok: false, short: "Sync paused", title: "Sync paused: can't find the sync folder", detail: `Last synced ${ago}. ${plural(status.waiting, "change")} here ${status.waiting === 1 ? "is" : "are"} kept and go out once it's back.`, icon: "missing" };
  }
  if (status.problem) return { ok: false, short: "Synced, with a problem", title: `Synced ${ago}`, detail: status.problem, icon: "paused" };
  if (status.to_check) return { ok: false, short: `Synced, ${status.to_check} to check`, title: `Synced ${ago}, ${plural(status.to_check, "thing")} to check`, detail: "The same thing was changed on two computers before they synced.", icon: "check" };
  const others = status.devices.filter((device) => !device.this_one).map((device) => device.name);
  if (!others.length) return { ok: true, short: "Waiting for the other computer", title: "Waiting for the other computer", detail: "Nothing else has joined this folder yet.", icon: "on" };
  const names = others.length === 1 ? others[0] : `${others.slice(0, -1).join(", ")} and ${others[others.length - 1]}`;
  return { ok: true, short: `Synced ${ago}`, title: `Synced ${ago}`, detail: `With ${names}.`, icon: "on" };
}

export type Piece = { text: string; kind: "same" | "added" | "removed" };

/** The words two versions don't share, for showing what differs (R123). */
export function wordDiff(older: string, newer: string): { older: Piece[]; newer: Piece[] } {
  const split = (text: string) => text.match(/\s+|[^\s]+/g) ?? [];
  const a = split(older);
  const b = split(newer);
  // Longest common run of words, by the usual table.
  const table = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--) for (let j = b.length - 1; j >= 0; j--) table[i][j] = a[i] === b[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
  const left: Piece[] = [];
  const right: Piece[] = [];
  const push = (list: Piece[], text: string, kind: Piece["kind"]) => {
    const last = list[list.length - 1];
    if (last && last.kind === kind) last.text += text;
    else list.push({ text, kind });
  };
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && a[i] === b[j]) {
      push(left, a[i], "same");
      push(right, b[j], "same");
      i++;
      j++;
    } else if (j < b.length && (i === a.length || table[i][j + 1] >= table[i + 1][j])) {
      push(right, b[j++], "added");
    } else {
      push(left, a[i++], "removed");
    }
  }
  return { older: left, newer: right };
}

const FIELDS: Record<string, string> = {
  front: "Front", back: "Back", text: "Question", explanation: "Explanation", name: "Name", options: "Answers", tags: "Tags", citations: "Sources",
  title: "Title", authors: "Authors", year: "Year", url: "Link", task: "Task", hints: "Hints", subject_name: "What it covers", points: "Points",
  archived: "Archived", module: "Module", topic: "Topic", short: "Accepted answers", cues: "Cues", extra: "Extra", value: "Setting", what: "What happened", why: "Why",
};

/** "note.back" is "Back", "subject_name" is "What it covers". */
export function fieldLabel(field: string): string {
  const name = field.includes(".") ? field.slice(field.lastIndexOf(".") + 1) : field;
  if (FIELDS[name]) return FIELDS[name];
  const words = name.replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Things to check, with the fields of one record together. */
export function groupChecks(list: ToCheck[]): { record: string; first: ToCheck; fields: ToCheck[] }[] {
  const groups: { record: string; first: ToCheck; fields: ToCheck[] }[] = [];
  for (const item of list) {
    const group = groups.find((existing) => existing.record === item.record);
    if (group) group.fields.push(item);
    else groups.push({ record: item.record, first: item, fields: [item] });
  }
  return groups;
}

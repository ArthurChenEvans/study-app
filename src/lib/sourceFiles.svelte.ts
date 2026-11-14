// Which file on this computer goes with which source (R182). Only the file's name is kept,
// never the file, and only on this computer, so it never travels with an export or sync.
import type { Id } from "./api";

const KEY = "source-files";

function read(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "{}") ?? {};
  } catch {
    return {};
  }
}

function write(files: Record<string, string>) {
  try {
    localStorage.setItem(KEY, JSON.stringify(files));
  } catch {
    // Remembering is only a convenience.
  }
}

export function fileFor(source: Id): string | null {
  return read()[source] ?? null;
}

export function remember(source: Id, name: string): void {
  write({ ...read(), [source]: name });
}

export function forget(source: Id): void {
  const files = read();
  delete files[source];
  write(files);
}

/** A question asking the preview to show where it cites. */
export const showing = $state<{ request: { source: Id; title: string; location: string; at: number } | null }>({ request: null });

export function showCitation(source: Id, title: string, location: string): void {
  showing.request = { source, title, location, at: Date.now() };
}

/** R202: the source of the tab being read and where, so a new question's "Fact came from"
 *  can fill itself in; and a count that goes up whenever the reader adds a source. */
export const reading = $state<{ source: Id | null; location: string; added: number }>({ source: null, location: "", added: 0 });

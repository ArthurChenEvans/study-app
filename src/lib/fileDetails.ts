// R202: a source filled in from the file's own details (a PDF's info, an Office file's core
// properties, an EPUB's metadata, a web page's tags), saying which fields came from the file
// so the check dialog can mark the rest "Not in the file".
import JSZip from "jszip";
import type { SourceFields } from "./api";
import type { WebDetails } from "./webPage";

export type FieldName = keyof SourceFields;
export interface FoundDetails { fields: SourceFields; fromFile: FieldName[]; from: string }

const empty = (): SourceFields => ({ authors: "", year: null, title: "", edition: null, description: null, publisher: null, url: null });
const yearOf = (text: string | undefined | null) => text?.match(/(1[5-9]|20)\d{2}/)?.[0] ?? null;
/** "lecture-5_virtual-memory.pdf" → "lecture 5 virtual memory" */
export const titleFromName = (name: string) => name.replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ").replace(/\s+/g, " ").trim();

function build(from: string, found: Partial<Record<FieldName, string | null | undefined>>, extra: Partial<SourceFields> = {}): FoundDetails {
  const fields = { ...empty(), ...extra };
  const fromFile: FieldName[] = [];
  for (const name of ["authors", "year", "title", "publisher", "url"] as FieldName[]) {
    const value = found[name]?.trim();
    if (value) {
      (fields as unknown as Record<string, string>)[name] = value;
      fromFile.push(name);
    }
  }
  if (!fields.title) fields.title = titleFromName(from);
  return { fields, fromFile, from };
}

export function detailsFromPdfInfo(info: Record<string, unknown>, name: string): FoundDetails {
  const text = (key: string) => (typeof info[key] === "string" ? (info[key] as string) : "");
  return build(name, { authors: text("Author"), year: yearOf(text("CreationDate").replace(/^D:/, "")), title: text("Title") });
}

export async function detailsFromOffice(data: ArrayBuffer, name: string): Promise<FoundDetails> {
  const slides = /\.(pptx|ppt|odp)$/i.test(name);
  const extra = slides ? { description: "Lecture slides" } : {};
  try {
    const zip = await JSZip.loadAsync(data);
    const core = await zip.file("docProps/core.xml")?.async("string");
    if (!core) return build(name, {}, extra);
    const doc = new DOMParser().parseFromString(core, "application/xml");
    const tag = (local: string) => doc.getElementsByTagNameNS("*", local)[0]?.textContent ?? "";
    return build(name, { authors: tag("creator"), year: yearOf(tag("created")), title: tag("title") }, extra);
  } catch {
    return build(name, {}, extra);
  }
}

export function detailsFromEpub(book: { title: string; creator?: string; date?: string; publisher?: string }, name: string): FoundDetails {
  return build(name, { authors: book.creator, year: yearOf(book.date), title: book.title === "Untitled book" ? "" : book.title, publisher: book.publisher });
}

export function detailsFromWeb(page: WebDetails): FoundDetails {
  return build(page.url, { authors: page.authors, year: page.year === null ? null : String(page.year), title: page.title, publisher: page.site, url: page.url });
}

const filled = (text: string | null) => (text && text.trim() ? text.trim() : null);

/** The same reference the backend writes (study-core bibliography::apa), for the check dialog. */
export function apaPreview(source: SourceFields): string {
  const authors = source.authors.trim();
  let reference = `${authors}${authors.endsWith(".") ? "" : "."} (${filled(source.year) ?? "n.d."}).`;
  reference += ` *${source.title.trim()}*`;
  if (filled(source.edition)) reference += ` (${filled(source.edition)} ed.)`;
  if (filled(source.description)) reference += ` [${filled(source.description)}]`;
  reference += ".";
  if (filled(source.publisher)) reference += ` ${filled(source.publisher)}.`;
  if (filled(source.url)) reference += ` ${filled(source.url)}`;
  return reference;
}

const squash = (text: string | null) => (text ?? "").toLowerCase().replace(/\s+/g, " ").trim();
const address = (url: string | null) => squash(url).replace(/^https?:\/\/(www\.)?/, "").replace(/\/+$/, "");

/** Whether a source is one already in the bibliography: the same web address, or the same title. */
export function sameSource(a: SourceFields, b: SourceFields): boolean {
  if (a.url && b.url && address(a.url) === address(b.url)) return true;
  return !!squash(a.title) && squash(a.title) === squash(b.title);
}

// R205: a source filled in from a DOI or an ISBN. A DOI is looked up through doi.org, which
// answers for every registry (Crossref, DataCite and the rest) with a standard citation record,
// so it works for journals whose own pages turn the app away. An ISBN is looked up in Google Books.
import type { SourceFields } from "./api";
import type { FieldName, FoundDetails } from "./fileDetails";

/** A DOI in a link or in text, as "10.xxxx/yyyy", or null. */
export function findDoi(text: string): string | null {
  let decoded = text;
  try { decoded = decodeURIComponent(text); } catch { /* left as typed */ }
  const match = decoded.match(/\b(10\.\d{4,9}\/[^\s"<>?#]+)/);
  return match ? match[1].replace(/[.,;:)\]]+$/, "") : null;
}

const isbn10Ok = (digits: string) => digits.split("").reduce((sum, digit, index) => sum + (digit === "X" ? 10 : Number(digit)) * (10 - index), 0) % 11 === 0;
const isbn13Ok = (digits: string) => digits.split("").reduce((sum, digit, index) => sum + Number(digit) * (index % 2 ? 3 : 1), 0) % 10 === 0;

/** An ISBN-10 or ISBN-13 with a correct check digit, without hyphens, or null. */
export function findIsbn(text: string): string | null {
  if (findDoi(text)) return null;
  for (const candidate of text.toUpperCase().match(/[\dX][\dX\s-]{8,16}[\dX]/g) ?? []) {
    const digits = candidate.replace(/[\s-]/g, "");
    if (/^97[89]\d{10}$/.test(digits) && isbn13Ok(digits)) return digits;
    if (/^\d{9}[\dX]$/.test(digits) && isbn10Ok(digits)) return digits;
  }
  return null;
}

/** "Megan A." → "M. A." */
const initials = (given: string) => given.split(/[\s.]+/).filter(Boolean).map((part) => part.split("-").map((piece) => `${piece[0].toUpperCase()}.`).join("-")).join(" ");

/** APA 7 author list: up to 20 names, "&" before the last, and past 20 the first 19, ". . ." and the last. */
function apaAuthors(names: string[]): string {
  if (names.length <= 1) return names[0] ?? "";
  if (names.length > 20) return `${names.slice(0, 19).join(", ")}, . . . ${names[names.length - 1]}`;
  return `${names.slice(0, -1).join(", ")}, & ${names[names.length - 1]}`;
}

function build(from: string, found: Partial<Record<FieldName, string | null>>): FoundDetails {
  const fields: SourceFields = { authors: "", year: null, title: "", edition: null, description: null, publisher: null, url: null };
  const fromFile: FieldName[] = [];
  for (const name of ["authors", "year", "title", "publisher", "url"] as FieldName[]) {
    const value = found[name]?.trim();
    if (value) {
      (fields as unknown as Record<string, string>)[name] = value;
      fromFile.push(name);
    }
  }
  return { fields, fromFile, from };
}

interface CslName { family?: string; given?: string; literal?: string }
interface Csl { type?: string; title?: string | string[]; "container-title"?: string | string[]; author?: CslName[]; editor?: CslName[]; issued?: { "date-parts"?: (number | string)[][] }; publisher?: string; DOI?: string }

const first = (value: string | string[] | undefined) => (Array.isArray(value) ? value[0] : value) ?? "";

export function cslToFields(record: Csl, doi: string): FoundDetails {
  const names = (record.author ?? record.editor ?? []).map((name) => name.literal ?? (name.family ? (name.given ? `${name.family}, ${initials(name.given)}` : name.family) : "")).filter(Boolean);
  const year = record.issued?.["date-parts"]?.[0]?.[0];
  return build(`DOI ${doi}`, {
    authors: apaAuthors(names),
    year: year ? String(year) : null,
    title: first(record.title),
    publisher: first(record["container-title"]) || record.publisher || null,
    url: `https://doi.org/${record.DOI ?? doi}`,
  });
}

interface Books { totalItems?: number; items?: { volumeInfo?: { title?: string; subtitle?: string; authors?: string[]; publisher?: string; publishedDate?: string } }[] }

export function booksToFields(result: Books, isbn: string): FoundDetails | null {
  const book = result.items?.[0]?.volumeInfo;
  if (!book?.title) return null;
  // "Noam Nisan" → "Nisan, N."
  const names = (book.authors ?? []).map((name) => {
    const parts = name.trim().split(/\s+/);
    return parts.length > 1 ? `${parts[parts.length - 1]}, ${initials(parts.slice(0, -1).join(" "))}` : name;
  });
  return build(`ISBN ${isbn}`, {
    authors: apaAuthors(names),
    year: book.publishedDate?.match(/^\d{4}/)?.[0] ?? null,
    title: book.subtitle ? `${book.title}: ${book.subtitle}` : book.title,
    publisher: book.publisher ?? null,
  });
}

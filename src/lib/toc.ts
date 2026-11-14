// Tables of contents and citation targets for the source preview (R179 to R181).

export interface Heading { title: string; depth: number; line: number }

/** Markdown headings, with the line each starts on. Code blocks are skipped. */
export function markdownHeadings(text: string): Heading[] {
  const headings: Heading[] = [];
  let fenced = false;
  text.split(/\r?\n/).forEach((line, index) => {
    if (/^\s*(```|~~~)/.test(line)) fenced = !fenced;
    if (fenced) return;
    const match = /^(#{1,6})\s+(.+?)\s*#*\s*$/.exec(line);
    if (match) headings.push({ title: match[2], depth: match[1].length, line: index + 1 });
  });
  return headings;
}

/** "p. 42", "pp. 42-45", "slide 3", "line 120" or a bare number. Null when it isn't a page or line. */
export function locationTarget(location: string): { page: number } | { line: number } | null {
  const text = location.trim().toLowerCase();
  const line = /^(?:lines?|ll?\.)\s*(\d+)/.exec(text);
  if (line) return { line: Number(line[1]) };
  const page = /^(?:(?:pages?|pp?\.|slides?)\s*)?(\d+)/.exec(text);
  return page ? { page: Number(page[1]) } : null;
}

export interface TocItem { title: string; depth: number; at: number }
interface TextBit { str: string; height: number }

/** R197: a page's heading is its biggest text, when it's clearly bigger than the rest. */
export function pageHeading(items: TextBit[]): string | null {
  const bits = items.filter((bit) => bit.str.trim());
  if (!bits.length) return null;
  const heights = bits.map((bit) => bit.height).sort((a, b) => a - b);
  const median = heights[Math.floor((heights.length - 1) / 2)];
  const biggest = heights[heights.length - 1];
  if (biggest < median * 1.25) return null;
  const title = bits.filter((bit) => bit.height >= biggest * 0.9).map((bit) => bit.str.trim()).join(" ").replace(/\s+/g, " ").trim();
  return title ? (title.length > 80 ? `${title.slice(0, 79)}…` : title) : null;
}

/** Contents for a PDF with no outline: each page's heading. Text on most pages (a running
 *  title or footer) is never a heading, and a heading carried over to the next page is listed once. */
export function pageHeadings(pages: TextBit[][]): TocItem[] {
  const seen = new Map<string, number>();
  for (const page of pages) for (const text of new Set(page.map((bit) => bit.str.trim()).filter(Boolean))) seen.set(text, (seen.get(text) ?? 0) + 1);
  const running = (text: string) => pages.length >= 3 && (seen.get(text) ?? 0) > pages.length / 2;
  const rows: TocItem[] = [];
  pages.forEach((page, index) => {
    const title = pageHeading(page);
    if (title && running(title)) return;
    if (title && rows.at(-1)?.title !== title) rows.push({ title, depth: 1, at: index + 1 });
  });
  return rows;
}

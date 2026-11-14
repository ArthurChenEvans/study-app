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

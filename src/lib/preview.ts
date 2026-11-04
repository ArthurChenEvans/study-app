// The source preview beside the editor (R11, R12). Files live only in memory, in open tabs.
export type Kind = "pdf" | "slides" | "epub" | "markdown" | "text";
export interface Tab { id: number; name: string; kind: Kind }

const kinds: Record<string, Kind> = {
  pdf: "pdf",
  pptx: "slides",
  ppt: "slides",
  odp: "slides",
  epub: "epub",
  md: "markdown",
  markdown: "markdown",
  txt: "text",
};

export function kindOf(name: string): Kind | null {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return null;
  return kinds[name.slice(dot + 1).toLowerCase()] ?? null;
}

export function numberLines(text: string): { number: number; text: string }[] {
  return text.split(/\r?\n/).map((line, index) => ({ number: index + 1, text: line }));
}

export function openTab(tabs: Tab[], tab: Tab): { tabs: Tab[]; active: number | null } {
  return { tabs: [...tabs, tab], active: tab.id };
}

export function closeTab(tabs: Tab[], active: number | null, id: number): { tabs: Tab[]; active: number | null } {
  const index = tabs.findIndex((tab) => tab.id === id);
  const remaining = tabs.filter((tab) => tab.id !== id);
  if (active !== id) return { tabs: remaining, active };
  const neighbour = remaining[index] ?? remaining[index - 1] ?? null;
  return { tabs: remaining, active: neighbour?.id ?? null };
}

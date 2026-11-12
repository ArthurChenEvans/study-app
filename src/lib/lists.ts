// Shared helpers for long lists (R138 to R144): filtering, sorting and remembering columns.

/** Whether any of `fields` contains `term`, ignoring case. An empty term matches everything. */
export function matchesFilter(term: string, ...fields: (string | null | undefined)[]): boolean {
  const needle = term.trim().toLowerCase();
  if (!needle) return true;
  return fields.some((field) => field?.toLowerCase().includes(needle));
}

/** Sorts by `value`, nulls last whichever way, then by `tiebreak`. */
export function sortBy<T>(list: T[], value: (item: T) => string | number | null, up: boolean, tiebreak: (item: T) => string = () => ""): T[] {
  return [...list].sort((a, b) => {
    const [left, right] = [value(a), value(b)];
    if (left === null && right === null) return tiebreak(a).localeCompare(tiebreak(b));
    if (left === null) return 1;
    if (right === null) return -1;
    const order = typeof left === "string" && typeof right === "string" ? left.localeCompare(right, undefined, { sensitivity: "base" }) : Number(left) - Number(right);
    return (up ? order : -order) || tiebreak(a).localeCompare(tiebreak(b));
  });
}

/** Columns shown, remembered on this device. Storage can throw or be empty, so it falls back. */
export function savedColumns(key: string, fallback: Record<string, boolean>): Record<string, boolean> {
  try {
    const saved = JSON.parse(localStorage.getItem(key) ?? "null");
    return saved && typeof saved === "object" ? { ...fallback, ...saved } : { ...fallback };
  } catch {
    return { ...fallback };
  }
}

export function saveColumns(key: string, columns: Record<string, boolean>): void {
  try {
    localStorage.setItem(key, JSON.stringify(columns));
  } catch {
    // Remembering is only a convenience.
  }
}

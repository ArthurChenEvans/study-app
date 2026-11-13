// navigation.feature (R162): on phones a list table is a stack of cards (see app.css). Each
// value shows its column's name in front, taken from the table's own headers, so a table only
// needs `use:stackTable` to stack properly.

/** The header's name for each column: blank for the first (the row's name, shown on top) and
 *  for columns with no visible name (a button column). */
function columnNames(table: HTMLTableElement): string[] {
  const headers = [...(table.tHead?.rows[0]?.cells ?? [])];
  return headers.map((header, index) => {
    if (index === 0) return "";
    const visible = header.cloneNode(true) as HTMLElement;
    visible.querySelectorAll(".sr-only").forEach((hidden) => hidden.remove());
    return visible.textContent?.trim() ?? "";
  });
}

function label(table: HTMLTableElement) {
  const names = columnNames(table);
  for (const body of table.tBodies) {
    for (const row of body.rows) {
      [...row.cells].forEach((cell, index) => {
        if (names[index] && !cell.hasAttribute("data-label")) cell.setAttribute("data-label", names[index]);
      });
    }
  }
}

export function stackTable(table: HTMLTableElement) {
  table.classList.add("stack-table");
  label(table);
  const watcher = new MutationObserver(() => label(table));
  watcher.observe(table, { childList: true, subtree: true });
  return { destroy: () => watcher.disconnect() };
}

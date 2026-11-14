// R197: the contents panel beside a source stays open, or shut, the way you left it.
const KEY = "studyapp-contents";

function read(): boolean {
  try {
    return localStorage.getItem(KEY) !== "shut";
  } catch {
    return true;
  }
}

export const contentsPanel = $state({ open: read() });

export function toggleContents(): void {
  contentsPanel.open = !contentsPanel.open;
  try {
    localStorage.setItem(KEY, contentsPanel.open ? "open" : "shut");
  } catch {
    // Remembered for this session only.
  }
}

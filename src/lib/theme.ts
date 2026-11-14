// appearance.feature (R34): dark by default, choice remembered on this device.
import { applyColours, initialColours } from "./colours";

export const DARK = "studyapp-dark";
export const LIGHT = "studyapp";
export type Theme = typeof DARK | typeof LIGHT;

const KEY = "studyapp-theme";

export function initialTheme(): Theme {
  try {
    return localStorage.getItem(KEY) === LIGHT ? LIGHT : DARK;
  } catch {
    return DARK;
  }
}

/** The chosen colour theme, in the current mode and palette (R183). */
export function refreshColours(): void {
  const root = document.documentElement;
  applyColours(initialColours(), root.dataset.theme !== LIGHT, root.dataset.palette === "cb");
}

/** Keeps the colours in step with the mode and palette, however they get set. */
export function followTheme(): () => void {
  const watcher = new MutationObserver(refreshColours);
  watcher.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-palette"] });
  return () => watcher.disconnect();
}

export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
  refreshColours();
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // Storage blocked: the theme still applies for this session.
  }
}

// R129: right, wrong and guessed in the standard green and red, or the colour-blind safe
// blue and orange. Both always come with an icon and a word.
export type Palette = "standard" | "colour-blind";
const PALETTE_KEY = "studyapp-palette";

export function initialPalette(): Palette {
  try {
    return localStorage.getItem(PALETTE_KEY) === "colour-blind" ? "colour-blind" : "standard";
  } catch {
    return "standard";
  }
}

export function applyPalette(palette: Palette): void {
  if (palette === "colour-blind") document.documentElement.dataset.palette = "cb";
  else delete document.documentElement.dataset.palette;
  refreshColours();
  try {
    localStorage.setItem(PALETTE_KEY, palette);
  } catch {
    // Storage blocked: the colours still apply for this session.
  }
}

// appearance.feature (R34): dark by default, choice remembered on this device.
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

export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // Storage blocked: the theme still applies for this session.
  }
}

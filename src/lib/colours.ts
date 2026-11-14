// appearance.feature (R183, R184): colour themes, built in OKLCH. Neutrals carry the mood,
// one accent sits on them, and the status colours (right, wrong, guessed, info, due soon) go
// in hue gaps away from the accent, at the same lightness in every theme. Light and dark each
// have their own lightness curve. Right sits further from the ground than wrong, a little in
// the standard set and a lot in the colour-blind safe set, so lightness alone tells them apart.
import { fromOklch, toOklch, wcag } from "./oklch";

export type ColoursId = "mono" | "coffee" | "honey" | "brick" | "rose" | "lavender" | "ocean" | "miku" | "sage";

interface Direction {
  id: ColoursId;
  name: string;
  blurb: string;
  neutral: [hue: number, chroma: number];
  accent: [hue: number, chroma: number];
  status: { right: number; wrong: number; info: number; warning: number };
  /** Black and white: the accent is the text colour itself. */
  mono?: boolean;
  /** Yellow turns olive when dark, so its light-mode button stays light with dark text. */
  solidLight?: [number, number];
  /** Red turns coral when lifted for dark mode, so it keeps a deep button with a white label. */
  solidDark?: [number, number];
  /** Bright for its lightness (teal): the light-mode button sits a touch deeper for its label. */
  deeper?: number;
}

/** Round the colour wheel, so similar ones sit together. */
export const COLOURS: Direction[] = [
  { id: "mono", name: "Mono", blurb: "Pure greys, black and white buttons", neutral: [0, 0], accent: [0, 0], mono: true, status: { right: 150, wrong: 25, info: 250, warning: 80 } },
  { id: "coffee", name: "Coffee", blurb: "Warm coffee greys, copper", neutral: [58, 0.03], accent: [48, 0.13], status: { right: 158, wrong: 8, info: 245, warning: 88 } },
  { id: "honey", name: "Honey", blurb: "Cream, honey gold", neutral: [88, 0.024], accent: [86, 0.14], solidLight: [0.8, 0.75], status: { right: 152, wrong: 22, info: 245, warning: 55 } },
  { id: "brick", name: "Brick", blurb: "Clay, brick red", neutral: [40, 0.022], accent: [32, 0.14], solidDark: [0.55, 0.59], status: { right: 152, wrong: 2, info: 240, warning: 85 } },
  { id: "rose", name: "Rose", blurb: "Blush, dusty rose", neutral: [12, 0.02], accent: [2, 0.11], status: { right: 158, wrong: 32, info: 250, warning: 85 } },
  { id: "lavender", name: "Lavender", blurb: "Lilac greys, lavender", neutral: [300, 0.02], accent: [295, 0.11], status: { right: 155, wrong: 20, info: 230, warning: 80 } },
  { id: "ocean", name: "Ocean", blurb: "Blue greys, ocean blue", neutral: [235, 0.02], accent: [245, 0.13], status: { right: 150, wrong: 25, info: 295, warning: 80 } },
  { id: "miku", name: "Miku", blurb: "Teal greys, Miku teal", neutral: [195, 0.02], accent: [188, 0.11], deeper: 0.03, status: { right: 140, wrong: 20, info: 265, warning: 80 } },
  { id: "sage", name: "Sage", blurb: "Green greys, sage", neutral: [135, 0.026], accent: [138, 0.075], status: { right: 172, wrong: 22, info: 265, warning: 82 } },
];

export const DEFAULT_COLOURS: ColoursId = "coffee";

// 12 steps with fixed jobs: 1 page, 2 card, 3 raised, 4 hover, 5 pressed, 6 border, 7 line,
// 8 strong line, 9 solid, 10 solid hover, 11 muted or accent text, 12 text.
const L_LIGHT = [0.99, 0.975, 0.955, 0.93, 0.905, 0.875, 0.83, 0.76, 0.56, 0.51, 0.47, 0.25];
const L_DARK = [0.18, 0.205, 0.24, 0.27, 0.3, 0.335, 0.39, 0.47, 0.72, 0.76, 0.82, 0.94];
// Accent chroma rises toward the solid steps and tapers at both ends. Neutrals hold their tint.
const C_ACCENT = [0.12, 0.2, 0.32, 0.42, 0.52, 0.62, 0.74, 0.88, 1, 1, 0.82, 0.38];
const C_NEUTRAL = [0.7, 0.8, 0.85, 0.9, 0.92, 0.95, 1, 1, 1, 1, 0.9, 0.55];
// The colour-blind safe pair: blue for right, orange for wrong.
const SAFE = { right: 250, wrong: 58 };

export type Tokens = Record<string, string>;

/** Every CSS variable the app's styles read, for one theme in one mode. */
export function buildColours(id: ColoursId, dark: boolean, safe: boolean): Tokens {
  const d = COLOURS.find((theme) => theme.id === id) ?? COLOURS.find((theme) => theme.id === DEFAULT_COLOURS)!;
  const steps = dark ? L_DARK : L_LIGHT;
  const ramp = (hue: number, peak: number, shape: number[]) => steps.map((l, i) => fromOklch(l, (dark ? peak * 0.82 : peak) * shape[i], hue));
  const n = ramp(d.neutral[0], d.neutral[1] * (dark ? 1.15 : 1), C_NEUTRAL);
  const a = ramp(d.accent[0], d.accent[1], C_ACCENT);
  if (!dark && d.solidLight) d.solidLight.forEach((l, i) => (a[8 + i] = fromOklch(l, d.accent[1], d.accent[0])));
  if (dark && d.solidDark) d.solidDark.forEach((l, i) => (a[8 + i] = fromOklch(l, d.accent[1] * 0.9, d.accent[0])));
  if (!dark && d.deeper) [8, 9].forEach((i) => { const o = toOklch(a[i]); a[i] = fromOklch(o.l - d.deeper!, o.c, o.h); });
  if (d.mono) { a[8] = n[11]; a[9] = n[10]; a[10] = n[11]; }

  const darkest = n[dark ? 0 : 11];
  const labelOn = (fill: string) => (wcag("#ffffff", fill) >= wcag(darkest, fill) ? "#ffffff" : darkest);
  // A button close to its ground gets an outline in the accent's text step.
  const outlined = (!dark && !!d.solidLight) || (dark && !!d.solidDark);
  const chart = dark && d.solidDark ? fromOklch(0.66, d.accent[1] * 0.9, d.accent[0]) : outlined ? a[10] : a[8];

  // Status colours: fixed lightness steps, warmed up to 12 degrees toward the neutrals.
  const S = dark ? { fill: 0.68, text: 0.8, tint: 0.3, c: 0.105 } : { fill: 0.54, text: 0.45, tint: 0.93, c: 0.11 };
  const away = dark ? 1 : -1;
  const warm = (h: number) => { if (d.mono) return h; const gap = ((d.neutral[0] - h + 540) % 360) - 180; return (h + Math.sign(gap) * Math.min(Math.abs(gap) * 0.3, 12) + 360) % 360; };
  const status = (hue: number, shift = 0) => {
    const h = warm(hue);
    const fill = fromOklch(S.fill + away * shift, S.c, h);
    return { fill, text: fromOklch(S.text, S.c, h), tint: fromOklch(S.tint, S.c * (dark ? 0.45 : 0.3), h), on: labelOn(fill) };
  };
  const right = safe ? status(SAFE.right, dark ? 0.1 : 0.14) : status(d.status.right, 0.06);
  const wrong = safe ? status(SAFE.wrong) : status(d.status.wrong);
  const info = status(d.status.info);
  // Error messages stay red in the colour-blind safe palette too: they come with words, not a pairing.
  const error = status(d.status.wrong);
  const warning = status(d.status.warning);

  return {
    "color-scheme": dark ? "dark" : "light",
    "--color-base-100": n[0],
    "--color-base-200": n[2],
    "--color-base-300": n[5],
    "--color-base-content": n[11],
    "--color-primary": a[8],
    "--color-primary-content": labelOn(a[8]),
    "--color-secondary": n[11],
    "--color-secondary-content": n[0],
    "--color-neutral": n[11],
    "--color-neutral-content": n[0],
    "--color-accent": warning.fill,
    "--color-accent-content": warning.on,
    "--color-info": info.fill,
    "--color-info-content": info.on,
    "--color-success": right.fill,
    "--color-success-content": right.on,
    "--color-warning": warning.fill,
    "--color-warning-content": warning.on,
    "--color-error": wrong.fill,
    "--color-error-content": wrong.on,
    "--surface": n[1],
    "--primary-ink": a[10],
    "--primary-edge": outlined ? a[10] : a[8],
    "--accent-ink": warning.text,
    "--info-ink": info.text,
    "--error-ink": error.text,
    "--muted": n[10],
    "--field-border": n[8],
    "--chart-right": right.fill,
    "--chart-wrong": wrong.fill,
    "--right-ink": right.text,
    "--wrong-ink": wrong.text,
    "--guessed-tint": right.tint,
    "--guessed-ink": right.text,
    "--on-right": right.on,
    "--on-wrong": wrong.on,
    "--chart-ink": chart,
    "--chart-grid": n[5],
    "--diagram-fill": n[2],
    "--shadow-float": `0 2px 0 ${n[5]}, 0 12px 28px oklch(${dark ? "0% 0 0 / .35" : "20% 0.02 60 / .16"})`,
  };
}

const KEY = "studyapp-colours";

export function initialColours(): ColoursId {
  try {
    const saved = localStorage.getItem(KEY);
    return COLOURS.some((theme) => theme.id === saved) ? (saved as ColoursId) : DEFAULT_COLOURS;
  } catch {
    return DEFAULT_COLOURS;
  }
}

/** Puts a theme's colours on the page and remembers the choice on this device. */
export function applyColours(id: ColoursId, dark: boolean, safe: boolean): void {
  const tokens = buildColours(id, dark, safe);
  const style = document.documentElement.style;
  for (const [name, value] of Object.entries(tokens)) style.setProperty(name, value);
  try {
    localStorage.setItem(KEY, id);
  } catch {
    // Storage blocked: the colours still apply for this session.
  }
}

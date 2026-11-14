// R193: confetti in the theme's own colours, or a rainbow that's the same in every theme.
export type ConfettiColours = "theme" | "rainbow";

const THEME_TOKENS = ["--color-primary", "--chart-right", "--color-info", "--accent-ink", "--primary-ink", "--color-base-content"];
const RAINBOW = ["#e8504f", "#f28c38", "#f5c542", "#62c370", "#3fb6c9", "#5b7fe0", "#a066d3"];

export function confettiColours(kind: ConfettiColours): string[] {
  if (kind === "rainbow") return [...RAINBOW];
  const style = getComputedStyle(document.documentElement);
  return THEME_TOKENS.map((token) => style.getPropertyValue(token).trim()).filter(Boolean);
}

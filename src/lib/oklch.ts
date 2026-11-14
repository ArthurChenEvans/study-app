// Colour maths for the themes (R183): OKLCH, WCAG 2 contrast, APCA 0.0.98G-4g, and the
// Machado 2009 colour blindness simulation. No dependencies.

const hexRgb = (hex: string): number[] => { const s = hex.replace("#", ""); return [0, 2, 4].map((i) => parseInt(s.slice(i, i + 2), 16)); };
const rgbHex = (channels: number[]): string => "#" + channels.map((v) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, "0")).join("");
const linear = (v: number) => { v /= 255; return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
const gamma = (v: number) => 255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(Math.max(v, 0), 1 / 2.4) - 0.055);

export interface Lch { l: number; c: number; h: number }

export function toOklch(hex: string): Lch {
  const [r, g, b] = hexRgb(hex).map(linear);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const lightness = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return { l: lightness, c: Math.hypot(a, bb), h: (Math.atan2(bb, a) * 180 / Math.PI + 360) % 360 };
}

function linearFrom(l: number, c: number, h: number): number[] {
  const a = c * Math.cos(h * Math.PI / 180), b = c * Math.sin(h * Math.PI / 180);
  const ll = Math.pow(l + 0.3963377774 * a + 0.2158037573 * b, 3);
  const mm = Math.pow(l - 0.1055613458 * a - 0.0638541728 * b, 3);
  const ss = Math.pow(l - 0.0894841775 * a - 1.291485548 * b, 3);
  return [4.0767416621 * ll - 3.3077115913 * mm + 0.2309699292 * ss, -1.2684380046 * ll + 2.6097574011 * mm - 0.3413193965 * ss, -0.0041960863 * ll - 0.7034186147 * mm + 1.707614701 * ss];
}

/** An OKLCH colour as hex. Keeps lightness and hue, and lowers chroma until it fits sRGB. */
export function fromOklch(l: number, c: number, h: number): string {
  let chroma = c;
  for (let i = 0; i < 60; i++) {
    if (linearFrom(l, chroma, h).every((v) => v >= -0.0005 && v <= 1.0005)) break;
    chroma *= 0.94;
  }
  return rgbHex(linearFrom(l, chroma, h).map(gamma));
}

const luminance = (hex: string) => { const [r, g, b] = hexRgb(hex).map(linear); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };

/** WCAG 2 contrast ratio. */
export function wcag(a: string, b: string): number {
  const x = luminance(a), y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

/** APCA lightness contrast (Lc) of text on a background. */
export function apca(text: string, background: string): number {
  const y = (hex: string) => { const [r, g, b] = hexRgb(hex).map((v) => Math.pow(v / 255, 2.4)); const value = 0.2126729 * r + 0.7151522 * g + 0.072175 * b; return value > 0.022 ? value : value + Math.pow(0.022 - value, 1.414); };
  const t = y(text), b = y(background);
  if (Math.abs(b - t) < 0.0005) return 0;
  if (b > t) { const c = (Math.pow(b, 0.56) - Math.pow(t, 0.57)) * 1.14; return c < 0.1 ? 0 : (c - 0.027) * 100; }
  const c = (Math.pow(b, 0.65) - Math.pow(t, 0.62)) * 1.14;
  return c > -0.1 ? 0 : (c + 0.027) * 100;
}

const MACHADO = {
  protanopia: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deuteranopia: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.01182, 0.04294, 0.968881]],
  tritanopia: [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.3039]],
};

/** How a colour looks with full protanopia, deuteranopia or tritanopia (Machado 2009). */
export function simulate(hex: string, kind: keyof typeof MACHADO): string {
  const c = hexRgb(hex).map(linear);
  return rgbHex(MACHADO[kind].map((row) => gamma(Math.max(0, Math.min(1, row[0] * c[0] + row[1] * c[1] + row[2] * c[2])))));
}

/** Distance between two colours in OKLab. Under about 0.1 is hard to tell apart. */
export function deltaE(a: string, b: string): number {
  const x = toOklch(a), y = toOklch(b), r = Math.PI / 180;
  return Math.hypot(x.l - y.l, x.c * Math.cos(x.h * r) - y.c * Math.cos(y.h * r), x.c * Math.sin(x.h * r) - y.c * Math.sin(y.h * r));
}

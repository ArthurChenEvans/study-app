// R7: Markdown with images, highlighted code and $LaTeX$. Output is sanitised, because
// question text can be pasted from anywhere and is shown inside the app's window.
import MarkdownIt from "markdown-it";
import katex from "katex";
import { escapeHtml, sanitiseRendered, trusted, trustedAttribute } from "./sanitise";
import { createHighlighterCoreSync, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import minDark from "shiki/themes/min-dark.mjs";
import minLight from "shiki/themes/min-light.mjs";

// Shiki's min themes have token colours under 4.5:1 (light comments are 1.7:1), which the
// design rules don't allow. Each failing colour is mixed toward black or white, keeping its
// hue, until it clears AA on the code block surface: worst case base-100 (#f9f4ee) in light
// and the raised panel (oklch(23% 0.014 65), about #2a2420) in dark.
const MIN_CONTRAST = 4.6;
function luminance(hex: string): number {
  const channels = [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16) / 255);
  const [red, green, blue] = channels.map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}
function contrast(first: number, second: number): number {
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}
export function readableOn(colour: string, background: string): string {
  const hex = colour.slice(0, 7).toLowerCase();
  if (!/^#[0-9a-f]{6}$/.test(hex)) return colour;
  const surface = luminance(background);
  const target = surface > 0.5 ? 0 : 255;
  for (let step = 0; step <= 20; step++) {
    const mixed = "#" + [1, 3, 5].map((start) => {
      const value = parseInt(hex.slice(start, start + 2), 16);
      return Math.round(value + (target - value) * (step / 20)).toString(16).padStart(2, "0");
    }).join("");
    if (contrast(luminance(mixed), surface) >= MIN_CONTRAST) return mixed;
  }
  return target === 0 ? "#000000" : "#ffffff";
}
type Theme = typeof minLight;
function accessible(theme: Theme, background: string): Theme {
  const fix = (colour?: string) => (colour ? readableOn(colour, background) : colour);
  return {
    ...theme,
    colors: { ...theme.colors, "editor.foreground": fix(theme.colors?.["editor.foreground"])! },
    tokenColors: theme.tokenColors?.map((rule) => ({ ...rule, settings: { ...rule.settings, foreground: fix(rule.settings?.foreground) } })),
  };
}

// Grammars are loaded on demand, each as its own chunk, so the app doesn't ship 2.5 MB
// of them up front. A code block shows as plain text until its grammar is in, then every
// Rich view re-renders. Both themes go out as CSS variables, so the Dark / Light switch
// recolours code without rendering it again (design rules section 11).
type Grammar = { default: Parameters<HighlighterCore["loadLanguageSync"]>[0] };
const grammars: Record<string, () => Promise<Grammar>> = {
  asm: () => import("shiki/langs/asm.mjs"),
  bash: () => import("shiki/langs/bash.mjs"),
  c: () => import("shiki/langs/c.mjs"),
  cpp: () => import("shiki/langs/cpp.mjs"),
  csharp: () => import("shiki/langs/csharp.mjs"),
  css: () => import("shiki/langs/css.mjs"),
  glsl: () => import("shiki/langs/glsl.mjs"),
  go: () => import("shiki/langs/go.mjs"),
  haskell: () => import("shiki/langs/haskell.mjs"),
  html: () => import("shiki/langs/html.mjs"),
  java: () => import("shiki/langs/java.mjs"),
  javascript: () => import("shiki/langs/javascript.mjs"),
  json: () => import("shiki/langs/json.mjs"),
  kotlin: () => import("shiki/langs/kotlin.mjs"),
  lua: () => import("shiki/langs/lua.mjs"),
  mipsasm: () => import("shiki/langs/mipsasm.mjs"),
  php: () => import("shiki/langs/php.mjs"),
  powershell: () => import("shiki/langs/powershell.mjs"),
  python: () => import("shiki/langs/python.mjs"),
  riscv: () => import("shiki/langs/riscv.mjs"),
  ruby: () => import("shiki/langs/ruby.mjs"),
  rust: () => import("shiki/langs/rust.mjs"),
  sql: () => import("shiki/langs/sql.mjs"),
  typescript: () => import("shiki/langs/typescript.mjs"),
  verilog: () => import("shiki/langs/verilog.mjs"),
  yaml: () => import("shiki/langs/yaml.mjs"),
};

const highlighter: HighlighterCore = createHighlighterCoreSync({
  themes: [accessible(minLight, "#f9f4ee"), accessible(minDark, "#2a2420")],
  langs: [],
  engine: createJavaScriptRegexEngine(),
});

/** Names people write after ``` that Shiki knows under another name. */
const aliases: Record<string, string> = { cuda: "cpp", cu: "cpp", "c++": "cpp", cc: "cpp", h: "c", hpp: "cpp", sh: "bash", shell: "bash", shellscript: "bash", zsh: "bash", console: "bash", py: "python", js: "javascript", ts: "typescript", rs: "rust", cs: "csharp", "c#": "csharp", rb: "ruby", kt: "kotlin", yml: "yaml", x86: "asm", nasm: "asm", mips: "mipsasm", sv: "verilog", ps1: "powershell" };

const loading = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();

function loadGrammar(name: string): void {
  if (loading.has(name)) return;
  const done = grammars[name]()
    .then((module) => {
      highlighter.loadLanguageSync(module.default);
      for (const listener of listeners) listener();
    })
    .catch(() => {
      loading.delete(name);
    });
  loading.set(name, done);
}

/** Called whenever a grammar finishes loading, so views can render again. Returns an unsubscribe. */
export function onGrammarLoaded(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Resolves once every grammar requested so far has loaded (tests and the e2e use this). */
export async function grammarsReady(): Promise<void> {
  await Promise.all(loading.values());
}

const escapeAttribute = (text: string) => text.replace(/[&<>"']/g, (character) => `&#${character.charCodeAt(0)};`);

function highlight(code: string, language: string): string {
  const name = aliases[language.toLowerCase()] ?? language.toLowerCase();
  if (!(name in grammars)) return "";
  if (!highlighter.getLoadedLanguages().includes(name)) {
    loadGrammar(name);
    return "";
  }
  const out = highlighter.codeToHtml(code.replace(/\n$/, ""), { lang: name, themes: { light: "min-light", dark: "min-dark" }, defaultColor: false });
  // Keep the language people wrote, for screen readers and the copy of the source.
  // Marked as the app's own output, so its colours (style and class) survive sanitising.
  return out.replace("<pre ", `<pre ${trustedAttribute} data-language="${escapeAttribute(language)}" `);
}

const markdown = new MarkdownIt({ html: true, linkify: false, breaks: false, highlight });

// Inline math: $...$ (not inside code, which markdown-it tokenises first).
markdown.inline.ruler.after("escape", "math_inline", (state, silent) => {
  if (state.src[state.pos] !== "$") return false;
  const end = state.src.indexOf("$", state.pos + 1);
  if (end <= state.pos + 1) return false;
  if (!silent) {
    const token = state.push("math_inline", "math", 0);
    token.content = state.src.slice(state.pos + 1, end);
  }
  state.pos = end + 1;
  return true;
});
markdown.renderer.rules.math_inline = (tokens, index) => trusted(katex.renderToString(tokens[index].content, { throwOnError: false, output: "html" }));

// Images pasted into the app are stored with the questions and written as ![alt](image:12).
// They render as a placeholder that Rich.svelte fills from the database.
export const IMAGE_REF = /^image:(\d+)$/;
const defaultImage = markdown.renderer.rules.image!;
markdown.renderer.rules.image = (tokens, index, options, env, self) => {
  const token = tokens[index];
  const stored = IMAGE_REF.exec(String(token.attrGet("src") ?? ""));
  if (!stored) return defaultImage(tokens, index, options, env, self);
  const alt = markdown.utils.escapeHtml(self.renderInlineAsText(token.children ?? [], options, env));
  return `<img data-image-id="${stored[1]}" alt="${alt}">`;
};

/** Rendering never fails the screen it's on: text the renderer chokes on shows as plain text. */
function safely(render: () => string, text: string): string {
  try {
    return render();
  } catch {
    return escapeHtml(text);
  }
}

export function renderMarkdown(text: string): string {
  return safely(() => sanitiseRendered(markdown.render(text)), text);
}

/** For short one-line content like answer options: no wrapping paragraph. */
export function renderInlineMarkdown(text: string): string {
  return safely(() => sanitiseRendered(markdown.renderInline(text)), text);
}

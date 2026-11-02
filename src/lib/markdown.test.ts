import { describe, expect, it } from "vitest";
import { renderInlineMarkdown, renderMarkdown } from "./markdown";

// R7: Markdown with images, highlighted code and $LaTeX$, in questions, options and explanations
describe("renderMarkdown", () => {
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

  it("renders fenced code blocks with their language", () => {
    expect(renderMarkdown("```cuda\nint index = 0;\n```")).toContain('<code class="language-cuda">');
  });

  it("colours code by language", () => {
    const html = renderMarkdown("```c\nint index = 0; // start\n```");
    expect(html).toContain('class="hljs-type"');
    expect(html).toContain('class="hljs-comment"');
  });

  it("leaves code in unknown languages uncoloured but escaped", () => {
    const html = renderMarkdown("```nonsense\n<b>hi</b>\n```");
    expect(html).toContain("&lt;b&gt;hi&lt;/b&gt;");
    expect(html).not.toContain("hljs-");
  });

  it("shows stored images from the app's image store", () => {
    const hash = "a".repeat(64);
    expect(renderMarkdown(`![Pipeline](image:${hash})`)).toContain(`src="studyimg://localhost/${hash}"`);
    expect(renderInlineMarkdown(`![x](image:${hash})`)).toContain("studyimg://localhost/");
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

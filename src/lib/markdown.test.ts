import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown";

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
    expect(renderMarkdown("```cuda\nint index = 0;\n```")).toContain('<code class="language-cuda">int index = 0;');
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

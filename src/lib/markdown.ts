// R7: Markdown with images, code and $LaTeX$. Output is sanitised, because question
// text can be pasted from anywhere and is shown inside the app's window.
import MarkdownIt from "markdown-it";
import katex from "katex";
import DOMPurify from "dompurify";

const markdown = new MarkdownIt({ html: true, linkify: false, breaks: false });

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
markdown.renderer.rules.math_inline = (tokens, index) => katex.renderToString(tokens[index].content, { throwOnError: false, output: "html" });

const allowedSchemes = /^(?:(?:https?|asset|data|blob):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i;

export function renderMarkdown(text: string): string {
  return DOMPurify.sanitize(markdown.render(text), { ALLOWED_URI_REGEXP: allowedSchemes });
}

/** For short one-line content like answer options: no wrapping paragraph. */
export function renderInlineMarkdown(text: string): string {
  return DOMPurify.sanitize(markdown.renderInline(text), { ALLOWED_URI_REGEXP: allowedSchemes });
}

// R7: Markdown with images, code and $LaTeX$. Output is sanitised, because question
// text can be pasted from anywhere and is shown inside the app's window.
import MarkdownIt from "markdown-it";
import katex from "katex";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import css from "highlight.js/lib/languages/css";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import typescript from "highlight.js/lib/languages/typescript";
import x86asm from "highlight.js/lib/languages/x86asm";
import xml from "highlight.js/lib/languages/xml";

for (const [name, language] of Object.entries({ bash, c, cpp, csharp, css, go, java, javascript, json, python, rust, sql, typescript, x86asm, xml })) {
  hljs.registerLanguage(name, language);
}
hljs.registerAliases(["cuda", "cu"], { languageName: "cpp" });
hljs.registerAliases(["asm", "nasm"], { languageName: "x86asm" });

const markdown = new MarkdownIt({
  html: true,
  linkify: false,
  breaks: false,
  // Fenced code is coloured when the language is known. Unknown languages fall back
  // to markdown-it's own escaping, so nothing in them is ever treated as HTML.
  highlight: (code, language) => (language && hljs.getLanguage(language) ? hljs.highlight(code, { language, ignoreIllegals: true }).value : ""),
});

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

// R8: images saved in the app are written as image:<sha-256> and served by the
// shell's studyimg protocol, so the Markdown stays short and works in history too.
const renderImage = markdown.renderer.rules.image!;
markdown.renderer.rules.image = (tokens, index, options, env, self) => {
  const token = tokens[index];
  const src = String(token.attrGet("src") ?? "");
  const stored = /^image:([0-9a-f]{64})$/.exec(src);
  if (stored) token.attrSet("src", `studyimg://localhost/${stored[1]}`);
  return renderImage(tokens, index, options, env, self);
};

const allowedSchemes = /^(?:(?:https?|asset|data|blob|studyimg):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i;

export function renderMarkdown(text: string): string {
  return DOMPurify.sanitize(markdown.render(text), { ALLOWED_URI_REGEXP: allowedSchemes });
}

/** For short one-line content like answer options: no wrapping paragraph. */
export function renderInlineMarkdown(text: string): string {
  return DOMPurify.sanitize(markdown.renderInline(text), { ALLOWED_URI_REGEXP: allowedSchemes });
}

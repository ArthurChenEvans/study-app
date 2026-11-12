// Everything shown with {@html} goes through here. Text can come from anywhere (pasted,
// imported from someone's file, synced, or a book), and it's shown inside the app's own
// window, so it may format but never act:
// - no script, by any route (DOMPurify's defaults)
// - no forms, controls or media: a form posts to wherever it likes, a button looks like
//   one of the app's
// - no style, class or id of its own: `position: fixed` covers the whole app, and the
//   app's own class names (`fixed inset-0`, `btn btn-primary`) do the same or fake its UI
// Maths and highlighted code are written by KaTeX and Shiki, which do need style and
// class. They're marked with a token made fresh each time the app starts, which no text
// written beforehand can contain, and only inside that mark are style and class kept.
import DOMPurify from "dompurify";

const TOKEN = Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, "0")).join("");
const MARK = "data-trusted";

/** Wraps HTML the app's own renderers made, so its style and class survive sanitising. */
export const trusted = (html: string) => `<span ${MARK}="${TOKEN}">${html}</span>`;
/** The same for one opening tag, e.g. `<pre ` becomes `<pre data-trusted="…" `. */
export const trustedAttribute = `${MARK}="${TOKEN}"`;

const FORBIDDEN_TAGS = ["form", "input", "button", "select", "option", "optgroup", "textarea", "label", "fieldset", "datalist", "output", "audio", "video", "source", "track", "dialog", "style", "link", "meta", "base", "marquee"];
// The only class names plain text may carry: the app's own cloze markers (written by the
// backend into card text) and the language of a code block.
const ALLOWED_CLASS = /^(cloze-gap|cloze-answer|language-[\w+#.-]+)$/;
const allowedSchemes = /^(?:(?:https?|asset|data|blob):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i;

function purifier(honourMarks: boolean) {
  const instance = DOMPurify(window);
  instance.addHook("uponSanitizeAttribute", (node, data) => {
    const name = data.attrName;
    if (name === MARK) {
      data.keepAttr = honourMarks && data.attrValue === TOKEN;
      return;
    }
    if (name !== "style" && name !== "class" && name !== "id" && name !== "name") return;
    if (honourMarks && (node as Element).closest?.(`[${MARK}="${TOKEN}"]`)) return;
    if (name === "class") {
      const kept = data.attrValue.split(/\s+/).filter((part) => ALLOWED_CLASS.test(part));
      if (kept.length > 0) {
        data.attrValue = kept.join(" ");
        return;
      }
    }
    data.keepAttr = false;
  });
  instance.addHook("afterSanitizeAttributes", (node) => {
    // Links never open in the app's window (see links.ts). Say where one goes on hover.
    if (node instanceof Element && node.tagName.toLowerCase() === "a" && node.hasAttribute("href")) {
      node.setAttribute("rel", "noopener noreferrer");
      if (!node.hasAttribute("title")) node.setAttribute("title", node.getAttribute("href") ?? "");
    }
  });
  return instance;
}

const ours = purifier(true);
const foreign = purifier(false);
const options = { ALLOWED_URI_REGEXP: allowedSchemes, FORBID_TAGS: FORBIDDEN_TAGS };
const unmark = new RegExp(` ${MARK}="${TOKEN}"`, "g");

/** HTML rendered from the app's own Markdown, with KaTeX and Shiki output marked `trusted`. */
export function sanitiseRendered(html: string): string {
  return ours.sanitize(html, options).replace(unmark, "");
}

/** HTML from someone else's file (an EPUB chapter): nothing in it is trusted. */
export function sanitiseForeign(html: string): string {
  return foreign.sanitize(html, options);
}

export const escapeHtml = (text: string) => text.replace(/[&<>"']/g, (character) => `&#${character.charCodeAt(0)};`);

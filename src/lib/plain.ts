// R214: a card's or question's text for a list or a menu, without its markup. The browser's own
// parser reads the HTML (in an inert document, so nothing loads or runs), and breaks and blocks
// become spaces.
const BLOCKS = "br, p, div, li, tr, h1, h2, h3, h4, h5, h6";

export function plainText(html: string): string {
  if (!/[<&]/.test(html)) return html.replace(/\s+/g, " ").trim();
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
  doc.body.querySelectorAll(BLOCKS).forEach((node) => node.before(" "));
  doc.body.querySelectorAll("script, style, template").forEach((node) => node.remove());
  return (doc.body.textContent ?? "").replace(/\s+/g, " ").trim();
}

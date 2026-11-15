// R201, R204: a web page as a source. Only the readable part is kept and sanitised like any
// other foreign HTML, with links and images pointing back at the site.
// - Wikipedia comes from Wikipedia's own API (clean article HTML, real maths, no edit links).
// - Other pages go through Mozilla's Readability (what Firefox's Reader View uses), which
//   finds the article even without an <article> tag.
// - When Readability can't find one, the article, main or body is used with the page's
//   navigation, headers, footers, asides, forms and script taken out.
import { Readability } from "@mozilla/readability";
import { sanitiseForeign } from "./sanitise";
import type { TocItem } from "./toc";

export interface WebDetails { title: string; authors: string; year: number | null; site: string; published: string; url: string }
export interface WebPage { title: string; html: string; headings: TocItem[]; details: WebDetails }

const CHROME = "script, style, noscript, template, nav, header, footer, aside, form, iframe, object, embed, svg, button, [role=navigation], [role=banner], [role=contentinfo], [aria-hidden=true]";
// Wikipedia's furniture: edit links, navigation boxes, maintenance notices, footnote markers and
// the picture of each formula (the formula itself is kept as MathML).
const WIKI_CHROME = ".mw-editsection, .navbox, .vertical-navbox, .metadata, .ambox, .mbox-small, .noprint, sup.reference, .mw-ref, .mwe-math-fallback-image-inline, .mwe-math-fallback-image-display, link, style";

const WIKI = /^(?:https?:\/\/)?([a-z-]+)(?:\.m)?\.wikipedia\.org\/wiki\/([^?#]+)/i;

/** Where to fetch a page from: a Wikipedia article comes from Wikipedia's API. */
export function pageAddress(url: string): string {
  const wiki = url.trim().match(WIKI);
  return wiki ? `https://${wiki[1].toLowerCase()}.wikipedia.org/api/rest_v1/page/html/${wiki[2]}` : url.trim();
}

/** The address to show and cite: a Wikipedia article's own address, tidied. */
export function articleAddress(typed: string, fetched: string): string {
  const wiki = typed.trim().match(WIKI);
  return wiki ? `https://${wiki[1].toLowerCase()}.wikipedia.org/wiki/${wiki[2]}` : fetched;
}

const meta = (doc: Document, ...names: string[]) => {
  for (const name of names) {
    const value = doc.querySelector(`meta[property="${name}"], meta[name="${name}"]`)?.getAttribute("content")?.trim();
    if (value) return value;
  }
  return "";
};

function pointAtSite(root: Element, base: string) {
  for (const node of root.querySelectorAll<HTMLElement>("[href], [src]")) {
    for (const attribute of ["href", "src"]) {
      const value = node.getAttribute(attribute);
      if (value === null) continue;
      try { node.setAttribute(attribute, new URL(value, base).href); } catch { node.removeAttribute(attribute); }
    }
  }
}

function markHeadings(root: Element): TocItem[] {
  const headings: TocItem[] = [];
  root.querySelectorAll("h1, h2, h3").forEach((node) => {
    const title = node.textContent?.replace(/\s+/g, " ").trim();
    if (!title) return;
    node.setAttribute("data-heading", String(headings.length));
    headings.push({ title, depth: Number(node.tagName[1]), at: headings.length });
  });
  return headings;
}

function readWikipedia(doc: Document, url: string): WebPage {
  const title = doc.title.trim() || decodeURIComponent(url.match(WIKI)?.[2] ?? "").replace(/_/g, " ");
  const root = doc.body;
  root.querySelectorAll(WIKI_CHROME).forEach((node) => node.remove());
  // The MathML is hidden behind its picture on Wikipedia, and only it is kept here.
  root.querySelectorAll<HTMLElement>(".mwe-math-mathml-inline, .mwe-math-mathml-display").forEach((node) => node.removeAttribute("style"));
  // Wikipedia's links are relative to the article ("./Mathematics").
  pointAtSite(root, url.replace(/\/wiki\/.*$/, "/wiki/"));
  const headings = markHeadings(root);
  return { title, html: sanitiseForeign(root.innerHTML), headings, details: { title, authors: "", year: null, site: "Wikipedia", published: "", url } };
}

export function readWebPage(html: string, url: string): WebPage {
  const doc = new DOMParser().parseFromString(html, "text/html");
  if (WIKI.test(url)) return readWikipedia(doc, url);
  // Relative links resolve against the page's own address (Readability rewrites them by it).
  if (!doc.querySelector("base[href]")) {
    const base = doc.createElement("base");
    base.href = url;
    doc.head.prepend(base);
  }
  const host = (() => { try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return ""; } })();
  const site = meta(doc, "og:site_name", "application-name") || host;

  const tagTitle = doc.title.trim();
  // "How DNS works | Example Blog" → "How DNS works"
  const fromTag = site ? tagTitle.replace(new RegExp(`\\s*[|·–—-]\\s*${site.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*$`, "i"), "") : tagTitle;
  const published = (meta(doc, "article:published_time", "citation_publication_date", "citation_date", "dc.date", "date") || doc.querySelector("time[datetime]")?.getAttribute("datetime") || "").slice(0, 10);
  const year = /^\d{4}/.test(published) ? Number(published.slice(0, 4)) : null;
  const metaAuthors = meta(doc, "citation_author", "author", "article:author", "dc.creator");

  // Readability changes the document it reads, so it gets a copy.
  const article = (() => {
    try { return new Readability(doc.cloneNode(true) as Document, { charThreshold: 400 }).parse(); } catch { return null; }
  })();
  let root: HTMLElement;
  if (article?.content) {
    root = new DOMParser().parseFromString(`<body>${article.content}</body>`, "text/html").body;
  } else {
    root = (doc.querySelector("article") ?? doc.querySelector("main, [role=main]") ?? doc.body) as HTMLElement;
  }
  root.querySelectorAll(CHROME).forEach((node) => node.remove());
  pointAtSite(root, url);
  const headings = markHeadings(root);

  const title = meta(doc, "og:title", "citation_title", "dc.title") || doc.querySelector("article h1, main h1, h1")?.textContent?.trim() || article?.title?.trim() || fromTag;
  const authors = metaAuthors || article?.byline?.trim() || "";
  return { title, html: sanitiseForeign(root.innerHTML), headings, details: { title, authors, year, site, published, url } };
}

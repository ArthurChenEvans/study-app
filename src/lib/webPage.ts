// R201: a web page as a source. Only the readable part is kept: the article (or main, or
// body) without navigation, headers, footers, asides, forms or script, and sanitised like
// any other foreign HTML. Links and images point back at the site.
import { sanitiseForeign } from "./sanitise";
import type { TocItem } from "./toc";

export interface WebDetails { title: string; authors: string; year: number | null; site: string; published: string; url: string }
export interface WebPage { title: string; html: string; headings: TocItem[]; details: WebDetails }

const CHROME = "script, style, noscript, template, nav, header, footer, aside, form, iframe, object, embed, svg, button, [role=navigation], [role=banner], [role=contentinfo], [aria-hidden=true]";

const meta = (doc: Document, ...names: string[]) => {
  for (const name of names) {
    const value = doc.querySelector(`meta[property="${name}"], meta[name="${name}"]`)?.getAttribute("content")?.trim();
    if (value) return value;
  }
  return "";
};

export function readWebPage(html: string, url: string): WebPage {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const host = (() => { try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return ""; } })();
  const site = meta(doc, "og:site_name", "application-name") || host;

  const root = (doc.querySelector("article") ?? doc.querySelector("main, [role=main]") ?? doc.body) as HTMLElement;
  root.querySelectorAll(CHROME).forEach((node) => node.remove());
  for (const node of root.querySelectorAll<HTMLElement>("[href], [src]")) {
    for (const attribute of ["href", "src"]) {
      const value = node.getAttribute(attribute);
      if (value === null) continue;
      try { node.setAttribute(attribute, new URL(value, url).href); } catch { node.removeAttribute(attribute); }
    }
  }
  const headings: TocItem[] = [];
  root.querySelectorAll("h1, h2, h3").forEach((node) => {
    const title = node.textContent?.replace(/\s+/g, " ").trim();
    if (!title) return;
    node.setAttribute("data-heading", String(headings.length));
    headings.push({ title, depth: Number(node.tagName[1]), at: headings.length });
  });

  const tagTitle = doc.title.trim();
  // "How DNS works | Example Blog" → "How DNS works"
  const fromTag = site ? tagTitle.replace(new RegExp(`\\s*[|·–—-]\\s*${site.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*$`, "i"), "") : tagTitle;
  const title = meta(doc, "og:title", "citation_title", "dc.title") || root.querySelector("h1")?.textContent?.trim() || fromTag;
  const published = (meta(doc, "article:published_time", "citation_publication_date", "citation_date", "dc.date", "date") || doc.querySelector("time[datetime]")?.getAttribute("datetime") || "").slice(0, 10);
  const year = /^\d{4}/.test(published) ? Number(published.slice(0, 4)) : null;
  const authors = meta(doc, "citation_author", "author", "article:author", "dc.creator");

  return { title, html: sanitiseForeign(root.innerHTML), headings, details: { title, authors, year, site, published, url } };
}

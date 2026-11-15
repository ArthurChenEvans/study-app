import { describe, expect, it } from "vitest";
import { pageAddress, readWebPage } from "./webPage";

const page = `<!doctype html><html><head>
<title>How DNS works | Example Blog</title>
<meta property="og:site_name" content="Example Blog">
<meta name="author" content="Ada Lovelace">
<meta property="article:published_time" content="2024-03-05T10:00:00Z">
<script>alert(1)</script><style>body{}</style>
</head><body>
<nav><a href="/">Home</a></nav>
<header>Site header</header>
<article>
  <h1>How DNS works</h1>
  <p>A <a href="/glossary#resolver" onclick="steal()">resolver</a> asks <img src="img/root.png" alt="root"> servers.</p>
  <h2>Caching</h2><p>Answers are kept for their TTL.</p>
  <h3>Negative caching</h3><p>Even "no such name" is cached.</p>
  <form><input name="q"></form>
</article>
<aside>Related posts</aside><footer>© Example</footer>
</body></html>`;

// R201: a web page opens as a reading view with its headings as contents and its details read
describe("readWebPage", () => {
  const read = readWebPage(page, "https://blog.example.com/posts/dns");

  it("keeps only the article, safely", () => {
    expect(read.html).toContain("Answers are kept for their TTL.");
    for (const gone of ["Site header", "Related posts", "© Example", "Home", "alert(1)", "onclick", "<form", "<input"]) expect(read.html).not.toContain(gone);
  });

  it("makes links and images point at the site", () => {
    expect(read.html).toContain('href="https://blog.example.com/glossary#resolver"');
    expect(read.html).toContain('src="https://blog.example.com/posts/img/root.png"');
  });

  it("lists the headings as contents", () => {
    // The page's own title heading isn't repeated in its contents.
    expect(read.headings.map((heading) => heading.title)).toEqual(expect.arrayContaining(["Caching", "Negative caching"]));
    expect(read.html).toContain('data-heading="1"');
  });

  it("reads the details a reference needs", () => {
    expect(read.details).toEqual({ title: "How DNS works", authors: "Ada Lovelace", year: 2024, site: "Example Blog", published: "2024-03-05", url: "https://blog.example.com/posts/dns" });
  });

  it("falls back to the body and the title tag when there's no article or metadata", () => {
    const bare = readWebPage("<html><head><title>Notes</title></head><body><p>Just text</p></body></html>", "https://x.org/n");
    expect(bare.html).toContain("Just text");
    expect(bare.details).toEqual({ title: "Notes", authors: "", year: null, site: "x.org", published: "", url: "https://x.org/n" });
    expect(bare.headings).toEqual([]);
  });
});

// R204: Wikipedia through its own API, and other pages through Readability
describe("pageAddress", () => {
  it("reads a Wikipedia article from Wikipedia's API", () => {
    expect(pageAddress("https://en.wikipedia.org/wiki/Boolean_algebra")).toBe("https://en.wikipedia.org/api/rest_v1/page/html/Boolean_algebra");
    expect(pageAddress("en.m.wikipedia.org/wiki/Logic_gate#History")).toBe("https://en.wikipedia.org/api/rest_v1/page/html/Logic_gate");
    expect(pageAddress("https://th.wikipedia.org/wiki/%E0%B8%95")).toBe("https://th.wikipedia.org/api/rest_v1/page/html/%E0%B8%95");
  });

  it("leaves other pages as they are", () => {
    expect(pageAddress("https://blog.example.com/dns")).toBe("https://blog.example.com/dns");
    expect(pageAddress("https://en.wikipedia.org/w/index.php?search=x")).toBe("https://en.wikipedia.org/w/index.php?search=x");
  });
});

const wiki = `<!DOCTYPE html><html><head><title>Boolean algebra</title><meta property="mw:pageId" content="1"></head><body>
<section data-mw-section-id="0"><p>In <a href="./Mathematics">mathematics</a>, Boolean algebra is a branch of algebra.</p>
<table class="infobox"><tr><td>Infobox</td></tr></table></section>
<section data-mw-section-id="1"><h2 id="Laws">Laws</h2><span class="mw-editsection">[edit]</span>
<p>Identity: <span class="mwe-math-element"><span class="mwe-math-mathml-inline" style="display: none;"><math><mi>x</mi><mo>∨</mo><mn>0</mn></math></span><img class="mwe-math-fallback-image-inline" src="//wikimedia.org/api/rest_v1/media/math/render/svg/abc" alt="x \\vee 0"></span></p>
<div role="navigation" class="navbox">Algebra navbox</div><sup class="reference"><a href="#cite_note-1">[1]</a></sup></section></body></html>`;

describe("readWebPage on Wikipedia", () => {
  const read = readWebPage(wiki, "https://en.wikipedia.org/wiki/Boolean_algebra");

  it("keeps the article and its maths, once", () => {
    expect(read.title).toBe("Boolean algebra");
    expect(read.html).toContain("<math>");
    expect(read.html).not.toContain("mwe-math-fallback");
    expect(read.html).not.toContain("render/svg");
    expect(read.html).toContain('href="https://en.wikipedia.org/wiki/Mathematics"');
  });

  it("leaves out edit links, navigation boxes and footnote markers", () => {
    for (const gone of ["[edit]", "Algebra navbox", "cite_note"]) expect(read.html).not.toContain(gone);
    expect(read.headings).toEqual([{ title: "Laws", depth: 2, at: 0 }]);
  });

  it("knows it's Wikipedia", () => {
    expect(read.details).toMatchObject({ title: "Boolean algebra", site: "Wikipedia", url: "https://en.wikipedia.org/wiki/Boolean_algebra" });
  });
});

describe("readWebPage with Readability", () => {
  const filler = "Packets are routed hop by hop across networks, each router choosing the next hop. ".repeat(12);
  const page = `<html><head><title>Routing basics - Net Notes</title></head><body>
    <div class="top-menu"><a href="/">Home</a> <a href="/about">About</a> <a href="/shop">Shop</a></div>
    <div class="layout"><div class="sidebar"><a href="/a">Popular post one</a><a href="/b">Popular post two</a><a href="/c">Popular post three</a></div>
    <div class="post"><h1>Routing basics</h1><p>${filler}</p><h2>Tables</h2><p>${filler}</p></div></div>
    <div class="cookie">We use cookies</div></body></html>`;
  const read = readWebPage(page, "https://notes.example.net/routing");

  it("finds the article without an article tag", () => {
    expect(read.html).toContain("Packets are routed hop by hop");
    for (const gone of ["Popular post one", "Shop", "We use cookies"]) expect(read.html).not.toContain(gone);
    expect(read.headings.map((heading) => heading.title)).toContain("Tables");
  });
});

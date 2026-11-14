import { describe, expect, it } from "vitest";
import { readWebPage } from "./webPage";

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
    expect(read.headings).toEqual([
      { title: "How DNS works", depth: 1, at: 0 },
      { title: "Caching", depth: 2, at: 1 },
      { title: "Negative caching", depth: 3, at: 2 },
    ]);
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

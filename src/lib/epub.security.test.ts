// Hazards 14 and 19: an EPUB is someone else's HTML in a zip. A tiny file can unpack to
// gigabytes, and its pages get the same treatment as question text.
import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { EPUB_LIMITS, readEpub } from "./epub";

async function epub(chapters: string[], extra: Record<string, Uint8Array | string> = {}): Promise<ArrayBuffer> {
  const zip = new JSZip();
  zip.file("META-INF/container.xml", `<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf"/></rootfiles></container>`);
  const items = chapters.map((_, i) => `<item id="c${i}" href="c${i}.xhtml" media-type="application/xhtml+xml"/>`).join("") + `<item id="img" href="big.png" media-type="image/png"/>`;
  const refs = chapters.map((_, i) => `<itemref idref="c${i}"/>`).join("");
  zip.file("OEBPS/content.opf", `<package xmlns="http://www.idpf.org/2007/opf"><metadata><title xmlns="http://purl.org/dc/elements/1.1/">Book</title></metadata><manifest>${items}</manifest><spine>${refs}</spine></package>`);
  chapters.forEach((body, i) => zip.file(`OEBPS/c${i}.xhtml`, body));
  for (const [name, data] of Object.entries(extra)) zip.file(name, data);
  return zip.generateAsync({ type: "arraybuffer", compression: "DEFLATE" });
}
const page = (body: string) => `<html xmlns="http://www.w3.org/1999/xhtml"><head><title>t</title></head><body>${body}</body></html>`;

describe("EPUB preview", () => {
  it("refuses a chapter that unpacks far bigger than a chapter is", async () => {
    const data = await epub([page("<p>" + "a".repeat(EPUB_LIMITS.chapterBytes + 1024 * 1024) + "</p>")]);
    expect(data.byteLength).toBeLessThan(200 * 1024);
    await expect(readEpub(data)).rejects.toThrow(/too big/);
  }, 60000);

  it("refuses a book whose pages and pictures add up to more than it will hold", async () => {
    const image = new Uint8Array(Math.floor(EPUB_LIMITS.totalBytes / 6));
    const data = await epub([page(Array.from({ length: 12 }, () => `<img src="big.png"/>`).join(""))], { "OEBPS/big.png": image });
    await expect(readEpub(data)).rejects.toThrow(/too big/);
  }, 120000);

  it("refuses a book with more chapters than any book has", async () => {
    const data = await epub(Array.from({ length: EPUB_LIMITS.chapters + 1 }, (_, i) => page(`<h1>c${i}</h1>`)));
    await expect(readEpub(data)).rejects.toThrow(/too many/);
  }, 120000);

  it("still opens an ordinary book, pictures and all", async () => {
    const png = Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    const book = await readEpub(await epub([page(`<h1>One</h1><p>Hello <em>there</em></p><img src="big.png" alt="pic"/>`), page(`<h2>Two</h2><table><tr><td>x</td></tr></table>`)], { "OEBPS/big.png": png }));
    expect(book.chapters.map((chapter) => chapter.title)).toEqual(["One", "Two"]);
    expect(book.chapters[0].html).toMatch(/<em[^>]*>there<\/em>/);
    expect(book.chapters[0].html).toContain("data:image/png;base64,iVBORw0KGgo=");
    expect(book.chapters[1].html).toContain("<td");
  });

  it("strips what a page could use to act on the app", async () => {
    const book = await readEpub(await epub([page(`<p onclick="x()" class="fixed inset-0" id="app">a</p><script>window.x=1</script><a href="javascript:x()">j</a><iframe src="https://example.com"/><form action="https://example.com"><input name="q"/><button>go</button></form><div style="position:fixed;inset:0">cover</div><style>body{display:none}</style><svg><a xlink:href="javascript:x()" xmlns:xlink="http://www.w3.org/1999/xlink">s</a></svg><audio src="x" autoplay="autoplay"></audio>`)]));
    const body = new DOMParser().parseFromString(`<body>${book.chapters[0].html}</body>`, "text/html").body;
    expect(body.querySelector("script, iframe, form, input, button, style, link, audio, video"), book.chapters[0].html).toBeNull();
    for (const element of body.querySelectorAll("*")) {
      for (const name of ["style", "class", "id", "onclick"]) expect(element.getAttribute(name), `${name} in ${book.chapters[0].html}`).toBeNull();
      for (const attribute of element.attributes) expect(/^\s*javascript:/i.test(attribute.value), attribute.value).toBe(false);
    }
  });
});

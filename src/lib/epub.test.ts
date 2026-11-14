import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { readEpub } from "./epub";

async function book(nav = false): Promise<ArrayBuffer> {
  const zip = new JSZip();
  zip.file("mimetype", "application/epub+zip");
  zip.file("META-INF/container.xml", `<?xml version="1.0"?>
    <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
      <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
    </container>`);
  zip.file("OEBPS/content.opf", `<?xml version="1.0"?>
    <package xmlns="http://www.idpf.org/2007/opf" version="3.0">
      <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Parallel Processors</dc:title></metadata>
      <manifest>
        <item id="one" href="text/one.xhtml" media-type="application/xhtml+xml"/>
        <item id="two" href="text/two.xhtml" media-type="application/xhtml+xml"/>
        <item id="pic" href="images/warp.png" media-type="image/png"/>
        ${nav ? '<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>' : ""}
      </manifest>
      <spine><itemref idref="two"/><itemref idref="one"/></spine>
    </package>`);
  zip.file("OEBPS/text/one.xhtml", `<html xmlns="http://www.w3.org/1999/xhtml"><head><title>One</title></head>
    <body><h1>Warps</h1><p onclick="steal()">A warp runs in lockstep.</p><script>steal()</script>
    <img src="../images/warp.png" alt="Warp diagram"/></body></html>`);
  zip.file("OEBPS/text/two.xhtml", `<html xmlns="http://www.w3.org/1999/xhtml"><body><h1>Introduction</h1></body></html>`);
  if (nav) zip.file("OEBPS/nav.xhtml", `<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body>
    <nav epub:type="toc"><ol><li><a href="text/two.xhtml">1. Introduction</a></li>
    <li><a href="text/one.xhtml">2. Warps</a><ol><li><a href="text/one.xhtml#lockstep">2.1 Lockstep</a></li></ol></li></ol></nav>
    <nav epub:type="landmarks"><ol><li><a href="text/one.xhtml">Start</a></li></ol></nav></body></html>`);
  zip.file("OEBPS/images/warp.png", new Uint8Array([137, 80, 78, 71]));
  return zip.generateAsync({ type: "arraybuffer" });
}

// sources.feature: Open an EPUB beside the editor
describe("readEpub", () => {
  it("reads the title and chapters in reading order", async () => {
    const epub = await readEpub(await book());
    expect(epub.title).toBe("Parallel Processors");
    expect(epub.chapters.map((chapter) => chapter.title)).toEqual(["Introduction", "Warps"]);
  });

  // sources.feature: A table of contents (R179)
  it("reads the book's own table of contents, or uses the chapter titles", async () => {
    expect((await readEpub(await book(true))).toc).toEqual([
      { title: "1. Introduction", depth: 1, at: 0, anchor: null },
      { title: "2. Warps", depth: 1, at: 1, anchor: null },
      { title: "2.1 Lockstep", depth: 2, at: 1, anchor: "lockstep" },
    ]);
    expect((await readEpub(await book())).toc).toEqual([
      { title: "Introduction", depth: 1, at: 0, anchor: null },
      { title: "Warps", depth: 1, at: 1, anchor: null },
    ]);
  });

  it("removes scripts and event handlers", async () => {
    const html = (await readEpub(await book())).chapters[1].html;
    expect(html).toContain("A warp runs in lockstep.");
    expect(html).not.toContain("<script");
    expect(html).not.toContain("onclick");
  });

  it("keeps images by embedding them", async () => {
    const html = (await readEpub(await book())).chapters[1].html;
    expect(html).toContain('src="data:image/png;base64,');
    expect(html).toContain('alt="Warp diagram"');
  });

  it("explains a file that isn't an EPUB", async () => {
    const zip = new JSZip();
    zip.file("hello.txt", "hi");
    await expect(readEpub(await zip.generateAsync({ type: "arraybuffer" }))).rejects.toThrow("This doesn't look like an EPUB book.");
  });
});

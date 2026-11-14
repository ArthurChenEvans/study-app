import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { apaPreview, detailsFromEpub, detailsFromOffice, detailsFromPdfInfo, detailsFromWeb, sameSource } from "./fileDetails";

// R202: a source filled in from the file's own details, saying which came from the file
describe("file details", () => {
  it("reads a PDF's info", () => {
    const found = detailsFromPdfInfo({ Title: "Operating Systems: Three Easy Pieces", Author: "Remzi Arpaci-Dusseau; Andrea Arpaci-Dusseau", CreationDate: "D:20180821120000Z" }, "ostep.pdf");
    expect(found.fields).toEqual({ authors: "Remzi Arpaci-Dusseau; Andrea Arpaci-Dusseau", year: "2018", title: "Operating Systems: Three Easy Pieces", edition: null, description: null, publisher: null, url: null });
    expect(found.fromFile).toEqual(["authors", "year", "title"]);
  });

  it("falls back to the file name when a PDF has no title", () => {
    const found = detailsFromPdfInfo({ Title: "  ", Author: "" }, "lecture-5_virtual-memory.pdf");
    expect(found.fields.title).toBe("lecture 5 virtual memory");
    expect(found.fromFile).toEqual([]);
  });

  it("reads an Office file's core properties and calls slides lecture slides", async () => {
    const zip = new JSZip();
    zip.file("docProps/core.xml", `<?xml version="1.0"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/"><dc:title>Lecture 5: Virtual memory</dc:title><dc:creator>J. Smith</dc:creator><dcterms:created>2026-02-10T09:00:00Z</dcterms:created></cp:coreProperties>`);
    const found = await detailsFromOffice(await zip.generateAsync({ type: "arraybuffer" }), "lecture-5.pptx");
    expect(found.fields).toMatchObject({ authors: "J. Smith", year: "2026", title: "Lecture 5: Virtual memory", description: "Lecture slides" });
    expect(found.fromFile).toEqual(["authors", "year", "title"]);
  });

  it("reads an EPUB's creator, date and publisher", () => {
    const found = detailsFromEpub({ title: "Dune", creator: "Frank Herbert", date: "1965-08-01", publisher: "Chilton Books" }, "dune.epub");
    expect(found.fields).toMatchObject({ authors: "Frank Herbert", year: "1965", title: "Dune", publisher: "Chilton Books" });
    expect(found.fromFile).toEqual(["authors", "year", "title", "publisher"]);
  });

  it("reads a web page's details, with its address", () => {
    const found = detailsFromWeb({ title: "How DNS works", authors: "", year: 2024, site: "Example Blog", published: "2024-03-05", url: "https://blog.example.com/dns" });
    expect(found.fields).toMatchObject({ authors: "", year: "2024", title: "How DNS works", publisher: "Example Blog", url: "https://blog.example.com/dns" });
    expect(found.fromFile).toEqual(["year", "title", "publisher", "url"]);
  });

  it("formats the reference the way the bibliography does", () => {
    expect(apaPreview({ authors: "Smith, J.", year: "2026", title: "Lecture 5: Virtual memory", edition: null, description: "Lecture slides", publisher: null, url: null })).toBe("Smith, J. (2026). *Lecture 5: Virtual memory* [Lecture slides].");
    expect(apaPreview({ authors: "", year: null, title: "Notes", edition: null, description: null, publisher: "Blog", url: "https://x.org" })).toBe(". (n.d.). *Notes*. Blog. https://x.org");
  });

  it("knows a source it already has, by address or title", () => {
    const fields = { authors: "", year: null, title: "How DNS works", edition: null, description: null, publisher: null, url: "https://blog.example.com/dns" };
    expect(sameSource(fields, { ...fields, title: "other", url: "https://blog.example.com/dns/" })).toBe(true);
    expect(sameSource(fields, { ...fields, url: null, title: "how dns  works" })).toBe(true);
    expect(sameSource(fields, { ...fields, url: null, title: "Caching" })).toBe(false);
  });
});

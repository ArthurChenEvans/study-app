import { describe, expect, it } from "vitest";
import { booksToFields, cslToFields, findDoi, findIsbn } from "./lookup";

// R205: a source filled in from its DOI or ISBN, which works where scraping the page doesn't
describe("findDoi", () => {
  it("finds a DOI in a link or typed on its own", () => {
    expect(findDoi("https://www.tandfonline.com/doi/full/10.1080/00220973.2024.2392684")).toBe("10.1080/00220973.2024.2392684");
    expect(findDoi("https://doi.org/10.1145/3290605.3300830")).toBe("10.1145/3290605.3300830");
    expect(findDoi("doi:10.1038/nature14539.")).toBe("10.1038/nature14539");
    expect(findDoi("https://link.springer.com/article/10.1007%2Fs11192-015-1765-5")).toBe("10.1007/s11192-015-1765-5");
  });

  it("finds nothing in an ordinary address", () => {
    expect(findDoi("https://blog.example.com/dns")).toBeNull();
    expect(findDoi("version 10.2 notes")).toBeNull();
  });
});

describe("findIsbn", () => {
  it("finds a valid ISBN-13 or ISBN-10, with or without hyphens", () => {
    expect(findIsbn("978-0-262-53980-7")).toBe("9780262539807");
    expect(findIsbn("ISBN 0-13-110362-8")).toBe("0131103628");
    expect(findIsbn("080442957X")).toBe("080442957X");
  });

  it("refuses a number whose check digit is wrong", () => {
    expect(findIsbn("978-0-262-53980-0")).toBeNull();
    expect(findIsbn("0131103627")).toBeNull();
    expect(findIsbn("10.1038/nature14539")).toBeNull();
  });
});

describe("cslToFields", () => {
  it("makes an APA-ready source from a DOI record", () => {
    const found = cslToFields({
      type: "article-journal", title: "Retrieval practice and transfer", "container-title": "The Journal of Experimental Education",
      author: [{ family: "Smith", given: "Megan A." }, { family: "Karpicke", given: "Jeffrey D." }],
      issued: { "date-parts": [[2024, 9]] }, DOI: "10.1080/00220973.2024.2392684", publisher: "Informa UK Limited",
    }, "10.1080/00220973.2024.2392684");
    expect(found.fields).toEqual({ authors: "Smith, M. A., & Karpicke, J. D.", year: "2024", title: "Retrieval practice and transfer", edition: null, description: null, publisher: "The Journal of Experimental Education", url: "https://doi.org/10.1080/00220973.2024.2392684" });
    expect(found.fromFile).toEqual(["authors", "year", "title", "publisher", "url"]);
    expect(found.from).toBe("DOI 10.1080/00220973.2024.2392684");
  });

  it("lists up to 20 authors the APA way, and copes with organisations and missing parts", () => {
    const many = Array.from({ length: 22 }, (_, index) => ({ family: `Author${index + 1}`, given: "A." }));
    expect(cslToFields({ title: "Big paper", author: many }, "10.1/x").fields.authors).toBe(`${many.slice(0, 19).map((name) => `${name.family}, A.`).join(", ")}, . . . Author22, A.`);
    expect(cslToFields({ title: "Report", author: [{ literal: "World Health Organization" }], publisher: "WHO" }, "10.1/y").fields).toMatchObject({ authors: "World Health Organization", year: null, publisher: "WHO" });
  });
});

describe("booksToFields", () => {
  it("makes a source from a Google Books record", () => {
    const found = booksToFields({ items: [{ volumeInfo: { title: "The Elements of Computing Systems", subtitle: "Building a Modern Computer from First Principles", authors: ["Noam Nisan", "Shimon Schocken"], publisher: "MIT Press", publishedDate: "2021-07-13" } }] }, "9780262539800");
    expect(found?.fields).toMatchObject({ authors: "Nisan, N., & Schocken, S.", year: "2021", title: "The Elements of Computing Systems: Building a Modern Computer from First Principles", publisher: "MIT Press", url: null });
    expect(found?.from).toBe("ISBN 9780262539800");
  });

  it("says when the ISBN isn't known", () => {
    expect(booksToFields({ totalItems: 0 }, "9780262539800")).toBeNull();
  });
});

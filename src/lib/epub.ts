// Reads an EPUB in memory for the source preview (R11). An EPUB is a zip of XHTML pages:
// follow container.xml to the package file, read the spine for reading order, and
// sanitise each chapter. Images are embedded as data URLs so nothing touches the disk.
import JSZip from "jszip";
import DOMPurify from "dompurify";

export interface Epub { title: string; chapters: { title: string; html: string }[] }

const NOT_EPUB = "This doesn't look like an EPUB book.";

function resolve(base: string, href: string): string {
  const parts = (base.includes("/") ? base.slice(0, base.lastIndexOf("/") + 1) : "").concat(decodeURIComponent(href.split("#")[0])).split("/");
  const out: string[] = [];
  for (const part of parts) {
    if (part === "..") out.pop();
    else if (part !== "." && part !== "") out.push(part);
  }
  return out.join("/");
}

function xml(text: string): Document {
  return new DOMParser().parseFromString(text, "application/xml");
}

const byLocalName = (root: Document | Element, name: string) => Array.from(root.getElementsByTagNameNS("*", name));

export async function readEpub(data: ArrayBuffer): Promise<Epub> {
  const zip = await JSZip.loadAsync(data).catch(() => {
    throw new Error(NOT_EPUB);
  });
  const container = await zip.file("META-INF/container.xml")?.async("text");
  if (!container) throw new Error(NOT_EPUB);
  const opfPath = byLocalName(xml(container), "rootfile")[0]?.getAttribute("full-path");
  const opfText = opfPath ? await zip.file(opfPath)?.async("text") : undefined;
  if (!opfPath || !opfText) throw new Error(NOT_EPUB);

  const opf = xml(opfText);
  const title = byLocalName(opf, "title")[0]?.textContent?.trim() ?? "Untitled book";
  const manifest = new Map(byLocalName(opf, "item").map((item) => [item.getAttribute("id"), { href: item.getAttribute("href") ?? "", type: item.getAttribute("media-type") ?? "" }]));
  const spine = byLocalName(opf, "itemref").map((ref) => manifest.get(ref.getAttribute("idref"))).filter((item) => item && item.type.includes("html"));

  const chapters = [];
  for (const item of spine) {
    const path = resolve(opfPath, item!.href);
    const text = await zip.file(path)?.async("text");
    if (!text) continue;
    const page = new DOMParser().parseFromString(text, "application/xhtml+xml");
    const body = page.getElementsByTagName("body")[0] ?? page.documentElement;

    for (const image of Array.from(body.querySelectorAll("img, image"))) {
      const attribute = image.hasAttribute("src") ? "src" : "href";
      const source = image.getAttribute(attribute) ?? image.getAttributeNS("http://www.w3.org/1999/xlink", "href");
      if (!source || /^[a-z]+:/i.test(source)) continue;
      const file = zip.file(resolve(path, source));
      if (!file) continue;
      const type = [...manifest.values()].find((entry) => resolve(opfPath, entry.href) === resolve(path, source))?.type || "image/png";
      image.setAttribute("src", `data:${type};base64,${await file.async("base64")}`);
    }

    const heading = body.querySelector("h1, h2, h3")?.textContent?.trim();
    const pageTitle = page.getElementsByTagName("title")[0]?.textContent?.trim();
    chapters.push({ title: heading || pageTitle || `Chapter ${chapters.length + 1}`, html: DOMPurify.sanitize(body.innerHTML, { FORBID_TAGS: ["style", "link"] }) });
  }
  if (chapters.length === 0) throw new Error(NOT_EPUB);
  return { title, chapters };
}

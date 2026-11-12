// Reads an EPUB in memory for the source preview (R11). An EPUB is a zip of XHTML pages:
// follow container.xml to the package file, read the spine for reading order, and
// sanitise each chapter. Images are embedded as data URLs so nothing touches the disk.
import JSZip from "jszip";
import { sanitiseForeign } from "./sanitise";

export interface Epub { title: string; chapters: { title: string; html: string }[] }

const NOT_EPUB = "This doesn't look like an EPUB book.";

/** A book is someone else's zip, and a zip of a few kilobytes can unpack to gigabytes.
 *  Nothing is unpacked past these, whatever the file's own headers claim. */
export const EPUB_LIMITS = { totalBytes: 200 * 1024 * 1024, chapterBytes: 20 * 1024 * 1024, chapters: 2000 };
const TOO_BIG = "This book is too big to preview: it unpacks to more than the app will open.";
const TOO_MANY = "This book has too many chapters to preview.";

interface Budget { left: number }
interface Stream {
  on(event: "data", handler: (chunk: Uint8Array) => void): Stream;
  on(event: "error", handler: (failure: Error) => void): Stream;
  on(event: "end", handler: () => void): Stream;
  pause(): Stream;
  resume(): Stream;
}

/** Unpacks one file from the zip, stopping the moment it passes `most` bytes or the book's budget runs out. */
function unpack(file: JSZip.JSZipObject, budget: Budget, most: number): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const chunks: Uint8Array[] = [];
    let size = 0;
    // internalStream is in JSZip's documented API but missing from its type definitions.
    const stream = (file as unknown as { internalStream(type: "uint8array"): Stream }).internalStream("uint8array");
    stream
      .on("data", (chunk: Uint8Array) => {
        size += chunk.length;
        budget.left -= chunk.length;
        if (size > most || budget.left < 0) {
          stream.pause();
          reject(new Error(TOO_BIG));
          return;
        }
        chunks.push(chunk);
      })
      .on("error", (failure: Error) => reject(failure))
      .on("end", () => {
        const whole = new Uint8Array(size);
        let offset = 0;
        for (const chunk of chunks) {
          whole.set(chunk, offset);
          offset += chunk.length;
        }
        resolve(whole);
      })
      .resume();
  });
}

function base64(bytes: Uint8Array): string {
  let binary = "";
  for (let start = 0; start < bytes.length; start += 0x8000) binary += String.fromCharCode(...bytes.subarray(start, start + 0x8000));
  return btoa(binary);
}

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
  const budget: Budget = { left: EPUB_LIMITS.totalBytes };
  const text = async (path: string, most = EPUB_LIMITS.chapterBytes) => {
    const file = zip.file(path);
    return file ? new TextDecoder().decode(await unpack(file, budget, most)) : undefined;
  };
  const container = await text("META-INF/container.xml");
  if (!container) throw new Error(NOT_EPUB);
  const opfPath = byLocalName(xml(container), "rootfile")[0]?.getAttribute("full-path");
  const opfText = opfPath ? await text(opfPath) : undefined;
  if (!opfPath || !opfText) throw new Error(NOT_EPUB);

  const opf = xml(opfText);
  const title = byLocalName(opf, "title")[0]?.textContent?.trim() ?? "Untitled book";
  const manifest = new Map(byLocalName(opf, "item").map((item) => [item.getAttribute("id"), { href: item.getAttribute("href") ?? "", type: item.getAttribute("media-type") ?? "" }]));
  const spine = byLocalName(opf, "itemref").map((ref) => manifest.get(ref.getAttribute("idref"))).filter((item) => item && item.type.includes("html"));
  if (spine.length > EPUB_LIMITS.chapters) throw new Error(TOO_MANY);

  // A picture used on many pages is unpacked once, but every use is counted: each one
  // becomes its own copy in the page.
  const pictures = new Map<string, string>();
  const chapters = [];
  for (const item of spine) {
    const path = resolve(opfPath, item!.href);
    const source = await text(path);
    if (!source) continue;
    const page = new DOMParser().parseFromString(source, "application/xhtml+xml");
    const body = page.getElementsByTagName("body")[0] ?? page.documentElement;

    for (const image of Array.from(body.querySelectorAll("img, image"))) {
      const attribute = image.hasAttribute("src") ? "src" : "href";
      const from = image.getAttribute(attribute) ?? image.getAttributeNS("http://www.w3.org/1999/xlink", "href");
      if (!from || /^[a-z]+:/i.test(from)) continue;
      const picturePath = resolve(path, from);
      let encoded = pictures.get(picturePath);
      if (encoded === undefined) {
        const file = zip.file(picturePath);
        if (!file) continue;
        encoded = base64(await unpack(file, budget, EPUB_LIMITS.totalBytes));
        pictures.set(picturePath, encoded);
      }
      budget.left -= encoded.length;
      if (budget.left < 0) throw new Error(TOO_BIG);
      const type = [...manifest.values()].find((entry) => resolve(opfPath, entry.href) === picturePath)?.type || "image/png";
      image.setAttribute("src", `data:${type};base64,${encoded}`);
    }

    const heading = body.querySelector("h1, h2, h3")?.textContent?.trim();
    const pageTitle = page.getElementsByTagName("title")[0]?.textContent?.trim();
    chapters.push({ title: heading || pageTitle || `Chapter ${chapters.length + 1}`, html: sanitiseForeign(body.innerHTML) });
  }
  if (chapters.length === 0) throw new Error(NOT_EPUB);
  return { title, chapters };
}

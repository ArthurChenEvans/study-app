// R7: images stored with the questions, shown through blob: URLs (allowed by the CSP).
import { api, type Id } from "./api";

const loaded = new Map<Id, Promise<string>>();

/** One blob: URL per image for the life of the window, so the quiz never re-fetches. */
export function imageUrl(id: Id): Promise<string> {
  let url = loaded.get(id);
  if (!url) {
    url = Promise.resolve().then(() => api.image(id)).then((bytes) => URL.createObjectURL(new Blob([bytes as BlobPart])));
    url.catch(() => loaded.delete(id));
    loaded.set(id, url);
  }
  return url;
}

/** Fills every image placeholder from renderMarkdown inside `root`. */
export function showImages(root: HTMLElement): void {
  for (const image of root.querySelectorAll<HTMLImageElement>("img[data-image-id]")) {
    const id = Number(image.dataset.imageId);
    imageUrl(id).then(
      (url) => { image.src = url; },
      () => { image.alt = `Image ${id} couldn't be loaded`; image.classList.add("image-missing"); },
    );
  }
}

/** The Markdown that goes into a field for a newly added image. */
export function imageMarkdown(id: Id, name = "image"): string {
  const alt = name.replace(/\.[a-z0-9]+$/i, "").replace(/[[\]\\]/g, "").trim() || "image";
  return `![${alt}](image:${id})`;
}

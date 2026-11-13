// questions.feature and flashcards.feature (R7): adding images to any field marked
// data-rich="true", by pasting, dropping a file on it, or picking one. Each image is saved to
// the database straight away and the field gets ![name](image:12) at the cursor.
import { api, errorMessage } from "./api";
import { imageMarkdown } from "./images";

export const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];

type RichField = HTMLInputElement | HTMLTextAreaElement;

const richField = (target: EventTarget | null): target is RichField =>
  (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) && !!target.dataset.rich;

const imagesIn = (files: FileList | File[] | undefined) => [...(files ?? [])].filter((file) => IMAGE_TYPES.includes(file.type));

/** "the question", "the back" or "option 2", for the status line. */
function fieldName(field: RichField): string {
  const named = field.getAttribute("aria-label");
  if (named) return named.toLowerCase();
  return `the ${(field.labels?.[0]?.textContent ?? "field").trim().toLowerCase()}`;
}

/** `fallback` is the field a picked file goes into when no rich field has been used yet. */
export function imageInput(fallback: () => RichField | null | undefined) {
  let last: RichField | null = null;
  const state = $state({ note: "", error: "" });

  /** Puts each image's Markdown at the cursor. `save` stores one and gives its id. */
  async function insert<T>(field: RichField, items: T[], save: (item: T) => Promise<number>, name: (item: T) => string) {
    state.note = "";
    state.error = "";
    for (const item of items) {
      try {
        const id = await save(item);
        const start = field.selectionStart ?? field.value.length;
        const end = field.selectionEnd ?? start;
        const spacer = start > 0 && !/\s$/.test(field.value.slice(0, start)) ? " " : "";
        field.setRangeText(spacer + imageMarkdown(id, name(item)), start, end, "end");
        field.dispatchEvent(new Event("input", { bubbles: true }));
        state.note = `Image added to ${fieldName(field)}`;
      } catch (failure) {
        state.error = errorMessage(failure);
      }
    }
  }
  const addTo = (field: RichField, files: File[]) => insert(field, files, (file) => api.addImage(file), (file) => file.name);

  /** WebKitGTK gives the page nothing for a pasted image, and only a path for a copied image
   *  file, so the app reads the system clipboard for those. */
  function fromSystem(clipboard: DataTransfer | null): boolean {
    const types = [...(clipboard?.types ?? [])];
    if (!types.length) return true;
    if (!types.includes("text/uri-list")) return false;
    // WebKitGTK often won't give the list itself, so an empty one is worth asking about too.
    const uris = (clipboard?.getData("text/uri-list") ?? "").split(/\r?\n/).filter((line) => line && !line.startsWith("#"));
    return uris.every((uri) => /^file:.*\.(png|jpe?g|gif|webp)$/i.test(uri));
  }

  return {
    get note() { return state.note; },
    get error() { return state.error; },
    clear() { state.note = ""; state.error = ""; },

    onFocus(event: FocusEvent) {
      if (richField(event.target)) last = event.target;
    },

    async onPaste(event: ClipboardEvent) {
      if (!richField(event.target)) return;
      const field = event.target;
      const images = imagesIn(event.clipboardData?.files);
      if (images.length) {
        event.preventDefault();
        return addTo(field, images);
      }
      if (!fromSystem(event.clipboardData)) return;
      event.preventDefault();
      const text = event.clipboardData?.getData("text/plain") ?? "";
      try {
        const ids = await api.pasteImage();
        if (ids.length) await insert(field, ids, async (id) => id, () => "image");
        else if (text) {
          // No image after all: paste the text as it would have gone in.
          field.setRangeText(text, field.selectionStart ?? field.value.length, field.selectionEnd ?? field.value.length, "end");
          field.dispatchEvent(new Event("input", { bubbles: true }));
        }
      } catch (failure) {
        state.error = errorMessage(failure);
      }
    },

    /** Lets a file be dropped on a rich field. */
    onDragOver(event: DragEvent) {
      if (!richField(event.target) || !event.dataTransfer?.types.includes("Files")) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
    },

    async onDrop(event: DragEvent) {
      if (!richField(event.target)) return;
      const images = imagesIn(event.dataTransfer?.files);
      if (!images.length) return;
      event.preventDefault();
      const field = event.target;
      field.focus();
      last = field;
      await addTo(field, images);
    },

    /** Files dropped on the window, by path, at a point in physical pixels (from Tauri). */
    async dropAt(paths: string[], point: { x: number; y: number }) {
      const scale = window.devicePixelRatio || 1;
      const target = document.elementFromPoint(point.x / scale, point.y / scale);
      if (!richField(target) || !paths.length) return;
      const field = target;
      try {
        const ids = await api.addImageFiles(paths);
        if (!ids.length) return;
        field.focus();
        last = field;
        await insert(field, ids, async (id) => id, () => "image");
      } catch (failure) {
        state.error = errorMessage(failure);
      }
    },

    /** Files from the "Add image" picker go where the cursor was last. */
    async add(files: File[]) {
      const field = last?.isConnected ? last : fallback();
      if (!field || !files.length) return;
      await addTo(field, imagesIn(files));
      field.focus();
    },
  };
}

/** Hands files dropped on the window to `images` while an editor is open. WebKitGTK doesn't
 *  give dropped files to the page, so this uses Tauri's own drop event. Returns a stop. */
export function listenForDrops(images: ReturnType<typeof imageInput>): () => void {
  let stop: (() => void) | null = null;
  let stopped = false;
  import("@tauri-apps/api/webview")
    .then(({ getCurrentWebview }) => getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") void images.dropAt(event.payload.paths, event.payload.position);
    }))
    .then((unlisten) => (stopped ? unlisten() : (stop = unlisten)))
    .catch(() => {}); // not the desktop app
  return () => {
    stopped = true;
    stop?.();
  };
}

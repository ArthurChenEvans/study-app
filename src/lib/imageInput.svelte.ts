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

  async function addTo(field: RichField, files: File[]) {
    state.note = "";
    state.error = "";
    for (const file of files) {
      try {
        const id = await api.addImage(file);
        const start = field.selectionStart ?? field.value.length;
        const end = field.selectionEnd ?? start;
        const spacer = start > 0 && !/\s$/.test(field.value.slice(0, start)) ? " " : "";
        field.setRangeText(spacer + imageMarkdown(id, file.name), start, end, "end");
        field.dispatchEvent(new Event("input", { bubbles: true }));
        state.note = `Image added to ${fieldName(field)}`;
      } catch (failure) {
        state.error = errorMessage(failure);
      }
    }
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
      const images = imagesIn(event.clipboardData?.files);
      if (!images.length) return;
      event.preventDefault();
      await addTo(event.target, images);
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

    /** Files from the "Add image" picker go where the cursor was last. */
    async add(files: File[]) {
      const field = last?.isConnected ? last : fallback();
      if (!field || !files.length) return;
      await addTo(field, imagesIn(files));
      field.focus();
    },
  };
}

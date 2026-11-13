import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (original) => ({ ...(await original<typeof import("./api")>()), api: { addImage: vi.fn() } }));
import { api } from "./api";
import { imageInput } from "./imageInput.svelte";

// questions.feature and flashcards.feature (R7): an image goes in by pasting, dropping it on a
// field, or picking a file. It's saved straight away and the field gets ![name](image:12).
const mocked = vi.mocked(api);
const png = (name = "cache diagram.png") => new File(["x"], name, { type: "image/png" });

function field(value = "", id = "card-back", label = "Back") {
  document.body.innerHTML = `<label for="${id}">${label}</label><textarea id="${id}" data-rich="true"></textarea>`;
  const box = document.querySelector("textarea")!;
  box.value = value;
  box.setSelectionRange(value.length, value.length);
  return box;
}
const dataTransfer = (files: File[]) => ({ files, types: ["Files"], dropEffect: "none" }) as unknown as DataTransfer;

beforeEach(() => {
  vi.resetAllMocks();
  mocked.addImage.mockResolvedValue(12);
});

describe("imageInput", () => {
  it("puts a pasted image where the cursor is and says where it went", async () => {
    const box = field("See");
    const images = imageInput(() => box);
    const event = new Event("paste", { bubbles: true, cancelable: true }) as ClipboardEvent;
    Object.defineProperty(event, "clipboardData", { value: dataTransfer([png()]) });
    Object.defineProperty(event, "target", { value: box });
    await images.onPaste(event);
    expect(event.defaultPrevented).toBe(true);
    expect(box.value).toBe("See ![cache diagram](image:12)");
    expect(images.note).toBe("Image added to the back");
  });

  it("leaves pasted text alone", async () => {
    const box = field("See");
    const images = imageInput(() => box);
    const event = new Event("paste", { cancelable: true }) as ClipboardEvent;
    Object.defineProperty(event, "clipboardData", { value: dataTransfer([]) });
    Object.defineProperty(event, "target", { value: box });
    await images.onPaste(event);
    expect(event.defaultPrevented).toBe(false);
    expect(mocked.addImage).not.toHaveBeenCalled();
  });

  it("takes an image dropped on a field, and ignores files that aren't images", async () => {
    const box = field();
    const images = imageInput(() => box);
    const over = new Event("dragover", { cancelable: true }) as DragEvent;
    Object.defineProperty(over, "dataTransfer", { value: dataTransfer([]) });
    Object.defineProperty(over, "target", { value: box });
    images.onDragOver(over);
    expect(over.defaultPrevented).toBe(true);
    const drop = new Event("drop", { cancelable: true }) as DragEvent;
    Object.defineProperty(drop, "dataTransfer", { value: dataTransfer([png(), new File(["x"], "notes.txt", { type: "text/plain" })]) });
    Object.defineProperty(drop, "target", { value: box });
    await images.onDrop(drop);
    expect(drop.defaultPrevented).toBe(true);
    expect(mocked.addImage).toHaveBeenCalledTimes(1);
    expect(box.value).toBe("![cache diagram](image:12)");
  });

  it("adds a picked file to the field last used, or the fallback", async () => {
    const box = field("", "card-front", "Front");
    const images = imageInput(() => box);
    await images.add([png("tlb.png")]);
    expect(box.value).toBe("![tlb](image:12)");
    expect(images.note).toBe("Image added to the front");
  });

  it("says why when an image can't be saved", async () => {
    const box = field();
    mocked.addImage.mockRejectedValue({ message: "that image is over 10 MB" });
    const images = imageInput(() => box);
    await images.add([png()]);
    expect(images.error).toBe("that image is over 10 MB");
    expect(box.value).toBe("");
  });
});

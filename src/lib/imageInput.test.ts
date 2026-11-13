import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (original) => ({ ...(await original<typeof import("./api")>()), api: { addImage: vi.fn(), pasteImage: vi.fn(), addImageFiles: vi.fn() } }));
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
const dataTransfer = (files: File[], types = files.length ? ["Files"] : []) => ({ files, types, getData: () => "", dropEffect: "none" }) as unknown as DataTransfer;
function pasteOn(box: HTMLTextAreaElement, transfer: DataTransfer) {
  const event = new Event("paste", { bubbles: true, cancelable: true }) as ClipboardEvent;
  Object.defineProperty(event, "clipboardData", { value: transfer });
  Object.defineProperty(event, "target", { value: box });
  return event;
}

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
    const event = pasteOn(box, dataTransfer([], ["text/plain"]));
    await images.onPaste(event);
    expect(event.defaultPrevented).toBe(false);
    expect(mocked.addImage).not.toHaveBeenCalled();
    expect(mocked.pasteImage).not.toHaveBeenCalled();
  });

  // WebKitGTK hands the page an empty clipboard for a pasted image, so the app reads it.
  it("asks the app for the clipboard's images when the page is given nothing", async () => {
    const box = field("See");
    mocked.pasteImage.mockResolvedValue([12, 13]);
    const images = imageInput(() => box);
    const event = pasteOn(box, dataTransfer([], []));
    await images.onPaste(event);
    expect(event.defaultPrevented).toBe(true);
    expect(box.value).toBe("See ![image](image:12) ![image](image:13)");
    expect(images.note).toBe("Image added to the back");
  });

  it("says nothing when the clipboard has no image either", async () => {
    const box = field("See");
    mocked.pasteImage.mockResolvedValue([]);
    const images = imageInput(() => box);
    await images.onPaste(pasteOn(box, dataTransfer([], [])));
    expect(box.value).toBe("See");
    expect(images.note).toBe("");
    expect(images.error).toBe("");
  });

  it("a copied file list it can't read still goes to the app, and plain text comes back if there's no image", async () => {
    const box = field("At ");
    mocked.pasteImage.mockResolvedValue([]);
    const images = imageInput(() => box);
    const transfer = { files: [], types: ["text/uri-list", "text/plain"], getData: (type: string) => (type === "text/plain" ? "/home/chen/notes.txt" : ""), dropEffect: "none" } as unknown as DataTransfer;
    await images.onPaste(pasteOn(box, transfer));
    expect(mocked.pasteImage).toHaveBeenCalled();
    expect(box.value).toBe("At /home/chen/notes.txt");
  });

  it("copied image files from a file manager come in as images, not their paths", async () => {
    const box = field();
    mocked.pasteImage.mockResolvedValue([12]);
    const images = imageInput(() => box);
    const transfer = { files: [], types: ["text/uri-list", "text/plain"], getData: (type: string) => (type === "text/uri-list" ? "file:///home/chen/pipeline.png" : "/home/chen/pipeline.png"), dropEffect: "none" } as unknown as DataTransfer;
    const event = pasteOn(box, transfer);
    await images.onPaste(event);
    expect(event.defaultPrevented).toBe(true);
    expect(box.value).toBe("![image](image:12)");
  });

  it("takes an image dropped on a field, and ignores files that aren't images", async () => {
    const box = field();
    const images = imageInput(() => box);
    const over = new Event("dragover", { cancelable: true }) as DragEvent;
    Object.defineProperty(over, "dataTransfer", { value: dataTransfer([], ["Files"]) });
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

  // WebKitGTK doesn't give dropped files to the page, so Tauri sends their paths and where
  // they landed (in physical pixels).
  it("adds files dropped on a field, found from where they landed", async () => {
    const box = field("See");
    mocked.addImageFiles.mockResolvedValue([12]);
    Object.defineProperty(window, "devicePixelRatio", { value: 2, configurable: true });
    document.elementFromPoint = vi.fn((x: number, y: number) => (x === 100 && y === 50 ? box : document.body));
    const images = imageInput(() => box);
    await images.dropAt(["/home/chen/pipeline.png"], { x: 200, y: 100 });
    expect(mocked.addImageFiles).toHaveBeenCalledWith(["/home/chen/pipeline.png"]);
    expect(box.value).toBe("See ![image](image:12)");
    expect(images.note).toBe("Image added to the back");
  });

  it("ignores a drop that isn't on one of its fields, or has no images", async () => {
    const box = field();
    Object.defineProperty(window, "devicePixelRatio", { value: 1, configurable: true });
    document.elementFromPoint = vi.fn(() => document.body);
    const images = imageInput(() => box);
    await images.dropAt(["/home/chen/pipeline.png"], { x: 5, y: 5 });
    expect(mocked.addImageFiles).not.toHaveBeenCalled();
    document.elementFromPoint = vi.fn(() => box);
    mocked.addImageFiles.mockResolvedValue([]);
    await images.dropAt(["/home/chen/notes.txt"], { x: 5, y: 5 });
    expect(box.value).toBe("");
    expect(images.note).toBe("");
  });
});

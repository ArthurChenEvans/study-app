import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({ api: { image: vi.fn() } }));
import { api } from "./api";
import { imageMarkdown, imageUrl, showImages } from "./images";

const mocked = vi.mocked(api);

beforeEach(() => {
  vi.resetAllMocks();
  globalThis.URL.createObjectURL = vi.fn(() => `blob:image-${Math.random()}`);
});

// questions.feature: Add an image by pasting or picking
describe("images", () => {
  it("writes Markdown named after the file", () => {
    expect(imageMarkdown(12, "pipeline diagram.png")).toBe("![pipeline diagram](image:12)");
    expect(imageMarkdown(4, "[evil]\\.png")).toBe("![evil](image:4)");
    expect(imageMarkdown(7)).toBe("![image](image:7)");
  });

  it("fetches each image once and fills every placeholder", async () => {
    mocked.image.mockResolvedValue(new Uint8Array([137, 80, 78, 71]));
    const root = document.createElement("div");
    root.innerHTML = '<img data-image-id="101" alt="a"><img data-image-id="101" alt="b">';
    showImages(root);
    await vi.waitFor(() => expect(root.querySelectorAll("img[src^='blob:']")).toHaveLength(2));
    expect(mocked.image).toHaveBeenCalledTimes(1);
    expect(await imageUrl(101)).toBe(root.querySelector("img")!.getAttribute("src"));
  });

  it("says when an image can't be loaded, and tries again next time", async () => {
    mocked.image.mockRejectedValueOnce({ message: "image not found" });
    const root = document.createElement("div");
    root.innerHTML = '<img data-image-id="202" alt="x">';
    showImages(root);
    await vi.waitFor(() => expect(root.querySelector("img")!.alt).toBe("Image 202 couldn't be loaded"));
    mocked.image.mockResolvedValue(new Uint8Array([1]));
    await expect(imageUrl(202)).resolves.toMatch(/^blob:/);
  });
});

import { describe, expect, it, vi } from "vitest";

vi.mock("./transport", () => ({ invoke: vi.fn() }));

import { invoke } from "./transport";
import { api } from "./api";

describe("slidesToPdf", () => {
  // Sent as JSON so it works on every Tauri IPC path (Linux can fall back to postMessage).
  it("sends the file name and base64 bytes, and accepts the PDF back as a buffer or a number array", async () => {
    const mocked = vi.mocked(invoke);
    mocked.mockResolvedValueOnce(new Uint8Array([37, 80]).buffer);
    const file = new File([new Uint8Array([1, 2, 255])], "Lecture 3.pptx");
    expect(await api.slidesToPdf(file)).toEqual(new Uint8Array([37, 80]));
    expect(mocked).toHaveBeenCalledWith("slides_to_pdf", { name: "Lecture 3.pptx", data: "AQL/" });

    mocked.mockResolvedValueOnce([37, 80]);
    expect(await api.slidesToPdf(file)).toEqual(new Uint8Array([37, 80]));
  });
});

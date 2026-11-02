import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", () => ({ api: { image: vi.fn(async () => new Uint8Array([137])) } }));
import Rich from "./Rich.svelte";

// questions.feature: Rich content in every part
describe("Rich", () => {
  it("highlights code once its grammar has loaded", async () => {
    const { container } = render(Rich, { text: "```sql\nSELECT 1;\n```" });
    await vi.waitFor(() => expect(container.querySelector("pre.shiki")).not.toBeNull());
    expect(container.textContent?.trim()).toBe("SELECT 1;");
  });

  it("shows a stored image in an inline option", async () => {
    globalThis.URL.createObjectURL = vi.fn(() => "blob:option-image");
    const { container } = render(Rich, { text: "![ALU](image:9)", inline: true });
    await vi.waitFor(() => expect(container.querySelector("img")?.getAttribute("src")).toBe("blob:option-image"));
    expect(container.querySelector("img")?.alt).toBe("ALU");
  });
});

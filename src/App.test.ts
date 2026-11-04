import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

vi.mock("./lib/api", async (original) => {
  const actual = await original<typeof import("./lib/api")>();
  const empty = vi.fn().mockResolvedValue([]);
  return {
    ...actual,
    api: new Proxy({}, { get: (_, name) => (name === "overview" ? vi.fn().mockResolvedValue("FirstRun") : name === "today" ? vi.fn().mockResolvedValue(1) : empty) }),
  };
});

import App from "./App.svelte";

describe("App shortcuts", () => {
  // R48
  it("/ opens search and focuses it", async () => {
    const u = userEvent.setup();
    render(App);
    await u.keyboard("/");
    const input = await screen.findByRole("searchbox", { name: "Search" });
    expect(input).toHaveFocus();
  });
});

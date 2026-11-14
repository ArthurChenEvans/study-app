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

  // R186: every screen comes in the same way, and nothing inside it adds a second entrance
  it("brings every screen in with the same entrance", async () => {
    const u = userEvent.setup();
    const { container } = render(App);
    const entrance = () => container.querySelector("main > .screen-enter");
    await screen.findByRole("heading", { level: 1 });
    expect(entrance()).not.toBeNull();
    const first = entrance();
    await u.click(screen.getAllByRole("link", { name: /^Quizzes/ })[0]);
    await screen.findByRole("heading", { level: 1, name: "Quizzes" });
    expect(entrance()).not.toBe(first);
    expect(container.querySelectorAll("main .rise").length).toBe(0);
  });
});

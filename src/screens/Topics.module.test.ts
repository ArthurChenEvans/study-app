import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { topics: vi.fn(), modules: vi.fn(), createModule: vi.fn() } };
});

import { api } from "../lib/api";
import Topics from "./Topics.svelte";

const mocked = vi.mocked(api);

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topics.mockResolvedValue([{ id: 1, name: "CPU", module: null, tags: [], question_count: 1, short_count: 0 }]);
  mocked.modules.mockResolvedValue([]);
});

// topics.feature: Put a topic in a module (needs modules to exist)
it("creates a module", async () => {
  const u = userEvent.setup();
  mocked.createModule.mockResolvedValue(5);
  render(Topics);
  await u.click(await screen.findByRole("button", { name: "New module" }));
  const dialog = screen.getByRole("dialog", { name: "New module" });
  await u.type(within(dialog).getByLabelText("Name"), "Networking");
  await u.click(within(dialog).getByRole("button", { name: "Create module" }));
  expect(mocked.createModule).toHaveBeenCalledWith("Networking");
});

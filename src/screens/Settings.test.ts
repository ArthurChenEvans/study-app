import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Settings from "./Settings.svelte";

beforeEach(() => {
  localStorage.clear();
  document.documentElement.dataset.theme = "studyapp-dark";
  delete document.documentElement.dataset.palette;
});

// navigation.feature (R158): every preference in one place
describe("Settings", () => {
  // appearance.feature: Switch to light
  it("switches theme", async () => {
    render(Settings, { onNavigate: vi.fn() });
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("aria-checked", "true");
    await userEvent.click(screen.getByRole("radio", { name: "Light" }));
    expect(document.documentElement.dataset.theme).toBe("studyapp");
    expect(screen.getByRole("radio", { name: "Light" })).toHaveAttribute("aria-checked", "true");
  });

  // appearance.feature: Colour-blind safe colours (R129)
  it("switches right and wrong to the colour-blind safe colours", async () => {
    const u = userEvent.setup();
    render(Settings, { onNavigate: vi.fn() });
    const toggle = screen.getByRole("switch", { name: "Colour-blind safe" });
    expect(toggle).not.toBeChecked();
    await u.click(toggle);
    expect(document.documentElement.dataset.palette).toBe("cb");
    await u.click(toggle);
    expect(document.documentElement.dataset.palette).toBeUndefined();
  });

  // UI polish: sounds can be turned off, and the choice is remembered on this device
  it("switches sound off and remembers it", async () => {
    render(Settings, { onNavigate: vi.fn() });
    const toggle = screen.getByRole("switch", { name: "Sound" });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(toggle).not.toBeChecked();
    expect(localStorage.getItem("studyapp-sound")).toBe("off");
  });

  // celebrations.feature (R152)
  it("opens the celebration settings", async () => {
    render(Settings, { onNavigate: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: /Celebrations/ }));
    expect(screen.getByRole("dialog", { name: "Celebrations" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(screen.queryByRole("dialog", { name: "Celebrations" })).not.toBeInTheDocument();
  });

  it("links to backups, sync and export", async () => {
    const onNavigate = vi.fn();
    render(Settings, { onNavigate });
    await userEvent.click(screen.getByRole("link", { name: /Your data/ }));
    expect(onNavigate).toHaveBeenCalledWith("data");
  });
});

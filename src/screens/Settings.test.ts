import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { ...actual.api, appVersion: vi.fn(), updateSetAuto: vi.fn(), updateCheck: vi.fn(), updateDownload: vi.fn(), updateInstall: vi.fn() } };
});

import { api } from "../lib/api";
import { updates } from "../lib/updates.svelte";
import Settings from "./Settings.svelte";

const mocked = vi.mocked(api);

beforeEach(() => {
  vi.mocked(api.updateCheck).mockReset();
  Object.assign(updates, { status: "idle", info: null, error: "", current: "0.1.4", auto: true });
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

  // updates.feature (R167, R168): updates live in Settings
  it("shows the version, checks by hand and shows the new version's notes", async () => {
    mocked.updateCheck.mockResolvedValue({ version: "0.1.5", current: "0.1.4", notes: "Week view on phones.", date: null, ready: false, asks_password: true });
    render(Settings, { onNavigate: vi.fn() });
    const section = screen.getByRole("region", { name: "Updates" });
    expect(section).toHaveTextContent("Shudhana 0.1.4");
    await userEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(await screen.findByText("Version 0.1.5 is available")).toBeInTheDocument();
    expect(section).toHaveTextContent("Week view on phones.");
    expect(section).toHaveTextContent(/asks for your password/);
    expect(screen.getByRole("button", { name: "Update to 0.1.5" })).toBeInTheDocument();
  });

  it("doesn't mention a password where installing doesn't ask for one (Windows)", async () => {
    mocked.updateCheck.mockResolvedValue({ version: "0.1.5", current: "0.1.4", notes: null, date: null, ready: false, asks_password: false });
    render(Settings, { onNavigate: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    const section = await screen.findByRole("region", { name: "Updates" });
    await screen.findByText("Version 0.1.5 is available");
    expect(section).not.toHaveTextContent(/password/);
  });

  it("says when you're up to date, and turns auto update off", async () => {
    mocked.updateCheck.mockResolvedValue(null);
    vi.mocked(api.updateSetAuto).mockResolvedValue();
    render(Settings, { onNavigate: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(await screen.findByText("You have the newest version")).toBeInTheDocument();
    const auto = screen.getByRole("switch", { name: "Auto update" });
    expect(auto).toBeChecked();
    await userEvent.click(auto);
    expect(localStorage.getItem("studyapp-auto-update")).toBe("off");
  });
});

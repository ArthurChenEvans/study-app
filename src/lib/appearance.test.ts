import { beforeEach, describe, expect, it, vi } from "vitest";

const transport = vi.hoisted(() => ({ desktop: false }));
vi.mock("./transport", async (original) => ({ ...(await original<typeof import("./transport")>()), inDesktopApp: () => transport.desktop }));
vi.mock("./api", () => ({ api: { appearance: vi.fn(), setAppearance: vi.fn() } }));

import { api } from "./api";
import { pullAppearance, saveAppearance } from "./appearance";
import { DARK, LIGHT } from "./theme";

const mocked = vi.mocked(api);

// R228: the theme follows you to every device.
describe("synced appearance", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    document.documentElement.dataset.theme = DARK;
    delete document.documentElement.dataset.palette;
    transport.desktop = false;
    mocked.setAppearance.mockResolvedValue();
  });

  it("applies what another device chose", async () => {
    mocked.appearance.mockResolvedValue({ colours: "honey", dark: false, colour_blind: true });
    await pullAppearance();
    expect(document.documentElement.dataset.theme).toBe(LIGHT);
    expect(document.documentElement.dataset.palette).toBe("cb");
    expect(localStorage.getItem("studyapp-colours")).toBe("honey");
    expect(mocked.setAppearance).not.toHaveBeenCalled();
  });

  it("ignores a theme this version doesn't know", async () => {
    mocked.appearance.mockResolvedValue({ colours: "neon", dark: false, colour_blind: false });
    await pullAppearance();
    expect(localStorage.getItem("studyapp-colours")).not.toBe("neon");
  });

  it("with nothing synced yet, the desktop app sends its look up, a browser doesn't", async () => {
    mocked.appearance.mockResolvedValue(null);
    await pullAppearance();
    expect(mocked.setAppearance).not.toHaveBeenCalled();
    transport.desktop = true;
    await pullAppearance();
    expect(mocked.setAppearance).toHaveBeenCalledWith({ colours: expect.any(String), dark: true, colour_blind: false });
  });

  it("saves the current look", async () => {
    document.documentElement.dataset.theme = LIGHT;
    document.documentElement.dataset.palette = "cb";
    localStorage.setItem("studyapp-colours", "miku");
    await saveAppearance();
    expect(mocked.setAppearance).toHaveBeenCalledWith({ colours: "miku", dark: false, colour_blind: true });
  });
});

import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (original) => {
  const actual = await original<typeof import("./api")>();
  return { ...actual, api: { appVersion: vi.fn(), updateSetAuto: vi.fn(), updateCheck: vi.fn(), updateDownload: vi.fn(), updateInstall: vi.fn() } };
});
const toast = vi.hoisted(() => vi.fn());
vi.mock("./celebrations.svelte", () => ({ toast }));

import { api, type UpdateInfo } from "./api";
import { checkNow, getUpdate, installNow, setAuto, startUpdates, updates } from "./updates.svelte";

const mocked = vi.mocked(api);
const newer: UpdateInfo = { version: "0.1.5", current: "0.1.4", notes: "Week view on phones.", date: "2026-11-14", ready: false };

beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  Object.assign(updates, { status: "idle", info: null, error: "", current: "", auto: true });
  mocked.appVersion.mockResolvedValue("0.1.4");
  mocked.updateSetAuto.mockResolvedValue();
});

// updates.feature (R166): on open, with auto update on
describe("startUpdates", () => {
  it("finds a newer release, downloads it in the background and says it'll install on close", async () => {
    mocked.updateCheck.mockResolvedValue(newer);
    mocked.updateDownload.mockResolvedValue({ ...newer, ready: true });
    await startUpdates();
    expect(updates.current).toBe("0.1.4");
    expect(mocked.updateSetAuto).toHaveBeenCalledWith(true);
    expect(mocked.updateDownload).toHaveBeenCalled();
    expect(updates.status).toBe("ready");
    expect(toast).toHaveBeenCalledWith(expect.objectContaining({ text: "Version 0.1.5 is ready", sub: expect.stringContaining("password"), action: expect.objectContaining({ label: "Install now" }) }), true);
  });

  it("does nothing more when the installed version is the newest (same or older on the feed)", async () => {
    mocked.updateCheck.mockResolvedValue(null);
    await startUpdates();
    expect(updates.status).toBe("uptodate");
    expect(mocked.updateDownload).not.toHaveBeenCalled();
    expect(toast).not.toHaveBeenCalled();
  });

  it("stays quiet when offline", async () => {
    mocked.updateCheck.mockRejectedValue({ message: "error sending request for url" });
    await startUpdates();
    expect(updates.status).toBe("idle");
    expect(toast).not.toHaveBeenCalled();
  });

  it("refuses a download whose signature doesn't match, and says so without installing", async () => {
    mocked.updateCheck.mockResolvedValue(newer);
    mocked.updateDownload.mockRejectedValue({ message: "signature verification failed" });
    await startUpdates();
    expect(updates.status).toBe("failed");
    expect(updates.error).toMatch(/didn't match its signature/);
    expect(mocked.updateInstall).not.toHaveBeenCalled();
  });

  it("only remembers the choice when auto update is off", async () => {
    localStorage.setItem("studyapp-auto-update", "off");
    await startUpdates();
    expect(updates.auto).toBe(false);
    expect(mocked.updateSetAuto).toHaveBeenCalledWith(false);
    expect(mocked.updateCheck).not.toHaveBeenCalled();
  });
});

// updates.feature (R167): checking by hand from Settings
describe("checkNow", () => {
  it("shows the new version and its notes without downloading", async () => {
    mocked.updateCheck.mockResolvedValue(newer);
    await checkNow();
    expect(updates.status).toBe("available");
    expect(updates.info?.notes).toBe("Week view on phones.");
    expect(mocked.updateDownload).not.toHaveBeenCalled();
  });

  it("says you're up to date", async () => {
    mocked.updateCheck.mockResolvedValue(null);
    await checkNow();
    expect(updates.status).toBe("uptodate");
  });

  it("says why when it can't reach GitHub", async () => {
    mocked.updateCheck.mockRejectedValue({ message: "error sending request for url" });
    await checkNow();
    expect(updates.status).toBe("failed");
    expect(updates.error).toMatch(/Couldn't reach GitHub/);
  });
});

describe("installing", () => {
  it("downloads if needed, then installs and restarts", async () => {
    updates.info = newer;
    updates.status = "available";
    mocked.updateDownload.mockResolvedValue({ ...newer, ready: true });
    mocked.updateInstall.mockResolvedValue();
    await installNow();
    expect(mocked.updateDownload.mock.invocationCallOrder[0]).toBeLessThan(mocked.updateInstall.mock.invocationCallOrder[0]);
    expect(mocked.updateInstall).toHaveBeenCalledWith(true);
  });

  it("says what went wrong if the install fails, such as a cancelled password prompt", async () => {
    updates.info = { ...newer, ready: true };
    updates.status = "ready";
    mocked.updateInstall.mockRejectedValue({ message: "PackageInstallFailed" });
    await installNow();
    expect(updates.status).toBe("failed");
    expect(updates.error).toMatch(/wasn't installed/);
  });

  it("can fetch an update found by hand", async () => {
    updates.info = newer;
    mocked.updateDownload.mockResolvedValue({ ...newer, ready: true });
    await getUpdate();
    expect(updates.status).toBe("ready");
  });
});

describe("setAuto", () => {
  it("remembers the choice on this computer and tells the app", async () => {
    await setAuto(false);
    expect(localStorage.getItem("studyapp-auto-update")).toBe("off");
    expect(mocked.updateSetAuto).toHaveBeenCalledWith(false);
    expect(updates.auto).toBe(false);
  });
});

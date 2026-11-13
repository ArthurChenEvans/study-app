// updates.feature (R165 to R168): signed updates from published releases. The Rust side does
// the checking, downloading (the plugin refuses a package whose signature doesn't match),
// backing up and installing. This keeps track of where that's got to for Settings and toasts.

import { api, errorMessage, type UpdateInfo } from "./api";
import { toast } from "./celebrations.svelte";

export type UpdateStatus = "idle" | "checking" | "uptodate" | "available" | "downloading" | "ready" | "installing" | "failed";

const AUTO_KEY = "studyapp-auto-update";

function loadAuto(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) !== "off";
  } catch {
    return true;
  }
}

export const updates = $state<{ status: UpdateStatus; info: UpdateInfo | null; error: string; current: string; auto: boolean }>({
  status: "idle",
  info: null,
  error: "",
  current: "",
  auto: loadAuto(),
});

/** Plain words for what went wrong. */
function explain(failure: unknown, stage: "check" | "download" | "install"): string {
  const message = errorMessage(failure);
  if (/signature/i.test(message)) return "The download didn't match its signature, so it wasn't installed.";
  if (stage === "check" || /request|connect|dns|network|timed? ?out/i.test(message)) return "Couldn't reach GitHub to check for updates. Try again when you're online.";
  if (stage === "install") return `The update wasn't installed (${message}). Your study data is untouched and was backed up first.`;
  return `The update couldn't be downloaded (${message}).`;
}

function readyToast(info: UpdateInfo) {
  toast({
    text: `Version ${info.version} is ready`,
    sub: info.asks_password ? "It installs when you close the app. Your computer will ask for your password." : "It installs when you close the app.",
    icon: "download",
    ms: 12_000,
    action: { label: "Install now", run: () => void installNow() },
  }, true);
}

/** On open: tell the app whether to install on close, and with auto update on, check and
 *  download in the background. Offline or nothing new stays quiet. */
export async function startUpdates(): Promise<void> {
  updates.auto = loadAuto();
  try {
    updates.current = await api.appVersion();
    await api.updateSetAuto(updates.auto);
  } catch {
    return; // not running as the desktop app
  }
  if (!updates.auto) return;
  let found: UpdateInfo | null;
  try {
    found = await api.updateCheck();
  } catch {
    return;
  }
  if (!found) {
    updates.status = "uptodate";
    return;
  }
  updates.info = found;
  updates.status = "available";
  await getUpdate();
  if ((updates.status as UpdateStatus) === "ready" && updates.info) readyToast(updates.info);
}

/** Settings' "Check for updates": says what it found, and why if it couldn't. */
export async function checkNow(): Promise<void> {
  updates.status = "checking";
  updates.error = "";
  try {
    const found = await api.updateCheck();
    updates.info = found;
    updates.status = found ? (found.ready ? "ready" : "available") : "uptodate";
  } catch (failure) {
    updates.status = "failed";
    updates.error = explain(failure, "check");
  }
}

export async function getUpdate(): Promise<void> {
  updates.status = "downloading";
  updates.error = "";
  try {
    updates.info = await api.updateDownload();
    updates.status = "ready";
  } catch (failure) {
    updates.status = "failed";
    updates.error = explain(failure, "download");
  }
}

/** Downloads if needed, then backs up, installs (on Linux, asking for a password) and restarts. */
export async function installNow(): Promise<void> {
  if (!updates.info?.ready) {
    await getUpdate();
    if (updates.status !== "ready") return;
  }
  updates.status = "installing";
  updates.error = "";
  try {
    await api.updateInstall(true);
  } catch (failure) {
    updates.status = "failed";
    updates.error = explain(failure, "install");
  }
}

export async function setAuto(on: boolean): Promise<void> {
  updates.auto = on;
  try {
    localStorage.setItem(AUTO_KEY, on ? "on" : "off");
  } catch {
    // Storage blocked: the choice lasts for this session.
  }
  try {
    await api.updateSetAuto(on);
  } catch {
    // Not running as the desktop app.
  }
}

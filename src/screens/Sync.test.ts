import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      today: vi.fn(), backupStatus: vi.fn(), backups: vi.fn(), backUpNow: vi.fn(), restorePreview: vi.fn(), restoreBackup: vi.fn(), setBackupSettings: vi.fn(),
      syncStatus: vi.fn(), syncNow: vi.fn(), syncSetup: vi.fn(), syncRename: vi.fn(), syncOff: vi.fn(), syncResume: vi.fn(), syncToCheck: vi.fn(), syncSettle: vi.fn(),
    },
  };
});

import { api, type BackupStatus, type SyncStatus, type ToCheck } from "../lib/api";
import YourData from "./YourData.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_761;
const NOW = Math.floor(Date.now() / 1000);
const backup: BackupStatus = {
  latest: { name: "b", day: TODAY, minute: 552, reason: "Daily", bytes: 6_100_000 }, count: 1, bytes: 6_100_000, folder: "/b",
  settings: { keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: null }, copy_problem: null, has_data: true,
};
const on = (overrides: Partial<SyncStatus> = {}): SyncStatus => ({
  on: true, paused: null, folder: "/home/chen/Sync/StudyApp", device: "aaaa0001", name: "Desktop",
  devices: [
    { id: "aaaa0001", name: "Desktop", this_one: true, last_seen: NOW, waiting: 0, newer_app: false },
    { id: "bbbb0002", name: "Laptop", this_one: false, last_seen: NOW - 120, waiting: 0, newer_app: false },
  ],
  last_synced: NOW - 120, problem: null, waiting: 0, held: 0, to_check: 0, ...overrides,
});
const off = (): SyncStatus => ({ ...on(), on: false, folder: null, devices: [on().devices[0]], last_synced: null });
const clash: ToCheck = {
  n: 7, record: "notes:x", what: "Card", label: "Cache latencies", context: "Caches · Computer Architecture", field: "note.back", deleted: false,
  kept: "About 4 cycles at L1, 12 at L2", other: "About 3 cycles at L1, 10 at L2", kept_by: "Laptop", kept_when: NOW - 3600, other_by: "Desktop", other_when: NOW - 7200,
};

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.backupStatus.mockResolvedValue(backup);
  mocked.backups.mockResolvedValue([backup.latest!]);
  mocked.syncToCheck.mockResolvedValue([]);
});

describe("Sync", () => {
  // sync.feature: Setting up the second computer (R119, R120)
  it("sets up sync in three numbered steps", async () => {
    const u = userEvent.setup();
    mocked.syncStatus.mockResolvedValue(off());
    render(YourData);
    const section = await screen.findByRole("region", { name: "Sync" });
    expect(section).toHaveTextContent("No account, no server, nothing to pay for");
    expect(await screen.findByRole("status")).toHaveTextContent("Sync is off");
    await u.click(within(section).getByRole("button", { name: "Set up sync" }));
    const dialog = screen.getByRole("dialog", { name: "Set up sync" });
    const steps = within(dialog).getAllByRole("listitem");
    expect(steps).toHaveLength(3);
    expect(steps[2]).toHaveTextContent("On your other computer, pick the same folder");
    expect(dialog).toHaveTextContent("never puts its database in the folder");
    await u.type(within(dialog).getByLabelText("Sync folder"), "/home/chen/Sync/StudyApp");
    const name = within(dialog).getByLabelText("Name this computer");
    expect(name).toHaveValue("Desktop");
    expect(steps[0]).toHaveTextContent("Done");
    mocked.syncSetup.mockResolvedValue({ sent: 12, received: 0, deferred: false, status: on() });
    await u.click(within(dialog).getByRole("button", { name: "Start syncing" }));
    expect(mocked.syncSetup).toHaveBeenCalledWith({ folder: "/home/chen/Sync/StudyApp", name: "Desktop" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    const table = await within(section).findByRole("table", { name: "Computers" });
    const rows = within(table).getAllByRole("row");
    expect(rows[1]).toHaveTextContent("DesktopThis one");
    expect(rows[2]).toHaveTextContent("Laptop2 minutes ago");
  });

  it("says why a folder can't be used, and keeps the dialog open", async () => {
    const u = userEvent.setup();
    mocked.syncStatus.mockResolvedValue(off());
    render(YourData);
    await u.click(await screen.findByRole("button", { name: "Set up sync" }));
    const dialog = screen.getByRole("dialog", { name: "Set up sync" });
    mocked.syncSetup.mockRejectedValue({ message: "there's no folder at /nope" });
    await u.type(within(dialog).getByLabelText("Sync folder"), "/nope");
    await u.click(within(dialog).getByRole("button", { name: "Start syncing" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent("there's no folder at /nope");
  });

  // R121
  it("syncs now, renames this computer and turns sync off", async () => {
    const u = userEvent.setup();
    mocked.syncStatus.mockResolvedValue(on());
    render(YourData);
    const section = await screen.findByRole("region", { name: "Sync" });
    expect(await screen.findByRole("status")).toHaveTextContent("Synced 2 minutes ago");
    expect(section).toHaveTextContent("/home/chen/Sync/StudyApp");
    mocked.syncNow.mockResolvedValue({ sent: 3, received: 2, deferred: false, status: on({ last_synced: NOW }) });
    await u.click(within(section).getByRole("button", { name: "Sync now" }));
    expect(mocked.syncNow).toHaveBeenCalledWith(false);
    expect(await screen.findByRole("status")).toHaveTextContent("Synced just now");

    await u.click(within(section).getByRole("button", { name: "Rename this computer" }));
    const dialog = screen.getByRole("dialog", { name: "Rename this computer" });
    const input = within(dialog).getByLabelText("Name");
    await u.clear(input);
    await u.type(input, "Big desktop");
    mocked.syncRename.mockResolvedValue(on({ name: "Big desktop" }));
    await u.click(within(dialog).getByRole("button", { name: "Rename" }));
    expect(mocked.syncRename).toHaveBeenCalledWith("Big desktop");

    mocked.syncOff.mockResolvedValue(off());
    await u.click(within(section).getByRole("button", { name: "Turn sync off" }));
    expect(mocked.syncOff).toHaveBeenCalled();
    expect(await within(section).findByRole("button", { name: "Set up sync" })).toBeInTheDocument();
  });

  // sync.feature: Editing the same part on both (R123)
  it("shows a thing to check side by side, marking what differs, and puts the other version back", async () => {
    const u = userEvent.setup();
    mocked.syncStatus.mockResolvedValue(on({ to_check: 1 }));
    mocked.syncToCheck.mockResolvedValue([clash]);
    const onChanged = vi.fn();
    render(YourData, { onRestored: onChanged });
    expect(await screen.findByRole("status")).toHaveTextContent("1 thing to check");
    const panel = await screen.findByRole("region", { name: "1 thing to check" });
    expect(panel).toHaveTextContent("Card in Cache latencies · Caches · Computer Architecture");
    const [kept, other] = within(panel).getAllByRole("group");
    expect(kept).toHaveAccessibleName("Laptop's version, in use");
    expect(kept).toHaveTextContent("In use, newer");
    expect(within(kept).getByText("4", { selector: "ins" })).toBeInTheDocument();
    expect(within(other).getByText("3", { selector: "del" })).toBeInTheDocument();
    expect(panel).toHaveTextContent("Underlined: only in the newer one. Struck through: only in the older one.");
    mocked.syncSettle.mockResolvedValue([]);
    mocked.syncStatus.mockResolvedValue(on());
    await u.click(within(panel).getByRole("button", { name: "Use Desktop's instead" }));
    expect(mocked.syncSettle).toHaveBeenCalledWith(7, true);
    expect(onChanged).toHaveBeenCalled();
    expect(screen.queryByRole("region", { name: /to check/ })).not.toBeInTheDocument();
  });

  // sync.feature: Deleting something the other computer changed (R124)
  it("explains a delete that was undone, and can delete it after all", async () => {
    const u = userEvent.setup();
    mocked.syncStatus.mockResolvedValue(on({ to_check: 1 }));
    mocked.syncToCheck.mockResolvedValue([{ ...clash, n: 9, record: "topics:t", what: "Topic", label: "Caches", context: "Computer Architecture", field: "~del", deleted: true, kept: null, other: null, kept_by: "Laptop", other_by: "Desktop" }]);
    render(YourData);
    const panel = await screen.findByRole("region", { name: "1 thing to check" });
    expect(panel).toHaveTextContent("Deleted on Desktop, but Laptop changed or added to it before it knew, so it's been kept.");
    mocked.syncSettle.mockResolvedValue([]);
    await u.click(within(panel).getByRole("button", { name: "Delete it after all" }));
    expect(mocked.syncSettle).toHaveBeenCalledWith(9, true);
  });

  // sync.feature: The folder goes missing (R127). The one primary is what fixes it.
  it("makes finding the folder the main thing when it's missing", async () => {
    mocked.syncStatus.mockResolvedValue(on({ problem: "Can't find the sync folder at /run/media/chen/USB. Changes here are kept and go out once it's back.", waiting: 3 }));
    render(YourData);
    expect(await screen.findByRole("status")).toHaveTextContent("Sync paused: can't find the sync folder");
    expect(screen.getByRole("status")).toHaveTextContent("3 changes here are kept");
    const section = screen.getByRole("region", { name: "Sync" });
    expect(within(section).getByRole("button", { name: "Change folder" })).toHaveClass("btn-primary");
    expect(screen.getByRole("button", { name: "Back up now" })).not.toHaveClass("btn-primary");
    expect(document.querySelectorAll(".btn-primary")).toHaveLength(1);
    const rows = within(within(section).getByRole("table", { name: "Computers" })).getAllByRole("row");
    expect(rows[1]).toHaveTextContent("3 waiting for the folder");
  });

  // sync.feature: Restoring a backup (R128)
  it("turns sync back on after a restore paused it", async () => {
    const u = userEvent.setup();
    mocked.syncStatus.mockResolvedValue(on({ paused: "Paused after restoring a backup" }));
    render(YourData);
    expect(await screen.findByRole("status")).toHaveTextContent("Paused after restoring a backup");
    const section = screen.getByRole("region", { name: "Sync" });
    mocked.syncResume.mockResolvedValue({ sent: 0, received: 4, deferred: false, status: on({ last_synced: NOW }) });
    await u.click(within(section).getByRole("button", { name: "Turn sync back on" }));
    expect(mocked.syncResume).toHaveBeenCalled();
  });

  it("names a computer on a newer version", async () => {
    mocked.syncStatus.mockResolvedValue(on({ problem: "Laptop has a newer version of the app, so its changes are skipped. Update this one to sync with it.", devices: [on().devices[0], { ...on().devices[1], newer_app: true }] }));
    render(YourData);
    const rows = within(await screen.findByRole("table", { name: "Computers" })).getAllByRole("row");
    expect(rows[2]).toHaveTextContent("Newer app");
    expect(screen.getByRole("status")).toHaveTextContent("Update this one to sync with it");
  });
});

import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { today: vi.fn(), backupStatus: vi.fn(), backups: vi.fn(), backUpNow: vi.fn(), restorePreview: vi.fn(), restoreBackup: vi.fn(), setBackupSettings: vi.fn() },
  };
});

import { api, type BackupInfo, type BackupStatus } from "../lib/api";
import YourData from "./YourData.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_761; // Wed 4 Nov 2026
const backup = (day: number, minute: number, reason: BackupInfo["reason"] = "Daily"): BackupInfo => ({ name: `study ${day} ${minute} ${reason}.sqlite`, day, minute, reason, bytes: 6_100_000 });
const LIST = [backup(TODAY, 552), backup(TODAY - 1, 527), backup(TODAY - 2, 1170, "BeforeUpdate"), backup(TODAY - 2, 545), backup(TODAY - 3, 600), backup(TODAY - 4, 600), backup(TODAY - 9, 600)];
const status = (overrides: Partial<BackupStatus> = {}): BackupStatus => ({
  latest: LIST[0], count: LIST.length, bytes: 42_700_000, folder: "/home/chen/.local/share/com.arthurchenevans.studyapp/backups",
  settings: { keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: null }, copy_problem: null, has_data: true, ...overrides,
});

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.backupStatus.mockResolvedValue(status());
  mocked.backups.mockResolvedValue(LIST);
});

describe("Your data", () => {
  // backup.feature: Backing up by hand (R104)
  it("leads with when the last backup was, and backs up by hand", async () => {
    const u = userEvent.setup();
    render(YourData);
    const top = await screen.findByRole("status");
    expect(top).toHaveTextContent("Backed up today at 09:12");
    expect(top).toHaveTextContent("7 kept, 42.7 MB");
    const rows = within(screen.getByRole("table", { name: "Backups" })).getAllByRole("row");
    expect(rows).toHaveLength(1 + 5 + 1); // header, five newest, "Show all 7"
    expect(rows[3]).toHaveTextContent("Mon 2 Nov, 19:30");
    expect(rows[3]).toHaveTextContent("Before update");
    expect(rows[1]).toHaveTextContent("6.1 MB");
    await u.click(screen.getByRole("button", { name: "Show all 7" }));
    expect(within(screen.getByRole("table", { name: "Backups" })).getAllByRole("row")).toHaveLength(1 + 7);

    const made = backup(TODAY, 845, "ByHand");
    mocked.backUpNow.mockResolvedValue(made);
    mocked.backups.mockResolvedValue([made, ...LIST]);
    mocked.backupStatus.mockResolvedValue(status({ latest: made, count: 8 }));
    const button = screen.getByRole("button", { name: /Back up now/ });
    expect(button).toHaveClass("btn-primary");
    await u.click(button);
    expect(mocked.backUpNow).toHaveBeenCalled();
    expect(await screen.findByText("Backed up today at 14:05")).toBeInTheDocument();
    expect(within(screen.getByRole("table", { name: "Backups" })).getAllByRole("row")[1]).toHaveTextContent("By hand");
  });

  it("explains the first time, when there's nothing yet", async () => {
    mocked.backupStatus.mockResolvedValue(status({ latest: null, count: 0, bytes: 0 }));
    mocked.backups.mockResolvedValue([]);
    render(YourData);
    expect(await screen.findByRole("status")).toHaveTextContent("No backups yet");
    expect(screen.getByText(/One is made each day you use the app/)).toBeInTheDocument();
    expect(screen.queryByRole("table", { name: "Backups" })).not.toBeInTheDocument();
  });

  // backup.feature: Restoring shows what will be undone / keeps a way back (R105)
  it("says what a restore undoes, starts on Cancel, and offers the way back after", async () => {
    const u = userEvent.setup();
    const onRestored = vi.fn();
    mocked.restorePreview.mockResolvedValue({ backup: LIST[3], undone: [{ count: 183, one: "card review", many: "card reviews" }, { count: 1, one: "quiz", many: "quizzes" }], older: false });
    render(YourData, { onRestored });
    const rows = within(await screen.findByRole("table", { name: "Backups" })).getAllByRole("row");
    await u.click(within(rows[4]).getByRole("button", { name: /Restore/ }));
    const dialog = await screen.findByRole("dialog", { name: "Restore the backup from Mon 2 Nov, 09:05?" });
    expect(dialog).toHaveTextContent("183 card reviews");
    expect(dialog).toHaveTextContent("1 quiz");
    expect(dialog).not.toHaveTextContent("1 quizzes");
    expect(dialog).toHaveTextContent(/backed up first/);
    expect(within(dialog).getByRole("button", { name: "Cancel" })).toHaveFocus();
    expect(mocked.restorePreview).toHaveBeenCalledWith(LIST[3].name);

    mocked.restoreBackup.mockResolvedValue(backup(TODAY, 850, "BeforeRestore"));
    await u.click(within(dialog).getByRole("button", { name: "Restore Mon 2 Nov" }));
    expect(mocked.restoreBackup).toHaveBeenCalledWith(LIST[3].name);
    expect(onRestored).toHaveBeenCalled();
    expect(await screen.findByRole("alert")).toHaveTextContent("Restored the backup from Mon 2 Nov, 09:05. Your data from just before is in the backup from today at 14:10, restore that to undo this.");
  });

  it("says when nothing would be undone, and when the backup is from an older version", async () => {
    const u = userEvent.setup();
    mocked.restorePreview.mockResolvedValue({ backup: LIST[2], undone: [], older: true });
    render(YourData);
    const rows = within(await screen.findByRole("table", { name: "Backups" })).getAllByRole("row");
    await u.click(within(rows[3]).getByRole("button", { name: /Restore/ }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Nothing you've done since then would be lost");
    expect(dialog).toHaveTextContent("made by an older version of the app");
    await u.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(mocked.restoreBackup).not.toHaveBeenCalled();
  });

  // R103
  it("changes how many are kept", async () => {
    const u = userEvent.setup();
    mocked.setBackupSettings.mockResolvedValue(status({ settings: { keep_days: 14, keep_weeks: 4, keep_months: 12, copy_to: null } }));
    render(YourData);
    expect(await screen.findByText("The last 7 days, 4 weeks and 12 months")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Change how many are kept" }));
    const dialog = screen.getByRole("dialog", { name: "How many backups to keep" });
    const days = within(dialog).getByLabelText("Days");
    await u.clear(days);
    await u.type(days, "14");
    await u.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(mocked.setBackupSettings).toHaveBeenCalledWith({ keep_days: 14, keep_weeks: 4, keep_months: 12, copy_to: null });
    expect(await screen.findByText("The last 14 days, 4 weeks and 12 months")).toBeInTheDocument();
  });

  // backup.feature: Copying backups to a second folder (R106)
  it("copies to a second folder, shows a problem with it, and shows errors in the dialog", async () => {
    const u = userEvent.setup();
    render(YourData);
    await u.click(await screen.findByRole("button", { name: /Add a place/ }));
    const dialog = screen.getByRole("dialog", { name: "Also copy backups to" });
    mocked.setBackupSettings.mockRejectedValueOnce({ message: "there's no folder at /media/usb" });
    await u.type(within(dialog).getByLabelText("Folder"), "/media/usb");
    await u.click(within(dialog).getByRole("button", { name: "Copy backups here" }));
    expect(within(dialog).getByRole("alert")).toHaveTextContent("there's no folder at /media/usb");
    mocked.setBackupSettings.mockResolvedValueOnce(status({ settings: { keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: "/media/usb" }, copy_problem: "Can't reach /media/usb, so backups are only being kept here" }));
    await u.click(within(dialog).getByRole("button", { name: "Copy backups here" }));
    expect(mocked.setBackupSettings).toHaveBeenLastCalledWith({ keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: "/media/usb" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Can't reach /media/usb");
    expect(screen.getByText("/media/usb")).toBeInTheDocument();
  });
});

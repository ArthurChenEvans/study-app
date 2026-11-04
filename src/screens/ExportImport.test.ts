import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      today: vi.fn(), backupStatus: vi.fn(), backups: vi.fn(), backUpNow: vi.fn(), restorePreview: vi.fn(), restoreBackup: vi.fn(), setBackupSettings: vi.fn(),
      topics: vi.fn(), modules: vi.fn(), exportSummary: vi.fn(), exportFile: vi.fn(), importPreview: vi.fn(), importApply: vi.fn(), cancelImport: vi.fn(),
    },
  };
});

import { api, type BackupStatus, type ImportPreview } from "../lib/api";
import YourData from "./YourData.svelte";

const mocked = vi.mocked(api);
const TODAY = 20_761; // Wed 4 Nov 2026
const status: BackupStatus = {
  latest: { name: "b", day: TODAY, minute: 552, reason: "Daily", bytes: 6_100_000 }, count: 1, bytes: 6_100_000, folder: "/b",
  settings: { keep_days: 7, keep_weeks: 4, keep_months: 12, copy_to: null }, copy_problem: null, has_data: true,
};
const preview = (overrides: Partial<ImportPreview> = {}): ImportPreview => ({
  file_name: "Networks.studyapp", everything: false, history: false, exported: TODAY - 2, from_here: false,
  rows: [{ what: "Topics", new: 1, same: 2, changed: 0 }, { what: "Questions", new: 18, same: 22, changed: 2 }, { what: "Cards", new: 12, same: 19, changed: 0 }],
  clashes: [
    { uid: "q1", what: "Question", label: "Which layer adds port numbers?", mine_day: TODAY - 1, theirs_day: TODAY - 2, theirs_newer: false },
    { uid: "q2", what: "Question", label: "Which layer routes packets?", mine_day: TODAY - 5, theirs_day: TODAY - 2, theirs_newer: true },
  ],
  suggested: "KeepMine", ...overrides,
});

beforeEach(() => {
  vi.resetAllMocks();
  mocked.today.mockResolvedValue(TODAY);
  mocked.backupStatus.mockResolvedValue(status);
  mocked.backups.mockResolvedValue([status.latest!]);
  mocked.modules.mockResolvedValue([{ id: 1, name: "Networks" }, { id: 2, name: "Security" }]);
  mocked.topics.mockResolvedValue([
    { id: 10, name: "OSI model", module: 1, tags: [], question_count: 6, short_count: 1 },
    { id: 11, name: "TCP/IP", module: 1, tags: [], question_count: 4, short_count: 1 },
    { id: 12, name: "Cryptography", module: 2, tags: [], question_count: 5, short_count: 1 },
  ]);
  mocked.exportSummary.mockResolvedValue({ name: "Study App 2026-11-04.studyapp", counts: [{ what: "Topics", count: 3 }, { what: "Questions", count: 15 }, { what: "Quizzes", count: 1 }] });
});

describe("Export", () => {
  // transfer.feature: Moving everything to another computer (R112)
  it("exports everything with history and says where it went", async () => {
    const u = userEvent.setup();
    render(YourData);
    await u.click(await screen.findByRole("button", { name: /^Export/ }));
    const dialog = screen.getByRole("dialog", { name: "Export" });
    expect(within(dialog).getByRole("radio", { name: /Everything/ })).toBeChecked();
    expect(within(dialog).getByRole("checkbox", { name: /Include my history/ })).toBeChecked();
    expect(await within(dialog).findByText(/Saves Study App 2026-11-04.studyapp: 3 topics, 15 questions and 1 quiz/)).toBeInTheDocument();
    mocked.exportFile.mockResolvedValue({ name: "Study App 2026-11-04.studyapp", path: "/home/chen/Downloads/Study App 2026-11-04.studyapp", data: null });
    await u.click(within(dialog).getByRole("button", { name: "Export everything" }));
    expect(mocked.exportFile).toHaveBeenCalledWith({ topics: null, history: true });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(await screen.findByRole("alert")).toHaveTextContent("Saved Study App 2026-11-04.studyapp to /home/chen/Downloads/Study App 2026-11-04.studyapp");
  });

  // transfer.feature: Sharing some topics (R113)
  it("shares picked topics without history, a whole module at a time, and downloads it in the browser", async () => {
    const u = userEvent.setup();
    const created: Blob[] = [];
    URL.createObjectURL = vi.fn((blob: Blob) => { created.push(blob); return "blob:x"; });
    URL.revokeObjectURL = vi.fn();
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    render(YourData);
    await u.click(await screen.findByRole("button", { name: /^Export/ }));
    const dialog = screen.getByRole("dialog", { name: "Export" });
    await u.click(within(dialog).getByRole("radio", { name: /Some topics, to share/ }));
    expect(within(dialog).getByRole("checkbox", { name: /Include my history/ })).not.toBeChecked();
    const button = within(dialog).getByRole("button", { name: /^Export/ });
    expect(button).toBeDisabled();
    await u.click(within(dialog).getByRole("checkbox", { name: "All of Networks" }));
    expect(within(dialog).getByRole("checkbox", { name: "OSI model" })).toBeChecked();
    expect(within(dialog).getByRole("checkbox", { name: "TCP/IP" })).toBeChecked();
    await u.type(within(dialog).getByRole("searchbox", { name: "Filter topics" }), "crypt");
    expect(within(dialog).queryByRole("checkbox", { name: "OSI model" })).not.toBeInTheDocument();
    mocked.exportFile.mockResolvedValue({ name: "Networks.studyapp", path: null, data: btoa("zip bytes") });
    await u.click(within(dialog).getByRole("button", { name: "Export 2 topics" }));
    expect(mocked.exportFile).toHaveBeenCalledWith({ topics: [10, 11], history: false });
    expect(click).toHaveBeenCalled();
    expect(await created[0].text()).toBe("zip bytes");
    expect(await screen.findByRole("alert")).toHaveTextContent("Downloaded Networks.studyapp");
    click.mockRestore();
  });
});

describe("Import", () => {
  // transfer.feature: An updated version of a shared topic (R115, R116)
  it("previews what comes in and settles clashes one by one", async () => {
    const u = userEvent.setup();
    mocked.importPreview.mockResolvedValue(preview());
    mocked.importApply.mockResolvedValue({ added: [{ what: "topics", count: 1 }, { what: "questions", count: 18 }, { what: "cards", count: 12 }], replaced: 1, backup: { name: "study 2026-11-04 14-20 before-import.sqlite", day: TODAY, minute: 860, reason: "BeforeImport", bytes: 6_000_000 } });
    const onChanged = vi.fn();
    render(YourData, { onRestored: onChanged });
    const file = new File(["zip"], "Networks.studyapp", { type: "application/zip" });
    await u.upload(await screen.findByLabelText("Import a .studyapp file"), file);
    expect(mocked.importPreview).toHaveBeenCalledWith(file, "Networks.studyapp");
    const dialog = await screen.findByRole("dialog", { name: "Import Networks.studyapp" });
    expect(dialog).toHaveTextContent("Some topics, no history · exported Mon 2 Nov");
    const rows = within(within(dialog).getByRole("table")).getAllByRole("row");
    expect(rows[2]).toHaveTextContent("Questions18222");
    expect(within(dialog).getByRole("radio", { name: /Keep mine/ })).toBeChecked();
    await u.click(within(dialog).getByRole("radio", { name: /Look at each one/ }));
    const clashes = within(dialog).getByRole("list", { name: "Changed in both places" });
    const second = within(clashes).getAllByRole("listitem")[1];
    expect(second).toHaveTextContent("Which layer routes packets?");
    expect(second).toHaveTextContent("Changed here Fri 30 Oct, in the file Mon 2 Nov");
    expect(within(second).getByRole("radio", { name: "The file's, newer" })).toBeChecked();
    expect(within(dialog).getByText("A backup is made first.")).toBeInTheDocument();
    await u.click(within(dialog).getByRole("button", { name: "Import" }));
    expect(mocked.importApply).toHaveBeenCalledWith({ rule: "EachOne", theirs: ["q2"] });
    expect(onChanged).toHaveBeenCalled();
    expect(await screen.findByRole("alert")).toHaveTextContent("Imported 1 topic, 18 questions and 12 cards, and used the file's version of 1 thing. To undo it, restore the backup from today at 14:20.");
  });

  it("says so when nothing in the file is new, and lets it go", async () => {
    const u = userEvent.setup();
    mocked.importPreview.mockResolvedValue(preview({ rows: [{ what: "Topics", new: 0, same: 3, changed: 0 }], clashes: [], from_here: true, everything: true, history: true, suggested: "Newer" }));
    render(YourData);
    await u.upload(await screen.findByLabelText("Import a .studyapp file"), new File(["zip"], "Study App.studyapp"));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Everything, with history · exported Mon 2 Nov from this computer");
    expect(dialog).toHaveTextContent("Everything in this file is here already.");
    await u.click(within(dialog).getByRole("button", { name: "Close" }));
    expect(mocked.cancelImport).toHaveBeenCalled();
    expect(mocked.importApply).not.toHaveBeenCalled();
  });

  // transfer.feature: Files that can't be trusted (R118)
  it("shows why a file can't be imported", async () => {
    const u = userEvent.setup();
    mocked.importPreview.mockRejectedValue({ message: "that file couldn't be imported: it isn't a .studyapp file" });
    render(YourData);
    await u.upload(await screen.findByLabelText("Import a .studyapp file"), new File(["nope"], "notes.zip", { type: "application/zip" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("that file couldn't be imported: it isn't a .studyapp file");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

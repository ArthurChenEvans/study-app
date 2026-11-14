import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      topics: vi.fn(), modules: vi.fn(), today: vi.fn(), importPreview: vi.fn(), importApply: vi.fn(), cancelImport: vi.fn(),
      ankiPreview: vi.fn(), ankiApply: vi.fn(), itemsPreview: vi.fn(), itemsApply: vi.fn(), stopImport: vi.fn(), undoLastImport: vi.fn(), readFileAt: vi.fn(),
    },
  };
});
const toast = vi.hoisted(() => vi.fn());
vi.mock("../lib/celebrations.svelte", async (original) => ({ ...(await original<typeof import("../lib/celebrations.svelte")>()), toast }));

import { api, type AnkiPreview, type ItemsPreview } from "../lib/api";
import { importer, startImport } from "../lib/importer.svelte";
import ImportHost from "./ImportHost.svelte";

const mocked = vi.mocked(api);
const anki: AnkiPreview = {
  file_name: "Networks.apkg", notes: 230, images: 31, sounds: 2, has_progress: true, already_here: 8,
  decks: [
    { name: "Networks::DNS", notes: 60, target: { kind: "NewTopic", name: "DNS", module: "Networks" } },
    { name: "Networks::TLS", notes: 52, target: { kind: "NewTopic", name: "TLS", module: "Networks" } },
    { name: "Misc", notes: 48, target: { kind: "NewTopic", name: "Misc", module: null } },
  ],
  types: [{ name: "Basic", notes: 104, becomes: "Basic", check: false }, { name: "Image Occlusion", notes: 12, becomes: "Basic", check: true }],
};
const csv: ItemsPreview = {
  file_name: "cpu-questions.csv", format: "csv", ready: 38, already_here: 0,
  csv: { columns: ["question", "a", "b", "c", "answer", "why", "notes"], header: true, kind: "Mcq", roles: ["Question", "Option", "Option", "Option", "Correct", "Explanation", "Ignore"], sample: [["Which design?", "Von Neumann", "Harvard", "RISC", "B", "Separate buses", "lecture 3"]], total_rows: 40 },
  topics: [{ key: "cpu-questions", name: "cpu-questions", module: null, questions: 38, cards: 0, checklists: 0, target: { kind: "NewTopic", name: "cpu-questions", module: null } }],
  problems: [{ at: "Row 14", text: "Which cache level is fastest?", why: "The answer says E, but there are only 3 options. Put A, B or C in the answer column." }],
};
const done = { added: [{ what: "topics", count: 2 }, { what: "notes", count: 174 }, { what: "cards", count: 190 }], replaced: 0, backup: { name: "b", day: 20_771, minute: 1390, reason: "BeforeImport" as const, bytes: 1 } };

beforeEach(() => {
  vi.resetAllMocks();
  importer.step = null;
  mocked.topics.mockResolvedValue([{ id: 7, name: "TLS handshakes", module: 1, tags: [], question_count: 3, short_count: 0 }, { id: 8, name: "CPU", module: null, tags: [], question_count: 0, short_count: 0 }]);
  mocked.modules.mockResolvedValue([{ id: 1, name: "Networks" }]);
  mocked.cancelImport.mockResolvedValue();
});

// import-export.feature: Anki decks (R171, R172)
describe("an Anki deck", () => {
  it("shows where each deck goes and its card types, and imports as chosen with Undo after", async () => {
    const u = userEvent.setup();
    mocked.ankiPreview.mockResolvedValue(anki);
    mocked.ankiApply.mockResolvedValue(done);
    render(ImportHost);
    await startImport(new File(["zip"], "Networks.apkg"));
    const dialog = await screen.findByRole("dialog", { name: "Import Networks.apkg" });
    expect(dialog).toHaveTextContent("An Anki deck: 230 notes in 3 decks, with 31 images.");
    expect(dialog).toHaveTextContent("Image Occlusion");
    expect(within(dialog).getByText("Check these")).toBeInTheDocument();
    await u.selectOptions(within(dialog).getByRole("combobox", { name: "Where Networks::TLS goes" }), "topic:7");
    await u.selectOptions(within(dialog).getByRole("combobox", { name: "Where Misc goes" }), "skip");
    expect(dialog).toHaveTextContent("8 notes are already here from an earlier import, and stay as they are.");
    await u.click(within(dialog).getByRole("radio", { name: "Keep Anki's progress" }));
    await u.click(within(dialog).getByRole("button", { name: "Import 104 notes" }));
    expect(mocked.ankiApply).toHaveBeenCalledWith({
      decks: [["Networks::DNS", { kind: "NewTopic", name: "DNS", module: "Networks" }], ["Networks::TLS", { kind: "Topic", topic: 7 }], ["Misc", { kind: "Skip" }]],
      keep_progress: true,
    });
    expect(toast).toHaveBeenCalledWith(expect.objectContaining({ text: "Imported 2 topics and 190 cards", action: expect.objectContaining({ label: "Undo" }) }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

// import-export.feature: Spreadsheets (R174)
describe("a spreadsheet", () => {
  it("matches columns, lets them be changed, and explains rows with problems", async () => {
    const u = userEvent.setup();
    mocked.itemsPreview.mockResolvedValue(csv);
    mocked.itemsApply.mockResolvedValue({ ...done, added: [{ what: "questions", count: 38 }] });
    render(ImportHost);
    await startImport(new File(["q"], "cpu-questions.csv"));
    const dialog = await screen.findByRole("dialog", { name: "Import cpu-questions.csv" });
    expect(dialog).toHaveTextContent("A spreadsheet with 40 rows and 7 columns.");
    expect(within(dialog).getByRole("combobox", { name: "Column 5 is" })).toHaveValue("Correct");
    expect(within(dialog).getByRole("radio", { name: "A multiple choice question" })).toBeChecked();
    await u.selectOptions(within(dialog).getByRole("combobox", { name: "Column 7 is" }), "Tags");
    expect(mocked.itemsPreview).toHaveBeenLastCalledWith(expect.any(File), "cpu-questions.csv", { kind: "Mcq", header: true, roles: ["Question", "Option", "Option", "Option", "Correct", "Explanation", "Tags"] });
    await u.click(within(dialog).getByRole("button", { name: "See which" }));
    expect(within(dialog).getByRole("heading", { name: "1 row has problems" })).toBeInTheDocument();
    expect(dialog).toHaveTextContent("Row 14");
    expect(dialog).toHaveTextContent("Put A, B or C in the answer column.");
    await u.click(within(dialog).getByRole("button", { name: "Back to the columns" }));
    await u.selectOptions(within(dialog).getByRole("combobox", { name: "Into" }), "topic:8");
    await u.click(within(dialog).getByRole("button", { name: "Import 38 questions" }));
    expect(mocked.itemsApply).toHaveBeenCalledWith({ targets: [["cpu-questions", { kind: "Topic", topic: 8 }]] });
  });
});

// import-export.feature: A file it can't read (R178)
describe("a file it can't read", () => {
  it("says so and offers another", async () => {
    render(ImportHost);
    await startImport(new File(["zip"], "lecture-notes.zip"));
    const dialog = await screen.findByRole("alertdialog", { name: "Couldn't import lecture-notes.zip" });
    expect(dialog).toHaveTextContent("It isn't a kind of file Shudhana can import, so nothing was added.");
    expect(within(dialog).getByRole("button", { name: "Choose another file" })).toBeInTheDocument();
    expect(mocked.ankiPreview).not.toHaveBeenCalled();
  });
});

// import-export.feature: Importing a big deck (R178)
describe("a big import", () => {
  it("shows progress and can be stopped", async () => {
    const u = userEvent.setup();
    render(ImportHost);
    importer.busy = true;
    importer.progress = { what: "Notes", done: 4120, total: 12800 };
    const dialog = await screen.findByRole("dialog", { name: /Importing/ });
    expect(within(dialog).getByRole("progressbar", { name: "Notes imported" })).toHaveAttribute("aria-valuenow", "4120");
    expect(dialog).toHaveTextContent("4,120 of 12,800");
    await u.click(within(dialog).getByRole("button", { name: "Stop importing" }));
    expect(mocked.stopImport).toHaveBeenCalled();
    importer.busy = false;
  });
});

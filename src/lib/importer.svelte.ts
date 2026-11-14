// import-export.feature (R171 to R178): one importer for every way a file arrives (the Import
// button, a file dropped on the window, a .studyapp opened with the app), every kind of file
// (.studyapp, Anki, spreadsheets, JSON, Markdown, Moodle), the progress of a big one, and
// the toast with Undo after.

import { api, errorMessage, type AnkiPreview, type CsvMapping, type ImportPreview, type ImportProgress, type ImportResult, type ItemsPreview } from "./api";
import { toast } from "./celebrations.svelte";

export type ImportStep =
  | { kind: "studyapp"; preview: ImportPreview }
  | { kind: "anki"; preview: AnkiPreview }
  | { kind: "items"; preview: ItemsPreview; file: File }
  | { kind: "unreadable"; name: string; message: string };

export const ACCEPT = ".studyapp,.apkg,.colpkg,.csv,.tsv,.json,.md,.markdown,.txt,.gift";

export const importer = $state<{ step: ImportStep | null; reading: string | null; busy: boolean; progress: ImportProgress | null }>({ step: null, reading: null, busy: false, progress: null });

const listeners = new Set<() => void>();
const changed = () => listeners.forEach((run) => run());
/** Screens refresh after an import or an undo. Returns how to stop listening. */
export function onImportChanged(run: () => void): () => void {
  listeners.add(run);
  return () => listeners.delete(run);
}

function kindOf(name: string): "studyapp" | "anki" | "items" | null {
  const extension = name.split(".").pop()?.toLowerCase() ?? "";
  if (extension === "studyapp") return "studyapp";
  if (extension === "apkg" || extension === "colpkg") return "anki";
  if (["csv", "tsv", "json", "md", "markdown", "txt", "gift"].includes(extension)) return "items";
  return null;
}

const UNREADABLE = "It isn't a kind of file Shudhana can import, so nothing was added.";

/** Reads a file and shows what importing it would do. Nothing is added yet. */
export async function startImport(file: File): Promise<void> {
  importer.step = null;
  const kind = kindOf(file.name);
  if (!kind) {
    importer.step = { kind: "unreadable", name: file.name, message: UNREADABLE };
    return;
  }
  importer.reading = file.name;
  try {
    if (kind === "studyapp") importer.step = { kind, preview: await api.importPreview(file, file.name) };
    else if (kind === "anki") importer.step = { kind, preview: await api.ankiPreview(file, file.name) };
    else importer.step = { kind, preview: await api.itemsPreview(file, file.name), file };
  } catch (failure) {
    const message = errorMessage(failure);
    importer.step = { kind: "unreadable", name: file.name, message: message.charAt(0).toUpperCase() + message.slice(1) + (message.endsWith(".") ? "" : ".") };
  } finally {
    importer.reading = null;
  }
}

/** A dropped file, or the one the app was opened with, by its path. */
export async function startImportAt(path: string): Promise<void> {
  try {
    const picked = await api.readFileAt(path);
    const bytes = Uint8Array.from(atob(picked.data), (letter) => letter.charCodeAt(0));
    await startImport(new File([bytes], picked.name));
  } catch (failure) {
    importer.step = { kind: "unreadable", name: path.split(/[\\/]/).pop() ?? path, message: errorMessage(failure) };
  }
}

/** A spreadsheet again, with the columns as chosen. */
export async function remap(mapping: CsvMapping): Promise<void> {
  const step = importer.step;
  if (step?.kind !== "items") return;
  try {
    importer.step = { kind: "items", preview: await api.itemsPreview(step.file, step.file.name, mapping), file: step.file };
  } catch (failure) {
    toast({ text: "Couldn't read the columns that way", sub: errorMessage(failure), icon: "alert", error: true });
  }
}

export function cancelImport(): void {
  importer.step = null;
  void Promise.resolve(api.cancelImport()).catch(() => {});
}

function summary(result: ImportResult): string {
  const shown = result.added.filter((count) => count.count > 0 && count.what !== "notes" && count.what !== "images");
  const words = shown.map((count) => {
    const what = count.what === "checklists" ? "recall checklists" : count.what;
    return `${count.count} ${count.count === 1 ? what.replace(/s$/, "") : what}`;
  });
  return words.length === 0 ? "Nothing new" : words.length === 1 ? words[0] : `${words.slice(0, -1).join(", ")} and ${words.at(-1)}`;
}

/** Runs the import, with progress for a big one, then a toast with Undo (R172, R177, R178). */
export async function runImport(name: string, work: () => Promise<ImportResult>): Promise<boolean> {
  importer.busy = true;
  importer.progress = null;
  try {
    const result = await work();
    importer.step = null;
    changed();
    const added = summary(result);
    toast({
      text: added === "Nothing new" ? "Nothing to import" : `Imported ${added}`,
      sub: added === "Nothing new" ? `Everything in ${name} is here already` : `From ${name}`,
      icon: added === "Nothing new" ? "info" : "check",
      ms: 8000,
      action: result.backup && added !== "Nothing new" ? { label: "Undo", run: () => void undoImport() } : undefined,
    });
    return true;
  } catch (failure) {
    const message = errorMessage(failure);
    if (/stopped/.test(message)) {
      importer.step = null;
      toast({ text: "Import stopped", sub: "Nothing was added", icon: "info" });
    } else {
      toast({ text: "Couldn't import", sub: message, icon: "alert", error: true });
    }
    return false;
  } finally {
    importer.busy = false;
    importer.progress = null;
  }
}

export async function undoImport(): Promise<void> {
  try {
    const restored = await api.undoLastImport();
    changed();
    const minute = restored.minute;
    toast({ text: "Import undone", sub: `Everything is back to how it was at ${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`, icon: "undo" });
  } catch (failure) {
    toast({ text: "Couldn't undo the import", sub: errorMessage(failure), icon: "alert", error: true });
  }
}

export function stopImport(): void {
  void Promise.resolve(api.stopImport()).catch(() => {});
}

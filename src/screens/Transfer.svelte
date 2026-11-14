<script lang="ts">
  // R189: Import & Export, its own page in Manage. transfer.feature and import-export.feature
  // (R112 to R118, R171 to R178): export everything or some topics in several formats, import
  // from Anki, spreadsheets, Moodle and other apps, and undo the last import.
  import { FileInput, FileOutput, Undo2, Upload } from "@lucide/svelte";
  import { api, type ExportFile, type LastImport } from "../lib/api";
  import { madeDay, madeLabel } from "../lib/backup";
  import { toast } from "../lib/celebrations.svelte";
  import { ACCEPT, onImportChanged, startImport, undoImport } from "../lib/importer.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ExportDialog from "../components/ExportDialog.svelte";

  let { onBackups = () => {} }: { onBackups?: () => void } = $props();

  let today = $state(0);
  let exporting = $state(false);
  let lastImport = $state<LastImport | null>(null);
  let undoing = $state(false);
  let over = $state(false);

  async function load() {
    today = await api.today();
    try {
      lastImport = (await api.lastImport()) ?? null;
    } catch {
      lastImport = null;
    }
  }

  function exported(file: ExportFile) {
    exporting = false;
    if (file.data) {
      // The browser version hands the file to the browser to save.
      const bytes = Uint8Array.from(atob(file.data), (letter) => letter.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
      const link = document.createElement("a");
      link.href = url;
      link.download = file.name;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 10_000);
      toast({ text: `Downloaded ${file.name}`, icon: "export" });
    } else {
      const path = file.path;
      toast({ text: `Exported ${file.name}`, sub: path ? `Saved to ${path.slice(0, path.length - file.name.length - 1)}` : undefined, icon: "export", ms: 8000, action: path ? { label: "Show in folder", run: () => void Promise.resolve(api.showInFolder(path)).catch(() => {}) } : undefined });
    }
  }

  async function pickFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) await startImport(file);
  }

  async function dropped(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    over = false;
    const file = event.dataTransfer?.files?.[0];
    if (file) await startImport(file);
  }

  async function undoLast() {
    undoing = false;
    await undoImport();
  }

  $effect(() => onImportChanged(() => void load()));
  load();
</script>

<div class="mx-auto max-w-5xl px-12 pb-24">
  <PageHeader title="Import & Export" subtitle="Bring in decks and question banks, or take your study somewhere else. A backup is made before every import." />

  <div class="grid grid-cols-1 gap-5 tab:grid-cols-2">
    <section class="panel flex flex-col gap-4 p-6" aria-labelledby="transfer-import-h">
      <h2 id="transfer-import-h" class="text-lg font-black">Import</h2>
      <label class="drop-zone" class:drop-over={over} ondragover={(event) => { event.preventDefault(); over = true; }} ondragleave={() => (over = false)} ondrop={dropped}>
        <Upload class="size-8 opacity-70" aria-hidden="true" />
        <span class="font-bold">Drop a file here</span>
        <span class="btn btn-primary"><FileInput class="size-4" />Choose a file</span>
        <input type="file" accept={ACCEPT} class="sr-only" aria-label="Import a file" onchange={pickFile} />
      </label>
      <div>
        <h3 id="reads-h" class="eyebrow">It can read</h3>
        <ul class="mt-2 grid grid-cols-1 gap-x-4 gap-y-1.5 text-[15px] wide:grid-cols-2" aria-labelledby="reads-h">
          <li><span class="font-extrabold">Anki</span> <span class="text-muted">.apkg, .colpkg</span></li>
          <li><span class="font-extrabold">Spreadsheets</span> <span class="text-muted">.csv, .tsv</span></li>
          <li><span class="font-extrabold">Shudhana</span> <span class="text-muted">a file from Export</span></li>
          <li><span class="font-extrabold">Question banks</span> <span class="text-muted">.json, .md, GIFT, Aiken</span></li>
        </ul>
      </div>
    </section>

    <section class="panel flex flex-col gap-4 p-6" aria-labelledby="transfer-export-h">
      <h2 id="transfer-export-h" class="text-lg font-black">Export</h2>
      <p class="text-[15px] text-muted">Everything or some topics, as:</p>
      <ul class="flex flex-col gap-2 text-[15px]">
        <li><span class="font-extrabold">A Shudhana file</span> <span class="text-muted">to move to another computer or share with a friend</span></li>
        <li><span class="font-extrabold">An Anki deck</span> <span class="text-muted">cards, and quiz questions as cards if you like</span></li>
        <li><span class="font-extrabold">A spreadsheet, JSON, Markdown, GIFT or Aiken</span> <span class="text-muted">for other apps</span></li>
      </ul>
      <button class="btn btn-outline mt-auto self-start" onclick={() => (exporting = true)}><FileOutput class="size-4" />Export…</button>
    </section>
  </div>

  {#if lastImport}
    <section class="panel mt-5 flex flex-wrap items-center gap-3 p-5" aria-labelledby="transfer-last-h">
      <div class="min-w-0 flex-1">
        <h2 id="transfer-last-h" class="text-sm font-semibold">Last import</h2>
        <p class="text-sm text-muted"><span class="path break-all">{lastImport.file_name}</span>, {madeDay(lastImport, today)} at {madeLabel(lastImport, today).split(", ")[1]}: {lastImport.summary}</p>
      </div>
      <button class="btn btn-sm btn-ghost" onclick={() => (undoing = true)}><Undo2 class="size-4" />Undo it</button>
    </section>
  {/if}

  <p class="mt-5 flex flex-wrap items-center gap-2 text-sm text-muted">Backups and sync stay in <button class="btn btn-ghost btn-sm" onclick={onBackups}>Your data</button></p>
</div>

{#if exporting}<ExportDialog onClose={() => (exporting = false)} onDone={exported} />{/if}
{#if undoing && lastImport}
  <dialog open class="modal modal-open" role="alertdialog" aria-labelledby="transfer-undo-h" aria-describedby="transfer-undo-d">
    <div class="modal-box max-w-md">
      <h2 id="transfer-undo-h" class="text-lg font-black">Undo the import of <span class="path break-all">{lastImport.file_name}</span>?</h2>
      <p id="transfer-undo-d" class="mt-2 text-[15px]">Everything goes back to how it was just before it. Anything you changed since then is undone too.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (undoing = false)}>Keep it</button>
        <button class="btn btn-error" onclick={undoLast}>Undo the import</button>
      </div>
    </div>
  </dialog>
{/if}

<script lang="ts">
  import { dropOnReader } from "../lib/readerDrop.svelte";
  // import-export.feature (R171 to R178): every import dialog, wherever the file came from,
  // plus the progress of a big one and the overlay for a file dropped on the window.
  import { FileInput, FileX } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { ACCEPT, cancelImport, importer, startImport, startImportAt, stopImport } from "../lib/importer.svelte";
  import AnkiImportDialog from "./AnkiImportDialog.svelte";
  import ImportDialog from "./ImportDialog.svelte";
  import ItemsImportDialog from "./ItemsImportDialog.svelte";

  let today = $state(0);
  let picker = $state<HTMLInputElement>();
  let dropping = $state<string | null>(null);
  const step = $derived(importer.step);

  onMount(() => {
    Promise.resolve(api.today()).then((day) => (today = day ?? 0)).catch(() => {});
    let stops: (() => void)[] = [];
    let gone = false;
    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const add = (stop: () => void) => (gone ? stop() : stops.push(stop));
        add(await listen<{ what: string; done: number; total: number }>("import-progress", (event) => (importer.progress = event.payload)));
        add(await listen<string>("open-file", (event) => void startImportAt(event.payload)));
        // A file dropped anywhere but a card or question field (those take images).
        const at = (x: number, y: number) => {
          const scale = window.devicePixelRatio || 1;
          const target = document.elementFromPoint(x / scale, y / scale);
          return target instanceof HTMLElement ? target : null;
        };
        const field = (x: number, y: number) => !!at(x, y)?.closest("[data-rich]");
        // R195: over the reader beside the question editor, a file opens there instead.
        const reader = (x: number, y: number) => !!at(x, y)?.closest("[data-reader-drop]");
        // Images are for card and question fields, so they never open an import.
        let ignoring = false;
        const image = (paths: string[]) => paths.length > 0 && paths.every((path) => /\.(png|jpe?g|gif|webp|svg|bmp|avif)$/i.test(path));
        add(await getCurrentWebview().onDragDropEvent((event) => {
          const payload = event.payload;
          if (payload.type === "enter") ignoring = image(payload.paths);
          if (ignoring) {
            dropping = null;
            if (payload.type === "drop" || payload.type === "leave") ignoring = false;
            return;
          }
          if (payload.type === "enter") dropping = payload.paths[0]?.split(/[\\/]/).pop() ?? "the file";
          else if (payload.type === "over") dropping = field(payload.position.x, payload.position.y) || reader(payload.position.x, payload.position.y) ? null : (dropping ?? "the file");
          else if (payload.type === "leave") dropping = null;
          else if (payload.type === "drop") {
            dropping = null;
            if (payload.paths.length && reader(payload.position.x, payload.position.y)) void dropOnReader(payload.paths).catch(() => {});
            else if (payload.paths.length && !field(payload.position.x, payload.position.y)) void startImportAt(payload.paths[0]);
          }
        }));
        const opened = await api.takeOpenedFile();
        if (opened) void startImportAt(opened);
      } catch {
        // Not the desktop app.
      }
    })();
    return () => {
      gone = true;
      stops.forEach((stop) => stop());
    };
  });

  function another() {
    importer.step = null;
    picker?.click();
  }

  async function picked(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) await startImport(file);
  }

  const kindOf = (name: string) => {
    const extension = name.split(".").pop()?.toLowerCase() ?? "";
    return { apkg: "an Anki deck", colpkg: "an Anki collection", studyapp: "a Shudhana file", csv: "a spreadsheet", tsv: "a spreadsheet", json: "a JSON file", md: "Markdown checklists", txt: "Moodle questions", gift: "Moodle questions" }[extension] ?? "";
  };
  const progressLabel = (value: number) => value.toLocaleString("en-GB");
</script>

<input bind:this={picker} type="file" class="hidden" accept={ACCEPT} aria-label="Choose a file to import" onchange={picked} />

{#if step?.kind === "studyapp"}
  <ImportDialog preview={step.preview} {today} onClose={cancelImport} onDone={() => (importer.step = null)} />
{:else if step?.kind === "anki"}
  <AnkiImportDialog preview={step.preview} />
{:else if step?.kind === "items"}
  <ItemsImportDialog preview={step.preview} />
{:else if step?.kind === "unreadable"}
  <dialog open class="modal modal-open" role="alertdialog" aria-labelledby="bad-h" aria-describedby="bad-d">
    <div class="modal-box max-w-md">
      <span class="well well-wrong" aria-hidden="true"><FileX class="size-5" /></span>
      <h2 id="bad-h" class="mt-4 text-xl font-black">Couldn't import <span class="path break-all">{step.name}</span></h2>
      <p id="bad-d" class="mt-2 text-[15px]">{step.message}</p>
      <p class="mt-2 text-sm text-muted">Try a <span class="path">.studyapp</span>, an Anki <span class="path">.apkg</span> or <span class="path">.colpkg</span>, a <span class="path">.csv</span>, <span class="path">.json</span>, <span class="path">.md</span>, or a Moodle <span class="path">.txt</span>.</p>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (importer.step = null)}>Close</button>
        <button type="button" class="btn btn-primary" onclick={another}>Choose another file</button>
      </div>
    </div>
  </dialog>
{/if}

{#if importer.reading}
  <p class="toast-reading" role="status">Reading {importer.reading}…</p>
{/if}

{#if importer.busy && importer.progress && importer.progress.total > 200}
  {@const progress = importer.progress}
  <dialog open class="modal modal-open" aria-labelledby="busy-h">
    <div class="modal-box max-w-lg">
      <h2 id="busy-h" class="text-xl font-black">Importing</h2>
      <p class="mt-1 text-[15px] text-muted">You can keep this open and come back. It's safe to wait.</p>
      <div class="mt-6">
        <div class="flex justify-between text-[15px] font-bold"><span>{progress.what}</span><span class="tabular-nums">{progressLabel(progress.done)} of {progressLabel(progress.total)}</span></div>
        <div class="mt-2 h-3.5 overflow-hidden rounded-full bg-base-300" role="progressbar" aria-label="{progress.what} imported" aria-valuemin={0} aria-valuemax={progress.total} aria-valuenow={progress.done}>
          <div class="h-full bg-[var(--chart-ink)]" style="width: {Math.round((progress.done / progress.total) * 100)}%"></div>
        </div>
      </div>
      <div class="mt-6 flex items-center gap-3 border-t-2 border-base-300 pt-4">
        <p class="flex-1 text-sm text-muted">Stopping adds nothing. It puts everything back to how it was.</p>
        <button type="button" class="btn btn-ghost" onclick={stopImport}>Stop importing</button>
      </div>
    </div>
  </dialog>
{/if}

{#if dropping}
  <div class="drop-overlay" role="dialog" aria-modal="true" aria-labelledby="drop-h">
    <div class="drop-card">
      <span class="drop-icon" aria-hidden="true"><FileInput class="size-9" /></span>
      <h2 id="drop-h" class="mt-5 text-[28px] font-black">Drop to import</h2>
      <p class="mt-2"><span class="path">{dropping}</span>{#if kindOf(dropping)}, {kindOf(dropping)}{/if}</p>
      <p class="mt-2 text-[15px] text-muted">You'll see what's in it before anything is added.</p>
      <p class="mt-6 text-sm text-muted">Dropping onto a card or question field adds an image there instead.</p>
    </div>
  </div>
{/if}

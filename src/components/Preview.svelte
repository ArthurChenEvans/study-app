<script lang="ts">
  // Source preview (R11, R12): files are read into memory for this session only, never stored.
  import { FileText, FileUp, Plus, Presentation, BookOpen, X } from "@lucide/svelte";
  import { api, errorMessage } from "../lib/api";
  import { closeTab, kindOf, numberLines, openTab, type Tab } from "../lib/preview";
  import { readEpub, type Epub } from "../lib/epub";
  import { safeLinks } from "../lib/links";
  import PdfView from "./PdfView.svelte";

  type Content =
    | { state: "loading" }
    | { state: "error"; message: string }
    | { state: "pdf"; data: Uint8Array }
    | { state: "text"; text: string }
    | { state: "epub"; epub: Epub };

  const ACCEPT = ".pdf,.pptx,.ppt,.odp,.epub,.md,.markdown,.txt";
  const MAX_BYTES = 200 * 1024 * 1024;

  let tabs = $state<Tab[]>([]);
  let active = $state<number | null>(null);
  let contents = $state<Record<number, Content>>({});
  let refused = $state("");
  let nextId = 1;
  let input: HTMLInputElement;

  const current = $derived(tabs.find((tab) => tab.id === active) ?? null);
  const icon = { pdf: FileText, slides: Presentation, epub: BookOpen, markdown: FileText, text: FileText };

  async function load(id: number, file: File, kind: NonNullable<ReturnType<typeof kindOf>>) {
    try {
      if (file.size > MAX_BYTES) throw new Error("that file is too big to preview (the limit is 200 MB)");
      if (kind === "pdf") contents[id] = { state: "pdf", data: new Uint8Array(await file.arrayBuffer()) };
      else if (kind === "slides") contents[id] = { state: "pdf", data: await api.slidesToPdf(file) };
      else if (kind === "epub") contents[id] = { state: "epub", epub: await readEpub(await file.arrayBuffer()) };
      else contents[id] = { state: "text", text: await file.text() };
    } catch (failure) {
      contents[id] = { state: "error", message: errorMessage(failure) };
    }
  }

  function onFiles(event: Event) {
    refused = "";
    const files = Array.from((event.currentTarget as HTMLInputElement).files ?? []);
    for (const file of files) {
      const kind = kindOf(file.name);
      if (!kind) {
        refused = `${file.name} can't be previewed. Open a PDF, PowerPoint, EPUB, Markdown or txt file.`;
        continue;
      }
      const id = nextId++;
      contents[id] = { state: "loading" };
      ({ tabs, active } = openTab(tabs, { id, name: file.name, kind }));
      load(id, file, kind);
    }
    input.value = "";
  }

  function close(id: number) {
    ({ tabs, active } = closeTab(tabs, active, id));
    delete contents[id];
  }
</script>

<input bind:this={input} type="file" class="hidden" accept={ACCEPT} multiple aria-label="Open file" onchange={onFiles} />

{#if tabs.length === 0}
  <div class="grid h-full place-items-center p-12">
    <div class="drop max-w-sm p-8 text-center">
      <span class="well mx-auto" aria-hidden="true"><FileUp class="size-5" /></span>
      <h2 class="mt-4 text-lg font-semibold">Read alongside as you write</h2>
      <p class="mt-2 text-sm text-muted">Markdown, txt, PDF, PowerPoint or EPUB. Files are only shown while you write, they aren't stored.</p>
      <button class="btn btn-outline btn-sm mt-5" onclick={() => input.click()}><FileUp class="size-4" />Open file</button>
      {#if refused}<p role="alert" class="mt-4 text-sm text-error">{refused}</p>{/if}
    </div>
  </div>
{:else}
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex items-center gap-1 overflow-x-auto border-b border-base-300 px-3 pt-3" role="tablist" aria-label="Open files">
      {#each tabs as tab (tab.id)}
        {@const Icon = icon[tab.kind]}
        <div class="flex items-center border-b-2 {tab.id === active ? 'border-primary-ink' : 'border-transparent'}">
          <button role="tab" aria-selected={tab.id === active} class="flex max-w-56 items-center gap-2 py-2 pl-3 pr-1 text-sm {tab.id === active ? 'font-semibold' : 'text-muted'}" onclick={() => (active = tab.id)}>
            <Icon class="size-4 shrink-0" /><span class="truncate">{tab.name}</span>
          </button>
          <button class="btn btn-ghost btn-xs btn-square" aria-label="Close {tab.name}" onclick={() => close(tab.id)}><X class="size-3" /></button>
        </div>
      {/each}
      <button class="btn btn-ghost btn-xs ml-1" aria-label="Open another file" onclick={() => input.click()}><Plus class="size-4" /></button>
    </div>
    {#if refused}<p role="alert" class="mx-6 mt-3 text-sm text-error">{refused}</p>{/if}

    {#if current}
      {@const content = contents[current.id]}
      {#key current.id}
        <div class="min-h-0 flex-1">
          {#if !content || content.state === "loading"}
            <p class="p-6 text-sm text-muted" aria-live="polite">{current.kind === "slides" ? "Converting slides…" : "Opening…"}</p>
          {:else if content.state === "error"}
            <p role="alert" class="p-6 text-sm text-error">{content.message}</p>
          {:else if content.state === "pdf"}
            <PdfView data={content.data} name={current.name} />
          {:else if content.state === "text"}
            <ol class="h-full overflow-auto bg-surface py-4 font-mono text-sm" aria-label={current.name}>
              {#each numberLines(content.text) as line (line.number)}
                <li class="flex gap-4 px-4 leading-6"><span class="w-10 shrink-0 select-none text-right text-muted tabular-nums">{line.number}</span><span class="whitespace-pre-wrap break-words">{line.text}</span></li>
              {/each}
            </ol>
          {:else if content.state === "epub"}
            <div class="h-full overflow-auto p-8">
              <article class="rich mx-auto max-w-prose rounded-lg bg-surface p-8" aria-label={content.epub.title}>
                <p class="eyebrow">{content.epub.title}</p>
                {#each content.epub.chapters as chapter, index (index)}
                  <section class="epub-chapter" aria-label={chapter.title} use:safeLinks>{@html chapter.html}</section>
                {/each}
              </article>
            </div>
          {/if}
        </div>
      {/key}
    {/if}
  </div>
{/if}

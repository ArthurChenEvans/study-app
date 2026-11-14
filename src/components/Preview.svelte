<script lang="ts">
  // Source preview (R11, R12): files are read into memory for this session only, never stored.
  // R179 to R182: a table of contents, going straight to a page or line (Ctrl+G), and showing
  // where a question cites, remembering which file is which source by its name only.
  import { FileText, FileUp, Plus, Presentation, BookOpen, X } from "@lucide/svelte";
  import { tick } from "svelte";
  import { api, errorMessage } from "../lib/api";
  import { closeTab, kindOf, numberLines, openTab, type Tab } from "../lib/preview";
  import { readEpub, type Epub } from "../lib/epub";
  import { safeLinks } from "../lib/links";
  import { fileFor, remember, showing } from "../lib/sourceFiles.svelte";
  import { locationTarget, markdownHeadings } from "../lib/toc";
  import PdfView from "./PdfView.svelte";
  import Contents from "./PreviewContents.svelte";
  import { readerDrop } from "../lib/readerDrop.svelte";

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

  // Where each tab is: the line picked in a text file, the page asked of a PDF.
  let lineAt = $state<Record<number, number>>({});
  let pageAt = $state<Record<number, { page: number; at: number }>>({});
  let focusAt = $state(0);
  import { contentsPanel, toggleContents } from "../lib/contentsPanel.svelte";
  let typed = $state("");
  let lineBox = $state<HTMLInputElement>();
  let chapterAt = $state(0);

  async function goToLine(id: number, line: number, lines: number) {
    const number = Math.min(Math.max(1, Math.round(line)), lines);
    lineAt[id] = number;
    typed = "";
    await tick();
    document.querySelector(`[data-line="${id}-${number}"]`)?.scrollIntoView({ block: "center" });
  }

  async function goToChapter(at: number, anchor: string | null = null) {
    chapterAt = at;
    await tick();
    const section = document.querySelector<HTMLElement>(`[data-chapter="${at}"]`);
    const spot = anchor ? section?.querySelector<HTMLElement>(`[id="${CSS.escape(anchor)}"]`) : null;
    (spot ?? section)?.scrollIntoView({ block: "start" });
  }

  function onKey(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "g" || !current) return;
    const content = contents[current.id];
    if (content?.state === "pdf") {
      event.preventDefault();
      focusAt = Date.now();
    } else if (content?.state === "text") {
      event.preventDefault();
      lineBox?.focus();
      lineBox?.select();
    }
  }

  // ---------- Showing where a question cites (R181, R182) ----------
  type Ask = { source: number; title: string; location: string };
  let asking = $state<(Ask & { pick: string }) | null>(null);
  let missing = $state<(Ask & { name: string | null }) | null>(null);

  const targetWords = (location: string) => {
    const target = locationTarget(location);
    return target ? ("line" in target ? `line ${target.line}` : `page ${target.page}`) : location.trim() || "it";
  };

  function show(tab: Tab, location: string) {
    active = tab.id;
    asking = null;
    missing = null;
    const target = locationTarget(location);
    const content = contents[tab.id];
    if (!target) return;
    if ("page" in target) pageAt[tab.id] = { page: target.page, at: Date.now() };
    else if (content?.state === "text") void goToLine(tab.id, target.line, numberLines(content.text).length);
    else pageAt[tab.id] = { page: target.line, at: Date.now() };
  }

  function answer(request: Ask) {
    const name = fileFor(request.source);
    const tab = name ? tabs.find((open) => open.name === name) : undefined;
    if (tab) show(tab, request.location);
    else if (!name && tabs.length) asking = { ...request, pick: current?.name ?? tabs[0].name };
    else missing = { ...request, name };
  }

  $effect(() => {
    const request = showing.request;
    if (request) {
      showing.request = null;
      answer(request);
    }
  });

  function picked() {
    if (!asking) return;
    const tab = tabs.find((open) => open.name === asking!.pick);
    if (!tab) return;
    remember(asking.source, tab.name);
    show(tab, asking.location);
  }
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
    openFiles(Array.from((event.currentTarget as HTMLInputElement).files ?? []));
    input.value = "";
  }

  // R195: a file dropped on the reader opens here. The browser hands it over directly, and the
  // desktop app's window drop leaves it in readerDrop.
  function dropped(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    openFiles(Array.from(event.dataTransfer?.files ?? []));
  }
  $effect(() => {
    if (!readerDrop.files.length) return;
    const files = readerDrop.files;
    readerDrop.files = [];
    openFiles(files);
  });

  function openFiles(files: File[]) {
    refused = "";
    for (const file of files) {
      const kind = kindOf(file.name);
      if (!kind) {
        refused = `${file.name} can't be previewed. Open a PDF, PowerPoint, EPUB, Markdown or txt file.`;
        continue;
      }
      const id = nextId++;
      contents[id] = { state: "loading" };
      ({ tabs, active } = openTab(tabs, { id, name: file.name, kind }));
      const waiting = missing;
      load(id, file, kind).then(() => {
        // The file a citation was waiting for. With no file remembered yet, the one just opened is it.
        if (waiting && missing === waiting && (waiting.name === null || waiting.name === file.name)) {
          remember(waiting.source, file.name);
          show(tabs.find((tab) => tab.id === id)!, waiting.location);
        }
      });
    }
  }

  function close(id: number) {
    ({ tabs, active } = closeTab(tabs, active, id));
    delete contents[id];
  }
</script>

<svelte:window onkeydown={onKey} />

{#snippet citationNotes()}
  {#if asking}
    {@const ask = asking}
    <div class="m-3 flex flex-wrap items-center gap-2 rounded-box border-2 border-base-300 bg-base-100 p-3 text-sm" role="group" aria-label="Which open file is {ask.title}?">
      <span class="w-full font-semibold">Which open file is {ask.title}?</span>
      <select class="select select-sm min-w-0 flex-1" aria-label="File" bind:value={ask.pick}>
        {#each tabs as tab (tab.id)}<option value={tab.name}>{tab.name}</option>{/each}
      </select>
      <button type="button" class="btn btn-sm btn-primary" onclick={picked}>Show {targetWords(ask.location)}</button>
      <button type="button" class="btn btn-sm btn-ghost" onclick={() => (asking = null)}>Cancel</button>
      <span class="w-full text-xs text-muted">Only the file's name is remembered, on this computer.</span>
    </div>
  {:else if missing}
    <div class="m-3 flex flex-wrap items-center gap-2 rounded-box border-2 border-base-300 bg-base-100 p-3 text-sm" role="status">
      <span class="min-w-0 flex-1">{missing.name ? `Open ${missing.name} to see ${targetWords(missing.location)} of ${missing.title}.` : `Open the file for ${missing.title} to see ${targetWords(missing.location)}.`}</span>
      <button type="button" class="btn btn-sm" onclick={() => input.click()}>{missing.name ? `Open ${missing.name}…` : "Open file…"}</button>
      <button type="button" class="btn btn-sm btn-ghost btn-square" aria-label="Dismiss" onclick={() => (missing = null)}><X class="size-4" /></button>
    </div>
  {/if}
{/snippet}

<input bind:this={input} type="file" class="hidden" accept={ACCEPT} multiple aria-label="Open file" onchange={onFiles} />

<div class="contents h-full" data-reader-drop role="region" aria-label="Reader" ondragover={(event) => event.preventDefault()} ondrop={dropped}>
{#if tabs.length === 0}
  <div class="grid h-full place-items-center p-12">
    <div class="drop max-w-sm p-8 text-center">
      <span class="well mx-auto" aria-hidden="true"><FileUp class="size-5" /></span>
      <h2 class="mt-4 text-lg font-semibold">Read alongside as you write</h2>
      <p class="mt-2 text-sm text-muted">Markdown, txt, PDF, PowerPoint or EPUB. Files are only shown while you write, they aren't stored.</p>
      <button class="btn btn-outline btn-sm mt-5" onclick={() => input.click()}><FileUp class="size-4" />Open file</button>
      {#if refused}<p role="alert" class="mt-4 text-sm text-error">{refused}</p>{/if}
      {@render citationNotes()}
    </div>
  </div>
{:else}
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex items-center gap-1 overflow-x-auto border-b border-base-300 px-3 pt-3" role="tablist" aria-label="Open files">
      {#each tabs as tab (tab.id)}
        {@const Icon = icon[tab.kind]}
        <div class="flex items-center border-b-2 {tab.id === active ? 'border-primary-ink' : 'border-transparent'}">
          <button role="tab" aria-selected={tab.id === active} class="flex max-w-56 items-center gap-2 py-2 pl-3 pr-1 text-sm {tab.id === active ? 'font-semibold' : 'text-muted'}" onclick={() => { active = tab.id; }}>
            <Icon class="size-4 shrink-0" /><span class="truncate">{tab.name}</span>
          </button>
          <button class="btn btn-ghost btn-xs btn-square" aria-label="Close {tab.name}" onclick={() => close(tab.id)}><X class="size-3" /></button>
        </div>
      {/each}
      <button class="btn btn-ghost btn-xs ml-1" aria-label="Open another file" onclick={() => input.click()}><Plus class="size-4" /></button>
    </div>
    {#if refused}<p role="alert" class="mx-6 mt-3 text-sm text-error">{refused}</p>{/if}
    {@render citationNotes()}

    {#if current}
      {@const content = contents[current.id]}
      {#key current.id}
        <div class="min-h-0 flex-1">
          {#if !content || content.state === "loading"}
            <p class="p-6 text-sm text-muted" aria-live="polite">{current.kind === "slides" ? "Converting slides…" : "Opening…"}</p>
          {:else if content.state === "error"}
            <p role="alert" class="p-6 text-sm text-error">{content.message}</p>
          {:else if content.state === "pdf"}
            <PdfView data={content.data} name={current.name} slides={current.kind === "slides"} target={pageAt[current.id] ?? null} {focusAt} />
          {:else if content.state === "text"}
            {@const lines = numberLines(content.text)}
            {@const headings = current.kind === "markdown" ? markdownHeadings(content.text).map((heading) => ({ title: heading.title, depth: heading.depth, at: heading.line })) : []}
            {@const id = current.id}
            <div class="flex h-full min-h-0 flex-col">
              <div class="flex flex-wrap items-center gap-2 border-b border-base-300 px-4 py-2">
                {#if headings.length}<button type="button" class="btn btn-ghost btn-sm" aria-expanded={contentsPanel.open} onclick={toggleContents}>Contents</button>{/if}
                <form class="ml-auto flex items-center gap-2 text-sm" onsubmit={(event) => event.preventDefault()}>
                  <label class="text-muted" for="go-line">Line</label>
                  <input id="go-line" bind:this={lineBox} type="number" min="1" max={lines.length} class="input input-sm w-20 tabular-nums" aria-label="Go to line" placeholder={String(lineAt[id] ?? 1)} bind:value={typed} onkeydown={(event) => { if (event.key === "Enter" && typed) { event.preventDefault(); void goToLine(id, Number(typed), lines.length); } }} />
                  <span class="text-muted tabular-nums">of {lines.length}</span>
                </form>
              </div>
              <div class="reader-body">
              {#if contentsPanel.open && headings.length}
                <Contents label="Contents of {current.name}" items={headings} current={lineAt[id] ?? 0} onPick={(line) => { void goToLine(id, line, lines.length); }} />
              {/if}
              <ol class="min-h-0 flex-1 overflow-auto bg-surface py-4 font-mono text-sm" aria-label={current.name}>
                {#each lines as line (line.number)}
                  <li data-line="{id}-{line.number}" aria-current={lineAt[id] === line.number ? "location" : undefined} class="flex gap-4 px-4 leading-6 {lineAt[id] === line.number ? 'bg-[color-mix(in_oklch,var(--primary-ink)_14%,transparent)]' : ''}"><span class="w-10 shrink-0 select-none text-right text-muted tabular-nums">{line.number}</span><span class="whitespace-pre-wrap break-words">{line.text}</span></li>
                {/each}
              </ol>
              </div>
            </div>
          {:else if content.state === "epub"}
            {@const epub = content.epub}
            <div class="flex h-full min-h-0 flex-col">
              <div class="flex items-center gap-2 border-b border-base-300 px-4 py-2">
                <button type="button" class="btn btn-ghost btn-sm" aria-expanded={contentsPanel.open} onclick={toggleContents}>Contents</button>
                <span class="ml-auto truncate text-sm text-muted">{epub.chapters[chapterAt]?.title}</span>
              </div>
              <div class="reader-body">
              {#if contentsPanel.open}
                <Contents label="Contents of {current.name}" items={epub.toc} current={chapterAt} onPick={(at) => { const entry = epub.toc.find((item) => item.at === at); void goToChapter(at, entry?.anchor ?? null); }} />
              {/if}
              <div class="min-h-0 flex-1 overflow-auto p-4 sm:p-8">
                <article class="rich mx-auto max-w-prose rounded-lg bg-surface p-5 sm:p-8" aria-label={epub.title}>
                  <p class="eyebrow">{epub.title}</p>
                  {#each epub.chapters as chapter, index (index)}
                    <section class="epub-chapter" data-chapter={index} aria-label={chapter.title} use:safeLinks>{@html chapter.html}</section>
                  {/each}
                </article>
              </div>
              </div>
            </div>
          {/if}
        </div>
      {/key}
    {/if}
  </div>
{/if}
</div>

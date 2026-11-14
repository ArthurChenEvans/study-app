<script lang="ts">
  // PDF (and converted slides) with a selectable text layer, rendered with PDF.js.
  // sources.feature (R179, R180): pages are drawn only as they come near the screen, so going
  // to page 300 doesn't wait for the 299 before it, and the book's bookmarks (or each slide's
  // title) make a table of contents.
  import { onDestroy, onMount, tick } from "svelte";
  import "pdfjs-dist/web/pdf_viewer.css";
  import Contents from "./PreviewContents.svelte";
  import type { PDFDocumentProxy } from "pdfjs-dist";

  let { data, name, slides = false, target = null, focusAt = 0 }: { data: Uint8Array; name: string; slides?: boolean; target?: { page: number; at: number } | null; focusAt?: number } = $props();

  let container = $state<HTMLDivElement>();
  let goBox = $state<HTMLInputElement>();
  let pages = $state(0);
  let current = $state(1);
  let typed = $state("");
  let error = $state("");
  let toc = $state<{ title: string; depth: number; at: number }[]>([]);
  import { contentsPanel, toggleContents } from "../lib/contentsPanel.svelte";
  import { pageHeadings } from "../lib/toc";
  let tocNote = $state("");
  let destroyed = false;
  let loading: { destroy: () => Promise<void> } | null = null;
  let pdf: PDFDocumentProxy | null = null;
  let pdfjs: typeof import("pdfjs-dist") | null = null;
  let observer: IntersectionObserver | null = null;
  const drawn = new Set<number>();
  let scale = 1;

  const word = $derived(slides ? "slide" : "page");

  async function draw(number: number, box: HTMLElement) {
    if (!pdf || !pdfjs || drawn.has(number) || destroyed) return;
    drawn.add(number);
    const page = await pdf.getPage(number);
    const viewport = page.getViewport({ scale });
    box.style.width = `${viewport.width}px`;
    box.style.height = `${viewport.height}px`;
    box.style.setProperty("--scale-factor", String(viewport.scale));
    const canvas = document.createElement("canvas");
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.floor(viewport.width * ratio);
    canvas.height = Math.floor(viewport.height * ratio);
    canvas.style.width = `${viewport.width}px`;
    canvas.style.height = `${viewport.height}px`;
    const text = document.createElement("div");
    text.className = "textLayer";
    box.replaceChildren(canvas, text);
    await page.render({ canvas, viewport, transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0] }).promise;
    await new pdfjs.TextLayer({ textContentSource: page.streamTextContent(), container: text, viewport }).render();
  }

  async function outline(document: PDFDocumentProxy) {
    const items = (await document.getOutline().catch(() => null)) ?? [];
    const rows: { title: string; depth: number; at: number }[] = [];
    const walk = async (list: typeof items, depth: number) => {
      for (const item of list) {
        if (rows.length >= 2000) return;
        try {
          const destination = typeof item.dest === "string" ? await document.getDestination(item.dest) : item.dest;
          const reference = destination?.[0];
          const index = typeof reference === "number" ? reference : reference ? await document.getPageIndex(reference) : null;
          if (index !== null && item.title.trim()) rows.push({ title: item.title.trim(), depth, at: index + 1 });
        } catch {
          // A bookmark to nowhere is left out.
        }
        if (depth < 3 && item.items?.length) await walk(item.items, depth + 1);
      }
    };
    await walk(items, 1);
    if (rows.length) return rows;
    if (!slides) {
      // R197: no bookmarks, so each page's biggest text stands in as its heading.
      const pages = [];
      for (let number = 1; number <= Math.min(document.numPages, 500) && !destroyed; number++) {
        const content = await (await document.getPage(number)).getTextContent();
        pages.push(content.items.flatMap((item) => ("str" in item ? [{ str: item.str, height: item.height || Math.abs(item.transform[3]) }] : [])));
      }
      const made = pageHeadings(pages);
      if (made.length) tocNote = "Made from each page's heading, as this file has no outline.";
      return made;
    }
    // Slides have no bookmarks, so each slide's first line of text is its title.
    for (let number = 1; number <= Math.min(document.numPages, 500) && !destroyed; number++) {
      const page = await document.getPage(number);
      const content = await page.getTextContent();
      const first = content.items.map((item) => ("str" in item ? item.str.trim() : "")).find(Boolean);
      rows.push({ title: first ? (first.length > 80 ? `${first.slice(0, 79)}…` : first) : `Slide ${number}`, depth: 1, at: number });
    }
    return rows;
  }

  onMount(async () => {
    try {
      pdfjs = await import("pdfjs-dist");
      const worker = await import("pdfjs-dist/build/pdf.worker.min.mjs?url");
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      const task = pdfjs.getDocument({ data: data.slice() });
      loading = task;
      pdf = await task.promise;
      const first = (await pdf.getPage(1)).getViewport({ scale: 1 });
      scale = Math.max(0.5, ((container?.clientWidth ?? 800) - 48) / first.width);
      observer = new IntersectionObserver((entries) => {
        for (const entry of entries) if (entry.isIntersecting) void draw(Number((entry.target as HTMLElement).dataset.page), entry.target as HTMLElement);
      }, { root: container, rootMargin: "1200px 0px" });
      // Every page gets its place straight away, sized like the first, and is drawn when near.
      for (let number = 1; number <= pdf.numPages; number++) {
        const box = document.createElement("div");
        box.className = "pdf-page";
        box.dataset.page = String(number);
        box.setAttribute("aria-label", `${slides ? "Slide" : "Page"} ${number}`);
        box.style.width = `${first.width * scale}px`;
        box.style.height = `${first.height * scale}px`;
        container?.append(box);
        observer.observe(box);
      }
      pages = pdf.numPages;
      if (target) void goTo(target.page);
      outline(pdf).then((rows) => (toc = rows)).catch(() => {});
    } catch (failure) {
      error = `${name} couldn't be opened: ${failure instanceof Error ? failure.message : String(failure)}`;
    }
  });

  onDestroy(() => {
    destroyed = true;
    observer?.disconnect();
    loading?.destroy();
  });

  async function goTo(page: number) {
    if (!pages || !container) return;
    const number = Math.min(Math.max(1, Math.round(page)), pages);
    const box = container.querySelector<HTMLElement>(`[data-page="${number}"]`);
    if (!box) return;
    await draw(number, box);
    await tick();
    container.scrollTop = box.offsetTop - container.offsetTop - 16;
    current = number;
    typed = "";
  }

  // A citation's "Show page", or Ctrl+G.
  $effect(() => {
    if (target && pages) void goTo(target.page);
  });
  $effect(() => {
    if (focusAt && goBox) {
      goBox.focus();
      goBox.select();
    }
  });

  function onScroll() {
    if (!container) return;
    const boxes = Array.from(container.querySelectorAll<HTMLElement>(".pdf-page"));
    const top = container.scrollTop + container.offsetTop + container.clientHeight / 3;
    const visible = boxes.filter((box) => box.offsetTop <= top).at(-1);
    if (visible) current = Number(visible.dataset.page);
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  {#if pages}
    <div class="flex flex-wrap items-center gap-2 border-b border-base-300 px-4 py-2">
      {#if toc.length}
        <button type="button" class="btn btn-ghost btn-sm" aria-expanded={contentsPanel.open} onclick={toggleContents}>Contents</button>
      {/if}
      <form class="ml-auto flex items-center gap-2 text-sm" onsubmit={(event) => event.preventDefault()}>
        <label class="text-muted" for="go-page">{slides ? "Slide" : "Page"}</label>
        <input id="go-page" bind:this={goBox} type="number" min="1" max={pages} class="input input-sm w-20 tabular-nums" aria-label="Go to {word}" placeholder={String(current)} bind:value={typed} onkeydown={(event) => { if (event.key === "Enter" && typed) { event.preventDefault(); void goTo(Number(typed)); } }} />
        <span class="text-muted tabular-nums" aria-live="polite">of {pages}</span>
      </form>
    </div>
  {/if}
  {#if error}<p role="alert" class="m-6 text-sm text-error">{error}</p>{/if}
  <div class="reader-body">
    {#if pages && contentsPanel.open && toc.length}
      <Contents label="Contents of {name}" items={toc} {current} unit={word} note={tocNote} onPick={(at) => { void goTo(at); }} />
    {/if}
    <div bind:this={container} onscroll={onScroll} class="pdf-pages min-h-0 flex-1 space-y-4 overflow-auto p-6" aria-label="{name} pages"></div>
  </div>
</div>

<style>
  :global(.pdf-page) { position: relative; margin: 0 auto; max-width: 100%; background: white; box-shadow: var(--shadow-panel); border-radius: 4px; overflow: hidden; }
  :global(.pdf-page canvas) { display: block; }
</style>

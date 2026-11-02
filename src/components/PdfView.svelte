<script lang="ts">
  // PDF (and converted slides) with a selectable text layer, rendered with PDF.js.
  import { onDestroy, onMount } from "svelte";
  import "pdfjs-dist/web/pdf_viewer.css";

  let { data, name }: { data: Uint8Array; name: string } = $props();

  let container: HTMLDivElement;
  let pages = $state(0);
  let current = $state(1);
  let error = $state("");
  let destroyed = false;
  let loading: { destroy: () => Promise<void> } | null = null;

  onMount(async () => {
    try {
      const pdfjs = await import("pdfjs-dist");
      const worker = await import("pdfjs-dist/build/pdf.worker.min.mjs?url");
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      const task = pdfjs.getDocument({ data: data.slice() });
      loading = task;
      const pdf = await task.promise;
      pages = pdf.numPages;
      const width = container.clientWidth - 48;
      for (let number = 1; number <= pdf.numPages && !destroyed; number++) {
        const page = await pdf.getPage(number);
        const base = page.getViewport({ scale: 1 });
        const viewport = page.getViewport({ scale: Math.max(0.5, width / base.width) });
        const pageBox = document.createElement("div");
        pageBox.className = "pdf-page";
        pageBox.dataset.page = String(number);
        pageBox.style.width = `${viewport.width}px`;
        pageBox.style.height = `${viewport.height}px`;
        pageBox.style.setProperty("--scale-factor", String(viewport.scale));
        const canvas = document.createElement("canvas");
        const ratio = window.devicePixelRatio || 1;
        canvas.width = Math.floor(viewport.width * ratio);
        canvas.height = Math.floor(viewport.height * ratio);
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;
        const text = document.createElement("div");
        text.className = "textLayer";
        pageBox.append(canvas, text);
        container.append(pageBox);
        await page.render({ canvas, viewport, transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0] }).promise;
        await new pdfjs.TextLayer({ textContentSource: page.streamTextContent(), container: text, viewport }).render();
      }
    } catch (failure) {
      error = `${name} couldn't be opened: ${failure instanceof Error ? failure.message : String(failure)}`;
    }
  });

  onDestroy(() => {
    destroyed = true;
    loading?.destroy();
  });

  function onScroll() {
    const boxes = Array.from(container.querySelectorAll<HTMLElement>(".pdf-page"));
    const top = container.scrollTop + container.clientHeight / 3;
    const visible = boxes.filter((box) => box.offsetTop <= top).at(-1);
    if (visible) current = Number(visible.dataset.page);
  }
</script>

<div class="flex h-full min-h-0 flex-col">
  {#if pages}<p class="px-6 pt-3 text-right text-xs text-muted tabular-nums" aria-live="polite">Page {current} of {pages}</p>{/if}
  {#if error}<p role="alert" class="m-6 text-sm text-error">{error}</p>{/if}
  <div bind:this={container} onscroll={onScroll} class="pdf-pages min-h-0 flex-1 space-y-4 overflow-auto p-6" aria-label="{name} pages"></div>
</div>

<style>
  :global(.pdf-page) { position: relative; margin: 0 auto; background: white; box-shadow: var(--shadow-panel); border-radius: 4px; overflow: hidden; }
  :global(.pdf-page canvas) { display: block; }
</style>

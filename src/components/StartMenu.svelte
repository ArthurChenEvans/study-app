<script lang="ts" module>
  export type StartKind = "cards" | "recall" | "quizzes" | "drills" | "pick";
</script>

<script lang="ts">
  // navigation.feature (R198): one Start button for everything due today. The menu lists only
  // what has something due, each with a letter, and S opens it from anywhere.
  import { ChevronDown, Play } from "@lucide/svelte";
  import { tick } from "svelte";

  let { due = {}, onStart, variant = "side" }: {
    due?: { calendar?: number; cards?: number; recall?: number; quizzes?: number; projects?: number };
    onStart: (kind: StartKind) => void;
    variant?: "side" | "fab" | "dock";
  } = $props();

  let open = $state(false);
  let menu = $state<HTMLDivElement>();
  let button = $state<HTMLButtonElement>();
  const total = $derived(due.calendar ?? (due.cards ?? 0) + (due.recall ?? 0) + (due.quizzes ?? 0) + (due.projects ?? 0));

  const entries = $derived(([
    { kind: "cards", label: "Review cards", count: due.cards ?? 0, key: "C" },
    { kind: "recall", label: "Free recall", count: due.recall ?? 0, key: "R" },
    { kind: "quizzes", label: "Quiz the due topics", count: due.quizzes ?? 0, key: "Q" },
    { kind: "drills", label: "Drills", count: due.projects ?? 0, key: "D" },
  ] as { kind: StartKind; label: string; count: number; key: string }[]).filter((entry) => entry.count > 0).concat([{ kind: "pick", label: "Pick topics to quiz…", count: 0, key: "P" }]));

  async function show() {
    open = true;
    await tick();
    menu?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
  }
  function close(refocus = true) {
    open = false;
    if (refocus) button?.focus();
  }
  function choose(kind: StartKind) {
    close(false);
    onStart(kind);
  }

  const typing = (target: EventTarget | null) => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

  function keys(event: KeyboardEvent) {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (open) {
      if (event.key === "Escape") { event.preventDefault(); close(); return; }
      const entry = entries.find((item) => item.key.toLowerCase() === event.key.toLowerCase());
      if (entry) { event.preventDefault(); choose(entry.kind); return; }
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const items = [...(menu?.querySelectorAll<HTMLElement>("[role=menuitem]") ?? [])];
        const at = items.indexOf(document.activeElement as HTMLElement);
        items[(at + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
      }
      return;
    }
    if (event.key.toLowerCase() === "s" && !typing(event.target) && !document.querySelector("[role=dialog][aria-modal=true]")) {
      event.preventDefault();
      void show();
    }
  }
</script>

<svelte:window onkeydown={keys} />

<div class="start-wrap start-at-{variant}">
  {#if variant === "dock"}
    <button bind:this={button} type="button" class="dock-tab dock-start" aria-label="Start, {total} due" aria-haspopup="menu" aria-expanded={open} onclick={() => (open ? close() : show())}>
      <span class="dock-start-disc"><Play class="size-5 fill-current" strokeWidth={2.4} /></span><span aria-hidden="true">Start</span>
    </button>
  {:else}
    <button bind:this={button} type="button" class="btn {variant === 'fab' ? 'start-fab' : 'btn-primary w-full'}" aria-label="Start, {total} due" aria-haspopup="menu" aria-expanded={open} title="Start (S)" onclick={() => (open ? close() : show())}>
      <Play class="size-4 fill-current" />START{#if total}<span class="start-count" aria-hidden="true">{total}</span>{/if}{#if variant === "side"}<ChevronDown class="ml-auto size-4" aria-hidden="true" />{/if}
    </button>
  {/if}
  {#if open}
    <button type="button" class="start-scrim" aria-label="Close Start" tabindex="-1" onclick={() => close()}></button>
    <div bind:this={menu} class="start-menu" role="menu" aria-label="Start">
      {#each entries as entry (entry.kind)}
        <button type="button" role="menuitem" class="start-item" onclick={() => choose(entry.kind)}>
          <span class="flex-1 text-left">{entry.label}</span>
          {#if entry.count}<span class="font-black text-[var(--primary-ink)] tabular-nums">{entry.count}</span>{/if}
          <kbd class="kbd kbd-sm">{entry.key}</kbd>
        </button>
      {/each}
    </div>
  {/if}
</div>

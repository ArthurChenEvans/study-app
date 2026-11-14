<script lang="ts">
  // A source's table of contents (R179): bookmarks, chapters, slide titles or headings.
  // R197: it sits beside the file and stays open, with a box to find a heading. On a narrow
  // screen it lays over the file and closes once you pick.
  import { contentsPanel, toggleContents } from "../lib/contentsPanel.svelte";
  import { matchesFilter } from "../lib/lists";

  let { label, items, current = 0, unit = "", note = "", onPick }: { label: string; items: { title: string; depth: number; at: number }[]; current?: number; unit?: string; note?: string; onPick: (at: number) => void } = $props();

  let find = $state("");
  // The entry for where you're reading: the last one that starts at or before it.
  const here = $derived(items.reduce((found, item, index) => (item.at <= current ? index : found), -1));
  const shown = $derived(items.map((item, index) => ({ ...item, index })).filter((item) => matchesFilter(find, item.title)));

  function pick(at: number) {
    onPick(at);
    if (window.innerWidth < 768) toggleContents();
  }
</script>

<nav class="contents-panel" aria-label={label}>
  {#if items.length > 2}
    <input type="search" class="input input-sm mb-2 w-full" aria-label="Find a heading" placeholder="Find a heading…" bind:value={find} />
  {/if}
  <ul>
    {#each shown as item (item.index)}
      <li>
        <button type="button" class="flex min-h-8 w-full items-baseline gap-3 rounded-md py-1.5 pr-2 text-left text-sm hover:bg-base-300 {item.index === here ? 'font-extrabold text-base-content' : ''}" style="padding-left: {0.5 + (item.depth - 1) * 1}rem" aria-current={item.index === here ? "location" : undefined} onclick={() => pick(item.at)}>
          <span class="min-w-0 flex-1 truncate" title={item.title}>{item.title}</span>
          {#if unit}<span class="shrink-0 text-xs text-muted tabular-nums" aria-hidden="true">{item.at}</span>{/if}
        </button>
      </li>
    {:else}
      <li class="px-2 py-1 text-sm text-muted">No heading matches.</li>
    {/each}
  </ul>
  {#if note}<p class="mt-3 px-2 text-xs text-muted">{note}</p>{/if}
</nav>

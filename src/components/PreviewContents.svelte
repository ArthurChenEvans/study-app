<script lang="ts">
  // A source's table of contents (R179): bookmarks, chapters, slide titles or headings.
  let { label, items, current = 0, unit = "", onPick }: { label: string; items: { title: string; depth: number; at: number }[]; current?: number; unit?: string; onPick: (at: number) => void } = $props();

  // The entry for where you're reading: the last one that starts at or before it.
  const here = $derived(items.reduce((found, item, index) => (item.at <= current ? index : found), -1));
</script>

<nav class="max-h-[40vh] overflow-y-auto border-b border-base-300 bg-base-200 px-2 py-2" aria-label={label}>
  <ul>
    {#each items as item, index (index)}
      <li>
        <button type="button" class="flex w-full items-baseline gap-3 rounded-md py-1.5 pr-2 text-left text-sm hover:bg-base-300 {index === here ? 'font-semibold' : ''}" style="padding-left: {0.5 + (item.depth - 1) * 1}rem" aria-current={index === here ? "location" : undefined} onclick={() => onPick(item.at)}>
          <span class="min-w-0 flex-1 truncate">{item.title}</span>
          {#if unit}<span class="shrink-0 text-xs text-muted tabular-nums" aria-hidden="true">{item.at}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
</nav>

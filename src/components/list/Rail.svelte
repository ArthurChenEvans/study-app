<script lang="ts" module>
  export type RailItem = { key: string; label: string; count: number };
</script>

<script lang="ts">
  // lists.feature (R138): a module rail, like SkyUI's category column. All, every module
  // and No module, each with its count. The picked one is pressed, in words as well as look.
  import { Folder, FolderOpen, Layers } from "@lucide/svelte";

  let { items, value = $bindable("all"), label = "Modules", onPick = () => {} }: { items: RailItem[]; value?: string; label?: string; onPick?: (key: string) => void } = $props();
</script>

<nav class="rail" aria-label={label}>
  <p class="eyebrow px-2.5 pb-2">{label}</p>
  {#each items as item, index (item.key)}
    {#if index === 1}<div class="my-1 border-t-2 border-base-300"></div>{/if}
    <button type="button" aria-pressed={value === item.key} aria-label="{item.label} {item.count} {item.count === 1 ? 'topic' : 'topics'}" onclick={() => { value = item.key; onPick(item.key); }}>
      {#if item.key === "all"}<Layers class="size-4 shrink-0" aria-hidden="true" />{:else if item.key === "none"}<FolderOpen class="size-4 shrink-0" aria-hidden="true" />{:else}<Folder class="size-4 shrink-0" aria-hidden="true" />{/if}
      <span class="truncate">{item.label}</span>{" "}<span class="n">{item.count}</span>
    </button>
  {/each}
</nav>

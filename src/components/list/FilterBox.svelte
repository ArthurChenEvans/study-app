<script lang="ts">
  // lists.feature (R139): filters as you type, says "9 of 171", and Esc clears it. `/` jumps
  // here from anywhere on the screen: App's shortcut looks for `data-list-filter` first.
  import { Search } from "@lucide/svelte";

  let { value = $bindable(""), shown, total, placeholder = "Filter…" }: { value?: string; shown: number; total: number; placeholder?: string } = $props();
</script>

<span class="flex items-center gap-3">
  <label class="input input-sm flex w-80 items-center gap-2">
    <Search class="size-4 text-muted" aria-hidden="true" />
    <input data-list-filter type="search" class="grow" aria-label="Filter" {placeholder} bind:value onkeydown={(event) => { if (event.key === "Escape" && value) { event.preventDefault(); event.stopPropagation(); value = ""; } }} />
    {#if !value}<kbd class="kbd kbd-xs" aria-hidden="true">/</kbd>{/if}
  </label>
  <span class="text-sm font-bold tabular-nums text-muted" aria-live="polite">{shown === total ? total : `${shown} of ${total}`}<span class="sr-only"> shown</span></span>
</span>

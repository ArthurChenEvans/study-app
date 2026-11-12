<script lang="ts" module>
  export type Sort = { key: string; up: boolean };
</script>

<script lang="ts">
  // lists.feature (R141): click to sort, again to flip it. aria-sort says which way.
  import { ArrowDown, ArrowUp, ArrowUpDown } from "@lucide/svelte";

  let { key, label, sort = $bindable(), numeric = false }: { key: string; label: string; sort: Sort; numeric?: boolean } = $props();
  const on = $derived(sort.key === key);
</script>

<th class={numeric ? "num" : ""} aria-sort={on ? (sort.up ? "ascending" : "descending") : "none"}>
  <button type="button" onclick={() => (sort = { key, up: on ? !sort.up : true })}>
    {label}
    {#if on}{#if sort.up}<ArrowUp class="size-3.5" aria-hidden="true" />{:else}<ArrowDown class="size-3.5" aria-hidden="true" />{/if}{:else}<ArrowUpDown class="size-3.5 opacity-50" aria-hidden="true" />{/if}
  </button>
</th>

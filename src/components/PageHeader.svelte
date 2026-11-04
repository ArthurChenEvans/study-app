<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, subtitle = "", children }: { title: string; subtitle?: string; children?: Snippet } = $props();
</script>

<header class="flex items-end justify-between gap-6 pb-8 pt-12">
  <div>
    <h1 class="text-3xl font-semibold leading-tight">{title}</h1>
    {#if subtitle}<p class="mt-2 text-muted">{subtitle}</p>{/if}
  </div>
  <div class="flex items-center gap-2">{@render children?.()}</div>
</header>

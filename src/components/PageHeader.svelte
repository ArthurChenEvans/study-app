<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, subtitle = "", children }: { title: string; subtitle?: string; children?: Snippet } = $props();
</script>

<header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-4 pb-6 pt-10">
  <div class="min-w-0">
    <h1 class="text-[28px] leading-tight">{title}</h1>
    {#if subtitle}<p class="mt-1 text-muted">{subtitle}</p>{/if}
  </div>
  <div class="page-actions flex flex-wrap items-center gap-2">{@render children?.()}</div>
</header>

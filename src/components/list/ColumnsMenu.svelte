<script lang="ts">
  // lists.feature (R141): show or hide the optional columns.
  import { Columns3 } from "@lucide/svelte";

  let { options, value = $bindable() }: { options: { key: string; label: string }[]; value: Record<string, boolean> } = $props();
  let open = $state(false);
  let box = $state<HTMLDivElement>();
</script>

<svelte:window onclick={(event) => { if (open && box && !box.contains(event.target as Node)) open = false; }} onkeydown={(event) => { if (open && event.key === "Escape") open = false; }} />

<div class="relative" bind:this={box}>
  <button type="button" class="btn btn-ghost btn-sm" aria-expanded={open} onclick={() => (open = !open)}><Columns3 class="size-4" />Columns</button>
  {#if open}
    <div class="row-menu w-52" role="group" aria-label="Columns">
      {#each options as option (option.key)}
        <label class="flex min-h-9 cursor-pointer items-center gap-3 rounded-lg px-2.5 text-sm font-bold hover:bg-base-300"><input type="checkbox" class="checkbox checkbox-sm" checked={value[option.key]} onchange={(event) => (value = { ...value, [option.key]: event.currentTarget.checked })} />{option.label}</label>
      {/each}
    </div>
  {/if}
</div>

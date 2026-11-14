<script lang="ts">
  // R199: the one menu every list's right-click and long-press opens. On a phone it rises as
  // a sheet from the bottom, elsewhere it sits at the pointer.
  import { tick } from "svelte";
  import { closeContextMenu, contextMenu } from "../lib/contextMenu.svelte";
  import type { MenuItem } from "./RowMenu.svelte";

  let menu = $state<HTMLDivElement>();
  let left = $state(0);
  let top = $state(0);
  const safe = $derived(contextMenu.items.filter((item) => !item.danger));
  const danger = $derived(contextMenu.items.filter((item) => item.danger));

  $effect(() => {
    if (!contextMenu.open) return;
    left = contextMenu.x;
    top = contextMenu.y;
    void tick().then(() => {
      if (!menu) return;
      // Kept inside the window.
      const box = menu.getBoundingClientRect();
      if (box.width && left + box.width > window.innerWidth - 8) left = Math.max(8, window.innerWidth - box.width - 8);
      if (box.height && top + box.height > window.innerHeight - 8) top = Math.max(8, top - box.height);
      menu.querySelector<HTMLElement>("[role=menuitem]")?.focus();
    });
  });

  function choose(item: MenuItem) {
    closeContextMenu(false);
    item.onSelect();
  }
  function keys(event: KeyboardEvent) {
    const entries = [...(menu?.querySelectorAll<HTMLElement>("[role=menuitem]") ?? [])];
    const at = entries.indexOf(document.activeElement as HTMLElement);
    if (event.key === "Escape") { event.preventDefault(); closeContextMenu(); }
    else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      entries[(at + (event.key === "ArrowDown" ? 1 : -1) + entries.length) % entries.length]?.focus();
    } else if (event.key === "Tab") closeContextMenu(false);
  }
</script>

{#if contextMenu.open}
  <button type="button" class="ctx-scrim" aria-label="Close menu" tabindex="-1" onclick={() => closeContextMenu(false)} oncontextmenu={(event) => { event.preventDefault(); closeContextMenu(false); }}></button>
  <div bind:this={menu} role="menu" aria-label={contextMenu.label} tabindex="-1" class="ctx-menu row-menu" style="left: {left}px; top: {top}px" onkeydown={keys}>
    <p class="ctx-title" aria-hidden="true">{contextMenu.label}</p>
    {#each safe as item (item.label)}
      <button type="button" role="menuitem" tabindex="-1" onclick={() => choose(item)}>{item.label}</button>
    {/each}
    {#if danger.length}
      <div role="separator" class="my-1 h-px bg-base-300"></div>
      {#each danger as item (item.label)}
        <button type="button" role="menuitem" tabindex="-1" class="text-error" onclick={() => choose(item)}>{item.label}</button>
      {/each}
    {/if}
  </div>
{/if}

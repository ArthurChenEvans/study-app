<script lang="ts">
  // A "…" button that opens a short menu of actions for one row. Destructive items
  // go last, after a divider, and only ever open a confirmation dialog.
  import { Ellipsis } from "@lucide/svelte";
  import { tick } from "svelte";

  export type MenuItem = { label: string; onSelect: () => void; danger?: boolean };

  // `text` shows a visible word beside the dots, for menus that would otherwise be easy to miss.
  let { label, items, small = false, text = "" }: { label: string; items: MenuItem[]; small?: boolean; text?: string } = $props();

  let open = $state(false);
  let button = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();

  const safe = $derived(items.filter((item) => !item.danger));
  const danger = $derived(items.filter((item) => item.danger));

  async function show() {
    open = true;
    await tick();
    menu?.querySelector<HTMLButtonElement>("[role=menuitem]")?.focus();
  }

  function close(returnFocus = true) {
    open = false;
    if (returnFocus) button?.focus();
  }

  function choose(item: MenuItem) {
    close(false);
    item.onSelect();
  }

  function keys(event: KeyboardEvent) {
    const entries = [...(menu?.querySelectorAll<HTMLButtonElement>("[role=menuitem]") ?? [])];
    const at = entries.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === "Escape") {
      event.preventDefault();
      close();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      entries[(at + step + entries.length) % entries.length]?.focus();
    } else if (event.key === "Tab") {
      close(false);
    }
  }

  function outside(event: MouseEvent) {
    if (open && !menu?.contains(event.target as Node) && !button?.contains(event.target as Node)) close(false);
  }
</script>

<svelte:window onclick={outside} />

<span class="relative inline-flex">
  <button
    bind:this={button}
    type="button"
    class="btn btn-ghost {text ? 'btn-outline' : 'btn-square'} {small ? 'btn-sm' : ''}"
    aria-label={label}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => (open ? close() : show())}><Ellipsis class="size-4" />{text}</button
  >
  {#if open}
    <div bind:this={menu} role="menu" aria-label={label} tabindex="-1" class="row-menu" onkeydown={keys}>
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
</span>

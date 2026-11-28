<script lang="ts">
  // A text field with suggestions underneath, in the app's style instead of the system
  // datalist. You can type anything; the list only offers past or common answers
  // (WAI-ARIA combobox with list autocomplete). Arrows move, Enter picks, Esc closes.
  import { tick } from "svelte";

  let {
    value = $bindable(""),
    suggestions,
    id,
    placeholder = "",
    class: className = "",
    autofocus = false,
    "aria-label": ariaLabel,
    onpick,
    onblur,
  }: { value?: string; suggestions: string[]; id?: string; placeholder?: string; class?: string; autofocus?: boolean; "aria-label"?: string; onpick?: (value: string) => void; onblur?: () => void } = $props();

  const uid = $props.id();
  let open = $state(false);
  let active = $state(-1);
  let input = $state<HTMLInputElement>();

  const shown = $derived.by(() => {
    const typed = value.trim().toLowerCase();
    const unique = [...new Set(suggestions)];
    return (typed ? unique.filter((item) => item.toLowerCase().includes(typed) && item.toLowerCase() !== typed) : unique).slice(0, 8);
  });

  function choose(item: string) {
    value = item;
    open = false;
    active = -1;
    onpick?.(item);
    input?.focus();
  }
  async function keys(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) { open = true; active = -1; }
      if (!shown.length) return;
      active = event.key === "ArrowDown" ? (active + 1) % shown.length : (active - 1 + shown.length) % shown.length;
      await tick();
      document.getElementById(`${uid}-opt-${active}`)?.scrollIntoView?.({ block: "nearest" });
    } else if (event.key === "Enter" && open && active >= 0 && shown[active]) {
      event.preventDefault();
      choose(shown[active]);
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation();
      open = false;
    }
  }
</script>

<div class="relative">
  <!-- svelte-ignore a11y_autofocus -->
  <input
    bind:this={input}
    {id}
    class="input w-full {className}"
    role="combobox"
    aria-label={ariaLabel}
    aria-autocomplete="list"
    aria-expanded={open && shown.length > 0}
    aria-controls="{uid}-list"
    aria-activedescendant={open && active >= 0 ? `${uid}-opt-${active}` : undefined}
    autocomplete="off"
    {placeholder}
    {autofocus}
    bind:value
    oninput={() => { open = true; active = -1; }}
    onfocus={() => (open = true)}
    onblur={() => { open = false; onblur?.(); }}
    onkeydown={keys}
  />
  {#if open && shown.length}
    <div id="{uid}-list" role="listbox" class="app-select-list suggest-list" aria-label="Suggestions">
      {#each shown as item, index (item)}
        <!-- svelte-ignore a11y_click_events_have_key_events (keys are handled on the field) -->
        <div id="{uid}-opt-{index}" role="option" tabindex="-1" aria-selected={index === active} class="app-select-option" class:app-select-active={index === active} onmousedown={(event) => event.preventDefault()} onclick={() => choose(item)}><span class="truncate">{item}</span></div>
      {/each}
    </div>
  {/if}
</div>

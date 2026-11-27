<script lang="ts" module>
  export type SelectValue = string | number | null;
  export type SelectOption = { value: SelectValue; label: string; disabled?: boolean; group?: string };
</script>

<script lang="ts">
  // The app's own dropdown, styled like the rest of the app instead of the system one.
  // A select-only combobox (WAI-ARIA APG): focus stays on the button, the highlighted
  // option is announced through aria-activedescendant. The list opens in the top layer
  // (popover) so a panel or dialog never clips it.
  import { ChevronDown, Check } from "@lucide/svelte";
  import { tick } from "svelte";

  let {
    value = $bindable(),
    options,
    onchange,
    id,
    class: className = "",
    small = false,
    placeholder = "Pick one",
    "aria-label": ariaLabel,
    "aria-describedby": ariaDescribedby,
  }: {
    value?: SelectValue;
    options: SelectOption[];
    onchange?: (value: SelectValue) => void;
    id?: string;
    class?: string;
    small?: boolean;
    placeholder?: string;
    "aria-label"?: string;
    "aria-describedby"?: string;
  } = $props();

  const uid = $props.id();
  // The top layer keeps the list above dialogs and outside clipped panels, where supported.
  const topLayer = typeof HTMLElement !== "undefined" && "showPopover" in HTMLElement.prototype;
  const listId = `${uid}-list`;
  const optionId = (index: number) => `${uid}-opt-${index}`;

  let open = $state(false);
  let active = $state(-1);
  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLDivElement>();
  let place = $state({ left: 0, top: 0, width: 0, maxHeight: 320, up: false });

  const selectedIndex = $derived(options.findIndex((option) => option.value === value));
  const selected = $derived(selectedIndex >= 0 ? options[selectedIndex] : null);
  // Options in their groups, keeping each option's index into `options`.
  const groups = $derived.by(() => {
    const out: { name: string | undefined; items: { option: SelectOption; index: number }[] }[] = [];
    options.forEach((option, index) => {
      const last = out[out.length - 1];
      if (last && last.name === option.group) last.items.push({ option, index });
      else out.push({ name: option.group, items: [{ option, index }] });
    });
    return out;
  });

  const enabled = (index: number) => index >= 0 && index < options.length && !options[index].disabled;
  function step(from: number, by: 1 | -1) {
    for (let index = from + by; index >= 0 && index < options.length; index += by) if (enabled(index)) return index;
    return from;
  }
  const first = () => step(-1, 1);
  const last = () => step(options.length, -1);

  function position() {
    if (!button) return;
    const rect = button.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 12;
    const above = rect.top - 12;
    const up = below < 200 && above > below;
    place = { left: rect.left, top: up ? rect.top - 4 : rect.bottom + 4, width: rect.width, maxHeight: Math.max(120, Math.min(320, up ? above : below)), up };
  }

  async function show(at = selectedIndex >= 0 ? selectedIndex : first()) {
    if (open) return;
    position();
    active = at;
    open = true;
    await tick();
    try { (list as HTMLElement & { showPopover?: () => void })?.showPopover?.(); } catch { /* not supported: shown inline */ }
    scrollToActive();
  }

  function hide() {
    if (!open) return;
    try { (list as HTMLElement & { hidePopover?: () => void })?.hidePopover?.(); } catch { /* already hidden */ }
    open = false;
  }

  function choose(index: number) {
    if (!enabled(index)) return;
    hide();
    button?.focus();
    const next = options[index].value;
    if (next === value) return;
    value = next;
    onchange?.(next);
  }

  async function scrollToActive() {
    await tick();
    list?.querySelector<HTMLElement>(`#${CSS.escape(optionId(active))}`)?.scrollIntoView?.({ block: "nearest" });
  }

  // Type a few letters to jump to the option that starts with them.
  let typed = "";
  let typedAt = 0;
  function typeahead(key: string) {
    const now = Date.now();
    typed = now - typedAt > 600 ? key : typed + key;
    typedAt = now;
    const from = open ? active : selectedIndex;
    const order = [...options.keys()].map((offset) => (from + 1 + offset) % options.length);
    const repeat = typed.length > 1 && [...typed].every((char) => char === typed[0]);
    const match = order.find((index) => enabled(index) && options[index].label.toLowerCase().startsWith(repeat ? typed[0] : typed.toLowerCase()));
    if (match === undefined) return;
    if (open) { active = match; scrollToActive(); }
    else choose(match);
  }

  function keys(event: KeyboardEvent) {
    const { key, altKey } = event;
    if (!open) {
      if (key === "ArrowDown" || key === "ArrowUp" || key === "Enter" || key === " ") {
        event.preventDefault();
        show(altKey || key === "Enter" || key === " " || selectedIndex < 0 ? undefined : step(selectedIndex, key === "ArrowDown" ? 1 : -1));
      } else if (key === "Home" || key === "End") {
        event.preventDefault();
        show(key === "Home" ? first() : last());
      } else if (key.length === 1 && !event.ctrlKey && !event.metaKey && !altKey) typeahead(key);
      return;
    }
    if (key === "ArrowDown" || key === "ArrowUp") {
      event.preventDefault();
      if (altKey && key === "ArrowUp") return choose(active);
      active = step(active, key === "ArrowDown" ? 1 : -1);
      scrollToActive();
    } else if (key === "Home" || key === "End" || key === "PageUp" || key === "PageDown") {
      event.preventDefault();
      active = key === "Home" ? first() : key === "End" ? last() : [...Array(10)].reduce((at) => step(at, key === "PageDown" ? 1 : -1), active);
      scrollToActive();
    } else if (key === "Enter" || key === " ") {
      event.preventDefault();
      choose(active);
    } else if (key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      hide();
    } else if (key === "Tab") {
      if (enabled(active) && options[active].value !== value) choose(active);
      hide();
    } else if (key.length === 1 && !event.ctrlKey && !event.metaKey && !altKey) typeahead(key);
  }

  /** Focuses the dropdown's button, as `select.focus()` would. */
  export function focus() { button?.focus(); }

  function outside(event: PointerEvent) {
    if (open && !list?.contains(event.target as Node) && !button?.contains(event.target as Node)) hide();
  }
  function moved(event: Event) {
    if (open && !list?.contains(event.target as Node)) position();
  }
</script>

<svelte:window onpointerdown={outside} onresize={moved} onscrollcapture={moved} />

<button
  bind:this={button}
  {id}
  type="button"
  role="combobox"
  class="app-select select {small ? 'select-sm' : ''} {className}"
  aria-label={ariaLabel}
  aria-describedby={ariaDescribedby}
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-controls={listId}
  aria-activedescendant={open && active >= 0 ? optionId(active) : undefined}
  onclick={() => (open ? hide() : show())}
  onkeydown={keys}
  onblur={(event) => { if (!list?.contains(event.relatedTarget as Node)) hide(); }}
><span class="truncate {selected ? '' : 'text-muted'}">{selected?.label ?? placeholder}</span><ChevronDown class="app-select-chevron size-4 shrink-0" aria-hidden="true" /></button>

{#if open}
  <div
    bind:this={list}
    id={listId}
    role="listbox"
    tabindex="-1"
    popover={topLayer ? "manual" : undefined}
    aria-label={ariaLabel}
    class="app-select-list"
    class:app-select-up={place.up}
    style="left: {place.left}px; top: {place.top}px; min-width: {place.width}px; max-height: {place.maxHeight}px;"
  >
    {#each groups as group, groupIndex (groupIndex)}
      {#if group.name}
        <div role="group" aria-labelledby="{uid}-g{groupIndex}">
          <div id="{uid}-g{groupIndex}" class="app-select-group" role="presentation">{group.name}</div>
          {#each group.items as { option, index } (index)}{@render row(option, index)}{/each}
        </div>
      {:else}
        {#each group.items as { option, index } (index)}{@render row(option, index)}{/each}
      {/if}
    {/each}
  </div>
{/if}

{#snippet row(option: SelectOption, index: number)}
  <!-- svelte-ignore a11y_click_events_have_key_events (keys are handled on the combobox) -->
  <div
    id={optionId(index)}
    role="option"
    tabindex="-1"
    aria-selected={index === selectedIndex}
    aria-disabled={option.disabled || undefined}
    class="app-select-option"
    class:app-select-active={index === active}
    onpointermove={() => { if (enabled(index)) active = index; }}
    onmousedown={(event) => event.preventDefault()}
    onclick={() => choose(index)}
  ><span class="truncate">{option.label}</span>{#if index === selectedIndex}<Check class="size-4 shrink-0" aria-hidden="true" />{/if}</div>
{/snippet}

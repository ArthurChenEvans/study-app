<script lang="ts">
  // The app's own date picker, in place of the system one. A button shows the date; it
  // opens a month grid (WAI-ARIA date picker dialog) in the top layer. Arrows move a day or
  // a week, Page Up and Page Down a month, Home and End the week, Enter picks, Esc closes.
  // Works in ISO dates ("2026-12-15"), like the input it replaces.
  import { CalendarDays, ChevronLeft, ChevronRight } from "@lucide/svelte";
  import { tick } from "svelte";

  let {
    value = $bindable(""),
    min = "",
    id,
    small = false,
    class: className = "",
    "aria-label": ariaLabel,
    onfocus,
  }: { value?: string; min?: string; id?: string; small?: boolean; class?: string; "aria-label"?: string; onfocus?: () => void } = $props();

  const uid = $props.id();
  const topLayer = typeof HTMLElement !== "undefined" && "showPopover" in HTMLElement.prototype;
  const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  const DAYS = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
  const DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

  // Dates as UTC midnights so arithmetic never trips over clock changes.
  const parse = (iso: string) => { const [y, m, d] = iso.split("-").map(Number); return y ? Date.UTC(y, m - 1, d) : NaN; };
  const iso = (time: number) => new Date(time).toISOString().slice(0, 10);
  const DAY = 86_400_000;
  const todayIso = () => { const now = new Date(); return iso(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate())); };
  const label = (time: number) => { const d = new Date(time); return `${DAY_NAMES[(d.getUTCDay() + 6) % 7]} ${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}`; };
  const short = (time: number) => { const d = new Date(time); return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()].slice(0, 3)} ${d.getUTCFullYear()}`; };

  let open = $state(false);
  let focused = $state(0);
  let button = $state<HTMLButtonElement>();
  let panel = $state<HTMLDivElement>();
  let place = $state({ left: 0, top: 0, up: false });

  const minTime = $derived(min ? parse(min) : -Infinity);
  const chosen = $derived(value ? parse(value) : NaN);
  // The six weeks shown for the focused day's month, starting on a Monday.
  const weeks = $derived.by(() => {
    const d = new Date(focused);
    const first = Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), 1);
    const start = first - ((new Date(first).getUTCDay() + 6) % 7) * DAY;
    return Array.from({ length: 6 }, (_, w) => Array.from({ length: 7 }, (_, i) => start + (w * 7 + i) * DAY));
  });
  const month = $derived(new Date(focused).getUTCMonth());
  const heading = $derived(`${MONTHS[month]} ${new Date(focused).getUTCFullYear()}`);

  function addMonths(time: number, by: number) {
    const d = new Date(time);
    const target = Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + by, 1);
    const last = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + by + 1, 0)).getUTCDate();
    return target + (Math.min(d.getUTCDate(), last) - 1) * DAY;
  }

  async function show() {
    onfocus?.();
    if (!button) return;
    const rect = button.getBoundingClientRect();
    const up = window.innerHeight - rect.bottom < 360 && rect.top > 360;
    place = { left: Math.min(rect.left, window.innerWidth - 300), top: up ? rect.top - 4 : rect.bottom + 4, up };
    focused = Number.isNaN(chosen) ? Math.max(parse(todayIso()), minTime) : chosen;
    open = true;
    await tick();
    try { (panel as HTMLElement & { showPopover?: () => void })?.showPopover?.(); } catch { /* shown inline */ }
    focusDay();
  }
  function hide(refocus = true) {
    if (!open) return;
    try { (panel as HTMLElement & { hidePopover?: () => void })?.hidePopover?.(); } catch { /* already hidden */ }
    open = false;
    if (refocus) button?.focus();
  }
  async function focusDay() {
    await tick();
    panel?.querySelector<HTMLElement>(`[data-day="${iso(focused)}"]`)?.focus();
  }
  function pick(time: number) {
    if (time < minTime) return;
    value = iso(time);
    hide();
  }
  function move(to: number) {
    focused = Math.max(to, minTime);
    focusDay();
  }
  function keys(event: KeyboardEvent) {
    const weekday = (new Date(focused).getUTCDay() + 6) % 7;
    const moves: Record<string, () => number> = {
      ArrowLeft: () => focused - DAY,
      ArrowRight: () => focused + DAY,
      ArrowUp: () => focused - 7 * DAY,
      ArrowDown: () => focused + 7 * DAY,
      Home: () => focused - weekday * DAY,
      End: () => focused + (6 - weekday) * DAY,
      PageUp: () => addMonths(focused, event.shiftKey ? -12 : -1),
      PageDown: () => addMonths(focused, event.shiftKey ? 12 : 1),
    };
    if (moves[event.key] && (event.target as HTMLElement).dataset.day) {
      event.preventDefault();
      move(moves[event.key]());
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      hide();
    } else if (event.key === "Tab") {
      // Keep focus inside while open: the month buttons, then the grid.
      const stops = [...(panel?.querySelectorAll<HTMLElement>("button.date-nav, [data-day][tabindex='0']") ?? [])];
      const at = stops.indexOf(event.target as HTMLElement);
      event.preventDefault();
      stops[(at + (event.shiftKey ? -1 : 1) + stops.length) % stops.length]?.focus();
    }
  }
  function outside(event: PointerEvent) {
    if (open && !panel?.contains(event.target as Node) && !button?.contains(event.target as Node)) hide(false);
  }
</script>

<svelte:window onpointerdown={outside} onresize={() => hide(false)} />

<button
  bind:this={button}
  {id}
  type="button"
  class="app-select input {small ? 'input-sm' : ''} {className}"
  aria-label={ariaLabel ? `${ariaLabel}${value ? `, ${label(chosen)}` : ""}` : undefined}
  aria-haspopup="dialog"
  aria-expanded={open}
  onclick={() => (open ? hide() : show())}
  onfocus={onfocus}
><span class="truncate {value ? '' : 'text-muted'}">{value ? short(chosen) : "Pick a day"}</span><CalendarDays class="app-select-chevron size-4 shrink-0" aria-hidden="true" /></button>

{#if open}
  <div
    bind:this={panel}
    role="dialog"
    aria-modal="true"
    aria-labelledby="{uid}-month"
    popover={topLayer ? "manual" : undefined}
    class="app-select-list date-panel"
    class:app-select-up={place.up}
    style="left: {place.left}px; top: {place.top}px;"
    tabindex="-1"
    onkeydown={keys}
  >
    <div class="flex items-center justify-between gap-2 pb-2">
      <button type="button" class="date-nav btn btn-ghost btn-sm btn-square" aria-label="Previous month" onclick={() => move(addMonths(focused, -1))}><ChevronLeft class="size-4" /></button>
      <p id="{uid}-month" class="text-[15px] font-extrabold" aria-live="polite">{heading}</p>
      <button type="button" class="date-nav btn btn-ghost btn-sm btn-square" aria-label="Next month" onclick={() => move(addMonths(focused, 1))}><ChevronRight class="size-4" /></button>
    </div>
    <table role="grid" aria-labelledby="{uid}-month" class="date-grid">
      <thead><tr>{#each DAYS as day, index (day)}<th scope="col" abbr={DAY_NAMES[index]}>{day}</th>{/each}</tr></thead>
      <tbody>
        {#each weeks as week, w (w)}
          <tr>
            {#each week as time (time)}
              {@const outsideMonth = new Date(time).getUTCMonth() !== month}
              {@const disabled = time < minTime}
              <td aria-selected={time === chosen}>
                <button
                  type="button"
                  data-day={iso(time)}
                  tabindex={time === focused ? 0 : -1}
                  class="date-day"
                  class:date-outside={outsideMonth}
                  class:date-today={iso(time) === todayIso()}
                  aria-label={label(time)}
                  aria-current={iso(time) === todayIso() ? "date" : undefined}
                  aria-disabled={disabled || undefined}
                  onclick={() => pick(time)}
                  onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); pick(time); } }}
                >{new Date(time).getUTCDate()}</button>
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

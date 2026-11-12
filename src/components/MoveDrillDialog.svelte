<script lang="ts">
  // Moving a drill (R99): the likely days are one press, any other day is a date box.
  // Matches "When do you want to do it?" in session review.
  import { untrack } from "svelte";
  import { dateOf, dayFromIso, fullLabel, isoOf } from "../lib/dates";

  let { task, today, current, error = "", onMove, onCancel }: {
    task: string; today: number; current: number | null; error?: string; onMove: (day: number) => void; onCancel: () => void;
  } = $props();

  /** The first day after `from` that falls on `weekday` (0 is Monday). */
  const nextWeekday = (from: number, weekday: number) => from + 1 + ((weekday - dateOf(from + 1).weekday + 7) % 7);

  const picks = $derived.by(() => {
    const tomorrow = today + 1;
    const saturday = nextWeekday(today, 5);
    const monday = nextWeekday(today, 0);
    const list = [{ day: tomorrow, label: `Tomorrow, ${fullLabel(tomorrow, today)}` }];
    if (saturday !== tomorrow) list.push({ day: saturday, label: `Saturday, ${fullLabel(saturday, today).replace(/^\w+ /, "")}` });
    if (monday !== tomorrow) list.push({ day: monday, label: `Monday, ${fullLabel(monday, today).replace(/^\w+ /, "")}` });
    return list.sort((a, b) => a.day - b.day);
  });

  // Start on the first quick pick that isn't where the drill already is.
  let chosen = $state<number | "other">(untrack(() => picks.find((pick) => pick.day !== current)?.day ?? "other"));
  let other = $state(untrack(() => isoOf(today + 2)));
  const day = $derived(chosen === "other" ? (other ? dayFromIso(other) : null) : chosen);

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (day !== null) onMove(day);
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="move-h">
  <form class="modal-box max-w-md" onsubmit={submit}>
    <h3 id="move-h" class="text-lg font-semibold">Move this drill</h3>
    <p class="mt-2 text-sm text-muted">{task}</p>
    <fieldset class="mt-5 space-y-2">
      <legend class="mb-2 text-sm font-semibold">Do it on</legend>
      {#each picks as pick (pick.day)}
        <label class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg border px-4 py-2 {chosen === pick.day ? 'border-2 border-primary-ink bg-base-200' : 'border-field-border'}">
          <input type="radio" name="move-day" class="radio radio-sm" checked={chosen === pick.day} onchange={() => (chosen = pick.day)} />
          <span class="flex-1">{pick.label}</span>
          {#if pick.day === current}<span class="text-xs text-muted">where it is now</span>{/if}
        </label>
      {/each}
      <label class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg border px-4 py-2 {chosen === 'other' ? 'border-2 border-primary-ink bg-base-200' : 'border-field-border'}">
        <input type="radio" name="move-day" class="radio radio-sm" checked={chosen === "other"} onchange={() => (chosen = "other")} />
        <span>Another day</span>
        <input type="date" class="input input-sm ml-auto w-40" aria-label="Pick a day" min={isoOf(today)} bind:value={other} onfocus={() => (chosen = "other")} />
      </label>
    </fieldset>
    {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      <button type="button" class="btn btn-ghost" onclick={onCancel}>Cancel</button>
      <button type="submit" class="btn btn-primary" disabled={day === null}>Move to {day === null ? "that day" : fullLabel(day, today)}</button>
    </div>
  </form>
</dialog>

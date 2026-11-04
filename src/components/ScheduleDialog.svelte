<script lang="ts">
  // An editable ladder (R60, R96): the recall schedule and the quiz schedule use the same one.
  import { Plus, RotateCcw, X } from "@lucide/svelte";
  import { untrack } from "svelte";
  import { errorMessage, type Schedule, type Unit } from "../lib/api";

  let {
    title, intro, passWhat, applies, schedule, onSave, onReset, onClose,
  }: {
    title: string; intro: string; passWhat: string; applies: string; schedule: Schedule;
    onSave: (schedule: Schedule) => Promise<void>; onReset: () => Promise<Schedule>; onClose: () => void;
  } = $props();

  let editing = $state<Schedule>(untrack(() => structuredClone($state.snapshot(schedule)))); // edited as a copy, saved on submit
  let error = $state("");
  const units: Unit[] = ["Days", "Weeks", "Months", "Years"];

  async function save(event: SubmitEvent) {
    event.preventDefault();
    try {
      await onSave($state.snapshot(editing));
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function reset() {
    editing = await onReset();
    error = "";
  }

  // What the steps add up to if you pass every time, so the numbers mean something.
  const DAYS: Record<Unit, number> = { Days: 1, Weeks: 7, Months: 30.44, Years: 365.25 };
  const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? "" : "s"}`;
  function span(days: number): string {
    if (days < 14) return plural(Math.round(days), "day");
    if (days < 60) return plural(Math.round(days / 7), "week");
    if (days < 365) return plural(Math.round(days / 30.44), "month");
    const years = Math.floor(days / 365.25);
    const months = Math.round((days - years * 365.25) / 30.44);
    return months ? `${plural(years, "year")} ${plural(months, "month")}` : plural(years, "year");
  }
  const reached = $derived.by(() => {
    let total = 0;
    return editing.steps.map((step, index) => {
      const every = (Number(step.every.count) || 0) * DAYS[step.every.unit];
      if (index === editing.steps.length - 1 && step.repeats === 0) {
        const unit = step.every.unit.toLowerCase().replace(/s$/, "");
        return Number(step.every.count) === 1 ? `Then every ${unit}` : `Then every ${step.every.count} ${unit}s`;
      }
      total += every * (Number(step.repeats) || 0);
      return span(total);
    });
  });

  function addStep() {
    const last = editing.steps.length - 1;
    editing.steps.splice(Math.max(0, last), 0, { name: "New step", every: { count: 1, unit: "Months" }, repeats: 1 });
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="schedule-h">
  <form class="modal-box max-w-3xl" onsubmit={save}>
    <h3 id="schedule-h" class="text-lg font-semibold">{title}</h3>
    <p class="mt-2 text-sm text-muted">{intro}</p>
    <table class="table mt-6">
      <thead><tr class="text-muted"><th>Step</th><th>Every</th><th>Times</th><th>If you pass every time</th><th><span class="sr-only">Remove</span></th></tr></thead>
      <tbody>
        {#each editing.steps as step, index (index)}
          {@const last = index === editing.steps.length - 1}
          <tr>
            <td><input class="step-name input input-sm w-36 font-semibold" aria-label="Step {index + 1} name" bind:value={step.name} /></td>
            <td><div class="flex gap-2">
              <input class="input input-sm w-16 tabular-nums" type="number" min="1" aria-label="{step.name} interval" bind:value={step.every.count} />
              <select class="select select-sm w-28" aria-label="{step.name} unit" bind:value={step.every.unit}>{#each units as unit (unit)}<option value={unit}>{unit.toLowerCase()}</option>{/each}</select>
            </div></td>
            <td>{#if last && step.repeats === 0}<span class="whitespace-nowrap text-sm text-muted">From then on</span>{:else}<input class="input input-sm w-16 tabular-nums" type="number" min="1" aria-label="{step.name} times" bind:value={step.repeats} />{/if}</td>
            <td class="whitespace-nowrap text-sm text-muted">{reached[index]}</td>
            <td class="text-right">{#if !last}<button type="button" class="row-actions btn btn-ghost btn-sm btn-square" aria-label="Remove {step.name}" onclick={() => editing.steps.splice(index, 1)}><X class="size-4" /></button>{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <button type="button" class="btn btn-ghost btn-sm mt-2" onclick={addStep}><Plus class="size-4" />Add a step before {editing.steps.at(-1)?.name}</button>
    <div class="mt-6 grid grid-cols-2 gap-6">
      <label class="block"><span class="block text-sm font-semibold">Pass mark</span>
        <span class="mt-1 flex items-center gap-2"><input class="input w-24 tabular-nums" type="number" min="1" max="100" aria-label="Pass mark" bind:value={editing.pass_mark} /><span class="text-sm text-muted">% of {passWhat}</span></span></label>
      <label class="block"><span class="block text-sm font-semibold">Under the pass mark</span>
        <select class="select mt-1 w-full" bind:value={editing.under_pass}><option value="BackOneStep">Go back one step</option><option value="Stay">Stay on the same step</option></select></label>
    </div>
    <p class="mt-6 text-sm text-muted">{applies}</p>
    {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      <button type="button" class="btn btn-ghost mr-auto" onclick={reset}><RotateCcw class="size-4" />Reset to default</button>
      <button type="button" class="btn btn-ghost" onclick={onClose}>Cancel</button>
      <button type="submit" class="btn btn-primary">Save schedule</button>
    </div>
  </form>
</dialog>

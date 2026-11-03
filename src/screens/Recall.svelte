<script lang="ts">
  // free-recall.feature (R52, R58, R60): checklists due today first, then the rest by date.
  import { Brain, CalendarCog, CircleCheck, Clock, Play, Plus, RotateCcw, X } from "@lucide/svelte";
  import { api, errorMessage, type ModuleRow, type RecallRow, type Schedule, type TopicRow, type Unit } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import Ladder from "../components/Ladder.svelte";
  import { dueLabel, onDay } from "../lib/dates";

  let { onOpen, onStart, onChanged = () => {} }: { onOpen: (id: number) => void; onStart: (id: number) => void; onChanged?: () => void } = $props();

  let rows = $state<RecallRow[]>([]);
  let schedule = $state<Schedule | null>(null);
  let today = $state(0);
  let loaded = $state(false);

  let creating = $state(false);
  let modules = $state<ModuleRow[]>([]);
  let topics = $state<TopicRow[]>([]);
  let subject = $state("");
  let createError = $state("");

  let editing = $state<Schedule | null>(null);
  let scheduleError = $state("");

  const due = $derived(rows.filter((row) => row.due_today));
  const later = $derived(rows.filter((row) => !row.due_today));

  async function load() {
    [rows, schedule, today] = await Promise.all([api.recallList(), api.recallSchedule(), api.today()]);
    loaded = true;
  }

  async function openCreate() {
    [modules, topics] = await Promise.all([api.modules(), api.topics()]);
    subject = "";
    createError = "";
    creating = true;
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (!subject) {
      createError = "Pick a module or topic to recall.";
      return;
    }
    const [kind, id] = subject.split(":");
    try {
      const checklist = await api.createChecklist(kind === "module" ? { Module: Number(id) } : { Topic: Number(id) });
      creating = false;
      onChanged();
      onOpen(checklist);
    } catch (failure) {
      createError = errorMessage(failure);
    }
  }

  function openSchedule() {
    if (!schedule) return;
    editing = structuredClone($state.snapshot(schedule));
    scheduleError = "";
  }

  async function saveSchedule(event: SubmitEvent) {
    event.preventDefault();
    if (!editing) return;
    try {
      await api.setRecallSchedule($state.snapshot(editing));
      editing = null;
      await load();
    } catch (failure) {
      scheduleError = errorMessage(failure);
    }
  }

  async function resetSchedule() {
    editing = await api.resetRecallSchedule();
    scheduleError = "";
  }

  function addStep() {
    if (!editing) return;
    const last = editing.steps.length - 1;
    editing.steps.splice(Math.max(0, last), 0, { name: "New step", every: { count: 1, unit: "Months" }, repeats: 1 });
  }

  const units: Unit[] = ["Days", "Weeks", "Months", "Years"];
  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Recall" subtitle={loaded && rows.length ? (due.length ? `${due.length} due today` : "Nothing due today") : ""}>
    {#if rows.length}
      <button class="btn btn-ghost" onclick={openSchedule}><CalendarCog class="size-4" />Schedule</button>
      <button class="btn btn-ghost" onclick={openCreate}><Plus class="size-4" />New checklist</button>
    {/if}
  </PageHeader>

  {#if loaded && rows.length === 0}
    <div class="panel max-w-xl p-10">
      <span class="grid size-10 place-items-center rounded-lg bg-base-200"><Brain class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No checklists yet</h2>
      <p class="mt-2 max-w-prose text-muted">Free recall means writing or saying everything you remember about a module or topic, then ticking it off against a checklist. The app schedules the next one from how you did.</p>
      <button class="btn btn-primary mt-6" onclick={openCreate}><Plus class="size-4" />Create your first checklist</button>
    </div>
  {/if}

  {#if due.length && schedule}
    <section aria-labelledby="due-h">
      <h2 id="due-h" class="eyebrow mb-3">Due today</h2>
      <div class="grid grid-cols-2 gap-6">
        {#each due as row, index (row.id)}
          <article class="panel flex flex-col gap-6 p-6" aria-label={row.subject_name}>
            <div class="flex items-start justify-between gap-4">
              <div>
                <p class="text-sm text-muted">{row.module ? "Module" : "Topic"} · {row.points} {row.points === 1 ? "point" : "points"}</p>
                <h3 class="mt-1 text-xl font-semibold"><button class="text-left hover:underline" onclick={() => onOpen(row.id)}>{row.subject_name}</button></h3>
              </div>
              {#if row.days_late}<span class="flex items-center gap-1 whitespace-nowrap text-sm text-muted"><Clock class="size-3.5" />{row.days_late} {row.days_late === 1 ? "day" : "days"} late</span>{/if}
            </div>
            <div><Ladder steps={schedule.steps} position={row.position} step={row.step} next={row.next_step} /></div>
            <div class="flex items-center justify-between gap-4">
              <p class="text-sm text-muted">{#if row.last_score !== null}Last check <span class="font-semibold tabular-nums text-base-content">{row.last_score}%</span>{:else}First check{/if}</p>
              {#if row.points}
                <button class="btn {index === 0 ? 'btn-primary' : 'btn-outline border-field-border'}" onclick={() => onStart(row.id)}><Play class="size-4" />Start recall</button>
              {:else}
                <button class="btn btn-outline border-field-border" onclick={() => onOpen(row.id)}><Plus class="size-4" />Add points</button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    </section>
  {:else if loaded && rows.length}
    <div class="panel flex max-w-xl items-center gap-4 p-6">
      <CircleCheck class="size-5" />
      <div><p class="font-semibold">All caught up</p><p class="text-sm text-muted">Next up is {later[0].subject_name}, {onDay(later[0].due, today)}.</p></div>
    </div>
  {/if}

  {#if later.length && schedule}
    <section class="mt-12" aria-labelledby="later-h">
      <h2 id="later-h" class="eyebrow mb-3">{due.length ? "Coming up" : "All checklists"}</h2>
      <div class="panel overflow-hidden">
        <table class="table table-fixed" aria-label="Checklists">
          <colgroup><col class="w-[28%]" /><col class="w-[12%]" /><col class="w-[30%]" /><col class="w-[15%]" /><col class="w-[15%]" /></colgroup>
          <thead><tr class="text-muted"><th>Checklist</th><th>Points</th><th>Step</th><th>Last check</th><th>Next</th></tr></thead>
          <tbody>
            {#each later as row (row.id)}
              <tr class="hover:bg-base-100">
                <td><button class="py-0.5 text-left font-semibold hover:underline" onclick={() => onOpen(row.id)}>{row.subject_name}</button><div class="text-xs text-muted">{row.module ? "Module" : "Topic"}{row.subject_exists ? "" : " (deleted)"}</div></td>
                <td class="tabular-nums">{row.points}</td>
                <td><div class="flex items-center gap-3"><div class="w-28"><Ladder compact steps={schedule.steps} position={row.position} step={row.step} next={row.next_step} /></div><span class="text-sm">{row.step ?? "Not started"}</span></div></td>
                <td class="tabular-nums">{row.last_score === null ? "None yet" : `${row.last_score}%`}</td>
                <td class="tabular-nums">{dueLabel(row.due, today)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}
</div>

{#if creating}
  <dialog open class="modal modal-open" aria-labelledby="new-checklist-h">
    <form class="modal-box max-w-md" onsubmit={create}>
      <h3 id="new-checklist-h" class="text-lg font-semibold">New checklist</h3>
      <label class="mt-6 block">
        <span class="block text-sm font-semibold">Recall about</span>
        <!-- svelte-ignore a11y_autofocus -->
        <select class="select mt-1 w-full" bind:value={subject} autofocus>
          <option value="">Pick a module or topic…</option>
          {#if modules.length}<optgroup label="Modules">{#each modules as module (module.id)}<option value="module:{module.id}">{module.name}</option>{/each}</optgroup>{/if}
          {#if topics.length}<optgroup label="Topics">{#each topics as topic (topic.id)}<option value="topic:{topic.id}">{topic.name}</option>{/each}</optgroup>{/if}
        </select>
        <span class="mt-1 block text-sm text-muted">A module covers all its topics. Pick a topic for something narrower.</span>
      </label>
      {#if createError}<p role="alert" class="mt-4 text-sm text-error">{createError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create checklist</button>
      </div>
    </form>
  </dialog>
{/if}

{#if editing}
  <dialog open class="modal modal-open" aria-labelledby="schedule-h">
    <form class="modal-box max-w-2xl" onsubmit={saveSchedule}>
      <h3 id="schedule-h" class="text-lg font-semibold">Recall schedule</h3>
      <p class="mt-2 text-sm text-muted">Every checklist climbs these steps. Passing moves it up one, and the next date counts from the day you did the check.</p>
      <table class="table mt-6">
        <thead><tr class="text-muted"><th>Step</th><th>Every</th><th>Times</th><th><span class="sr-only">Remove</span></th></tr></thead>
        <tbody>
          {#each editing.steps as step, index (index)}
            {@const last = index === editing.steps.length - 1}
            <tr>
              <td><input class="input input-sm w-36" aria-label="Step {index + 1} name" bind:value={step.name} /></td>
              <td><div class="flex gap-2">
                <input class="input input-sm w-16 tabular-nums" type="number" min="1" aria-label="{step.name} interval" bind:value={step.every.count} />
                <select class="select select-sm w-28" aria-label="{step.name} unit" bind:value={step.every.unit}>{#each units as unit (unit)}<option value={unit}>{unit.toLowerCase()}</option>{/each}</select>
              </div></td>
              <td>{#if last && step.repeats === 0}<span class="text-sm text-muted">From then on</span>{:else}<input class="input input-sm w-16 tabular-nums" type="number" min="1" aria-label="{step.name} times" bind:value={step.repeats} />{/if}</td>
              <td class="text-right">{#if !last}<button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Remove {step.name}" onclick={() => editing?.steps.splice(index, 1)}><X class="size-4" /></button>{/if}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <button type="button" class="btn btn-ghost btn-sm mt-2" onclick={addStep}><Plus class="size-4" />Add a step before {editing.steps.at(-1)?.name}</button>
      <div class="mt-6 grid grid-cols-2 gap-6">
        <label class="block"><span class="block text-sm font-semibold">Pass mark</span>
          <span class="mt-1 flex items-center gap-2"><input class="input w-24 tabular-nums" type="number" min="1" max="100" aria-label="Pass mark" bind:value={editing.pass_mark} /><span class="text-sm text-muted">% of points recalled</span></span></label>
        <label class="block"><span class="block text-sm font-semibold">Under the pass mark</span>
          <select class="select mt-1 w-full" bind:value={editing.under_pass}><option value="BackOneStep">Go back one step</option><option value="Stay">Stay on the same step</option></select></label>
      </div>
      <p class="mt-6 text-sm text-muted">Changes apply from each checklist's next check.</p>
      {#if scheduleError}<p role="alert" class="mt-4 text-sm text-error">{scheduleError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost mr-auto" onclick={resetSchedule}><RotateCcw class="size-4" />Reset to default</button>
        <button type="button" class="btn btn-ghost" onclick={() => (editing = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Save schedule</button>
      </div>
    </form>
  </dialog>
{/if}

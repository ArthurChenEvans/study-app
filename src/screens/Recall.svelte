<script lang="ts">
  import { stackTable } from "../lib/stack";
  // free-recall.feature (R52, R58, R60): checklists due today first, then the rest by date.
  import { Brain, CalendarCog, CircleCheck, Clock, Play, Plus } from "@lucide/svelte";
  import { api, errorMessage, type ModuleRow, type RecallRow, type Schedule, type TopicRow } from "../lib/api";
  import { toast } from "../lib/celebrations.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Ladder from "../components/Ladder.svelte";
  import ScheduleDialog from "../components/ScheduleDialog.svelte";
  import RowMenu from "../components/RowMenu.svelte";
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
  let find = $state("");
  // A find box over a grouped list, not a dropdown of every module and topic.
  const matches = (name: string) => name.toLowerCase().includes(find.trim().toLowerCase());
  const moduleOptions = $derived(modules.filter((module) => matches(module.name)));
  const topicOptions = $derived(topics.filter((topic) => matches(topic.name)));
  const hasChecklist = (name: string, module: boolean) => rows.some((row) => row.module === module && row.subject_name === name);
  const topicCount = (module: number) => { const count = topics.filter((topic) => topic.module === module).length; return `${count} ${count === 1 ? "topic" : "topics"}`; };
  const moduleName = (id: number | null) => modules.find((module) => module.id === id)?.name ?? "No module";
  let createError = $state("");

  let editing = $state<Schedule | null>(null);

  const due = $derived(rows.filter((row) => row.due_today));
  const later = $derived(rows.filter((row) => !row.due_today));

  // R185: deleting a checklist from the list, after asking
  let deleting = $state<RecallRow | null>(null);

  async function remove() {
    if (!deleting) return;
    const { id, subject_name } = deleting;
    deleting = null;
    try {
      await api.deleteChecklist(id);
      toast({ text: "Checklist deleted", sub: subject_name, icon: "trash" });
      onChanged();
      await load();
    } catch (failure) {
      toast({ text: "Couldn't delete the checklist", sub: errorMessage(failure), icon: "alert", error: true });
    }
  }

  async function load() {
    [rows, schedule, today] = await Promise.all([api.recallList(), api.recallSchedule(), api.today()]);
    loaded = true;
  }

  async function openCreate() {
    [modules, topics] = await Promise.all([api.modules(), api.topics()]);
    subject = "";
    find = "";
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
      toast({ text: "Checklist created", icon: "plus" });
      onChanged();
      onOpen(checklist);
    } catch (failure) {
      createError = errorMessage(failure);
    }
  }

  function openSchedule() {
    if (!schedule) return;
    editing = structuredClone($state.snapshot(schedule));
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Free Recall" subtitle={loaded && rows.length ? (due.length ? `${due.length} due today` : "Nothing due today") : ""}>
    {#if rows.length}
      <button class="btn btn-ghost" onclick={openSchedule}><CalendarCog class="size-4" />Schedule</button>
      <button class="btn btn-ghost" onclick={openCreate}><Plus class="size-4" />New checklist</button>
    {/if}
  </PageHeader>

  {#if loaded && rows.length === 0}
    <div class="panel max-w-xl p-10">
      <span class="well" aria-hidden="true"><Brain class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No checklists yet</h2>
      <p class="mt-2 max-w-prose text-muted">Free recall means writing or saying everything you remember about a module or topic, then ticking it off against a checklist. The app schedules the next one from how you did.</p>
      <button class="btn btn-primary mt-6" onclick={openCreate}><Plus class="size-4" />Create your first checklist</button>
    </div>
  {/if}

  {#if due.length && schedule}
    <section aria-labelledby="due-h">
      <h2 id="due-h" class="eyebrow mb-3">Due today</h2>
      <div class="flex flex-col gap-3">
        {#each due as row, index (row.id)}
          <!-- The due checklist is a raised hero tile, with its step pips and last score -->
          <article class="due-tile flex items-center gap-6 px-6 py-5" aria-label={row.subject_name}>
            <div class="min-w-0 flex-[1_1_16rem]">
              <p><span class="chip">{row.module ? "Module" : "Topic"} · {row.points} {row.points === 1 ? "point" : "points"}</span></p>
              <h3 class="mt-1.5 text-[26px] font-black leading-tight"><button class="text-left hover:underline" onclick={() => onOpen(row.id)}>{row.subject_name}</button></h3>
              <p class="mt-1.5 flex flex-wrap items-center gap-3">
                {#if row.days_late}<span class="chip chip-due"><Clock class="size-3.5" />Due {row.days_late} {row.days_late === 1 ? "day" : "days"} ago</span>{/if}
                <span class="w-40"><Ladder compact steps={schedule.steps} position={row.position} step={row.step} next={row.next_step} /></span>
                <span class="text-sm">{row.step ?? "Not started"}{#if row.next_step}{" "}<span class="text-muted">then {row.next_step}</span>{/if}</span>
              </p>
            </div>
            <div class="text-center">
              {#if row.last_score !== null}<p class="text-[30px] font-black leading-none tabular-nums">{row.last_score}%</p><p class="mt-1 text-[13px] text-muted">Last check</p>{:else}<p class="text-[15px] text-muted">First check</p>{/if}
            </div>
            {#if row.points}
              <button class="btn {index === 0 ? 'btn-primary' : 'btn-ghost'} min-w-48" onclick={() => onStart(row.id)}><Play class="size-4 fill-current" />Start recall</button>
            {:else}
              <button class="btn btn-ghost" onclick={() => onOpen(row.id)}><Plus class="size-4" />Add points</button>
            {/if}
            <RowMenu label="{row.subject_name} actions" items={[{ label: "Open checklist", onSelect: () => onOpen(row.id) }, { label: "Delete checklist…", danger: true, onSelect: () => (deleting = row) }]} />
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
        <table class="table table-fixed" aria-label="Checklists" use:stackTable>
          <colgroup><col class="w-[27%]" /><col class="w-[11%]" /><col class="w-[28%]" /><col class="w-[15%]" /><col class="w-[13%]" /><col class="w-14" /></colgroup>
          <thead><tr class="text-muted"><th>Checklist</th><th>Points</th><th>Step</th><th>Last check</th><th>Next</th><th><span class="sr-only">Actions</span></th></tr></thead>
          <tbody>
            {#each later as row (row.id)}
              <tr class="hover:bg-base-100">
                <td><button class="py-0.5 text-left font-semibold hover:underline" onclick={() => onOpen(row.id)}>{row.subject_name}</button><div class="text-xs text-muted">{row.module ? "Module" : "Topic"}{row.subject_exists ? "" : " (deleted)"}</div></td>
                <td class="tabular-nums">{row.points}</td>
                <td><div class="flex items-center gap-3"><div class="w-28"><Ladder compact steps={schedule.steps} position={row.position} step={row.step} next={row.next_step} /></div><span class="text-sm">{row.step ?? "Not started"}</span></div></td>
                <td class="tabular-nums">{#if row.last_score === null}<span class="text-muted">None yet</span>{:else}<span class="flex min-w-0 items-center gap-2"><span class="inline-block h-3 w-24 min-w-6 shrink overflow-hidden rounded-full bg-base-300" aria-hidden="true"><span class="block h-full rounded-full" style="width: {row.last_score}%; background: var(--chart-right)"></span></span>{row.last_score}%</span>{/if}</td>
                <td class="tabular-nums">{dueLabel(row.due, today)}</td>
                <td class="text-right"><RowMenu small label="{row.subject_name} actions" items={[{ label: "Open checklist", onSelect: () => onOpen(row.id) }, { label: "Delete checklist…", danger: true, onSelect: () => (deleting = row) }]} /></td>
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
      <p class="mt-1 text-sm text-muted">Recall about a module (all its topics) or one topic.</p>
      <fieldset class="mt-5">
        <legend class="sr-only">Recall about</legend>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="search" class="input w-full" aria-label="Find a module or topic" placeholder="Find a module or topic…" bind:value={find} autofocus />
        <div class="mt-2 max-h-80 overflow-y-auto rounded-lg border border-base-300 bg-base-100 py-1" role="presentation">
          {#each [["Modules", moduleOptions, "module"], ["Topics", topicOptions, "topic"]] as const as [group, options, kind] (group)}
            {#if options.length}
              <p class="eyebrow px-4 pb-1 pt-3" aria-hidden="true">{group}</p>
              {#each options as option (option.id)}
                {@const value = `${kind}:${option.id}`}
                <label class="flex cursor-pointer items-center gap-3 px-4 py-2 hover:bg-base-200 {subject === value ? 'bg-base-200 shadow-[inset_3px_0_0_var(--primary-ink)]' : ''}">
                  <input type="radio" name="recall-about" class="radio radio-sm" checked={subject === value} onchange={() => (subject = value)} />
                  <span class="flex-1"><span class="block">{option.name}</span><span class="block text-xs text-muted">{kind === "module" ? topicCount(option.id) : moduleName((option as TopicRow).module)}</span></span>
                  {#if hasChecklist(option.name, kind === "module")}<span class="text-xs text-muted">Has a checklist</span>{/if}
                </label>
              {/each}
            {/if}
          {/each}
          {#if !moduleOptions.length && !topicOptions.length}<p class="px-4 py-3 text-sm text-muted">Nothing matches "{find.trim()}".</p>{/if}
        </div>
        <p class="mt-2 text-xs text-muted">{modules.length} {modules.length === 1 ? "module" : "modules"} and {topics.length} {topics.length === 1 ? "topic" : "topics"}. Type to narrow it down.</p>
      </fieldset>
      {#if createError}<p role="alert" class="mt-4 text-sm text-error">{createError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create checklist</button>
      </div>
    </form>
  </dialog>
{/if}

{#if editing}
  <ScheduleDialog title="Recall schedule" intro="Every checklist climbs these steps. Passing moves it up one, and the next date counts from the day you did the check." passWhat="points recalled" applies="Changes apply from each checklist's next check." schedule={editing}
    onSave={async (next) => { await api.setRecallSchedule(next); editing = null; await load(); }} onReset={() => api.resetRecallSchedule()} onClose={() => (editing = null)} />
{/if}

{#if deleting}
  <dialog open class="modal modal-open" aria-labelledby="delete-checklist-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-checklist-h" class="text-lg font-semibold">Delete the {deleting.subject_name} checklist?</h3>
      <p class="mt-2 text-sm">Its points and past checks go too. The {deleting.module ? "module" : "topic"} itself stays.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = null)}>Keep it</button>
        <button class="btn btn-error" onclick={remove}>Delete checklist</button>
      </div>
    </div>
  </dialog>
{/if}

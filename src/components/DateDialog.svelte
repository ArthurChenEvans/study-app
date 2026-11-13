<script lang="ts">
  // dates.feature (R130 to R132, R136): add or edit a date on the calendar. What it covers
  // and whether it tightens reviews come before the preview of what moves, so you can see
  // the effect of a choice before saving it.
  import { GraduationCap, Trash2 } from "@lucide/svelte";
  import { untrack } from "svelte";
  import { api, errorMessage, type DatePreview, type DateSave, type DateView, type Id, type ModuleRow, type TopicQuizView, type TopicRow } from "../lib/api";
  import { deleteWithUndo, toast } from "../lib/celebrations.svelte";
  import { dayFromIso, fullLabel, isoOf } from "../lib/dates";

  let { today, date = null, modules, topics, quizzes = [], startDay = null, onSaved, onCancel, onHide = () => {} }: {
    today: number;
    /** The date being edited, or null to add one. */
    date?: DateView | null;
    modules: ModuleRow[];
    topics: TopicRow[];
    quizzes?: TopicQuizView[];
    /** The day a new date starts on, when it's added from a day. */
    startDay?: number | null;
    onSaved: () => void;
    onCancel: () => void;
    /** Take a date off the calendar while its delete waits for Undo, or put it back. */
    onHide?: (id: Id, hidden: boolean) => void;
  } = $props();

  type CoverChoice = "nothing" | "module" | "topics";
  const KINDS = ["Exam", "Deadline", "Coursework", "Mock exam"];
  const initial = untrack(() => date);
  let name = $state(initial?.name ?? "");
  let iso = $state(isoOf(initial?.day ?? untrack(() => startDay) ?? untrack(() => today) + 14));
  let kind = $state(initial?.kind ?? "");
  let covers = $state<CoverChoice>(initial ? (initial.covers === "Nothing" ? "nothing" : "Module" in initial.covers ? "module" : "topics") : "nothing");
  let module = $state<Id | null>(initial && initial.covers !== "Nothing" && "Module" in initial.covers ? initial.covers.Module : (untrack(() => modules)[0]?.id ?? null));
  let picked = $state<Id[]>(initial && initial.covers !== "Nothing" && "Topics" in initial.covers ? [...initial.covers.Topics] : []);
  let tighten = $state(initial?.tighten ?? false);
  /** Once the tick is touched, the kind stops setting it. */
  let tightenTouched = $state(initial !== null);
  let filter = $state("");
  let preview = $state<DatePreview | null>(null);
  let busy = $state(false);
  let confirmingDelete = $state(false);
  let error = $state("");

  const day = $derived(iso ? dayFromIso(iso) : null);
  const coversSomething = $derived(covers === "module" ? module !== null : covers === "topics" ? picked.length > 0 : false);
  const save = $derived<DateSave | null>(day === null ? null : { name: name.trim(), day, kind: kind.trim(), module: covers === "module" ? module : null, topics: covers === "topics" ? picked : [], tighten: tighten && coversSomething });
  const moduleName = (id: Id | null) => modules.find((m) => m.id === id)?.name;
  const shownTopics = $derived(topics.filter((t) => !filter.trim() || `${t.name} ${moduleName(t.module) ?? ""}`.toLowerCase().includes(filter.trim().toLowerCase())));
  const quizOf = (topic: Id) => quizzes.find((q) => q.topic === topic);

  // An exam tightens reviews unless you've said otherwise (R132).
  $effect(() => {
    const isExam = kind.trim().toLowerCase() === "exam";
    if (!untrack(() => tightenTouched)) tighten = isExam;
  });

  // The preview follows what's picked, a moment after it stops changing.
  $effect(() => {
    const current = save;
    if (!current || !current.tighten) {
      preview = null;
      return;
    }
    const timer = setTimeout(async () => {
      try {
        preview = await api.previewDate(date?.id ?? null, current);
      } catch {
        preview = null;
      }
    }, 200);
    return () => clearTimeout(timer);
  });

  function toggle(topic: Id) {
    picked = picked.includes(topic) ? picked.filter((id) => id !== topic) : [...picked, topic];
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!save) return;
    if (!save.name) {
      error = "Give the date a name.";
      return;
    }
    busy = true;
    error = "";
    try {
      if (date) await api.updateDate(date.id, save);
      else await api.addDate(save);
      toast({ text: date ? "Date saved" : "Date added", sub: `${save.name}, ${fullLabel(save.day, today)}`, icon: date ? "check" : "calendar" });
      onSaved();
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  // toasts.feature (R157): it still asks first, then goes with Undo like every other delete.
  function remove() {
    if (!date) return;
    const { id, name } = date;
    confirmingDelete = false;
    deleteWithUndo({
      text: "Date deleted",
      sub: name,
      hide: () => onHide(id, true),
      restore: () => onHide(id, false),
      commit: async () => {
        await api.deleteDate(id);
        onHide(id, false);
      },
    });
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="date-h">
  <form class="modal-box max-w-lg" onsubmit={submit} novalidate>
    <h3 id="date-h" class="text-lg font-semibold">{date ? "Edit date" : "Add a date"}</h3>
    {#if confirmingDelete && date}
      <p class="mt-4">Delete <b class="font-extrabold">{date.name}</b>?</p>
      <p class="mt-1 text-sm text-muted">{date.tighten ? "Anything it tightened goes back to its normal schedule." : "It's only taken off the calendar."}</p>
      {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (confirmingDelete = false)}>Keep it</button>
        <button type="button" class="btn btn-error" disabled={busy} onclick={remove}><Trash2 class="size-4" />Delete date</button>
      </div>
    {:else}
      <div class="mt-5 grid grid-cols-[1fr_11rem] gap-4">
        <label class="block"><span class="text-sm font-semibold">Name</span>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="input mt-1 w-full" bind:value={name} placeholder="e.g. Networks exam" autofocus /></label>
        <label class="block"><span class="text-sm font-semibold">Date</span><input type="date" class="input mt-1 w-full" bind:value={iso} /></label>
      </div>
      <label class="mt-4 block"><span class="text-sm font-semibold">Kind <span class="font-normal text-muted">(optional)</span></span>
        <input class="input mt-1 w-full" list="date-kinds" bind:value={kind} placeholder="Exam, Deadline, Coursework…" /></label>
      <datalist id="date-kinds">{#each KINDS as option (option)}<option value={option}></option>{/each}</datalist>

      <fieldset class="mt-4"><legend class="text-sm font-semibold">Covers</legend>
        <div class="seg mt-1" role="radiogroup" aria-label="Covers">
          {#each [["nothing", "Nothing"], ["module", "A module"], ["topics", "Topics"]] as [key, label] (key)}
            <button type="button" role="radio" aria-checked={covers === key} onclick={() => (covers = key as CoverChoice)}>{label}</button>
          {/each}
        </div>
      </fieldset>
      {#if covers === "module"}
        {#if modules.length}
          <label class="mt-3 block"><span class="sr-only">Module</span>
            <select class="select w-full" bind:value={module}>
              {#each modules as row (row.id)}<option value={row.id}>{row.name} ({topics.filter((t) => t.module === row.id).length} {topics.filter((t) => t.module === row.id).length === 1 ? "topic" : "topics"})</option>{/each}
            </select></label>
        {:else}<p class="mt-3 text-sm text-muted">You don't have any modules yet. Pick topics instead.</p>{/if}
      {:else if covers === "topics"}
        <div class="mt-3">
          {#if topics.length > 8}<input type="search" class="input input-sm mb-2 w-full" aria-label="Filter topics" placeholder="Filter topics…" bind:value={filter} />{/if}
          <ul class="note-box max-h-56 divide-y overflow-y-auto font-normal" aria-label="Topics it covers">
            {#each shownTopics as topic (topic.id)}
              {@const quiz = quizOf(topic.id)}
              <li><label class="flex cursor-pointer items-start gap-3 px-3 py-2">
                <input type="checkbox" class="checkbox checkbox-sm mt-0.5" checked={picked.includes(topic.id)} onchange={() => toggle(topic.id)} />
                <span><span class="block text-[15px] font-extrabold">{topic.name}</span>
                  <span class="block text-xs text-muted">{moduleName(topic.module) ?? "No module"} · {quiz ? `Next quiz ${fullLabel(quiz.normal_due, today)}` : "Not quizzed yet"}</span></span>
              </label></li>
            {:else}<li class="px-3 py-2 text-sm text-muted">No topics match.</li>{/each}
          </ul>
          <p class="mt-1 text-xs text-muted">{picked.length} {picked.length === 1 ? "topic" : "topics"} picked</p>
        </div>
      {/if}

      <label class="mt-5 flex items-start gap-3 {coversSomething ? 'cursor-pointer' : 'opacity-60'}">
        <input type="checkbox" class="checkbox mt-0.5" checked={tighten && coversSomething} disabled={!coversSomething} onchange={(event) => { tighten = event.currentTarget.checked; tightenTouched = true; }} />
        <span><span class="block text-sm font-semibold">Tighten reviews before this</span>
          <span class="block text-sm text-muted">{coversSomething ? "Until the day, no quiz, card or drill for these topics waits longer than half the days left. Nothing lands after it." : "Pick a module or topics for this to do anything."}</span></span>
      </label>

      {#if save?.tighten && preview}
        <div class="note-box mt-4 p-4 text-sm" aria-live="polite">
          <p class="flex items-center gap-2 font-extrabold"><GraduationCap class="size-4" />{date ? "If you save this" : "If you add this"} today</p>
          <ul class="mt-2 space-y-1 font-normal text-muted">
            {#each preview.quizzes as move (move.topic)}<li><b class="font-extrabold text-base-content">{move.name}</b> quiz moves from {fullLabel(move.from, today)} to {fullLabel(move.to, today)}</li>{/each}
            {#each preview.unquizzed as topic (topic.topic)}<li><b class="font-extrabold text-base-content">{topic.name}</b> hasn't been quizzed yet, so it has no schedule to tighten</li>{/each}
            {#if preview.cards}<li><b class="font-extrabold text-base-content">{preview.cards} {preview.cards === 1 ? "card" : "cards"}</b> come in sooner</li>{/if}
            {#if preview.drills}<li><b class="font-extrabold text-base-content">{preview.drills} {preview.drills === 1 ? "drill" : "drills"}</b> come in sooner</li>{/if}
            {#if !preview.quizzes.length && !preview.unquizzed.length && !preview.cards && !preview.drills}<li>Nothing moves yet. Everything it covers is already due soon enough.</li>{/if}
          </ul>
        </div>
      {/if}

      {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        {#if date}<button type="button" class="btn btn-ghost mr-auto text-error" onclick={() => { error = ""; confirmingDelete = true; }}><Trash2 class="size-4" />Delete</button>{/if}
        <button type="button" class="btn btn-ghost" onclick={onCancel}>Cancel</button>
        <button type="submit" class="btn btn-primary" disabled={busy || day === null}>{date ? "Save" : "Add date"}</button>
      </div>
    {/if}
  </form>
</dialog>

<script lang="ts">
  // projects.feature (R88 to R90): sessions on a project, logging struggles, and the list.
  import { ArrowLeft, CircleDot, ListChecks, Play, Plus, Square, Timer } from "@lucide/svelte";
  import { onDestroy, tick } from "svelte";
  import { api, errorMessage, type Id, type ProjectView, type StruggleKind, type StruggleView, type TopicRow } from "../lib/api";
  import { fullLabel, onDay, pastOn } from "../lib/dates";

  let { id, onBack = () => {}, onReview = () => {} }: { id: Id; onBack?: () => void; onReview?: (project: Id) => void } = $props();

  let view = $state<ProjectView | null>(null);
  let topics = $state<TopicRow[]>([]);
  let today = $state(0);
  let now = $state(Date.now());
  let what = $state("");
  let why = $state("");
  let kind = $state<StruggleKind>("Unsure");
  let topic = $state("");
  let search = $state("");
  let onlyToReview = $state(false);
  let error = $state("");
  let status = $state("");
  let whatBox = $state<HTMLTextAreaElement | null>(null);

  const timer = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(timer));
  const session = $derived(view?.row.session ?? null);
  const elapsed = $derived(session ? Math.max(0, Math.floor(now / 1000) - session.started) : 0);
  const clock = (seconds: number) => `${Math.floor(seconds / 3600)}:${String(Math.floor((seconds % 3600) / 60)).padStart(2, "0")}`;
  const time = (minute: number) => `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
  const kindLabel = { Recall: "Couldn't recall it", Apply: "Couldn't apply it", Unsure: "Not sure yet" } as const;

  const shown = $derived((view?.struggles ?? []).filter((s) => (!onlyToReview || s.outcome === null) && `${s.what} ${s.why} ${s.issue ?? ""} ${s.topic_name ?? ""}`.toLowerCase().includes(search.trim().toLowerCase())));
  const byDay = $derived.by(() => {
    const groups: { day: number; struggles: StruggleView[] }[] = [];
    for (const struggle of shown) {
      const last = groups.at(-1);
      if (last && last.day === struggle.day) last.struggles.push(struggle);
      else groups.push({ day: struggle.day, struggles: [struggle] });
    }
    return groups;
  });

  async function load() {
    [view, topics, today] = await Promise.all([api.project(id), api.topics(), api.today()]);
  }

  async function start() {
    error = "";
    try {
      view = await api.startProjectSession(id);
      await tick();
      whatBox?.focus();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function end() {
    error = "";
    try {
      await api.endProjectSession(id);
      const left = (view?.struggles ?? []).filter((s) => s.outcome === null).length;
      if (left) onReview(id);
      else await load();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function log(event?: Event) {
    event?.preventDefault();
    if (!what.trim()) return;
    error = "";
    try {
      await api.logStruggle(id, { what, why, kind, topic: topic ? Number(topic) : null });
      status = "Logged. You'll decide what to do with it when you end the session.";
      what = "";
      why = "";
      kind = "Unsure";
      await load();
      whatBox?.focus();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function keys(event: KeyboardEvent) {
    if (!session) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "l") {
      event.preventDefault();
      whatBox?.focus();
    } else if ((event.ctrlKey || event.metaKey) && event.key === "Enter" && (event.target as HTMLElement | null)?.closest("form[aria-label='Log a struggle']")) {
      event.preventDefault();
      log();
    }
  }

  const outcome = (struggle: StruggleView) => {
    const value = struggle.outcome;
    if (value === null) return session ? "To review at the end of this session" : "To review";
    if (value === "Card") return "Card suggested";
    if (value === "Nothing") return "Kept as a note";
    if ("Checklist" in value) return `Added to the ${value.Checklist.name} checklist`;
    return `${struggle.issue ? `Issue: ${struggle.issue} · ` : ""}drill ${value.Drill.due === null ? "ticked off" : onDay(value.Drill.due, today)}`;
  };

  const toReviewCount = $derived((view?.struggles ?? []).filter((s) => s.outcome === null).length);

  load();
</script>

<!-- What became of a struggle. Anything still to do stands out: a labelled chip to review,
     or the drill day in bold. Settled outcomes stay quiet. -->
{#snippet outcomeOf(struggle: StruggleView)}
  {@const value = struggle.outcome}
  {#if value === null}
    <span class="todo-chip"><CircleDot class="size-3.5" aria-hidden="true" />{outcome(struggle)}</span>
  {:else if typeof value === "object" && "Drill" in value && value.Drill.due !== null && value.Drill.due <= today}
    <span class="text-muted">{struggle.issue ? `Issue: ${struggle.issue} · ` : ""}</span><span class="font-semibold text-base-content">drill {onDay(value.Drill.due, today)}</span>
  {:else}
    <span class="text-muted">{outcome(struggle)}</span>
  {/if}
{/snippet}

<svelte:window onkeydown={keys} />

<div class="mx-auto max-w-6xl px-12 pb-24">
  <button class="mt-8 inline-flex items-center gap-1 py-1 text-sm text-muted hover:underline" onclick={onBack}><ArrowLeft class="size-4" />Projects</button>
  {#if view}
    <div class="flex items-end justify-between gap-6 pb-6 pt-4">
      <div><p class="text-sm text-muted">{view.row.kind ? `${view.row.kind} · ` : ""}started {pastOn(view.row.created, today).replace(/^on /, "")} · {view.row.struggles} {view.row.struggles === 1 ? "struggle" : "struggles"}</p><h1 class="mt-1 text-3xl font-semibold">{view.row.name}</h1></div>
      <div class="flex items-center gap-3">
        {#if session}
          <span class="timer-chip" role="timer" aria-label="Session time"><Timer class="size-4" />{clock(elapsed)}</span>
          <button class="btn btn-outline border-field-border" onclick={end}><Square class="size-4" />End session{view.row.to_review ? " and review" : ""}</button>
        {:else}
          {#if view.row.to_review}<button class="btn btn-ghost" onclick={() => onReview(id)}><ListChecks class="size-4" />Review {view.row.to_review} {view.row.to_review === 1 ? "struggle" : "struggles"}</button>{/if}
          <button class="btn btn-primary" onclick={start}><Play class="size-4" />Start a session</button>
        {/if}
      </div>
    </div>
    {#if error}<p role="alert" class="mb-4 text-sm text-error">{error}</p>{/if}

    {#if session}
      <form class="panel p-6" aria-label="Log a struggle" onsubmit={log}>
        <div class="flex items-baseline justify-between"><h2 class="text-sm font-semibold">Log a struggle</h2><span class="text-xs text-muted"><kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">L</kbd> from anywhere in this project</span></div>
        <div class="mt-4 grid grid-cols-2 gap-6">
          <div><label class="block text-sm font-semibold" for="struggle-what">What tripped you up</label>
            <textarea id="struggle-what" class="textarea mt-1 w-full text-[15px]" rows="2" placeholder="e.g. Loop ran one too many times reading the file…" bind:this={whatBox} bind:value={what}></textarea></div>
          <div><label class="block text-sm font-semibold" for="struggle-why">Why wasn't it obvious?</label>
            <textarea id="struggle-why" class="textarea mt-1 w-full text-[15px]" rows="2" placeholder="The cue that was missing, in your words…" bind:value={why}></textarea></div>
        </div>
        <div class="mt-4 flex flex-wrap items-center justify-between gap-4">
          <div class="flex items-center gap-4">
            <div class="seg" role="radiogroup" aria-label="What kind of struggle">
              {#each ["Recall", "Apply", "Unsure"] as const as option (option)}<button type="button" role="radio" aria-checked={kind === option} onclick={() => (kind = option)}>{kindLabel[option]}</button>{/each}
            </div>
            <select class="select select-sm w-44" aria-label="Topic (optional)" bind:value={topic}><option value="">Topic (optional)</option>{#each topics as row (row.id)}<option value={String(row.id)}>{row.name}</option>{/each}</select>
          </div>
          <button type="submit" class="btn btn-primary" disabled={!what.trim()}><Plus class="size-4" />Log it <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">Enter</kbd></button>
        </div>
        <p class="mt-3 text-sm text-muted" role="status">{status || `Logged under today, ${fullLabel(today, today)}. You'll decide what to do with it when you end the session.`}</p>
      </form>
    {/if}

    <div class="mb-3 mt-8 flex items-center justify-between gap-4">
      <h2 class="text-lg font-semibold">Struggles</h2>
      {#if view.struggles.length}
        <div class="flex items-center gap-3">
          <input type="search" class="input input-sm w-56" aria-label="Search struggles" placeholder="Search…" bind:value={search} />
          <div class="seg" role="radiogroup" aria-label="Show"><button type="button" role="radio" aria-checked={!onlyToReview} onclick={() => (onlyToReview = false)}>All</button><button type="button" role="radio" aria-checked={onlyToReview} onclick={() => (onlyToReview = true)}>To review{#if toReviewCount}<span class="ml-1.5 tabular-nums text-muted">{toReviewCount}</span>{/if}</button></div>
        </div>
      {/if}
    </div>
    {#if view.struggles.length === 0}
      <p class="panel p-8 text-muted">{session ? "Nothing logged yet this session." : "Start a session, then log anything that trips you up while you work."}</p>
    {:else if byDay.length === 0}
      <p class="panel p-8 text-muted">No struggles match.</p>
    {/if}
    {#each byDay as group (group.day)}
      <p class="eyebrow mb-2 mt-6">{group.day === today ? `Today · ${fullLabel(group.day, today)}` : fullLabel(group.day, today)}</p>
      <ul class="panel divide-y" aria-label="Struggles on {fullLabel(group.day, today)}">
        {#each group.struggles as struggle (struggle.id)}
          <li class="flex items-start gap-6 px-6 py-4">
            <div class="flex-1"><p class="text-[15px]">{struggle.what}</p>{#if struggle.why}<p class="mt-1 text-sm text-muted">Why it wasn't obvious: {struggle.why}</p>{/if}
              <div class="mt-2 flex flex-wrap items-center gap-2"><span class="kind">{kindLabel[struggle.kind]}</span>{#if struggle.topic_name}<span class="badge-src">{struggle.topic_name}</span>{/if}<span class="text-xs text-muted tabular-nums">{time(struggle.minute)}</span></div></div>
            <span class="max-w-72 text-right text-sm">{@render outcomeOf(struggle)}</span>
          </li>
        {/each}
      </ul>
    {/each}
  {/if}
</div>

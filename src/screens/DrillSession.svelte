<script lang="ts">
  // projects.feature (R93): do the drills, then rate each one Struggled, Shaky or Solid.
  import { CalendarArrowUp, ChevronRight, CircleCheck, LogOut, Repeat, Timer } from "@lucide/svelte";
  import MoveDrillDialog from "../components/MoveDrillDialog.svelte";
  import { onDestroy } from "svelte";
  import { api, errorMessage, type DrillRating, type DrillView, type Id } from "../lib/api";
  import { dayLabel, fullLabel, onDay } from "../lib/dates";

  let { ids, onLeave = () => {} }: { ids: Id[]; onLeave?: () => void } = $props();

  let drills = $state<DrillView[]>([]);
  let index = $state(0);
  let rated = $state<DrillView | null>(null);
  let note = $state("");
  let seconds = $state(0);
  let today = $state(0);
  let busy = $state(false);
  let error = $state("");
  let moving = $state(false);
  let took = $state(0);

  const timer = setInterval(() => { if (!rated) seconds += 1; }, 1000);
  onDestroy(() => clearInterval(timer));
  const clock = (total: number) => `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  const current = $derived(drills[index] ?? null);

  // The same struggle logged several times reads once, with how often and when last.
  const reasons = $derived.by(() => {
    const groups = new Map<string, { what: string; project: string; times: number; last: number }>();
    for (const struggle of current?.from ?? []) {
      const key = `${struggle.project_name}\u0000${struggle.what}`;
      const seen = groups.get(key);
      if (seen) { seen.times += 1; seen.last = Math.max(seen.last, struggle.day); }
      else groups.set(key, { what: struggle.what, project: struggle.project_name, times: 1, last: struggle.day });
    }
    return [...groups.values()].sort((a, b) => b.last - a.last);
  });
  const lastSeen = (day: number) => { const label = dayLabel(day, today); return label === "Today" || label === "Yesterday" ? label.toLowerCase() : label; };
  const ratings: { rating: DrillRating; when: string }[] = [
    { rating: "Struggled", when: "Again tomorrow" },
    { rating: "Shaky", when: "Again tomorrow" },
    { rating: "Solid", when: "Ticked off" },
  ];

  async function load() {
    [drills, today] = await Promise.all([Promise.all(ids.map((id) => api.drill(id))), api.today()]);
  }

  async function rate(rating: DrillRating) {
    if (!current || busy || rated) return;
    busy = true;
    error = "";
    try {
      took = seconds;
      rated = await api.rateDrill(current.id, rating, seconds, note);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function next() {
    if (index + 1 >= drills.length) return onLeave();
    index += 1;
    rated = null;
    note = "";
    seconds = 0;
  }

  async function move(day: number) {
    if (!rated) return;
    try {
      rated = await api.moveDrill(rated.id, day);
      moving = false;
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function keys(event: KeyboardEvent) {
    if ((event.target as HTMLElement | null)?.closest("input, textarea, select") || moving) return;
    if (!rated && /^[1-3]$/.test(event.key)) { event.preventDefault(); rate(ratings[Number(event.key) - 1].rating); }
    else if (rated && event.key === "Enter") { event.preventDefault(); next(); }
  }

  load();
</script>

<svelte:window onkeydown={keys} />

{#if current}
  <div class="sticky top-0 z-10 flex items-center justify-between gap-6 border-b border-base-300 bg-base-100 px-12 py-4">
    <div class="flex items-center gap-3 text-sm"><span class="text-muted">Drills</span><ChevronRight class="size-3.5" /><span class="font-semibold tabular-nums">{index + 1} of {drills.length}</span></div>
    <!-- No ticking clock while you work: the time is kept and shown once you've rated it. -->
    <span></span>
    <button class="btn btn-ghost btn-sm font-normal" onclick={onLeave}><LogOut class="size-4" />Leave</button>
  </div>

  {#if rated}
    {@const solid = rated.due === null}
    <div class="mx-auto max-w-3xl px-12 pb-24 pt-12">
      <p class="text-sm text-muted">{rated.issue ?? rated.task}</p>
      <h1 class="mt-2 flex items-center gap-3 text-3xl font-semibold">{#if solid}<CircleCheck class="size-7" />Ticked off{:else}<Repeat class="size-7" />Again {onDay(rated.due!, today)}{/if}</h1>
      <p class="mt-2 flex items-center gap-1.5 text-sm text-muted"><Timer class="size-3.5" aria-hidden="true" />Took {clock(took)}</p>
      <p class="mt-3 max-w-prose text-muted">{solid ? "This drill is done. It only comes back if the same issue turns up in a session review, and you'll be asked first." : `Rated ${rated.last?.rating}, so it's back on ${fullLabel(rated.due!, today)}. It comes back daily until it's Solid.`}</p>
      {#if rated.last?.note}<div class="panel mt-6 p-5"><p class="text-sm font-semibold">Your note</p><p class="mt-1">{rated.last.note}</p></div>{/if}
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <div class="mt-10 flex items-center justify-between gap-4">
        {#if !solid}<button class="btn btn-ghost" onclick={() => { error = ""; moving = true; }}><CalendarArrowUp class="size-4" />Move it to another day</button>{:else}<span></span>{/if}
        <button class="btn btn-primary" onclick={next}>{index + 1 < drills.length ? `Next drill` : "Done"} <kbd class="kbd kbd-xs">Enter</kbd></button>
      </div>
    </div>
  {:else}
    <div class="mx-auto max-w-prose px-6 pb-24 pt-14">
      <p class="text-sm text-muted">{[current.issue, current.topic_name, current.from[0] ? `from ${current.from[0].project_name}` : null].filter(Boolean).join(" · ")}</p>
      <div class="mt-2 text-2xl font-semibold leading-snug" role="heading" aria-level="1">{current.task}</div>
      <p class="mt-3 text-muted">Do it outside the app, in your editor or on paper. Come back and rate it when you're done.</p>
      {#if current.hints}<details class="panel mt-8 p-5"><summary class="cursor-pointer text-sm font-semibold">Show the hints</summary><p class="mt-3 whitespace-pre-line text-sm">{current.hints}</p></details>{/if}
      {#if current.from.length}
        <div class="panel mt-6 p-5"><p class="text-sm font-semibold">Why it was made</p>
          <ul class="mt-2 space-y-2 text-sm">{#each reasons.slice(0, 5) as reason (reason.project + reason.what)}<li><span class="block">"{reason.what}"</span><span class="block text-muted">{reason.times > 1 ? `${reason.times} times in ${reason.project}, last ${lastSeen(reason.last)}` : `${dayLabel(reason.last, today)}, ${reason.project}`}</span></li>{/each}</ul></div>
      {/if}
      <label class="mt-6 block"><span class="text-sm font-semibold">Note <span class="font-normal text-muted">(optional)</span></span>
        <textarea class="textarea mt-1 w-full" rows="2" placeholder="What went wrong, or what made it click…" bind:value={note}></textarea></label>
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <!-- The ratings sit under the task, not on the bottom edge, so zoomed in you keep both in view. -->
      <div class="mt-10 flex flex-col items-center gap-3">
        <p id="how-h" class="font-semibold">How did it go?</p>
        <div class="flex items-center justify-center gap-3" role="group" aria-labelledby="how-h">
          {#each ratings as option, position (option.rating)}
            <button class="btn rate {option.rating === 'Solid' ? 'btn-primary' : 'btn-outline border-field-border'}" style={option.rating === "Struggled" ? "color: var(--wrong-ink)" : ""} disabled={busy} onclick={() => rate(option.rating)}>
              <span>{option.rating} <kbd class="kbd kbd-xs">{position + 1}</kbd></span><span class="when">{option.when}</span>
            </button>
          {/each}
        </div>
        <p class="text-xs text-muted">Press 1, 2 or 3. Your time is kept and shown after you rate.</p>
      </div>
    </div>
  {/if}
{:else if drills.length === 0 && today}
  <div class="mx-auto max-w-3xl px-12 pt-16"><h1 class="text-3xl font-semibold">No drills due</h1><button class="btn btn-primary mt-6" onclick={onLeave}>Back</button></div>
{/if}

{#if moving && rated}
  <MoveDrillDialog task={rated.task} {today} current={rated.due} {error} onMove={move} onCancel={() => (moving = false)} />
{/if}

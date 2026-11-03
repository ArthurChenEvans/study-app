<script lang="ts">
  // projects.feature (R93): do the drills, then rate each one Struggled, Shaky or Solid.
  import { CalendarArrowUp, ChevronRight, CircleCheck, LogOut, Repeat, Timer } from "@lucide/svelte";
  import { onDestroy } from "svelte";
  import { api, errorMessage, type DrillRating, type DrillView, type Id } from "../lib/api";
  import { dayFromIso, dayLabel, fullLabel, isoOf, onDay } from "../lib/dates";

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
  let moveTo = $state("");

  const timer = setInterval(() => { if (!rated) seconds += 1; }, 1000);
  onDestroy(() => clearInterval(timer));
  const clock = (total: number) => `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  const current = $derived(drills[index] ?? null);
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

  async function move(event: SubmitEvent) {
    event.preventDefault();
    if (!rated) return;
    try {
      rated = await api.moveDrill(rated.id, dayFromIso(moveTo));
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
    {#if !rated}<span class="timer-chip" role="timer" aria-label="Time on this drill"><Timer class="size-4" />{clock(seconds)}</span>{:else}<span></span>{/if}
    <button class="btn btn-ghost btn-sm font-normal" onclick={onLeave}><LogOut class="size-4" />Leave</button>
  </div>

  {#if rated}
    {@const solid = rated.due === null}
    <div class="mx-auto max-w-3xl px-12 pb-24 pt-12">
      <p class="text-sm text-muted">{rated.issue ?? rated.task}</p>
      <h1 class="mt-2 flex items-center gap-3 text-3xl font-semibold">{#if solid}<CircleCheck class="size-7" />Ticked off{:else}<Repeat class="size-7" />Again {onDay(rated.due!, today)}{/if}</h1>
      <p class="mt-3 max-w-prose text-muted">{solid ? "This drill is done. It only comes back if the same issue turns up in a session review, and you'll be asked first." : `Rated ${rated.last?.rating}, so it's back on ${fullLabel(rated.due!, today)}. It comes back daily until it's Solid.`}</p>
      {#if rated.last?.note}<div class="panel mt-6 p-5"><p class="text-sm font-semibold">Your note</p><p class="mt-1">{rated.last.note}</p></div>{/if}
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <div class="mt-10 flex items-center justify-between gap-4">
        {#if !solid}<button class="btn btn-ghost" onclick={() => { moveTo = isoOf(today + 2); moving = true; }}><CalendarArrowUp class="size-4" />Move it to another day</button>{:else}<span></span>{/if}
        <button class="btn btn-primary" onclick={next}>{index + 1 < drills.length ? `Next drill` : "Done"} <kbd class="kbd kbd-xs">Enter</kbd></button>
      </div>
    </div>
  {:else}
    <div class="mx-auto max-w-prose px-6 pb-48 pt-16">
      <p class="text-sm text-muted">{[current.issue, current.topic_name, current.from[0] ? `from ${current.from[0].project_name}` : null].filter(Boolean).join(" · ")}</p>
      <div class="mt-3 text-xl leading-relaxed" role="heading" aria-level="1">{current.task}</div>
      <p class="mt-6 text-muted">Do it outside the app, in your editor or on paper. Come back and rate it when you're done.</p>
      {#if current.hints}<details class="panel mt-8 p-5"><summary class="cursor-pointer text-sm font-semibold">Show the hints</summary><p class="mt-3 whitespace-pre-line text-sm">{current.hints}</p></details>{/if}
      {#if current.from.length}
        <div class="panel mt-6 p-5"><p class="text-sm font-semibold">Why it was made</p>
          <ul class="mt-2 space-y-1 text-sm text-muted">{#each current.from.slice(0, 5) as struggle, position (position)}<li>{dayLabel(struggle.day, today)}, {struggle.project_name}: "{struggle.what}"</li>{/each}</ul></div>
      {/if}
      <label class="mt-6 block"><span class="text-sm font-semibold">Note <span class="font-normal text-muted">(optional)</span></span>
        <textarea class="textarea mt-1 w-full" rows="2" placeholder="What went wrong, or what made it click…" bind:value={note}></textarea></label>
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
    </div>
    <div class="fixed bottom-0 left-0 right-0 border-t border-base-300 bg-base-100 py-6">
      <div class="mx-auto flex max-w-prose items-center justify-center gap-3" role="group" aria-label="How did it go?">
        {#each ratings as option, position (option.rating)}
          <button class="btn rate {option.rating === 'Solid' ? 'btn-primary' : 'btn-outline border-field-border'}" style={option.rating === "Struggled" ? "color: var(--wrong-ink)" : ""} disabled={busy} onclick={() => rate(option.rating)}>
            <span>{option.rating} <kbd class="kbd kbd-xs">{position + 1}</kbd></span><span class="when">{option.when}</span>
          </button>
        {/each}
      </div>
      <p class="mt-3 text-center text-xs text-muted">How did it go?</p>
    </div>
  {/if}
{:else if drills.length === 0 && today}
  <div class="mx-auto max-w-3xl px-12 pt-16"><h1 class="text-3xl font-semibold">No drills due</h1><button class="btn btn-primary mt-6" onclick={onLeave}>Back</button></div>
{/if}

{#if moving}
  <dialog open class="modal modal-open" aria-labelledby="move-h">
    <form class="modal-box max-w-md" onsubmit={move}>
      <h3 id="move-h" class="text-lg font-semibold">Move this drill</h3>
      <label class="mt-5 block"><span class="text-sm font-semibold">Do it on</span><input type="date" class="input mt-1 w-full" min={isoOf(today)} bind:value={moveTo} /></label>
      <div class="modal-action"><button type="button" class="btn btn-ghost" onclick={() => (moving = false)}>Cancel</button><button type="submit" class="btn btn-primary">Move to {moveTo ? fullLabel(dayFromIso(moveTo), today) : "that day"}</button></div>
    </form>
  </dialog>
{/if}

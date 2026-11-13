<script lang="ts">
  // flashcards.feature (R65 to R69, R71): review the due cards, grade them, undo, leave.
  import { CircleCheck, Flag, Pencil, Undo2, X } from "@lucide/svelte";
  import { onDestroy } from "svelte";
  import { api, errorMessage, type CardRating, type ReviewCard, type ReviewDone, type ReviewStep, type Subject } from "../lib/api";
  import Rich from "../components/Rich.svelte";
  import { dueLabel } from "../lib/dates";
  import { celebrate, praise, startSession } from "../lib/celebrations.svelte";

  let { scope = null, scopeName = "All topics", onLeave, onEdit }: { scope?: Subject | null; scopeName?: string; onLeave: () => void; onEdit: (topic: number, note: number) => void } = $props();

  let card = $state<ReviewCard | null>(null);
  let done = $state<ReviewDone | null>(null);
  let shown = $state(false);
  let busy = $state(false);
  let error = $state("");
  let today = $state(0);
  let started = Date.now();
  let session: ReturnType<typeof startSession> | null = null;

  const ratings: CardRating[] = ["Again", "Hard", "Good", "Easy"];
  const cloze = $derived(card?.kind.startsWith("Cloze") ?? false);
  const queueName = { new: "new", learning: "learning", review: "a review" } as const;

  function interval(days: number) {
    if (days <= 0) return "Later today";
    if (days === 1) return "Tomorrow";
    if (days < 30) return `${days} days`;
    if (days < 365) return `${Math.round(days / 30)} ${Math.round(days / 30) === 1 ? "month" : "months"}`;
    const years = Math.round((days / 365) * 10) / 10;
    return `${years} ${years === 1 ? "year" : "years"}`;
  }

  function show(step: ReviewStep) {
    shown = false;
    started = Date.now();
    if ("Card" in step) {
      card = step.Card;
      done = null;
      session ??= startSession(step.Card.total - step.Card.done);
    } else {
      card = null;
      done = step.Done;
      // Only a session that ran here celebrates, not reopening a finished day.
      if (session && step.Done.reviewed > 0) {
        celebrate("reviews", {
          title: praise("reviews"),
          line: "Every card due today is cleared.",
          stats: [
            { label: "Cards", value: step.Done.reviewed, tone: "primary" },
            { label: "Minutes", value: Math.max(1, Math.round(step.Done.seconds / 60)), tone: "accent" },
            ...(step.Done.remembered === null ? [] : [{ label: "Remembered", value: step.Done.remembered, suffix: "%", tone: "success" as const }]),
          ],
        });
        session = null;
      }
    }
  }

  async function run(action: () => Promise<ReviewStep>) {
    if (busy) return;
    busy = true;
    error = "";
    try {
      show(await action());
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function grade(rating: CardRating) {
    if (busy) return;
    session?.answered(rating !== "Again");
    run(() => api.gradeCard(rating, Math.min(600, Math.round((Date.now() - started) / 1000))));
  }
  const undo = () => run(() => api.undoCard());

  async function edit() {
    if (!card) return;
    const view = await api.noteView(card.note);
    await api.leaveReview();
    onEdit(view.note.topic, card.note);
  }

  async function leave() {
    try { await api.leaveReview(); } catch { /* nothing to leave */ }
    onLeave();
  }

  const rewrite = (note: number, topic: number) => onEdit(topic, note);

  function keys(event: KeyboardEvent) {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, textarea, select")) return;
    if (!card) return;
    if (event.key.toLowerCase() === "e") { event.preventDefault(); edit(); return; }
    if (event.key.toLowerCase() === "z" && card.can_undo) { event.preventDefault(); undo(); return; }
    if (!shown) {
      if (event.key === " " || event.key === "Enter") { event.preventDefault(); shown = true; }
      return;
    }
    if (event.key === " ") { event.preventDefault(); grade("Good"); }
    else if (/^[1-4]$/.test(event.key)) { event.preventDefault(); grade(ratings[Number(event.key) - 1]); }
  }

  onDestroy(() => { if (card) api.leaveReview().catch(() => {}); });

  api.today().then((day) => (today = day));
  run(() => api.startReview(scope));
</script>

<svelte:window onkeydown={keys} />

{#if card}
  <!-- The lesson shell (design/playful-redesign: Card review): leave, progress and what's left on
       top, the card as one tile, and the four ratings as full-width buttons in a bottom band. -->
  <div class="flex min-h-screen flex-col">
  <div class="sticky top-0 z-10 bg-base-100 px-12 pb-3 pt-6">
    <div class="mx-auto flex max-w-5xl items-center gap-2 tab:gap-5">
      <button class="btn btn-quiet btn-sm btn-square" aria-label="Leave" title="Leave" onclick={leave}><X class="size-6" strokeWidth={2.6} /></button>
      <div class="h-[18px] min-w-16 flex-1 overflow-hidden rounded-full bg-base-300" role="progressbar" aria-label="Session progress" aria-valuenow={card.done} aria-valuemin={0} aria-valuemax={card.total}><div class="lesson-fill h-full rounded-full" style="width: max(18px, {card.total ? (card.done / card.total) * 100 : 0}%)"></div></div>
      <p class="hidden items-center gap-1.5 tab:flex" aria-label="Left today: {card.left.new} new, {card.left.learning} learning, {card.left.review} to review. This card is {queueName[card.queue]}.">
        <span class="chip chip-info tabular-nums {card.queue === 'new' ? 'chip-now' : ''}">{card.left.new} new</span>
        <span class="chip tabular-nums {card.queue === 'learning' ? 'chip-now' : ''}">{card.left.learning} learning</span>
        <span class="chip chip-primary tabular-nums {card.queue === 'review' ? 'chip-now' : ''}">{card.left.review} to review</span>
      </p>
      <button class="btn btn-quiet btn-sm" title="Edit this card (E)" onclick={edit}><Pencil class="size-4" />Edit</button>
      <button class="btn btn-quiet btn-sm" title="Undo the last grade (Z)" disabled={!card.can_undo || busy} onclick={undo}><Undo2 class="size-4" />Undo</button>
    </div>
  </div>

  <div class="flex-1 px-12 pb-10 pt-6">
    <div class="mx-auto max-w-[44rem]">
      <div class="flashcard">
        <p><span class="chip">{card.topic_name}{card.source ? ` · ${card.source}` : cloze ? ` · ${card.kind}` : ""}</span></p>
        {#if cloze}
          <div class="mt-3 text-[24px] font-extrabold leading-snug" role="heading" aria-level="1" aria-live="polite"><Rich text={shown ? card.back : card.front} /></div>
        {:else}
          <div class="mt-3 text-[24px] font-extrabold leading-snug" role="heading" aria-level="1"><Rich text={card.front} /></div>
          {#if shown}<hr class="my-6 border-0 border-t-2 border-dashed border-base-300" /><div class="text-[19px] leading-relaxed" aria-live="polite"><Rich text={card.back} /></div>{/if}
        {/if}
      </div>
      {#if error}<p role="alert" class="mt-6 text-sm text-error">{error}</p>{/if}

      <!-- Only after you've tried it: seeing this first would tempt you to edit instead of recall. -->
      {#if shown && card.leech}
        <div class="tip mt-3" role="note"><Flag class="size-5 shrink-0 fill-current text-[var(--accent-ink)]" />
          <p class="flex-1 text-[14.5px]"><b>This card has been tricky: Again {card.lapses} times.</b> A better cue usually helps.</p>
          <button class="btn btn-ghost btn-sm shrink-0" onclick={edit}><Pencil class="size-4" />Rewrite it <kbd class="kbd kbd-xs">E</kbd></button>
        </div>
      {/if}
    </div>
  </div>

  <div class="sheet-foot">
    <div class="mx-auto max-w-5xl">
      {#if shown}
        <div class="grid grid-cols-2 gap-3.5 tab:grid-cols-4">
          {#each ratings as rating, index (rating)}
            <button class="btn grade grade-{rating.toLowerCase()} w-full" disabled={busy} onclick={() => grade(rating)}>
              <span>{rating} <kbd class="kbd kbd-xs">{index + 1}</kbd></span><span class="when">{interval(card.intervals[index])}</span>
            </button>
          {/each}
        </div>
      {:else}
        <div class="flex justify-center"><button class="btn btn-primary btn-lg min-w-80" onclick={() => (shown = true)}>Show answer <kbd class="kbd kbd-xs">Space</kbd></button></div>
      {/if}
      <p class="mt-3 text-center text-[13px] text-muted">{shown ? "Space picks Good. Again shows the card again later in this session." : "Say it in your head first, then check."}</p>
    </div>
  </div>
  </div>
{:else if done}
  <div class="mx-auto max-w-3xl px-12 pb-24 pt-16">
    <section aria-labelledby="done-h">
      <p class="text-sm text-muted">Cards · {scopeName.toLowerCase() === "all topics" ? "all topics" : scopeName}</p>
      <h1 id="done-h" class="mt-2 flex items-center gap-3 text-4xl font-semibold"><CircleCheck class="size-8" />{done.reviewed ? "Done for today" : "Nothing due"}</h1>
      {#if done.reviewed}
        <div class="mt-8 grid grid-cols-3 gap-6">
          <div class="panel p-5"><p class="text-sm text-muted">Reviewed</p><p class="mt-1 text-3xl font-semibold tabular-nums">{done.reviewed}</p><p class="text-sm text-muted">{done.new} new</p></div>
          <div class="panel p-5"><p class="text-sm text-muted">Remembered</p><p class="mt-1 text-3xl font-semibold tabular-nums">{done.remembered === null ? "None yet" : `${done.remembered}%`}</p><p class="text-sm text-muted">Aiming for {done.target}% · {done.again} {done.again === 1 ? "time" : "times"} Again</p></div>
          <div class="panel p-5"><p class="text-sm text-muted">Time</p><p class="mt-1 text-3xl font-semibold tabular-nums">{done.seconds < 60 ? "Under a minute" : `${Math.round(done.seconds / 60)} min`}</p><p class="text-sm text-muted">{Math.round(done.seconds / done.reviewed)} s a card</p></div>
        </div>
      {:else}
        <p class="mt-3 text-muted">There are no cards due here today.</p>
      {/if}
    </section>
    {#if done.worth_rewriting.length}
      <section class="panel mt-10 p-6" aria-labelledby="slip-h">
        <h2 id="slip-h" class="flex items-center gap-2 text-sm font-semibold"><Flag class="size-4" />Worth rewriting</h2>
        <p class="mt-1 text-sm text-muted">These went to Again more than once today.</p>
        <ul class="mt-4 divide-y">
          {#each done.worth_rewriting as item (item.note)}
            <li class="flex items-center justify-between gap-4 py-3"><span class="text-[15px]"><Rich inline text={item.front} /></span><button class="btn btn-ghost btn-sm shrink-0" onclick={() => rewrite(item.note, item.topic)}><Pencil class="size-4" />Rewrite</button></li>
          {/each}
        </ul>
      </section>
    {/if}
    <div class="mt-10 flex items-center justify-between gap-4">
      <p class="text-sm text-muted">{done.due_tomorrow} due tomorrow.{#if done.busiest}{" "}The busiest day this fortnight is {dueLabel(done.busiest[0], today)} with {done.busiest[1]}.{/if}</p>
      <button class="btn btn-primary" onclick={onLeave}>Back to Cards</button>
    </div>
  </div>
{:else if error}
  <div class="mx-auto max-w-3xl px-12 pt-16"><p role="alert" class="text-error">{error}</p><button class="btn btn-ghost mt-4" onclick={onLeave}>Back to Cards</button></div>
{/if}

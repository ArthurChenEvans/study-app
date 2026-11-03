<script lang="ts">
  // calendar.feature (R98, R99): what's due each day, and what was done on past days.
  import { ArrowLeft, Brain, CalendarArrowUp, ChevronLeft, ChevronRight, Dumbbell, ListChecks, Play, SquareStack, TriangleAlert } from "@lucide/svelte";
  import { untrack } from "svelte";
  import { api, errorMessage, type CalendarDay, type DayView, type DrillView, type Id, type StruggleView } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import { dateOf, dayFromIso, dueLabel, fullLabel, isoOf, monthGrid, monthLabel, onDay } from "../lib/dates";

  let {
    initialDay = null, onOpenDay = () => {},
    onCards = () => {}, onRecall = () => {}, onQuiz = () => {}, onDrills = () => {}, onResults = () => {}, onProject = () => {},
  }: {
    /** A day to open straight away, so coming back from a drill lands on it. */
    initialDay?: number | null; onOpenDay?: (day: number | null) => void;
    onCards?: () => void; onRecall?: (id: Id) => void; onQuiz?: (topics: Id[]) => void; onDrills?: (ids: Id[]) => void;
    onResults?: (attempt: Id) => void; onProject?: (project: Id) => void;
  } = $props();

  let today = $state(0);
  let year = $state(0);
  let month = $state(0);
  let days = $state<CalendarDay[]>([]);
  let selected = $state<number | null>(null);
  let day = $state<DayView | null>(null);
  let moving = $state<DrillView | null>(null);
  let moveTo = $state("");
  let error = $state("");

  const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const grid = $derived(year ? monthGrid(year, month) : []);
  const counts = (entry: CalendarDay) => [
    { key: "cards", icon: SquareStack, count: entry.cards, one: "card", many: "cards" },
    { key: "checklists", icon: Brain, count: entry.checklists, one: "checklist", many: "checklists" },
    { key: "quizzes", icon: ListChecks, count: entry.quizzes, one: "quiz", many: "quizzes" },
    { key: "drills", icon: Dumbbell, count: entry.drills, one: "drill", many: "drills" },
    { key: "struggles", icon: TriangleAlert, count: entry.struggles, one: "struggle", many: "struggles" },
  ].filter((item) => item.count > 0);
  const longDate = (value: number) => (value === today ? `Today, ${fullLabel(value, today)}` : fullLabel(value, today));

  async function load() {
    today = await api.today();
    const start = untrack(() => initialDay);
    const now = dateOf(start ?? today);
    [year, month] = [now.year, now.month];
    if (start !== null) await open(start);
    else await loadMonth();
  }

  async function loadMonth() {
    if (!grid.length) return;
    days = await api.calendarMonth(grid[0], grid[grid.length - 1]);
  }

  function shift(by: number) {
    const next = new Date(Date.UTC(year, month + by, 1));
    [year, month] = [next.getUTCFullYear(), next.getUTCMonth()];
    loadMonth();
  }

  async function open(value: number) {
    selected = value;
    onOpenDay(value);
    error = "";
    day = await api.calendarDay(value);
  }

  async function step(by: number) {
    if (selected === null) return;
    const next = selected + by;
    const date = dateOf(next);
    if (date.month !== month || date.year !== year) {
      [year, month] = [date.year, date.month];
      loadMonth();
    }
    await open(next);
  }

  async function back() {
    selected = null;
    onOpenDay(null);
    day = null;
    await loadMonth();
  }

  async function moveDrill(event: SubmitEvent) {
    event.preventDefault();
    if (!moving || selected === null) return;
    try {
      await api.moveDrill(moving.id, dayFromIso(moveTo));
      moving = null;
      await open(selected);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  const outcome = (struggle: StruggleView) => {
    const value = struggle.outcome;
    if (value === null) return "Not reviewed yet";
    if (value === "Card") return "Card suggested";
    if (value === "Nothing") return "Kept as a note";
    if ("Checklist" in value) return `Added to the ${value.Checklist.name} checklist`;
    return `${struggle.issue ? `Issue: ${struggle.issue} · ` : ""}drill ${value.Drill.due === null ? "ticked off" : onDay(value.Drill.due, today)}`;
  };
  const kindLabel = { Recall: "Couldn't recall", Apply: "Couldn't apply", Unsure: "Not sure yet" } as const;
  const first = $derived(day ? (day.drills.length ? "drills" : day.cards.due.new + day.cards.due.learning + day.cards.due.review > 0 && day.when === "Today" ? "cards" : day.checklists.length ? "recall" : day.quizzes.length ? "quizzes" : null) : null);
  const total = $derived(day ? day.drills.length + (day.when === "Today" ? day.cards.due.new + day.cards.due.learning + day.cards.due.review : day.cards.due.review) + day.checklists.length + day.quizzes.length : 0);

  load();
</script>

{#if selected === null}
  <div class="mx-auto max-w-6xl px-12 pb-24">
    <PageHeader title="Calendar" subtitle="Everything due, day by day. Past days show what you did.">
      <button class="btn btn-ghost btn-sm btn-square" aria-label="Previous month" onclick={() => shift(-1)}><ChevronLeft class="size-4" /></button>
      <span class="w-40 text-center font-semibold" aria-live="polite">{year ? monthLabel(year, month) : ""}</span>
      <button class="btn btn-ghost btn-sm btn-square" aria-label="Next month" onclick={() => shift(1)}><ChevronRight class="size-4" /></button>
      <button class="btn btn-outline btn-sm ml-2 border-field-border" onclick={() => { const now = dateOf(today); [year, month] = [now.year, now.month]; loadMonth(); }}>Today</button>
    </PageHeader>
    <p class="mb-3 text-right text-xs text-muted">Future cards are a forecast and move as you review</p>
    <div class="panel overflow-hidden">
      <div class="grid grid-cols-7 border-b border-base-300 text-xs font-semibold text-muted" aria-hidden="true">{#each WEEKDAYS as name (name)}<div class="px-2 py-2">{name}</div>{/each}</div>
      <div class="grid grid-cols-7" role="grid" aria-label={monthLabel(year, month)}>
        {#each grid as value (value)}
          {@const entry = days.find((d) => d.day === value)}
          {@const date = dateOf(value)}
          <button class="cal-day {date.month !== month ? 'other' : ''}" aria-current={value === today ? "date" : undefined}
            aria-label="{longDate(value)}{entry && counts(entry).length ? `: ${counts(entry).map((c) => `${c.count} ${c.count === 1 ? c.one : c.many}`).join(", ")}${entry.when === 'Past' ? ' done' : ' due'}` : ''}"
            onclick={() => open(value)}>
            <span class="num">{date.date}</span>
            {#if entry}{#each counts(entry) as item (item.key)}<span class="cal-row"><item.icon class="size-3.5" /><b>{item.count}</b> {item.count === 1 ? item.one : item.many}</span>{/each}{/if}
          </button>
        {/each}
      </div>
    </div>
  </div>
{:else if day}
  <div class="mx-auto max-w-4xl px-12 pb-24">
    <button class="mt-8 inline-flex items-center gap-1 py-1 text-sm text-muted hover:underline" onclick={back}><ArrowLeft class="size-4" />{monthLabel(year, month)}</button>
    <div class="flex items-end justify-between gap-6 pb-8 pt-4">
      <div><h1 class="text-3xl font-semibold">{longDate(day.day)}</h1>
        <p class="mt-2 text-muted">{day.when === "Past" ? "What you did" : total ? `${total} ${total === 1 ? "thing" : "things"} due${day.when === "Today" && day.cards.minutes ? ` · about ${day.cards.minutes + day.drills.length * 10} minutes` : ""}` : "Nothing due"}</p></div>
      <div class="flex gap-2"><button class="btn btn-ghost btn-sm btn-square" aria-label="Previous day" onclick={() => step(-1)}><ChevronLeft class="size-4" /></button><button class="btn btn-ghost btn-sm btn-square" aria-label="Next day" onclick={() => step(1)}><ChevronRight class="size-4" /></button></div>
    </div>

    {#if day.when === "Today" && first}
      <div class="mb-6">
        {#if first === "drills"}<button class="btn btn-primary" onclick={() => onDrills(day!.drills.map((d) => d.id))}><Play class="size-4" />Start with the drills</button>
        {:else if first === "cards"}<button class="btn btn-primary" onclick={onCards}><Play class="size-4" />Start with the cards</button>
        {:else if first === "recall"}<button class="btn btn-primary" onclick={() => onRecall(day!.checklists[0].id)}><Play class="size-4" />Start with recall</button>
        {:else}<button class="btn btn-primary" onclick={() => onQuiz(day!.quizzes.map((q) => q.topic))}><Play class="size-4" />Start with the quizzes</button>{/if}
      </div>
    {/if}
    {#if error}<p role="alert" class="mb-4 text-sm text-error">{error}</p>{/if}

    <div class="space-y-6">
      {#if day.drills.length || day.drill_runs.length}
        <section class="panel p-6" aria-labelledby="h-drills"><h2 id="h-drills" class="flex items-center gap-2 text-sm font-semibold"><Dumbbell class="size-4" />Drills</h2>
          <ul class="mt-3 divide-y">
            {#each day.drills as drill (drill.id)}
              <li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px]">{drill.task}</p><p class="text-sm text-muted">{drill.issue ? `Issue: ${drill.issue}` : "No issue"}{drill.from[0] ? ` · from ${drill.from[0].project_name}` : ""}{drill.last && drill.last.rating !== "Solid" ? ` · again, it was ${drill.last.rating}` : ""}{drill.due !== null && drill.due < day.day ? ` · was due ${dueLabel(drill.due, drill.due)}` : ""}</p></div>
                <div class="flex shrink-0 gap-2">
                  <button class="btn btn-ghost btn-sm" onclick={() => { moving = drill; moveTo = isoOf(Math.max(today + 1, day!.day + 1)); }}><CalendarArrowUp class="size-4" />Move</button>
                  {#if day.when === "Today"}<button class="btn btn-outline btn-sm border-field-border" onclick={() => onDrills([drill.id])}><Play class="size-4" />Do drill</button>{/if}
                </div></li>
            {/each}
            {#each day.drill_runs as run, index (index)}<li class="py-3"><p class="text-[15px]">{run.task}</p><p class="text-sm text-muted">Rated {run.rating}{run.rating === "Solid" ? ", ticked off" : ", back the next day"}</p></li>{/each}
          </ul></section>
      {/if}

      {#if day.when === "Past" ? day.cards.reviewed : day.cards.due.new + day.cards.due.learning + day.cards.due.review}
        <section class="panel p-6" aria-labelledby="h-cards"><h2 id="h-cards" class="flex items-center gap-2 text-sm font-semibold"><SquareStack class="size-4" />Cards</h2>
          <div class="mt-3 flex items-center justify-between gap-4 py-2">
            {#if day.when === "Past"}<div><p class="text-[15px]">{day.cards.reviewed} {day.cards.reviewed === 1 ? "card" : "cards"} reviewed</p><p class="text-sm text-muted">{day.cards.reviewed - day.cards.again} remembered · {day.cards.again} {day.cards.again === 1 ? "time" : "times"} Again · {Math.max(1, Math.round(day.cards.seconds / 60))} min</p></div>
            {:else if day.when === "Today"}<div><p class="text-[15px]">{day.cards.due.new + day.cards.due.learning + day.cards.due.review} cards due</p><p class="text-sm text-muted">{day.cards.due.new} new · {day.cards.due.learning} learning · {day.cards.due.review} to review · about {day.cards.minutes} minutes</p></div>
              <button class="btn btn-outline btn-sm border-field-border" onclick={onCards}><Play class="size-4" />Start review</button>
            {:else}<div><p class="text-[15px]">About {day.cards.due.review} {day.cards.due.review === 1 ? "card" : "cards"}</p><p class="text-sm text-muted">A forecast, it changes as you review</p></div>{/if}
          </div></section>
      {/if}

      {#if day.checklists.length || day.checks.length}
        <section class="panel p-6" aria-labelledby="h-recall"><h2 id="h-recall" class="flex items-center gap-2 text-sm font-semibold"><Brain class="size-4" />Recall</h2>
          <ul class="mt-3 divide-y">
            {#each day.checklists as checklist (checklist.id)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px]">{checklist.name}</p><p class="text-sm text-muted">Checklist · {checklist.step ? `${checklist.step} step` : "first check"} · {checklist.points} points</p></div>
              {#if day.when === "Today"}<button class="btn btn-outline btn-sm border-field-border" onclick={() => onRecall(checklist.id)}><Play class="size-4" />Start recall</button>{/if}</li>{/each}
            {#each day.checks as check, index (index)}<li class="py-3"><p class="text-[15px]">{check.name}</p><p class="text-sm text-muted">{check.score}% · {check.passed ? "passed" : "under the pass mark"} · now on {check.to}</p></li>{/each}
          </ul></section>
      {/if}

      {#if day.quizzes.length || day.attempts.length}
        <section class="panel p-6" aria-labelledby="h-quizzes"><h2 id="h-quizzes" class="flex items-center gap-2 text-sm font-semibold"><ListChecks class="size-4" />Quizzes</h2>
          <ul class="mt-3 divide-y">
            {#each day.quizzes as quiz (quiz.topic)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px]">{quiz.name}</p><p class="text-sm text-muted">Quiz due · {quiz.step} step · {quiz.questions} {quiz.questions === 1 ? "question" : "questions"}</p></div>
              {#if day.when === "Today"}<button class="btn btn-outline btn-sm border-field-border" onclick={() => onQuiz([quiz.topic])}><Play class="size-4" />Quiz</button>{/if}</li>{/each}
            {#if day.when === "Today" && day.quizzes.length > 1}<li class="flex justify-end pt-3"><button class="btn btn-ghost btn-sm" onclick={() => onQuiz(day!.quizzes.map((q) => q.topic))}><Play class="size-4" />Quiz all {day.quizzes.length} together</button></li>{/if}
            {#each day.attempts as attempt (attempt.attempt)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px]">Quiz: {attempt.topics.join(", ")}</p><p class="text-sm text-muted">{attempt.right} of {attempt.total} right{attempt.moves.length ? ` · ${attempt.moves.join(" · ")}` : ""}</p></div>
              <button class="btn btn-ghost btn-sm" onclick={() => onResults(attempt.attempt)}>View results</button></li>{/each}
          </ul></section>
      {/if}

      {#if day.struggles.length}
        <section class="panel p-6" aria-labelledby="h-struggles"><h2 id="h-struggles" class="flex items-center gap-2 text-sm font-semibold"><TriangleAlert class="size-4" />Struggles logged</h2>
          <ul class="mt-3 divide-y">
            {#each day.struggles as struggle (struggle.id)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px]">{struggle.what}</p><p class="text-sm text-muted">{struggle.project_name} · {kindLabel[struggle.kind]} · {outcome(struggle)}</p></div>
              <button class="btn btn-ghost btn-sm" onclick={() => onProject(struggle.project)}>Open project</button></li>{/each}
          </ul></section>
      {/if}

      {#if !total && !day.cards.reviewed && !day.checks.length && !day.attempts.length && !day.drill_runs.length && !day.struggles.length}
        <p class="panel p-8 text-muted">{day.when === "Past" ? "Nothing was done on this day." : "Nothing is due on this day yet."}</p>
      {/if}
    </div>
  </div>
{/if}

{#if moving}
  <dialog open class="modal modal-open" aria-labelledby="move-h">
    <form class="modal-box max-w-md" onsubmit={moveDrill}>
      <h3 id="move-h" class="text-lg font-semibold">Move this drill</h3>
      <p class="mt-2 text-sm text-muted">{moving.task}</p>
      <label class="mt-5 block"><span class="text-sm font-semibold">Do it on</span><input type="date" class="input mt-1 w-full" min={isoOf(today)} bind:value={moveTo} /></label>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (moving = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Move to {moveTo ? dueLabel(dayFromIso(moveTo), today) : "that day"}</button>
      </div>
    </form>
  </dialog>
{/if}

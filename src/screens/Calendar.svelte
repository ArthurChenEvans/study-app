<script lang="ts">
  // calendar.feature (R98, R99): what's due each day, and what was done on past days.
  // dates.feature (R130 to R137): exams, deadlines and other dates, and what they tighten.
  import { ArrowLeft, Brain, CalendarArrowUp, ChevronLeft, ChevronRight, Dumbbell, Flag, GraduationCap, ListChecks, Pencil, Play, Plus, SquareStack, TriangleAlert } from "@lucide/svelte";
  import { untrack } from "svelte";
  import { api, errorMessage, type CalendarDay, type DateView, type DayView, type DrillView, type Id, type ModuleRow, type StruggleView, type TopicQuizView, type TopicRow } from "../lib/api";
  import { toast } from "../lib/celebrations.svelte";
  import DateDialog from "../components/DateDialog.svelte";
  import MoveDrillDialog from "../components/MoveDrillDialog.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { dateOf, dueLabel, fullLabel, monthGrid, monthLabel, onDay } from "../lib/dates";

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
  let error = $state("");
  let dates = $state<DateView[]>([]);
  /** The date dialog: "new" to add one, a date to edit it. */
  let editing = $state<DateView | "new" | null>(null);
  let dialogData = $state<{ modules: ModuleRow[]; topics: TopicRow[]; quizzes: TopicQuizView[] } | null>(null);

  const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const WEEKDAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
  /** A cell shows at most this many kinds, then "+N more", so busy days stay readable. */
  const MAX_LINES = 3;
  const grid = $derived(year ? monthGrid(year, month) : []);
  const weeks = $derived(Array.from({ length: Math.ceil(grid.length / 7) }, (_, index) => grid.slice(index * 7, index * 7 + 7)));
  let gridEl = $state<HTMLDivElement>();

  // Arrow keys move a day or a week, as in any date grid.
  function gridKeys(event: KeyboardEvent, value: number) {
    const by = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 }[event.key];
    if (by === undefined) return;
    const target = gridEl?.querySelector<HTMLButtonElement>(`[data-day="${value + by}"]`);
    if (!target) return;
    event.preventDefault();
    target.focus();
  }
  const counts = (entry: CalendarDay) => [
    { key: "cards", icon: SquareStack, count: entry.cards, one: "card", many: "cards" },
    { key: "checklists", icon: Brain, count: entry.checklists, one: "checklist", many: "checklists" },
    { key: "quizzes", icon: ListChecks, count: entry.quizzes, one: "quiz", many: "quizzes" },
    { key: "drills", icon: Dumbbell, count: entry.drills, one: "drill", many: "drills" },
    { key: "struggles", icon: TriangleAlert, count: entry.struggles, one: "struggle", many: "struggles" },
  ].filter((item) => item.count > 0);
  const longDate = (value: number) => (value === today ? `Today, ${fullLabel(value, today)}` : fullLabel(value, today));
  const isExam = (date: DateView) => /exam/i.test(date.kind);
  /** "today", "tomorrow", "in 10 days", "3 days ago". */
  const inDays = (left: number) => (left === 0 ? "today" : left === 1 ? "tomorrow" : left > 0 ? `in ${left} days` : left === -1 ? "yesterday" : `${-left} days ago`);
  const comingUp = $derived(dates.filter((d) => d.days_left >= 0).slice(0, 3));

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
    [days, dates] = await Promise.all([api.calendarMonth(grid[0], grid[grid.length - 1]), api.dates()]);
  }

  async function openDialog(date: DateView | "new") {
    error = "";
    if (!dialogData) {
      const [modules, topics, quizzes] = await Promise.all([api.modules(), api.topics(), api.topicQuizzes()]);
      dialogData = { modules, topics, quizzes };
    }
    editing = date;
  }

  async function dateSaved() {
    editing = null;
    dialogData = null;
    if (selected !== null) await open(selected);
    await loadMonth();
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

  async function moveDrill(to: number) {
    if (!moving || selected === null) return;
    try {
      await api.moveDrill(moving.id, to);
      toast({ text: `Drill moved to ${fullLabel(to, today)}`, icon: "calendar" });
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
      <span class="w-44 text-center text-[19px] font-extrabold" aria-live="polite">{year ? monthLabel(year, month) : ""}</span>
      <button class="btn btn-ghost btn-sm btn-square" aria-label="Next month" onclick={() => shift(1)}><ChevronRight class="size-4" /></button>
      <button class="btn btn-outline btn-sm ml-2" onclick={() => { const now = dateOf(today); [year, month] = [now.year, now.month]; loadMonth(); }}>Today</button>
      <button class="btn btn-primary btn-sm ml-2" onclick={() => openDialog("new")}><Plus class="size-4" />Add a date</button>
    </PageHeader>
    {#if comingUp.length}
      <section class="mb-4 flex flex-wrap items-center gap-x-6 gap-y-2" aria-label="Coming up">
        <span class="eyebrow">Coming up</span>
        {#each comingUp as date (date.id)}
          <button class="flex min-h-6 items-center gap-2 py-1 text-sm hover:underline" onclick={() => open(date.day)}>
            {#if isExam(date)}<GraduationCap class="size-4 text-primary-ink" aria-hidden="true" />{:else}<Flag class="size-4 text-primary-ink" aria-hidden="true" />{/if}
            <b class="font-extrabold">{date.name}</b>{" "}<span class="text-muted">{fullLabel(date.day, today)}, {inDays(date.days_left)}</span>
            {#if date.tighten}{" "}<span class="chip chip-primary">Reviews tightened</span>{/if}
          </button>
        {/each}
      </section>
    {/if}
    <!-- Each day is a tile, each kind of work a chip. Today stands on an orange ledge, past days sit flat. -->
    <div>
      <div bind:this={gridEl} role="grid" aria-label={monthLabel(year, month)} class="flex flex-col gap-2">
        <div role="row" class="grid grid-cols-7 gap-2 text-xs font-extrabold uppercase tracking-[.07em] text-muted">
          {#each WEEKDAYS as name, index (name)}<div role="columnheader" class="px-2.5"><abbr title={WEEKDAY_NAMES[index]} class="no-underline" aria-label={WEEKDAY_NAMES[index]}>{name}</abbr></div>{/each}
        </div>
        {#each weeks as week (week[0])}
          <div role="row" class="grid grid-cols-7 gap-2">
            {#each week as value (value)}
              {@const entry = days.find((d) => d.day === value)}
              {@const date = dateOf(value)}
              {@const items = entry ? counts(entry) : []}
              {@const onThisDay = dates.filter((d) => d.day === value)}
              {@const room = Math.max(1, MAX_LINES - onThisDay.length)}
              {@const shown = items.length > room ? items.slice(0, room - 1) : items}
              <div role="gridcell" class="cal-cell">
                <button class="cal-day {date.month !== month ? 'other' : ''} {value < today ? 'past' : ''}" data-day={value} tabindex={value === (selected ?? today) ? 0 : -1} aria-current={value === today ? "date" : undefined}
                  aria-label="{longDate(value)}{onThisDay.map((d) => `, ${d.name}`).join('')}{items.length ? `: ${items.map((c) => `${c.count} ${c.count === 1 ? c.one : c.many}`).join(", ")}${entry?.when === 'Past' ? ' done' : ' due'}` : ''}"
                  onclick={() => open(value)} onkeydown={(event) => gridKeys(event, value)}>
                  <span class="flex items-center gap-2"><span class="num">{date.date}</span>{#if value === today}<span class="cal-today">Today</span>{/if}</span>
                  {#each onThisDay as marked (marked.id)}<span class="date-chip {isExam(marked) ? 'exam' : ''}">{#if isExam(marked)}<GraduationCap class="size-3.5 shrink-0" aria-hidden="true" />{:else}<Flag class="size-3.5 shrink-0" aria-hidden="true" />{/if}<span class="truncate">{marked.name}</span></span>{/each}
                  {#each shown as item (item.key)}<span class="chip cal-chip {value < today ? '' : value === today ? 'chip-due' : `cal-${item.key}`}">{item.count} {item.count === 1 ? item.one : item.many}</span>{/each}
                  {#if shown.length < items.length}<span class="chip cal-chip">+{items.length - shown.length} more</span>{/if}
                </button>
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
    <p class="mt-3 text-[13px] text-muted">Future cards are a forecast and move as you review.</p>
  </div>
{:else if day}
  <div class="mx-auto max-w-4xl px-12 pb-24">
    <button class="mt-8 inline-flex items-center gap-1 py-1 text-xs font-extrabold uppercase tracking-wider text-muted hover:underline" onclick={back}><ArrowLeft class="size-4" />{monthLabel(year, month)}</button>
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
      {#each day.soon as date (date.id)}
        <section class="panel flex items-center gap-3 p-5" aria-label="{date.name} coming up">
          <span class="well size-10" aria-hidden="true">{#if isExam(date)}<GraduationCap class="size-5" />{:else}<Flag class="size-5" />{/if}</span>
          <p class="text-[15px]"><b class="font-extrabold">{date.name}</b> in {date.day - day.day} {date.day - day.day === 1 ? "day" : "days"}, on {fullLabel(date.day, today)}{date.tighten ? ". Reviews for it are tightened" : ""}</p>
        </section>
      {/each}
      {#each day.dates as panel (panel.date.id)}
        <section class="panel p-6" aria-labelledby="h-date-{panel.date.id}">
          <div class="flex items-start justify-between gap-4">
            <div class="flex items-center gap-3"><span class="well size-10" aria-hidden="true">{#if isExam(panel.date)}<GraduationCap class="size-5" />{:else}<Flag class="size-5" />{/if}</span>
              <div><h2 id="h-date-{panel.date.id}" class="text-lg font-black">{panel.date.name}</h2>
                <p class="text-sm text-muted">{[panel.date.kind, panel.date.covers_label, inDays(panel.date.days_left)].filter(Boolean).join(" · ")}</p></div></div>
            <button class="btn btn-ghost btn-sm" onclick={() => openDialog(panel.date)}><Pencil class="size-4" />Edit</button>
          </div>
          {#if panel.quizzes.length || panel.unquizzed.length || panel.cards_before}
            <h3 class="mt-5 text-sm font-extrabold">{panel.date.tighten ? "Before it, with reviews tightened" : "Before it"}</h3>
            <ul class="mt-2 divide-y">
              {#each panel.quizzes as quiz (quiz.topic)}
                <li class="py-3"><p class="text-[15px] font-extrabold">{quiz.name}</p><p class="text-sm text-muted">{quiz.step} step · next quiz {fullLabel(quiz.to, today)}{quiz.to < quiz.from ? `, pulled in from ${fullLabel(quiz.from, today)}` : ""}</p></li>
              {/each}
              {#each panel.unquizzed as topic (topic.topic)}
                <li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px] font-extrabold">{topic.name}</p><p class="text-sm text-muted">Not quizzed yet, so nothing to tighten</p></div>
                  {#if topic.questions}<button class="btn btn-outline btn-sm text-primary-ink" onclick={() => onQuiz([topic.topic])}><Play class="size-4" />Quiz it</button>{/if}</li>
              {/each}
              {#if panel.cards_before}<li class="py-3"><p class="text-[15px] font-extrabold">{panel.cards_before} {panel.cards_before === 1 ? "card" : "cards"} due before it</p>{#if panel.cards_pulled}<p class="text-sm text-muted">{panel.cards_pulled} of them pulled in from after it</p>{/if}</li>{/if}
            </ul>
            {#if panel.date.tighten}<p class="mt-4 text-sm text-muted">From the day after, these go back to their normal schedules.</p>{/if}
          {:else if !panel.date.topics.length}
            <p class="mt-3 text-sm text-muted">It doesn't cover any topics, so nothing is tightened for it.</p>
          {/if}
        </section>
      {/each}
      {#if day.drills.length || day.drill_runs.length}
        <section class="panel p-6" aria-labelledby="h-drills"><h2 id="h-drills" class="flex items-center gap-3 text-lg font-black"><span class="well size-10" aria-hidden="true"><Dumbbell class="size-5" /></span>Drills</h2>
          <ul class="mt-3 divide-y">
            {#each day.drills as drill (drill.id)}
              <li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px] font-extrabold">{drill.task}</p><p class="text-sm text-muted">{drill.issue ? `Issue: ${drill.issue}` : "No issue"}{drill.from[0] ? ` · from ${drill.from[0].project_name}` : ""}{drill.last && drill.last.rating !== "Solid" ? ` · again, it was ${drill.last.rating}` : ""}{drill.due !== null && drill.due < day.day ? ` · was due ${dueLabel(drill.due, drill.due)}` : ""}</p></div>
                <div class="flex shrink-0 gap-2">
                  <button class="btn btn-ghost btn-sm" onclick={() => { error = ""; moving = drill; }}><CalendarArrowUp class="size-4" />Move</button>
                  {#if day.when === "Today"}<button class="btn btn-outline btn-sm text-primary-ink" onclick={() => onDrills([drill.id])}><Play class="size-4" />Do drill</button>{/if}
                </div></li>
            {/each}
            {#each day.drill_runs as run, index (index)}<li class="py-3"><p class="text-[15px] font-extrabold">{run.task}</p><p class="text-sm text-muted">Rated {run.rating}{run.rating === "Solid" ? ", ticked off" : ", back the next day"}</p></li>{/each}
          </ul></section>
      {/if}

      {#if day.when === "Past" ? day.cards.reviewed : day.cards.due.new + day.cards.due.learning + day.cards.due.review}
        <section class="panel p-6" aria-labelledby="h-cards"><h2 id="h-cards" class="flex items-center gap-3 text-lg font-black"><span class="well size-10" aria-hidden="true"><SquareStack class="size-5" /></span>Cards</h2>
          <div class="mt-3 flex items-center justify-between gap-4 py-2">
            {#if day.when === "Past"}<div><p class="text-[15px] font-extrabold">{day.cards.reviewed} {day.cards.reviewed === 1 ? "card" : "cards"} reviewed</p><p class="text-sm text-muted">{day.cards.reviewed - day.cards.again} remembered · {day.cards.again} {day.cards.again === 1 ? "time" : "times"} Again · {Math.max(1, Math.round(day.cards.seconds / 60))} min</p></div>
            {:else if day.when === "Today"}<div><p class="text-[15px] font-extrabold">{day.cards.due.new + day.cards.due.learning + day.cards.due.review} cards due</p><p class="text-sm text-muted">{day.cards.due.new} new · {day.cards.due.learning} learning · {day.cards.due.review} to review · about {day.cards.minutes} minutes</p></div>
              <button class="btn btn-outline btn-sm text-primary-ink" onclick={onCards}><Play class="size-4" />Start review</button>
            {:else}<div><p class="text-[15px] font-extrabold">About {day.cards.due.review} {day.cards.due.review === 1 ? "card" : "cards"}</p><p class="text-sm text-muted">A forecast, it changes as you review</p></div>{/if}
          </div></section>
      {/if}

      {#if day.checklists.length || day.checks.length}
        <section class="panel p-6" aria-labelledby="h-recall"><h2 id="h-recall" class="flex items-center gap-3 text-lg font-black"><span class="well size-10" aria-hidden="true"><Brain class="size-5" /></span>Recall</h2>
          <ul class="mt-3 divide-y">
            {#each day.checklists as checklist (checklist.id)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px] font-extrabold">{checklist.name}</p><p class="text-sm text-muted">Checklist · {checklist.step ? `${checklist.step} step` : "first check"} · {checklist.points} points</p></div>
              {#if day.when === "Today"}<button class="btn btn-outline btn-sm text-primary-ink" onclick={() => onRecall(checklist.id)}><Play class="size-4" />Start recall</button>{/if}</li>{/each}
            {#each day.checks as check, index (index)}<li class="py-3"><p class="text-[15px] font-extrabold">{check.name}</p><p class="text-sm text-muted">{check.score}% · {check.passed ? "passed" : "under the pass mark"} · now on {check.to}</p></li>{/each}
          </ul></section>
      {/if}

      {#if day.quizzes.length || day.attempts.length}
        <section class="panel p-6" aria-labelledby="h-quizzes"><h2 id="h-quizzes" class="flex items-center gap-3 text-lg font-black"><span class="well size-10" aria-hidden="true"><ListChecks class="size-5" /></span>Quizzes</h2>
          <ul class="mt-3 divide-y">
            {#each day.quizzes as quiz (quiz.topic)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px] font-extrabold">{quiz.name}</p><p class="text-sm text-muted">Quiz due · {quiz.step} step · {quiz.questions} {quiz.questions === 1 ? "question" : "questions"}{quiz.tightened_for ? ` · tightened for the ${quiz.tightened_for}` : ""}</p></div>
              {#if day.when === "Today"}<button class="btn btn-outline btn-sm text-primary-ink" onclick={() => onQuiz([quiz.topic])}><Play class="size-4" />Quiz</button>{/if}</li>{/each}
            {#if day.waiting.length}<li class="py-3"><p class="text-[15px] font-extrabold">{day.waiting.length} more waiting</p><p class="text-sm text-muted">Only {day.daily} topic quizzes come due a day, most overdue first. They keep their step{day.clears_by ? `, and the last comes in ${onDay(day.clears_by, today)}` : ""}.</p></li>{/if}
            {#if day.when === "Today" && day.quizzes.length > 1}<li class="flex justify-end pt-3"><button class="btn btn-ghost btn-sm" onclick={() => onQuiz(day!.quizzes.map((q) => q.topic))}><Play class="size-4" />Quiz all {day.quizzes.length} together</button></li>{/if}
            {#each day.attempts as attempt (attempt.attempt)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px] font-extrabold">Quiz: {attempt.topics.join(", ")}</p><p class="text-sm text-muted">{attempt.right} of {attempt.total} right{attempt.moves.length ? ` · ${attempt.moves.join(" · ")}` : ""}</p></div>
              <button class="btn btn-ghost btn-sm" onclick={() => onResults(attempt.attempt)}>View results</button></li>{/each}
          </ul></section>
      {/if}

      {#if day.struggles.length}
        <section class="panel p-6" aria-labelledby="h-struggles"><h2 id="h-struggles" class="flex items-center gap-3 text-lg font-black"><span class="well size-10" aria-hidden="true"><TriangleAlert class="size-5" /></span>Struggles logged</h2>
          <ul class="mt-3 divide-y">
            {#each day.struggles as struggle (struggle.id)}<li class="flex items-center justify-between gap-4 py-3"><div><p class="text-[15px] font-extrabold">{struggle.what}</p><p class="text-sm text-muted">{struggle.project_name} · {kindLabel[struggle.kind]} · {outcome(struggle)}</p></div>
              <button class="btn btn-ghost btn-sm" onclick={() => onProject(struggle.project)}>Open project</button></li>{/each}
          </ul></section>
      {/if}

      {#if !total && !day.dates.length && !day.cards.reviewed && !day.checks.length && !day.attempts.length && !day.drill_runs.length && !day.struggles.length}
        <p class="panel p-8 text-muted">{day.when === "Past" ? "Nothing was done on this day." : "Nothing is due on this day yet."}</p>
      {/if}
    </div>
  </div>
{/if}

{#if editing && dialogData}
  <DateDialog {today} date={editing === "new" ? null : editing} startDay={editing === "new" ? selected : null} modules={dialogData.modules} topics={dialogData.topics} quizzes={dialogData.quizzes} onSaved={dateSaved} onCancel={() => (editing = null)} />
{/if}

{#if moving && selected !== null}
  <MoveDrillDialog task={moving.task} {today} current={moving.due} {error} onMove={moveDrill} onCancel={() => (moving = null)} />
{/if}

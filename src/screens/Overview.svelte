<script lang="ts">
  // overview.feature (R33): progress from saved attempts only.
  import { Brain, Flag, Hammer, Library, PencilLine, Play, SquareStack, Target, TrendingDown, TrendingUp, ArrowRight, Zap } from "@lucide/svelte";
  import { api, type DueToday, type Id, type Overview, type Tally, type TopicQuizView, type TopicRow } from "../lib/api";
  import { dayLabel } from "../lib/dates";
  import type { Screen } from "../lib/router";
  import PageHeader from "../components/PageHeader.svelte";
  import ScoreChart from "../components/ScoreChart.svelte";
  import SegBar from "../components/SegBar.svelte";
  import { WEEKLY_AFTER, weekly } from "../lib/weekly";

  let { onNavigate, onQuizTopics = () => onNavigate("setup") }: { onNavigate: (screen: Screen) => void; onQuizTopics?: (topics: Id[]) => void } = $props();

  let overview = $state<Overview | null>(null);
  let topics = $state<TopicRow[]>([]);
  let today = $state(0);
  // What's due leads (design/playful-redesign: Overview): a banner for due quizzes and a list
  // of everything due, so the sidebar counts become the first thing you act on.
  let due = $state<DueToday | null>(null);
  let quizzes = $state<TopicQuizView[]>([]);
  const dueTopics = $derived(quizzes.filter((quiz) => quiz.due_today && topics.some((topic) => topic.id === quiz.topic && topic.question_count > 0)).map((quiz) => quiz.topic));
  const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;
  const together = (names: string[]) => (names.length <= 1 ? names.join("") : names.length === 2 ? `${names[0]} and ${names[1]}` : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`);
  const dueRows = $derived(
    due
      ? ([
          { count: due.quizzes, text: `Quiz ${plural(due.quizzes, "topic")}`, screen: "topics", icon: Library, tone: "info" },
          { count: due.cards, text: `Review ${plural(due.cards, "card")}`, screen: "cards", icon: SquareStack, tone: "primary" },
          { count: due.checklists, text: `Recall ${plural(due.checklists, "checklist")}`, screen: "recall", icon: Brain, tone: "due" },
          { count: due.drills, text: `Do ${plural(due.drills, "project drill")}`, screen: "projects", icon: Hammer, tone: "info" },
        ] as const).filter((row) => row.count > 0)
      : [],
  );

  const progress = $derived(overview && overview !== "FirstRun" ? overview.Progress : null);
  const right = (tally: Tally) => tally.right_sure + tally.right_guessed;
  const total = (tally: Tally) => right(tally) + tally.wrong + tally.unanswered;
  // lists.feature (R149): the 8 weakest, narrowed to one module if you like.
  let weakModule = $state("all");
  const moduleOf = (id: number) => topics.find((topic) => topic.id === id)?.module ?? null;
  let modules = $state<{ id: number; name: string }[]>([]);
  Promise.resolve().then(() => api.modules()).then((rows) => (modules = rows ?? [])).catch(() => {});

  const name = (id: number) => topics.find((topic) => topic.id === id)?.name ?? "Deleted topic";
  const maxReason = $derived(Math.max(1, ...(progress?.reasons.map(([, count]) => count) ?? [1])));
  const scores = $derived(progress?.scores ?? []);
  const byWeek = $derived(scores.length > WEEKLY_AFTER);
  const weeks = $derived(byWeek ? weekly(scores, today) : []);
  const weekPoints = $derived(weeks.map((week) => ({ label: dayLabel(week.start, today), percent: week.percent, retry: false, hollow: week.partial, detail: `${week.partial ? "This week so far, " : "Week of "}${week.quizzes} ${week.quizzes === 1 ? "quiz" : "quizzes"}` })));
  const quizPoints = $derived(progress?.scores.map((score) => ({ label: dayLabel(score.day, today), percent: score.percent, retry: score.retry, detail: score.retry ? "Retry" : "Quiz" })) ?? []);

  const steps: { icon: typeof Library; title: string; text: string; screen: Screen; action: string }[] = [
    { icon: Library, title: "Create a topic", text: "A topic holds questions on one subject, like CPU.", screen: "topics", action: "Create topic" },
    { icon: PencilLine, title: "Write questions", text: "Open your lecture slides beside the editor and write as you read.", screen: "topics", action: "Open topics" },
    { icon: Play, title: "Take a quiz", text: "Mix topics, pick study or exam mode, and see what sticks.", screen: "setup", action: "Set up quiz" },
  ];

  Promise.all([api.overview(), api.topics(), api.today()]).then(([data, topicRows, day]) => {
    overview = data;
    topics = topicRows;
    today = day;
  });
  Promise.all([api.dueToday(), api.topicQuizzes()]).then(([counts, rows]) => {
    due = counts;
    quizzes = rows ?? [];
  }).catch(() => {});
</script>

<div class="mx-auto max-w-7xl px-10 pb-24">
  {#if overview === "FirstRun"}
    <PageHeader title="Welcome" subtitle="Three steps to your first quiz" />
    <div class="grid grid-cols-1 gap-6 tab:grid-cols-3">
      {#each steps as step, index (step.title)}
        <div class="panel rise p-6">
          <span class="well" aria-hidden="true"><step.icon class="size-5" /></span>
          <p class="eyebrow mt-6">Step {index + 1}</p>
          <h2 class="mt-1 text-lg font-semibold">{step.title}</h2>
          <p class="mt-2 text-sm leading-relaxed text-muted">{step.text}</p>
          <button class="btn btn-sm mt-6 {index === 0 ? 'btn-primary' : 'btn-ghost'}" onclick={() => onNavigate(step.screen)}>{step.action}</button>
        </div>
      {/each}
    </div>
    <p class="mt-8 text-sm text-muted">Your progress charts appear here after your first quiz.</p>
  {:else if progress}
    <PageHeader title="Overview" subtitle="Last 7 days, compared with the 7 days before" />

    <div class="grid grid-cols-1 items-start gap-6 wide:grid-cols-[minmax(0,1fr)_20rem]">
      <div class="flex min-w-0 flex-col gap-4">
        <!-- The one primary: start what's due, or a new quiz when nothing is -->
        {#if dueTopics.length}
          <section class="hero rise" aria-labelledby="hero-h">
            <div>
              <p id="hero-h" class="eyebrow hero-eyebrow">{plural(dueTopics.length, "quiz", "quizzes")} due today</p>
              <h2 class="mt-1 text-[21px]">{together(dueTopics.map(name))} {dueTopics.length === 1 ? "is" : "are"} ready</h2>
            </div>
            <button class="btn hero-button" onclick={() => onQuizTopics(dueTopics)}><Play class="size-4 fill-current" />Start quiz</button>
          </section>
        {:else}
          <section class="hero rise" aria-labelledby="hero-h">
            <div>
              <p id="hero-h" class="eyebrow hero-eyebrow">Nothing due today</p>
              <h2 class="mt-1 text-[21px]">Quiz anything you like, or come back tomorrow</h2>
            </div>
            <button class="btn hero-button" onclick={() => onNavigate("setup")}><Play class="size-4 fill-current" />New quiz</button>
          </section>
        {/if}

        <section class="grid grid-cols-1 gap-4 tab:grid-cols-3">
          <div class="panel rise flex gap-3 p-4" role="region" aria-labelledby="tile-accuracy">
            <span class="well" aria-hidden="true"><Target class="size-5" /></span>
            <div>
              {#if progress.accuracy === null}
                <p class="text-[15px]">No quizzes in the last 7 days</p>
              {:else}
                <p class="text-[26px] font-black leading-none">{progress.accuracy}%</p>
              {/if}
              <p id="tile-accuracy" class="mt-1 text-[13px] text-muted">Accuracy</p>
              {#if progress.accuracy !== null && progress.change !== null}
                <p class="flex items-center gap-1 text-[13px] font-extrabold {progress.change >= 0 ? 'text-right-ink' : 'text-wrong-ink'}">
                  {#if progress.change >= 0}<TrendingUp class="size-4" />{:else}<TrendingDown class="size-4" />{/if}
                  {progress.change >= 0 ? "+" : ""}{progress.change} points <span class="sr-only">vs previous 7 days</span>
                </p>
              {/if}
            </div>
          </div>
          <div class="panel rise rise-2 flex gap-3 p-4" role="region" aria-labelledby="tile-answered">
            <span class="well" style="--well: var(--color-accent); --well-ink: var(--accent-ink)" aria-hidden="true"><Zap class="size-5 fill-current" /></span>
            <div>
              <p class="text-[26px] font-black leading-none">{progress.answered_last_7_days}</p>
              <p id="tile-answered" class="mt-1 text-[13px] text-muted">Questions answered</p>
              <p class="text-[13px] text-muted">across {plural(progress.quizzes_last_7_days, "quiz", "quizzes")}</p>
            </div>
          </div>
          <div class="panel rise rise-3 flex gap-3 p-4" role="region" aria-labelledby="tile-guessed">
            <span class="well" style="background: var(--guessed-tint); color: var(--guessed-ink)" aria-hidden="true"><Flag class="size-5 fill-current" /></span>
            <div>
              <p class="text-[26px] font-black leading-none">{progress.guessed_share === null ? "None" : `${progress.guessed_share}%`}</p>
              <p id="tile-guessed" class="mt-1 text-[13px] text-muted" title="Lower means you really know it">Right answers that were guesses</p>
            </div>
          </div>
        </section>

        <section class="panel rise rise-2 p-5">
          <div class="flex items-start justify-between gap-4">
            {#if byWeek}
              <div><h2 class="text-[18px]">Weekly average score</h2><p class="text-[13px] text-muted">{scores.length} quizzes since {dayLabel(scores[0].day, today)}, one point per week. Hollow is this week so far.</p></div>
            {:else}
              <div><h2 class="text-[18px]">Score per quiz</h2><p class="text-[13px] text-muted">% right. Hollow points are retries.</p></div>
            {/if}
            <button class="btn btn-link btn-sm text-[13px]" onclick={() => onNavigate("history")}>History <ArrowRight class="size-4" /></button>
          </div>
          <div class="mt-3">{#if byWeek}<ScoreChart points={weekPoints} title="Weekly average score" detailLabel="Quizzes" />{:else}<ScoreChart points={quizPoints} />{/if}</div>
        </section>
      </div>

      <aside class="flex flex-col gap-4">
        <section class="panel rise p-5">
          <div class="flex items-center justify-between"><h2 id="due-h" class="text-[18px]">Due today</h2><button class="btn btn-link btn-sm text-[13px]" onclick={() => onNavigate("calendar")}>Calendar</button></div>
          <ul class="mt-2" aria-labelledby="due-h">
            {#each dueRows as row (row.screen)}
              <li class="flex items-center gap-3 border-t-2 border-base-300 py-2.5 first:border-t-0"><span class="well chip-{row.tone}" style="--well: var(--chip); --well-ink: var(--chip-ink)" aria-hidden="true"><row.icon class="size-5" /></span><a href="#{row.screen}" class="font-extrabold hover:underline" onclick={(event) => { event.preventDefault(); onNavigate(row.screen); }}>{row.text}</a></li>
            {:else}
              <li class="py-2 text-sm text-muted">All done for today</li>
            {/each}
          </ul>
        </section>

        <section class="panel rise rise-3 p-5">
          <h2 id="weakest" class="text-[18px]">Topics, weakest first</h2>
          <div class="flex items-center justify-between gap-2"><p class="text-[13px] text-muted">Latest quiz on each topic</p>
            {#if modules.length > 1}<select class="select select-sm w-40" aria-label="Weakest topics in" bind:value={weakModule}><option value="all">All modules</option>{#each modules as m (m.id)}<option value={String(m.id)}>{m.name}</option>{/each}</select>{/if}</div>
          <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
            <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--chart-right)"></span>Right, sure</span>
            <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--guessed-fill)"></span>Guessed</span>
            <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--chart-wrong)"></span>Wrong</span>
          </div>
          <ul class="mt-4 space-y-3" aria-labelledby="weakest">
            {#each progress.weakest_topics.filter(([id]) => weakModule === "all" || String(moduleOf(id)) === weakModule).slice(0, 8) as [id, tally] (id)}
              <li>
                <div class="flex items-baseline justify-between text-sm"><span class="font-extrabold">{name(id)}</span>{" "}<span class="tabular-nums">{Math.round((right(tally) / Math.max(1, total(tally))) * 100)}% right</span></div>
                <div class="mt-1.5"><SegBar {tally} height={12} /></div>
              </li>
            {/each}
          </ul>
          {#if progress.weakest_topics.filter(([id]) => weakModule === "all" || String(moduleOf(id)) === weakModule).length > 8}<p class="mt-3 text-[13px] text-muted">The 8 weakest shown. Topics lists them all.</p>{/if}
          {#if progress.never_quizzed.length}<p class="mt-4 text-[13px] text-muted">Not quizzed yet: {progress.never_quizzed.map(name).join(", ")}</p>{/if}
        </section>

        <section class="panel rise rise-4 p-5">
          <h2 id="reasons" class="text-[18px]">Why you got things wrong</h2>
          <p class="text-[13px] text-muted">Reasons on wrong answers, last 30 days</p>
          {#if progress.reasons.length === 0}
            <p class="mt-4 text-sm text-muted">No reasons given yet. Add them on the results screen after a quiz.</p>
          {:else}
            <ul class="mt-4 space-y-3" aria-labelledby="reasons">
              {#each progress.reasons as [reason, count] (reason)}
                <li class="text-sm">
                  <span class="flex justify-between gap-2"><span class="truncate">{reason}</span>{" "}<span class="tabular-nums">{count}</span></span>
                  <span class="mt-1 block h-3 overflow-hidden rounded-full bg-base-300"><span class="block h-full rounded-full" style="width: {Math.max((count / maxReason) * 100, 2)}%; background: var(--chart-ink)"></span></span>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </aside>
    </div>
  {/if}
</div>

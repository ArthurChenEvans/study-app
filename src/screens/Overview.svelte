<script lang="ts">
  // overview.feature (R33): progress from saved attempts only.
  import { Library, PencilLine, Play, TrendingDown, TrendingUp, ArrowRight } from "@lucide/svelte";
  import { api, type Overview, type Tally, type TopicRow } from "../lib/api";
  import { dayLabel } from "../lib/dates";
  import type { Screen } from "../lib/router";
  import PageHeader from "../components/PageHeader.svelte";
  import ScoreChart from "../components/ScoreChart.svelte";
  import SegBar from "../components/SegBar.svelte";
  import { WEEKLY_AFTER, weekly } from "../lib/weekly";

  let { onNavigate }: { onNavigate: (screen: Screen) => void } = $props();

  let overview = $state<Overview | null>(null);
  let topics = $state<TopicRow[]>([]);
  let today = $state(0);

  const progress = $derived(overview && overview !== "FirstRun" ? overview.Progress : null);
  const right = (tally: Tally) => tally.right_sure + tally.right_guessed;
  const total = (tally: Tally) => right(tally) + tally.wrong + tally.unanswered;
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
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  {#if overview === "FirstRun"}
    <PageHeader title="Welcome" subtitle="Three steps to your first quiz" />
    <div class="grid grid-cols-3 gap-6">
      {#each steps as step, index (step.title)}
        <div class="panel rise p-6">
          <span class="grid size-10 place-items-center rounded-lg bg-base-200"><step.icon class="size-5" /></span>
          <p class="eyebrow mt-6">Step {index + 1}</p>
          <h2 class="mt-1 text-lg font-semibold">{step.title}</h2>
          <p class="mt-2 text-sm leading-relaxed text-muted">{step.text}</p>
          <button class="btn btn-sm mt-6 {index === 0 ? 'btn-primary' : 'btn-ghost'}" onclick={() => onNavigate(step.screen)}>{step.action}</button>
        </div>
      {/each}
    </div>
    <p class="mt-8 text-sm text-muted">Your progress charts appear here after your first quiz.</p>
  {:else if progress}
    <PageHeader title="Overview" subtitle="Last 7 days, compared with the 7 days before">
      <button class="btn btn-primary" onclick={() => onNavigate("setup")}><Play class="size-4" />New quiz</button>
    </PageHeader>

    <section class="grid grid-cols-[1.4fr_1fr_1fr] gap-6">
      <div class="panel rise p-6" role="region" aria-labelledby="tile-accuracy">
        <p id="tile-accuracy" class="text-sm text-muted">Accuracy</p>
        {#if progress.accuracy === null}
          <p class="mt-3 text-lg">No quizzes in the last 7 days</p>
        {:else}
          <p class="mt-3 text-5xl font-semibold leading-none">{progress.accuracy}%</p>
          {#if progress.change !== null}
            <p class="mt-3 flex items-center gap-1 text-sm font-semibold {progress.change >= 0 ? 'text-success' : 'text-error'}">
              {#if progress.change >= 0}<TrendingUp class="size-4" />{:else}<TrendingDown class="size-4" />{/if}
              {progress.change >= 0 ? "+" : ""}{progress.change} points <span class="font-normal text-muted">vs previous 7 days</span>
            </p>
          {/if}
        {/if}
      </div>
      <div class="panel rise rise-2 p-6" role="region" aria-labelledby="tile-answered">
        <p id="tile-answered" class="text-sm text-muted">Questions answered</p>
        <p class="mt-3 text-3xl font-semibold leading-none">{progress.answered_last_7_days}</p>
        <p class="mt-3 text-sm text-muted">across {progress.quizzes_last_7_days} quizzes</p>
      </div>
      <div class="panel rise rise-3 p-6" role="region" aria-labelledby="tile-guessed">
        <p id="tile-guessed" class="text-sm text-muted">Right answers that were guesses</p>
        <p class="mt-3 text-3xl font-semibold leading-none">{progress.guessed_share === null ? "None" : `${progress.guessed_share}%`}</p>
        <p class="mt-3 text-sm text-muted">Lower means you really know it</p>
      </div>
    </section>

    <section class="panel rise rise-2 mt-6 p-6">
      <div class="flex items-baseline justify-between">
        {#if byWeek}
          <div><h2 class="font-semibold">Weekly average score</h2><p class="text-sm text-muted">{scores.length} quizzes since {dayLabel(scores[0].day, today)}, one point per week. Hollow is this week so far.</p></div>
        {:else}
          <div><h2 class="font-semibold">Score per quiz</h2><p class="text-sm text-muted">% right. Hollow points are retries.</p></div>
        {/if}
        <button class="btn btn-ghost btn-sm" onclick={() => onNavigate("history")}>History <ArrowRight class="size-4" /></button>
      </div>
      <div class="mt-4">{#if byWeek}<ScoreChart points={weekPoints} title="Weekly average score" detailLabel="Quizzes" />{:else}<ScoreChart points={quizPoints} />{/if}</div>
    </section>

    <section class="mt-6 grid grid-cols-2 gap-6">
      <div class="panel rise rise-3 p-6">
        <h2 id="weakest" class="font-semibold">Topics, weakest first</h2>
        <p class="text-sm text-muted">Latest quiz on each topic</p>
        <div class="mt-4 flex items-center gap-4 text-xs text-muted">
          <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--chart-right)"></span>Right, sure</span>
          <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--guessed-fill)"></span>Right, guessed</span>
          <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--chart-wrong)"></span>Wrong</span>
        </div>
        <ul class="mt-5 space-y-5" aria-labelledby="weakest">
          {#each progress.weakest_topics as [id, tally] (id)}
            <li>
              <div class="flex items-baseline justify-between text-sm"><span class="font-semibold">{name(id)}</span>{" "}<span class="tabular-nums text-muted">{Math.round((right(tally) / Math.max(1, total(tally))) * 100)}% right</span></div>
              <div class="mt-2"><SegBar {tally} height={10} /></div>
            </li>
          {/each}
        </ul>
        {#if progress.never_quizzed.length}<p class="mt-5 text-sm text-muted">Not quizzed yet: {progress.never_quizzed.map(name).join(", ")}</p>{/if}
      </div>
      <div class="panel rise rise-4 p-6">
        <h2 id="reasons" class="font-semibold">Why you got things wrong</h2>
        <p class="text-sm text-muted">Reasons on wrong answers, last 30 days</p>
        {#if progress.reasons.length === 0}
          <p class="mt-5 text-sm text-muted">No reasons given yet. Add them on the results screen after a quiz.</p>
        {:else}
          <ul class="mt-5 space-y-3" aria-labelledby="reasons">
            {#each progress.reasons as [reason, count] (reason)}
              <li class="grid grid-cols-[220px_1fr] items-center gap-4 text-sm">
                <span class="truncate">{reason}</span>{" "}
                <span class="flex items-center gap-2"><span class="hbar" style="width: {Math.max((count / maxReason) * 100, 2)}%"></span><span class="text-xs tabular-nums text-muted">{count}</span></span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </section>
  {/if}
</div>

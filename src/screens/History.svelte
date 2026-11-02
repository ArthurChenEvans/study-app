<script lang="ts">
  // history.feature (R26, R27, R29). Resuming an incomplete attempt is open (Q13).
  import { CirclePause, History as HistoryIcon, Play, RotateCcw } from "@lucide/svelte";
  import { api, type HistoryRow, type Id, type Tally, type TopicRow } from "../lib/api";
  import { dayLabel } from "../lib/dates";
  import PageHeader from "../components/PageHeader.svelte";
  import ScoreChart from "../components/ScoreChart.svelte";
  import SegBar from "../components/SegBar.svelte";

  let { onView, onNewQuiz = () => {} }: { onView: (attempt: Id) => void; onNewQuiz?: () => void } = $props();

  let rows = $state<HistoryRow[]>([]);
  let topics = $state<TopicRow[]>([]);
  let today = $state(0);
  let loaded = $state(false);

  const right = (tally: Tally) => tally.right_sure + tally.right_guessed;
  const total = (tally: Tally) => right(tally) + tally.wrong + tally.unanswered;
  const percent = (tally: Tally) => (total(tally) ? Math.round((right(tally) / total(tally)) * 100) : 0);
  const names = (ids: Id[]) => ids.map((id) => topics.find((topic) => topic.id === id)?.name ?? "Deleted topic").join(", ");

  const points = $derived(
    [...rows].reverse().map((row) => ({
      label: dayLabel(row.day, today),
      percent: percent(row.summary.overall),
      retry: row.retry_of !== null,
      detail: `${names(row.topics)} · ${right(row.summary.overall)} / ${total(row.summary.overall)}`,
    })),
  );

  Promise.all([api.history(), api.topics(), api.today()]).then(([history, topicRows, day]) => {
    rows = history;
    topics = topicRows;
    today = day;
    loaded = true;
  });
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="History" subtitle={rows.length ? `${rows.length} attempts` : ""} />

  {#if loaded && rows.length === 0}
    <div class="panel max-w-lg p-10">
      <span class="grid size-10 place-items-center rounded-lg bg-base-200"><HistoryIcon class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No quizzes yet</h2>
      <p class="mt-2 text-muted">Every quiz you finish or quit is saved here, with a chart of how your scores change.</p>
      <button class="btn btn-primary mt-6" onclick={onNewQuiz}><Play class="size-4" />Start a quiz</button>
    </div>
  {:else if loaded}
    <section class="panel p-6">
      <h2 class="font-semibold">Score per quiz</h2>
      <p class="text-sm text-muted">% right. Hollow points are retries.</p>
      <div class="mt-4"><ScoreChart {points} height={200} /></div>
    </section>

    <div class="panel mt-8 overflow-hidden">
      <table class="table">
        <thead><tr class="text-muted"><th>Date</th><th>Topics</th><th class="w-56">Result</th><th class="text-right">Score</th><th></th><th></th></tr></thead>
        <tbody>
          {#each rows as row (row.id)}
            <tr class="hover:bg-base-100" aria-label="Quiz {row.id}">
              <td class="tabular-nums">{dayLabel(row.day, today)}</td>
              <td>{names(row.topics)}</td>
              <td><SegBar tally={row.summary.overall} height={6} /></td>
              <td class="text-right font-semibold tabular-nums">{right(row.summary.overall)} / {total(row.summary.overall)}</td>
              <td>
                {#if row.retry_of !== null}<span class="badge badge-ghost badge-sm gap-1"><RotateCcw class="size-3" />Retry</span>
                {:else if row.status === "Incomplete"}<span class="badge badge-ghost badge-sm gap-1"><CirclePause class="size-3" />Incomplete</span>{/if}
              </td>
              <td class="text-right"><button class="btn btn-ghost btn-sm" onclick={() => onView(row.id)}>View</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

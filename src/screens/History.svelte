<script lang="ts">
  // history.feature (R26, R27, R29). Resuming an incomplete attempt is open (Q13).
  import { CirclePause, History as HistoryIcon, Play, RotateCcw } from "@lucide/svelte";
  import { api, type HistoryRow, type Id, type Tally, type TopicRow } from "../lib/api";
  import { ChevronRight } from "@lucide/svelte";
  import { dateOf, dayLabel, monthLabel } from "../lib/dates";
  import FilterBox from "../components/list/FilterBox.svelte";
  import { matchesFilter } from "../lib/lists";
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

  // lists.feature (R148): the chart shows the last 30 quizzes unless asked for more, and the
  // table is grouped by month, folded after the first two, with a filter.
  let range = $state<"30" | "90" | "all">("30");
  let filter = $state("");
  let opened = $state<Record<string, boolean>>({});
  const charted = $derived(range === "30" ? rows.slice(0, 30) : range === "90" ? rows.filter((row) => row.day > today - 90) : rows);
  const listed = $derived(rows.filter((row) => matchesFilter(filter, names(row.topics))));
  const months = $derived.by(() => {
    const groups: { key: string; label: string; rows: HistoryRow[] }[] = [];
    for (const row of listed) {
      const { year, month } = dateOf(row.day);
      const key = `${year}-${month}`;
      if (groups.at(-1)?.key !== key) groups.push({ key, label: monthLabel(year, month), rows: [] });
      groups.at(-1)!.rows.push(row);
    }
    return groups;
  });
  const isOpen = (key: string, index: number) => opened[key] ?? (index < 2 || filter.trim() !== "");
  const average = (list: HistoryRow[]) => Math.round(list.reduce((sum, row) => sum + percent(row.summary.overall), 0) / list.length);

  const points = $derived(
    [...charted].reverse().map((row) => ({
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

<div class="mx-auto max-w-6xl px-10 pb-24">
  <PageHeader title="History" subtitle={rows.length ? `${rows.length} attempts` : ""} />

  {#if loaded && rows.length === 0}
    <div class="panel max-w-lg p-10">
      <span class="well" aria-hidden="true"><HistoryIcon class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No quizzes yet</h2>
      <p class="mt-2 text-muted">Every quiz you finish or quit is saved here, with a chart of how your scores change.</p>
      <button class="btn btn-primary mt-6" onclick={onNewQuiz}><Play class="size-4" />Start a quiz</button>
    </div>
  {:else if loaded}
    <section class="panel p-6">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div><h2 class="font-semibold">Score per quiz</h2>
          <p class="text-sm text-muted">% right. Hollow points are retries.</p></div>
        <div class="seg" role="radiogroup" aria-label="Chart range">
          {#each [["30", "Last 30"], ["90", "90 days"], ["all", "All"]] as const as [key, label] (key)}<button type="button" role="radio" aria-checked={range === key} onclick={() => (range = key)}>{label}</button>{/each}
        </div>
      </div>
      <div class="mt-4"><ScoreChart {points} height={200} /></div>
    </section>

    <div class="mt-8 mb-3"><FilterBox bind:value={filter} shown={listed.length} total={rows.length} placeholder="Filter by topic…" /></div>
    {#each months as group, index (group.key)}
      <section class="panel mb-3 overflow-hidden" aria-label={group.label}>
        <button type="button" class="flex w-full items-center gap-3 px-5 py-3 text-left" aria-expanded={isOpen(group.key, index)} onclick={() => (opened = { ...opened, [group.key]: !isOpen(group.key, index) })}>
          <ChevronRight class="size-4 transition-transform {isOpen(group.key, index) ? 'rotate-90' : ''}" aria-hidden="true" />
          <span class="font-black">{group.label}</span>
          <span class="text-sm text-muted">{group.rows.length} {group.rows.length === 1 ? "quiz" : "quizzes"} · average {average(group.rows)}%</span>
        </button>
        {#if isOpen(group.key, index)}
      <table class="table border-t-2 border-base-300">
        <thead><tr class="text-muted"><th>Date</th><th>Topics</th><th class="w-56">Result</th><th class="text-right">Score</th><th><span class="sr-only">Status</span></th></tr></thead>
        <tbody>
          {#each group.rows as row (row.id)}
            <!-- The whole row opens the results: one button, stretched over the row. -->
            <tr class="link-row hover:bg-base-100" aria-label="Quiz {row.id}">
              <td class="font-extrabold tabular-nums"><button class="row-link" onclick={() => onView(row.id)}><span class="sr-only">View results from</span>{" "}{dayLabel(row.day, today)}</button></td>
              <td>{names(row.topics)}</td>
              <td><SegBar tally={row.summary.overall} height={14} /></td>
              <td class="text-right font-semibold tabular-nums">{right(row.summary.overall)} / {total(row.summary.overall)}</td>
              <td>
                <span class="flex items-center justify-end gap-3">
                {#if row.retry_of !== null}<span class="chip"><RotateCcw class="size-3" />Retry</span>
                {:else if row.status === "Incomplete"}<span class="chip"><CirclePause class="size-3" />Incomplete</span>{/if}
                <span class="whitespace-nowrap text-[13px] font-extrabold uppercase tracking-[.06em] text-primary-ink" aria-hidden="true">View results</span>
                </span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
        {/if}
      </section>
    {:else}
      <p class="panel p-6 text-sm text-muted">No quizzes on "{filter.trim()}". <button class="btn btn-link btn-sm" onclick={() => (filter = "")}>Clear the filter</button></p>
    {/each}
  {/if}
</div>

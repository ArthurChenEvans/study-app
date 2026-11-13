<script lang="ts">
  // results.feature, retry.feature, short-answer.feature (R24, R25, R26, R84)
  import { ArrowDown, ArrowUp, Check, CircleCheck, CircleX, Flag, GraduationCap, RotateCcw, Star, Undo2, X } from "@lucide/svelte";
  import { api, errorMessage, type GivenReason, type Id, type QuizView, type ResultsView, type ShortFeedback, type Tally, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import Rich from "../components/Rich.svelte";
  import CountUp from "../components/CountUp.svelte";
  import { fullLabel, onDay } from "../lib/dates";
  import { play } from "../lib/sound";
  import { celebrate, praise } from "../lib/celebrations.svelte";

  // `fresh` is true when you've just finished the quiz, false when reopening it from History.
  let { attempt, onRetry, fresh = false }: { attempt: Id; onRetry: (view: QuizView) => void; fresh?: boolean } = $props();

  let results = $state<ResultsView | null>(null);
  let topics = $state<TopicRow[]>([]);
  let today = $state(0);
  let error = $state("");
  let ownReason = $state<Record<number, string>>({});

  const presets: { value: Exclude<GivenReason, { Other: string }>; label: string }[] = [
    { value: "Blank", label: "Blank" },
    { value: "Misremembered", label: "Misremembered" },
    { value: "ConfusedWithSimilar", label: "Confused with something similar" },
    { value: "MisreadQuestion", label: "Misread the question" },
    { value: "CarelessSlip", label: "Careless slip" },
    { value: "SecondGuessed", label: "Second-guessed" },
    { value: "NeverLearned", label: "Never learned it" },
  ];

  const right = (tally: Tally) => tally.right_sure + tally.right_guessed;
  const total = (tally: Tally) => right(tally) + tally.wrong + tally.unanswered;
  const topicName = (id: Id | string) => topics.find((topic) => topic.id === Number(id))?.name ?? "Deleted topic";
  const share = (part: number, whole: number) => (whole === 0 ? 0 : (part / whole) * 100);

  async function load() {
    [results, topics, today] = await Promise.all([api.results(attempt), api.topics(), Promise.resolve().then(() => api.today()).catch(() => 0)]);
    if (fresh) {
      const overall = results.summary.overall;
      const count = total(overall);
      const score = count ? Math.round((right(overall) / count) * 100) : 0;
      const names = Object.keys(results.summary.per_topic).map(topicName);
      const mode = results.status === "Finished"
        ? celebrate("quiz", {
            title: praise("quiz", score),
            line: (names.length > 2 ? `${names.slice(0, 2).join(" · ")} and ${names.length - 2} more` : names.join(" · ")) + (score < 80 && results.to_prompt.length ? ". The ones you missed come back sooner." : ""),
            score,
            stats: [
              { label: "Score", value: score, suffix: "%", tone: "primary" },
              { label: "Right", value: right(overall), suffix: `/${count}`, tone: "success" },
              { label: "Guessed", value: overall.right_guessed, tone: "accent" },
            ],
          })
        : "off";
      if (mode !== "full") play("finished");
    }
  }

  async function give(index: number, reason: GivenReason) {
    error = "";
    try {
      results = await api.giveReason(attempt, index, reason);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function presetValue(reason: GivenReason | null) {
    return reason && typeof reason === "string" ? reason : "";
  }

  // R84: count a short answer the other way, or undo that.
  async function overrule(index: number) {
    error = "";
    try {
      results = await api.overruleResult(attempt, index);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  const typoIn = (short: ShortFeedback) => (typeof short.marking.detail === "object" && "Matched" in short.marking.detail ? short.marking.detail.Matched.typo : null);
  function howChecked(short: ShortFeedback) {
    const detail = short.marking.detail;
    if (detail === "NoMatch") return "Didn't match any accepted answer";
    if ("Matched" in detail) return `Matched "${detail.Matched.answer}"${detail.Matched.typo ? " with a typo" : ""}`;
    const { hit, missed } = detail.Keywords;
    return `${hit.length} of ${hit.length + missed.length} keywords${missed.length ? `, missed ${missed.join(", ")}` : ""}`;
  }

  async function retry() {
    onRetry(await api.retry(attempt));
  }

  load();
</script>

{#snippet bar(tally: Tally, height: number, grow = false)}
  {@const whole = total(tally)}
  <div class="seg-bar {grow ? 'fill-in' : ''}" style="height: {height}px" role="img" aria-label="{tally.right_sure} right and sure, {tally.right_guessed} guessed, {tally.wrong + tally.unanswered} wrong or unanswered">
    {#if tally.right_sure}<span style="width: {share(tally.right_sure, whole)}%; background: var(--chart-right)"></span>{/if}
    {#if tally.right_guessed}<span style="width: {share(tally.right_guessed, whole)}%; background: var(--guessed-fill)"></span>{/if}
    {#if tally.wrong + tally.unanswered}<span style="width: {share(tally.wrong + tally.unanswered, whole)}%; background: var(--chart-wrong)"></span>{/if}
  </div>
{/snippet}

{#if results}
  {@const overall = results.summary.overall}
  <div class="mx-auto max-w-7xl px-10 pb-24">
    <PageHeader title={fresh ? "Quiz finished" : "Results"} subtitle="{results.exam ? 'Exam' : 'Study'} mode · {total(overall)} questions{results.status === 'Incomplete' ? ' · Quit early' : ''}" />

    <div class="grid grid-cols-[minmax(0,1fr)] wide:grid-cols-[minmax(0,1fr)_20rem] items-start gap-6">
      <div class="flex min-w-0 flex-col gap-4">
        <!-- The score splits into four framed tiles: score, right, guessed (hatched) and wrong -->
        <section class="rise grid grid-cols-4 gap-3" aria-label="Score">
          <div class="score-tile" style="--tile: var(--color-neutral); --tile-on: var(--color-neutral-content); --tile-ink: var(--color-base-content)" role="group" aria-labelledby="score-label">
            <span id="score-label" class="score-label">Score</span>
            <b><Star class="size-5 fill-current" aria-hidden="true" /><span><CountUp value={right(overall)} /></span>{" "}<span>/ {total(overall)}</span></b>
            <span class="sr-only">{overall.right_guessed} of those guessed</span>
          </div>
          <div class="score-tile" style="--tile: var(--chart-right); --tile-on: var(--on-right); --tile-ink: var(--right-ink)" role="group" aria-label="Right, sure">
            <span class="score-label" aria-hidden="true">Right, sure</span>
            <b><Check class="size-5" strokeWidth={3} aria-hidden="true" />{overall.right_sure}</b>
          </div>
          <div class="score-tile score-tile-guessed" role="group" aria-label="Right, guessed">
            <span class="score-label" aria-hidden="true">Right, guessed</span>
            <b><Flag class="size-5 fill-current" aria-hidden="true" />{overall.right_guessed}</b>
          </div>
          <div class="score-tile" style="--tile: var(--chart-wrong); --tile-on: var(--on-wrong); --tile-ink: var(--wrong-ink)" role="group" aria-label="Wrong">
            <span class="score-label" aria-hidden="true">Wrong</span>
            <b><X class="size-5" strokeWidth={3} aria-hidden="true" />{overall.wrong + overall.unanswered}</b>
          </div>
        </section>

        <section class="panel rise rise-2 p-5">
          <h2 class="text-[18px]">By topic</h2>
          <ul class="mt-2 space-y-2" aria-label="By topic">
            {#each Object.entries(results.summary.per_topic) as [id, tally] (id)}
              <li class="grid grid-cols-[110px_minmax(0,1fr)_56px] items-center gap-3 text-[15px]"><span class="truncate">{topicName(id)}</span>{@render bar(tally, 14, true)}<span class="text-right tabular-nums">{right(tally)} / {total(tally)}</span></li>
            {/each}
          </ul>
        </section>
        {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}

        <div class="mt-2 flex items-baseline justify-between"><h2 class="text-[20px]">Review</h2><p class="text-[13px] text-muted">Wrong and guessed first</p></div>
        <ul class="flex flex-col gap-3">
      {#each results.review as item (item.index)}
        <li class="panel relative flex flex-col gap-1 p-5" aria-label="Question {item.index + 1}">
          <div class="max-w-prose flex-1">
            <p><span class="chip">{topicName(item.topic)}{item.short || (item.options.length === 0 && item.status === "Unanswered") ? " · Short answer" : ""}</span></p>
            <Rich class="mt-2 text-[18px] font-extrabold" text={item.text} />
            <dl class="mt-3 space-y-1 text-sm">
              {#if item.short}
                <div><dt class="inline text-muted">Your answer:</dt>{" "}<dd class="inline whitespace-pre-line">{item.short.typed}</dd></div>
                <div><dt class="inline text-muted">Model answer:</dt>{" "}<dd class="inline"><Rich inline text={item.short.shown_answer} /></dd></div>
                <div><dt class="inline text-muted">Checked:</dt>{" "}<dd class="inline text-muted">{howChecked(item.short)}</dd></div>
              {:else}
                <div><dt class="inline text-muted">Your answer:</dt>{" "}<dd class="inline">{#each item.picked as index, position (index)}{position ? ", " : ""}<Rich inline text={item.options[index]} />{:else}None{/each}</dd></div>
                {#if item.options.length}<div><dt class="inline text-muted">Correct:</dt>{" "}<dd class="inline">{#each item.correct as index, position (index)}{position ? ", " : ""}<Rich inline text={item.options[index]} />{/each}</dd></div>{/if}
              {/if}
            </dl>
            {#if item.explanation}<Rich class="mt-3 text-sm leading-relaxed" text={item.explanation} />{/if}
          </div>
          <div class="w-full">
            <span class="absolute right-5 top-5">{#if item.status === "Right"}<span class="chip chip-right"><CircleCheck class="size-4" />{item.short && !item.short.overruled && typoIn(item.short) ? "Right, with a typo" : "Right"}</span>
            {:else if item.status === "RightGuessed"}<span class="chip chip-guessed"><Flag class="size-3.5" />Right, guessed</span>
            {:else}<span class="chip chip-wrong"><CircleX class="size-4" />{item.status === "Unanswered" ? "Not answered" : "Wrong"}</span>{/if}</span>

            {#if item.short?.overruled}<span class="overruled mt-1 inline-block">You overruled this</span>{/if}
            {#if item.short}
              {#if item.short.overruled}
                <button class="btn btn-ghost btn-sm mt-3" onclick={() => overrule(item.index)}><Undo2 class="size-4" />Undo, count it as {item.short.marking.right ? "right" : "wrong"}</button>
              {:else if item.status === "Wrong"}
                <button class="btn btn-outline btn-sm mt-3 w-full" onclick={() => overrule(item.index)}><Check class="size-4" />Count it as right</button>
              {:else}
                <button class="btn btn-ghost btn-sm mt-3" onclick={() => overrule(item.index)}><X class="size-4" />Count it as wrong</button>
              {/if}
            {/if}

            {#if item.auto_reason}
              <p class="mt-3 text-sm">Reason: <b>{item.auto_reason === "Incomplete" ? "Incomplete" : "Timed out"}</b> <span class="text-muted">(recorded automatically)</span></p>
            {:else if item.status === "Wrong"}
              <!-- Reasons as tap chips, one picked at a time -->
              <p id="why-{item.index}" class="mt-3 text-sm">Why did you get it wrong? <span class="text-muted">(optional)</span></p>
              <div class="mt-2 flex flex-wrap gap-2" role="radiogroup" aria-labelledby="why-{item.index}">
                {#each presets as preset (preset.value)}
                  {@const on = presetValue(item.reason) === preset.value}
                  <button type="button" role="radio" aria-checked={on} class="word {on ? 'word-on' : ''}" onclick={() => give(item.index, preset.value)}>{preset.label}</button>
                {/each}
              </div>
              <input
                class="input input-sm mt-3 w-full max-w-md"
                aria-label="Your own reason"
                placeholder={item.reason && typeof item.reason === "object" ? item.reason.Other : "Or write your own reason…"}
                bind:value={ownReason[item.index]}
                onkeydown={(event) => { if (event.key === "Enter" && ownReason[item.index]?.trim()) { give(item.index, { Other: ownReason[item.index].trim() }); ownReason[item.index] = ""; } }}
              />
            {/if}
          </div>
        </li>
      {/each}
        </ul>
      </div>

      <aside class="flex flex-col gap-4">
        {#if results.retry_count > 0}
          <div>
            <button class="btn btn-primary btn-wrap w-full" onclick={retry}><RotateCcw class="size-4" />Retry the {results.retry_count} wrong and guessed</button>
            <p class="mt-2 text-center text-[13px] text-muted">Same settings. Retries don't move the schedule.</p>
          </div>
        {:else}
          <p class="panel p-5 text-sm text-muted">Nothing to retry. Every answer was right and none were guesses.</p>
        {/if}
        {#if results.moves.length}
          <section class="panel p-5" aria-labelledby="schedule-h">
            <h2 id="schedule-h" class="text-[18px]">Quiz schedule</h2>
            <ul class="mt-1">
              {#each results.moves as move (move.topic)}
                <li class="border-t-2 border-base-300 py-2.5 first:border-t-0">
                  <span class="flex items-center justify-between gap-2"><span class="flex flex-wrap items-center gap-2"><span class="font-extrabold">{move.name}</span>{" "}<span class="status-chip {move.passed ? 'status-pass' : 'status-under'}">{#if move.passed}<ArrowUp class="size-3.5" />Passed{:else}<ArrowDown class="size-3.5" />Under pass mark{/if}</span></span>{" "}<span class="tabular-nums">{move.score}%</span></span>{" "}
                  <span class="mt-0.5 block text-[13px] text-muted">{move.from === null ? `Starts on ${move.to}` : move.passed ? (move.from === move.to ? `Stays on ${move.to}` : `Moves up to ${move.to}`) : move.from === move.to ? `Stays on ${move.to}` : `Back to ${move.to}`} · Next quiz {onDay(move.tightened_due ?? move.next_due, today)}{move.tightened_due !== null ? `, tightened for the ${move.tightened_for} (would be ${fullLabel(move.next_due, today)})` : ""}</span>
                  {#if move.tightened_due !== null}<span class="chip chip-primary mt-1.5"><GraduationCap class="size-3.5" aria-hidden="true" />Tightened</span>{/if}
                </li>
              {/each}
            </ul>
            <p class="mt-2 text-[13px] text-muted">Each topic is scored on its own questions in this quiz.</p>
          </section>
        {/if}
      </aside>
    </div>
  </div>
{/if}

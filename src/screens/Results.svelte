<script lang="ts">
  // results.feature, retry.feature, short-answer.feature (R24, R25, R26, R84)
  import { Check, CircleCheck, CircleX, Flag, RotateCcw, Undo2, X } from "@lucide/svelte";
  import { api, errorMessage, type GivenReason, type Id, type QuizView, type ResultsView, type ShortFeedback, type Tally, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import Rich from "../components/Rich.svelte";

  let { attempt, onRetry }: { attempt: Id; onRetry: (view: QuizView) => void } = $props();

  let results = $state<ResultsView | null>(null);
  let topics = $state<TopicRow[]>([]);
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
    [results, topics] = await Promise.all([api.results(attempt), api.topics()]);
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

{#snippet bar(tally: Tally, height: number)}
  {@const whole = total(tally)}
  <div class="seg-bar" style="height: {height}px" role="img" aria-label="{tally.right_sure} right and sure, {tally.right_guessed} guessed, {tally.wrong + tally.unanswered} wrong or unanswered">
    {#if tally.right_sure}<span style="width: {share(tally.right_sure, whole)}%; background: var(--chart-right)"></span>{/if}
    {#if tally.right_guessed}<span style="width: {share(tally.right_guessed, whole)}%; background: var(--guessed-fill)"></span>{/if}
    {#if tally.wrong + tally.unanswered}<span style="width: {share(tally.wrong + tally.unanswered, whole)}%; background: var(--chart-wrong)"></span>{/if}
  </div>
{/snippet}

{#if results}
  {@const overall = results.summary.overall}
  <div class="mx-auto max-w-6xl px-12 pb-24">
    <PageHeader title="Results" subtitle="{results.exam ? 'Exam' : 'Study'} mode · {total(overall)} questions{results.status === 'Incomplete' ? ' · Quit early' : ''}" />

    <section class="panel rise p-8">
      <div class="grid grid-cols-[auto_1fr_auto] items-center gap-12">
        <div>
          <p class="text-6xl font-semibold leading-none"><span>{right(overall)}</span> <span class="text-3xl font-normal text-muted">/ {total(overall)}</span></p>
          <p class="mt-3 text-sm text-muted">{overall.right_guessed} of those guessed</p>
        </div>
        <div>
          {@render bar(overall, 28)}
          <div class="mt-3 flex items-center gap-4 text-xs text-muted">
            <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--chart-right)"></span>Right, sure</span>
            <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--guessed-fill)"></span>Right, guessed</span>
            <span class="flex items-center gap-1.5"><span class="swatch" style="background: var(--chart-wrong)"></span>Wrong</span>
          </div>
        </div>
        <div class="text-right">
          {#if results.retry_count > 0}
            <button class="btn btn-primary" onclick={retry}><RotateCcw class="size-4" />Retry {results.retry_count} questions</button>
            <p class="mt-2 text-xs text-muted">Wrong and guessed, same settings</p>
          {:else}
            <p class="max-w-48 text-sm text-muted">Nothing to retry. Every answer was right and none were guesses.</p>
          {/if}
        </div>
      </div>
      <div class="mt-8 border-t border-base-300 pt-6">
        <p class="text-sm font-semibold">By topic</p>
        <ul class="mt-3 max-w-xl space-y-3" aria-label="By topic">
          {#each Object.entries(results.summary.per_topic) as [id, tally] (id)}
            <li class="grid grid-cols-[120px_1fr_56px] items-center gap-3 text-sm"><span class="truncate">{topicName(id)}</span>{@render bar(tally, 8)}<span class="text-right tabular-nums text-muted">{right(tally)} / {total(tally)}</span></li>
          {/each}
        </ul>
      </div>
    </section>

    {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}

    <div class="mt-12 flex items-baseline justify-between"><h2 class="text-lg font-semibold">Review</h2><p class="text-sm text-muted">Wrong and guessed first</p></div>
    <ul class="panel mt-4 divide-y">
      {#each results.review as item (item.index)}
        <li class="flex items-start gap-8 p-6" aria-label="Question {item.index + 1}">
          <div class="max-w-prose flex-1">
            <p class="eyebrow">{topicName(item.topic)}{item.short || (item.options.length === 0 && item.status === "Unanswered") ? " · Short answer" : ""}</p>
            <Rich class="mt-1 font-semibold" text={item.text} />
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
          <div class="w-64 shrink-0">
            {#if item.status === "Right"}<span class="flex items-center gap-1 text-sm font-semibold text-right-ink"><CircleCheck class="size-4" />{item.short && !item.short.overruled && typoIn(item.short) ? "Right, with a typo" : "Right"}</span>
            {:else if item.status === "RightGuessed"}<span class="badge guessed-chip gap-1 font-semibold"><Flag class="size-3" />Right, guessed</span>
            {:else}<span class="flex items-center gap-1 text-sm font-semibold text-wrong-ink"><CircleX class="size-4" />{item.status === "Unanswered" ? "Not answered" : "Wrong"}</span>{/if}

            {#if item.short?.overruled}<span class="overruled mt-1 inline-block">You overruled this</span>{/if}
            {#if item.short}
              {#if item.short.overruled}
                <button class="btn btn-ghost btn-sm mt-3" onclick={() => overrule(item.index)}><Undo2 class="size-4" />Undo, count it as {item.short.marking.right ? "right" : "wrong"}</button>
              {:else if item.status === "Wrong"}
                <button class="btn btn-outline btn-sm mt-3 w-full border-field-border" onclick={() => overrule(item.index)}><Check class="size-4" />Count it as right</button>
              {:else}
                <button class="btn btn-ghost btn-sm mt-3" onclick={() => overrule(item.index)}><X class="size-4" />Count it as wrong</button>
              {/if}
            {/if}

            {#if item.auto_reason}
              <p class="mt-3 text-sm">Reason: <b>{item.auto_reason === "Incomplete" ? "Incomplete" : "Timed out"}</b> <span class="text-muted">(recorded automatically)</span></p>
            {:else if item.status === "Wrong"}
              <label class="mt-3 block">
                <span class="text-sm text-muted">Why did you get it wrong? <span class="text-xs">(optional)</span></span>
                <select class="select select-sm mt-1 w-full" value={presetValue(item.reason)} onchange={(event) => event.currentTarget.value && give(item.index, event.currentTarget.value as GivenReason)}>
                  <option value="">Pick a reason…</option>
                  {#each presets as preset (preset.value)}<option value={preset.value}>{preset.label}</option>{/each}
                </select>
              </label>
              <input
                class="input input-sm mt-2 w-full"
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
{/if}

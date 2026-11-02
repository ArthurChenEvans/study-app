<script lang="ts">
  // taking-quiz.feature (R18 to R23, R29, R32)
  import { onDestroy } from "svelte";
  import { BookOpen, Check, CircleCheck, CircleX, Flag, LogOut, Timer, X } from "@lucide/svelte";
  import { api, errorMessage, type AnswerResult, type Feedback, type Id, type QuizView, type TrackCell } from "../lib/api";
  import Rich from "../components/Rich.svelte";

  let { initial, onFinished }: { initial: QuizView; onFinished: (attempt: Id) => void } = $props();

  // svelte-ignore state_referenced_locally
  let current = $state<QuizView>(initial);
  let picked = $state<number[]>([]);
  let feedback = $state<Feedback | null>(null);
  let pendingNext = $state<AnswerResult["next"] | null>(null);
  let busy = $state(false);
  let error = $state("");

  // Timers: deadlines in ms, ticked every 250 ms.
  let now = $state(Date.now());
  // svelte-ignore state_referenced_locally
  const totalDeadline = initial.total_seconds ? Date.now() + initial.total_seconds * 1000 : null;
  let questionDeadline = $state<number | null>(null);
  const tick = setInterval(() => {
    now = Date.now();
    if (busy) return;
    if (totalDeadline !== null && now >= totalDeadline) endTotal();
    else if (questionDeadline !== null && feedback === null && now >= questionDeadline) timeOutQuestion();
  }, 250);
  onDestroy(() => clearInterval(tick));

  function startQuestionClock(view: QuizView) {
    questionDeadline = view.per_question_seconds ? Date.now() + view.per_question_seconds * 1000 : null;
  }
  // svelte-ignore state_referenced_locally
  startQuestionClock(initial);

  const remaining = (deadline: number | null) => (deadline === null ? null : Math.max(0, Math.ceil((deadline - now) / 1000)));
  const clock = (seconds: number) => `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  const totalLeft = $derived(remaining(totalDeadline));
  const questionLeft = $derived(remaining(questionDeadline));
  const low = (left: number | null, of: number | null) => left !== null && of !== null && left <= of * 0.2;

  const trackColour: Record<TrackCell, string> = {
    RightSure: "background: var(--chart-right)",
    RightGuessed: "background: var(--guessed-fill)",
    Wrong: "background: var(--chart-wrong)",
    Answered: "background: var(--chart-ink)",
    Current: "background: var(--color-base-content)",
    Upcoming: "",
  };

  function pick(index: number) {
    if (feedback || busy) return;
    if (current.multiple) picked = picked.includes(index) ? picked.filter((value) => value !== index) : [...picked, index];
    else picked = [index];
  }

  function show(next: AnswerResult["next"]) {
    if ("Finished" in next) {
      onFinished(next.Finished);
      return;
    }
    current = next.Question;
    picked = [];
    feedback = null;
    pendingNext = null;
    startQuestionClock(current);
  }

  async function run(action: () => Promise<AnswerResult>) {
    busy = true;
    error = "";
    try {
      const result = await action();
      if (result.feedback) {
        feedback = result.feedback;
        pendingNext = result.next;
      } else show(result.next);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function submit() {
    if (picked.length === 0 || feedback || busy) return;
    const answer = [...picked].sort((a, b) => a - b);
    run(() => api.answer(answer));
  }

  function timeOutQuestion() {
    run(() => api.questionTimedOut());
  }

  async function endTotal() {
    busy = true;
    onFinished(await api.totalTimedOut());
  }

  async function toggleFlag() {
    if (feedback || busy) return;
    current = await api.toggleFlag();
  }

  async function quit() {
    busy = true;
    onFinished(await api.quitQuiz());
  }

  function next() {
    if (pendingNext) show(pendingNext);
  }

  function onKey(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
    if (/^[1-5]$/.test(event.key)) {
      const option = current.options[Number(event.key) - 1];
      if (option) pick(option.index);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (feedback) next();
      else submit();
    } else if (event.key.toLowerCase() === "f") {
      toggleFlag();
    }
  }

  function optionState(index: number) {
    if (!feedback) return picked.includes(index) ? "border-primary ring-1 ring-primary" : "border-base-300 hover:border-field-border";
    if (feedback.correct.includes(index)) return "is-right";
    if (picked.includes(index)) return "is-wrong";
    return "border-base-300 opacity-60";
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="flex min-h-screen flex-col">
  <div class="sticky top-0 z-10 border-b border-base-300 bg-base-100 px-12 pb-4 pt-5">
    <div class="flex items-center gap-6">
      <span class="badge {current.exam ? 'badge-neutral' : 'badge-ghost'}">{current.exam ? "Exam" : "Study"}</span>
      <span class="text-sm tabular-nums"><b>{current.position}</b> <span class="text-muted">of {current.total}</span></span>
      <span class="ml-auto flex items-center gap-4">
        {#if totalLeft !== null}
          <span role="timer" aria-label="Time left" class="flex items-center gap-2 font-semibold tabular-nums {low(totalLeft, initial.total_seconds) ? 'rounded-lg bg-warning px-3 py-1 text-warning-content' : ''}"><Timer class="size-4" />{clock(totalLeft)}</span>
        {/if}
        {#if questionLeft !== null && feedback === null}
          <span role="timer" aria-label="Time for this question" class="flex items-center gap-2 font-semibold tabular-nums {low(questionLeft, current.per_question_seconds) ? 'rounded-lg bg-warning px-3 py-1 text-warning-content' : ''}"><Timer class="size-4" />{clock(questionLeft)}</span>
        {/if}
      </span>
      <button class="btn btn-ghost btn-sm font-normal" onclick={quit} disabled={busy}><LogOut class="size-4" />Quit</button>
    </div>
    <div class="track mt-4" aria-label="Progress, question {current.position} of {current.total}">
      {#each current.track as cell, index (index)}<span style={trackColour[cell]}></span>{/each}
    </div>
  </div>

  <div class="flex-1 px-12 py-12">
    <div class="mx-auto max-w-prose">
      {#if current.multiple}<p class="eyebrow">Select all that apply{#if current.answers_to_pick} · <span>Pick {current.answers_to_pick}</span>{/if}</p>{/if}
      <div role="heading" aria-level="1" class="mt-2 text-xl leading-snug"><Rich text={current.text} /></div>

      <div class="mt-8 space-y-3" role={current.multiple ? "group" : "radiogroup"} aria-label="Answers">
        {#each current.options as option, position (option.index)}
          <button
            class="flex min-h-12 w-full items-center gap-4 rounded-lg border bg-surface px-4 py-3 text-left transition-colors duration-150 {optionState(option.index)}"
            aria-pressed={picked.includes(option.index)}
            onclick={() => pick(option.index)}
          >
            <kbd class="kbd kbd-sm tabular-nums">{position + 1}</kbd> <Rich inline text={option.text} class="flex-1" />
            {#if feedback?.correct.includes(option.index)}<span class="flex items-center gap-1 text-sm font-semibold text-right-ink"><Check class="size-4" />Correct</span>
            {:else if feedback && picked.includes(option.index)}<span class="flex items-center gap-1 text-sm font-semibold text-wrong-ink"><X class="size-4" />Your pick</span>{/if}
          </button>
        {/each}
      </div>

      {#if feedback}
        <div class="panel rise mt-8 p-6" role="status">
          <p class="flex items-center gap-2 font-semibold {feedback.right ? 'text-right-ink' : 'text-wrong-ink'}">
            {#if feedback.right}<CircleCheck class="size-5" />Correct{:else}<CircleX class="size-5" />{feedback.auto_reason === "Incomplete" ? "Wrong: incomplete, you missed a correct option" : feedback.auto_reason === "TimedOut" ? "Wrong: time ran out" : "Wrong"}{/if}
            {#if current.flagged}<span class="badge guessed-chip ml-2"><Flag class="size-3" />Guessed</span>{/if}
          </p>
          {#if feedback.explanation}<Rich class="mt-3 leading-relaxed" text={feedback.explanation} />{/if}
          {#each feedback.citations as citation, index (index)}<p class="mt-3 flex items-center gap-2 text-sm text-muted"><BookOpen class="size-4" />{citation.location}</p>{/each}
        </div>
      {/if}

      {#if error}<p role="alert" class="mt-6 text-sm text-error">{error}</p>{/if}

      <div class="mt-8 flex items-center gap-3">
        {#if feedback}
          <button class="btn btn-primary" onclick={next}>Next question <kbd class="kbd kbd-xs">Enter</kbd></button>
        {:else}
          <button class="btn btn-primary" onclick={submit} disabled={picked.length === 0 || busy}>Submit <kbd class="kbd kbd-xs">Enter</kbd></button>
          <button
            class="btn {current.flagged ? '' : 'btn-ghost'}"
            style={current.flagged ? "background: var(--guessed-tint); color: var(--guessed-ink); border-color: var(--chart-right)" : ""}
            aria-pressed={current.flagged}
            onclick={toggleFlag}><Flag class="size-4" />{current.flagged ? "Flagged as guessed" : "I guessed"} <kbd class="kbd kbd-xs">F</kbd></button>
        {/if}
        {#if current.exam}<span class="ml-auto text-sm text-muted">No going back in exam mode</span>{/if}
      </div>
    </div>
  </div>
</div>

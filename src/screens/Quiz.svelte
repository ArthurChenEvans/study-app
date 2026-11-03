<script lang="ts">
  // taking-quiz.feature, short-answer.feature (R18 to R23, R29, R32, R83, R84)
  import { onDestroy } from "svelte";
  import { BookOpen, Check, CircleCheck, CircleX, Flag, LogOut, Timer, Undo2, X } from "@lucide/svelte";
  import { api, errorMessage, type AnswerResult, type Feedback, type Id, type QuizView, type TrackCell } from "../lib/api";
  import Rich from "../components/Rich.svelte";

  let { initial, onFinished }: { initial: QuizView; onFinished: (attempt: Id) => void } = $props();

  // svelte-ignore state_referenced_locally
  let current = $state<QuizView>(initial);
  let picked = $state<number[]>([]);
  let typed = $state("");
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
    // Outlined, not filled: a solid near-white bar was the loudest thing on the dark theme.
    Current: "background: transparent; box-shadow: inset 0 0 0 2px var(--color-base-content)",
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
    typed = "";
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
    if (feedback || busy) return;
    if (current.short) {
      if (!typed.trim()) return;
      const answer = typed;
      run(() => api.answerText(answer));
      return;
    }
    if (picked.length === 0) return;
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

  // R84: count a short answer the other way. Once the quiz has finished, the last
  // answer lives in the saved attempt, so that's what gets overruled.
  async function overrule() {
    if (!feedback?.short || busy) return;
    busy = true;
    error = "";
    try {
      const attempt = pendingNext && "Finished" in pendingNext ? pendingNext.Finished : null;
      const index = feedback.index;
      feedback = await api.overruleAnswer(attempt, index);
      if (pendingNext && "Question" in pendingNext) {
        const cell: TrackCell = feedback.right ? (current.flagged ? "RightGuessed" : "RightSure") : "Wrong";
        pendingNext.Question.track[index] = current.exam ? "Answered" : cell;
      }
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function answerKeys(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  }

  const verdict = $derived.by(() => {
    const short = feedback?.short;
    if (!feedback || !short) return null;
    const detail = short.marking.detail;
    const typo = typeof detail === "object" && "Matched" in detail ? detail.Matched.typo : null;
    const keywords = typeof detail === "object" && "Keywords" in detail ? detail.Keywords : null;
    const label = short.overruled ? (feedback.right ? "Counted as right" : "Counted as wrong") : feedback.right ? (typo ? "Correct, with a typo" : "Correct") : "Wrong";
    const note = short.overruled ? "" : typo ? `you wrote "${typo}"` : keywords && !feedback.right ? `needs ${keywords.needed === keywords.hit.length + keywords.missed.length ? `all ${keywords.needed}` : keywords.needed} keywords, you had ${keywords.hit.length}` : "";
    return { label, note, keywords };
  });

  function onKey(event: KeyboardEvent) {
    // R83: Ctrl+G flags a short answer as guessed, even while typing it.
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "g" && current.short) {
      event.preventDefault();
      toggleFlag();
      return;
    }
    const target = event.target as HTMLElement | null;
    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) && !(target as HTMLTextAreaElement).readOnly) return;
    if (event.key.toLowerCase() === "o" && feedback?.short && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      overrule();
      return;
    }
    if (/^[1-5]$/.test(event.key)) {
      const option = current.options[Number(event.key) - 1];
      if (option) pick(option.index);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (feedback) next();
      else submit();
    } else if (event.key.toLowerCase() === "f" && !current.short) {
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
      {#if current.short}<p class="eyebrow">Short answer</p>
      {:else if current.multiple}<p class="eyebrow">Select all that apply{#if current.answers_to_pick} · <span>Pick {current.answers_to_pick}</span>{/if}</p>{/if}
      <div role="heading" aria-level="1" class="mt-2 text-xl leading-snug"><Rich text={current.text} /></div>

      {#if current.short}
        <!-- svelte-ignore a11y_autofocus -->
        <textarea class="textarea answer-box mt-8 w-full {feedback && !feedback.short?.overruled ? (feedback.right ? 'is-right' : 'is-wrong') : ''}" rows="2" aria-label="Your answer" placeholder="Type your answer…" readonly={feedback !== null} autofocus bind:value={typed} onkeydown={answerKeys}></textarea>
        {#if !feedback}<p class="mt-2 text-xs text-muted"><kbd class="kbd kbd-xs">Enter</kbd> submits, <kbd class="kbd kbd-xs">Shift</kbd><kbd class="kbd kbd-xs">Enter</kbd> adds a new line</p>{/if}
      {:else}
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
      {/if}

      {#if feedback?.short && verdict}
        {@const short = feedback.short}
        <div class="panel rise mt-8 p-6" role="status">
          <p class="flex flex-wrap items-center gap-2 font-semibold {feedback.right ? 'text-right-ink' : 'text-wrong-ink'}">
            {#if feedback.right}<CircleCheck class="size-5" />{:else}<CircleX class="size-5" />{/if}{verdict.label}
            {#if verdict.note}<span class="font-normal text-muted">{verdict.note}</span>{/if}
            {#if short.overruled}<span class="overruled">You overruled this</span>{/if}
            {#if current.flagged}<span class="badge guessed-chip"><Flag class="size-3" />Guessed</span>{/if}
          </p>
          {#if verdict.keywords}
            <div class="mt-3 flex flex-wrap gap-2" aria-label="Keywords">
              {#each verdict.keywords.hit as word (word)}<span class="kw kw-hit"><Check class="size-3" />{word}<span class="sr-only">, found</span></span>{/each}
              {#each verdict.keywords.missed as word (word)}<span class="kw kw-miss"><X class="size-3" />{word}<span class="sr-only">, missed</span></span>{/each}
            </div>
          {/if}
          <p class="mt-4 text-sm text-muted">Model answer</p>
          <Rich class="mt-1" text={short.shown_answer} />
          {#if !feedback.right && short.also_accepted.length}<p class="mt-3 text-sm text-muted">Also accepted: {short.also_accepted.join(", ")}</p>{/if}
          {#if feedback.explanation}<Rich class="mt-4 leading-relaxed" text={feedback.explanation} />{/if}
          {#each feedback.citations as citation, index (index)}<p class="mt-3 flex items-center gap-2 text-sm text-muted"><BookOpen class="size-4" />{citation.location}</p>{/each}
          {#if short.overruled}
            <button class="btn btn-ghost btn-sm mt-4" disabled={busy} onclick={overrule}><Undo2 class="size-4" />Undo, count it as {short.marking.right ? "right" : "wrong"} <kbd class="kbd kbd-xs">O</kbd></button>
          {:else}
            <div class="mt-5 flex items-center gap-3 border-t border-base-300 pt-4">
              <span class="text-sm text-muted">{feedback.right ? "Marked right but shouldn't be?" : "Think you had it?"}</span>
              <button class="btn btn-outline btn-sm border-field-border" disabled={busy} onclick={overrule}>{#if feedback.right}<X class="size-4" />{:else}<Check class="size-4" />{/if}Count it as {feedback.right ? "wrong" : "right"} <kbd class="kbd kbd-xs">O</kbd></button>
            </div>
          {/if}
        </div>
      {:else if feedback}
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
          <button class="btn btn-primary" onclick={submit} disabled={(current.short ? !typed.trim() : picked.length === 0) || busy}>Submit <kbd class="kbd kbd-xs">Enter</kbd></button>
          <button
            class="btn {current.flagged ? '' : 'btn-ghost'}"
            style={current.flagged ? "background: var(--guessed-tint); color: var(--guessed-ink); border-color: var(--chart-right)" : ""}
            aria-pressed={current.flagged}
            onclick={toggleFlag}><Flag class="size-4" />{current.flagged ? "Flagged as guessed" : "I guessed"} {#if current.short}<kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">G</kbd>{:else}<kbd class="kbd kbd-xs">F</kbd>{/if}</button>
        {/if}
        {#if current.exam}<span class="ml-auto text-sm text-muted">No going back in exam mode{current.short ? ". You can overrule the marking on the results page." : ""}</span>{/if}
      </div>
    </div>
  </div>
</div>

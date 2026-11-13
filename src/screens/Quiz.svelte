<script lang="ts">
  // taking-quiz.feature, short-answer.feature (R18 to R23, R29, R32, R83, R84)
  import { onDestroy } from "svelte";
  import { BookOpen, Check, CircleCheck, CircleX, Flag, LogOut, Timer, Undo2, X } from "@lucide/svelte";
  import { api, errorMessage, type AnswerResult, type Feedback, type Id, type QuizView, type TrackCell } from "../lib/api";
  import Rich from "../components/Rich.svelte";
  import { play } from "../lib/sound";
  import { startSession } from "../lib/celebrations.svelte";

  let { initial, onFinished }: { initial: QuizView; onFinished: (attempt: Id) => void } = $props();

  // svelte-ignore state_referenced_locally
  let current = $state<QuizView>(initial);
  // svelte-ignore state_referenced_locally
  const session = startSession(initial.total);
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
    Current: "background: transparent; box-shadow: inset 0 0 0 3px var(--primary-ink)",
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
        play(result.feedback.right ? "right" : "wrong");
        session.answered(result.feedback.right);
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
    if (!feedback) return picked.includes(index) ? "option-picked" : "";
    if (feedback.correct.includes(index)) return "option-right";
    if (picked.includes(index)) return "option-wrong";
    return "text-muted";
  }
</script>

<svelte:window onkeydown={onKey} />

<!-- The lesson shell (design/playful-redesign: Quiz): close, progress and count on top, one
     question, and a band at the bottom that holds Submit, then becomes the feedback sheet. -->
<div class="flex min-h-screen flex-col">
  <div class="sticky top-0 z-10 bg-base-100 px-12 pb-3 pt-6">
    <div class="mx-auto flex max-w-5xl items-center gap-5">
      <button class="btn btn-quiet btn-sm btn-square" aria-label="Quit" title="Quit" onclick={quit} disabled={busy}><X class="size-6" strokeWidth={2.6} /></button>
      <div class="track flex-1" role="progressbar" aria-valuemin={1} aria-valuemax={current.total} aria-valuenow={current.position} aria-label="Progress" aria-valuetext="Question {current.position} of {current.total}">
        {#each current.track as cell, index (index)}<span style={trackColour[cell]}></span>{/each}
      </div>
      {#if totalLeft !== null}
        <span role="timer" aria-label="Time left" class="timer-chip {low(totalLeft, initial.total_seconds) ? 'timer-low' : ''}"><Timer class="size-4" />{clock(totalLeft)}</span>
      {/if}
      {#if questionLeft !== null && feedback === null}
        <span role="timer" aria-label="Time for this question" class="timer-chip {low(questionLeft, current.per_question_seconds) ? 'timer-low' : ''}"><Timer class="size-4" />{clock(questionLeft)}</span>
      {/if}
      <span class="chip {current.exam ? 'chip-info' : ''}">{current.exam ? "Exam" : "Study"}</span>
      <span class="whitespace-nowrap tabular-nums"><b>{current.position}</b> <span class="text-muted">of {current.total}</span></span>
    </div>
  </div>

  <div class="flex-1 px-12 pb-10 pt-8">
    <div class="mx-auto max-w-[44rem]">
      {#key current.position}<div class="rise">
      {#if current.short}<p class="eyebrow">Short answer</p>
      {:else if current.multiple}<p class="eyebrow">Select all that apply{#if current.answers_to_pick} · <span>Pick {current.answers_to_pick}</span>{/if}</p>{/if}
      <div role="heading" aria-level="1" class="mt-2 text-[28px] font-extrabold leading-tight"><Rich text={current.text} /></div>

      {#if current.short}
        <!-- svelte-ignore a11y_autofocus -->
        <textarea class="textarea answer-box mt-8 w-full {feedback && !feedback.short?.overruled ? (feedback.right ? 'is-right' : 'is-wrong') : ''}" rows="2" aria-label="Your answer" placeholder="Type your answer…" readonly={feedback !== null} autofocus bind:value={typed} onkeydown={answerKeys}></textarea>
        {#if !feedback}<p class="mt-3 text-[13px] text-muted"><kbd class="kbd kbd-xs">Enter</kbd> submits, <kbd class="kbd kbd-xs">Shift</kbd><kbd class="kbd kbd-xs">Enter</kbd> adds a new line, <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">G</kbd> flags a guess</p>{/if}
      {:else}
      <div class="mt-7 space-y-3" role={current.multiple ? "group" : "radiogroup"} aria-label="Answers">
        {#each current.options as option, position (option.index)}
          <button
            class="option {optionState(option.index)}"
            aria-pressed={picked.includes(option.index)}
            onclick={() => pick(option.index)}
          >
            {#if feedback?.correct.includes(option.index)}<span class="option-key option-key-right pop"><Check class="size-4" strokeWidth={3.4} aria-hidden="true" /><span class="sr-only">{position + 1}</span></span>
            {:else if feedback && picked.includes(option.index)}<span class="option-key option-key-wrong pop"><X class="size-4" strokeWidth={3.4} aria-hidden="true" /><span class="sr-only">{position + 1}</span></span>
            {:else}<span class="option-key tabular-nums">{position + 1}</span>{/if}
            <Rich inline text={option.text} class="flex-1" />
            {#if feedback?.correct.includes(option.index)}<span class="flex items-center gap-1.5 text-[13px] font-extrabold uppercase tracking-[.06em] text-right-ink"><Check class="size-4" strokeWidth={3} />Correct</span>
            {:else if feedback && picked.includes(option.index)}<span class="flex items-center gap-1.5 text-[13px] font-extrabold uppercase tracking-[.06em] text-wrong-ink"><X class="size-4" strokeWidth={3} />Your pick</span>{/if}
          </button>
        {/each}
      </div>
      {#if !feedback}<p class="mt-3 text-[13px] text-muted">Press <kbd class="kbd kbd-xs">1</kbd> to <kbd class="kbd kbd-xs">{current.options.length}</kbd> to answer, <kbd class="kbd kbd-xs">F</kbd> to flag a guess.</p>{/if}
      {/if}
      </div>{/key}
      {#if error}<p role="alert" class="mt-6 text-sm text-error">{error}</p>{/if}
    </div>
  </div>

  {#if feedback}
    {@const tone = feedback.right ? "right" : "wrong"}
    <!-- Feedback sheet: verdict, explanation and Next share one band, with an icon and a word -->
    <div class="sheet sheet-{tone} rise" role="status">
      <div class="mx-auto flex max-w-5xl items-start gap-6">
        <span class="sheet-icon" aria-hidden="true">{#if feedback.right}<Check class="size-8" strokeWidth={3.4} />{:else}<X class="size-8" strokeWidth={3.4} />{/if}</span>
        <div class="min-w-0 flex-1">
          {#if feedback.short && verdict}
            {@const short = feedback.short}
            <p class="flex flex-wrap items-center gap-2 text-[24px] font-black leading-tight sheet-ink">
              {verdict.label}
              {#if verdict.note}<span class="text-[15px] font-bold text-muted">{verdict.note}</span>{/if}
              {#if short.overruled}<span class="overruled">You overruled this</span>{/if}
              {#if current.flagged}<span class="chip chip-guessed"><Flag class="size-3" />Guessed</span>{/if}
            </p>
            {#if verdict.keywords}
              <div class="mt-2 flex flex-wrap gap-2" aria-label="Keywords">
                {#each verdict.keywords.hit as word (word)}<span class="kw kw-hit"><Check class="size-3" />{word}<span class="sr-only">, found</span></span>{/each}
                {#each verdict.keywords.missed as word (word)}<span class="kw kw-miss"><X class="size-3" />{word}<span class="sr-only">, missed</span></span>{/each}
              </div>
            {/if}
            <p class="mt-3 text-[13px] text-muted">Model answer</p>
            <Rich class="mt-0.5" text={short.shown_answer} />
            {#if !feedback.right && short.also_accepted.length}<p class="mt-2 text-sm text-muted">Also accepted: {short.also_accepted.join(", ")}</p>{/if}
            {#if feedback.explanation}<Rich class="mt-3 leading-relaxed" text={feedback.explanation} />{/if}
            {#each feedback.citations as citation, index (index)}<p class="mt-2 flex items-center gap-2 text-[13px] font-extrabold uppercase tracking-[.06em] sheet-ink"><BookOpen class="size-4" />{citation.location}</p>{/each}
            {#if short.overruled}
              <button class="btn btn-ghost btn-sm mt-3" disabled={busy} onclick={overrule}><Undo2 class="size-4" />Undo, count it as {short.marking.right ? "right" : "wrong"} <kbd class="kbd kbd-xs">O</kbd></button>
            {:else}
              <div class="mt-3 flex items-center gap-3">
                <span class="text-sm text-muted">{feedback.right ? "Marked right but shouldn't be?" : "Think you had it?"}</span>
                <button class="btn btn-ghost btn-sm" disabled={busy} onclick={overrule}>{#if feedback.right}<X class="size-4" />{:else}<Check class="size-4" />{/if}Count it as {feedback.right ? "wrong" : "right"} <kbd class="kbd kbd-xs">O</kbd></button>
              </div>
            {/if}
          {:else}
            <p class="flex flex-wrap items-center gap-2 text-[24px] font-black leading-tight sheet-ink">
              {#if feedback.right}Correct{:else}{feedback.auto_reason === "Incomplete" ? "Wrong: incomplete, you missed a correct option" : feedback.auto_reason === "TimedOut" ? "Wrong: time ran out" : "Wrong"}{/if}
              {#if current.flagged}<span class="chip chip-guessed"><Flag class="size-3" />Guessed</span>{/if}
            </p>
            {#if feedback.explanation}<Rich class="mt-1 max-w-prose leading-relaxed" text={feedback.explanation} />{/if}
            {#each feedback.citations as citation, index (index)}<p class="mt-2 flex items-center gap-2 text-[13px] font-extrabold uppercase tracking-[.06em] sheet-ink"><BookOpen class="size-4" />{citation.location}</p>{/each}
          {/if}
        </div>
        <button class="btn btn-{tone === 'right' ? 'success' : 'wrong'} min-w-48 self-center" onclick={next}>Next question <kbd class="kbd kbd-xs">Enter</kbd></button>
      </div>
    </div>
  {:else}
    <div class="sheet-foot">
      <div class="mx-auto flex max-w-5xl items-center gap-3">
        <button
          class="btn {current.flagged ? 'btn-guessed' : 'btn-ghost'}"
          aria-pressed={current.flagged}
          onclick={toggleFlag}><Flag class="size-4" />{current.flagged ? "Flagged as guessed" : "I guessed"} {#if current.short}<kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">G</kbd>{:else}<kbd class="kbd kbd-xs">F</kbd>{/if}</button>
        {#if current.exam}<span class="text-sm text-muted">No going back in exam mode{current.short ? ". You can overrule the marking on the results page." : ""}</span>{/if}
        <button class="btn btn-primary ml-auto min-w-48" onclick={submit} disabled={(current.short ? !typed.trim() : picked.length === 0) || busy}>Submit <kbd class="kbd kbd-xs">Enter</kbd></button>
      </div>
    </div>
  {/if}
</div>

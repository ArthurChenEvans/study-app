<script lang="ts">
  // free-recall.feature (R55 to R57, R61): dump, then tick the checklist, then the result.
  import { ArrowDown, ArrowUp, Check, EyeOff, Pencil, Play, Plus, Timer, X } from "@lucide/svelte";
  import { onDestroy } from "svelte";
  import { api, errorMessage, type CheckResult, type ChecklistView, type Preview, type RecallRow, type Schedule } from "../lib/api";
  import Ladder from "../components/Ladder.svelte";
  import Rich from "../components/Rich.svelte";
  import { onDay } from "../lib/dates";
  import { celebrate, praise } from "../lib/celebrations.svelte";

  let { id, onLeave, onEdit, onNext }: { id: number; onLeave: () => void; onEdit: (id: number) => void; onNext: (id: number) => void } = $props();

  let view = $state<ChecklistView | null>(null);
  let schedule = $state<Schedule | null>(null);
  let today = $state(0);
  let stage = $state<"dump" | "check" | "result">("dump");
  let outside = $state(false);
  let dump = $state("");
  let seconds = $state(0);
  let ticked = $state<Set<number>>(new Set());
  let preview = $state<Preview | null>(null);
  let result = $state<CheckResult | null>(null);
  let nextDue = $state<RecallRow | null>(null);
  let error = $state("");
  let busy = $state(false);
  let leaving = $state(false);
  let addingPoint = $state(false);
  let extra = $state("");

  const timer = setInterval(() => { if (stage === "dump") seconds += 1; }, 1000);
  onDestroy(() => clearInterval(timer));
  const clock = (total: number) => `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  const words = $derived(dump.trim() ? dump.trim().split(/\s+/).length : 0);
  const steps = ["Recall", "Check", "Result"];
  const stageIndex = $derived(stage === "dump" ? 0 : stage === "check" ? 1 : 2);

  async function load() {
    [view, schedule, today] = await Promise.all([api.checklist(id), api.recallSchedule(), api.today()]);
  }

  async function updatePreview() {
    try {
      preview = await api.previewCheck(id, ticked.size);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function toCheck() {
    stage = "check";
    updatePreview();
  }

  function toggle(index: number) {
    const next = new Set(ticked);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    ticked = next;
    updatePreview();
  }

  async function addMissing(event: SubmitEvent) {
    event.preventDefault();
    if (!view || !extra.trim()) return;
    view.points = await api.savePoints(id, [...view.points, { text: extra.trim(), question: null, note: null }]);
    view.row.points = view.points.length;
    extra = "";
    addingPoint = false;
    updatePreview();
  }

  async function finish() {
    if (busy) return;
    busy = true;
    error = "";
    try {
      result = await api.finishCheck(id, outside ? null : dump, [...ticked]);
      stage = "result";
      celebrate("recall", {
        title: praise("recall"),
        line: result.from === null ? `Started on ${result.to}.` : result.passed ? `Up to ${result.to}.` : `Everything you remembered is logged.`,
        stats: [
          { label: "Score", value: result.score, suffix: "%", tone: "primary" },
          { label: "Recalled", value: result.recalled, suffix: `/${result.total}`, tone: "info" },
        ],
      });
      const rows = await api.recallList();
      nextDue = rows.find((row) => row.due_today && row.id !== id && row.points > 0) ?? null;
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function leave() {
    if (stage !== "result" && (dump.trim() || ticked.size)) leaving = true;
    else onLeave();
  }

  function keys(event: KeyboardEvent) {
    if (event.key !== "Enter" || !(event.ctrlKey || event.metaKey) || leaving) return;
    event.preventDefault();
    if (stage === "dump") toCheck();
    else if (stage === "check") finish();
  }

  load();
</script>

<svelte:window onkeydown={keys} />

{#if view && schedule}
  <div class="sticky top-0 z-10 grid grid-cols-[auto_minmax(0,1fr)_auto] tab:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-3 tab:gap-6 bg-base-100 px-12 py-4">
    <div class="flex items-center gap-3"><button class="btn btn-quiet btn-sm btn-square" aria-label="Leave" title="Leave" onclick={leave}><X class="size-6" strokeWidth={2.6} /></button><span class="sr-only">Recall: {view.row.subject_name}</span></div>
    <ol class="flex items-center gap-2" aria-label="Session steps">
      {#each steps as name, index (name)}
        <li class="flex items-center gap-2 text-xs font-extrabold uppercase tracking-wider {index <= stageIndex ? '' : 'text-muted'}" aria-current={index === stageIndex ? "step" : undefined}>
          <span class="step-dot {index <= stageIndex ? 'done' : ''}">{#if index < stageIndex}<Check class="size-4" strokeWidth={3} /><span class="sr-only">Done:</span>{:else}{index + 1}{/if}</span><span class={index === stageIndex ? '' : 'max-tab:sr-only'}>{name}</span>
        </li>
        {#if index < 2}<li aria-hidden="true" class="h-1 w-6 rounded-full bg-base-300"></li>{/if}
      {/each}
    </ol>
    <span class="justify-self-end"><span class="chip">{view.row.step ? `${view.row.step} step` : "First check"}</span></span>
  </div>

  {#if stage === "dump"}
    <div class="mx-auto max-w-3xl px-12 pb-24">
      <div class="flex items-end justify-between gap-6 pt-6">
        <div><p class="text-sm text-muted">Everything you remember about</p><h1 class="mt-1 text-[30px] font-black">{view.row.subject_name}</h1></div>
        <!-- No ticking clock while you recall: it pulls attention away. The time shows on the result. -->
      </div>
      <div class="mt-8 flex items-center justify-between gap-4">
        <div class="seg" role="radiogroup" aria-label="Where you recall">
          <button type="button" role="radio" aria-checked={!outside} onclick={() => (outside = false)}>Type it here</button>
          <button type="button" role="radio" aria-checked={outside} onclick={() => (outside = true)}>Outside the app</button>
        </div>
        <p class="flex items-center gap-2 text-sm text-muted"><EyeOff class="size-4" />The checklist stays hidden until you're done</p>
      </div>
      {#if outside}
        <div class="panel mt-6 p-8">
          <p class="text-lg font-semibold">Recall it your way</p>
          <p class="mt-2 max-w-prose text-muted">Write on paper, talk it through out loud, walk your memory palace or draw it. Come back when you've got everything you can, and tick off the checklist.</p>
        </div>
      {:else}
        <!-- svelte-ignore a11y_autofocus -->
        <textarea class="textarea mt-6 min-h-[22rem] w-full text-base leading-relaxed" aria-label="Your recall" placeholder="Start anywhere and keep going. Order and wording don't matter…" bind:value={dump} autofocus></textarea>
        <p class="mt-2 flex justify-between text-xs tabular-nums text-muted"><span>Saved with this check, so you can look back at it in History</span><span>{words} {words === 1 ? "word" : "words"}</span></p>
      {/if}
      <div class="mt-8 flex justify-end">
        <button class="btn btn-primary" onclick={toCheck}>{outside ? "I'm done, show the checklist" : "Done, check it"} <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">Enter</kbd></button>
      </div>
    </div>
  {:else if stage === "check"}
    <div class="grid h-[calc(100vh-70px)] grid-cols-2">
      <section class="overflow-y-auto border-r border-base-300 bg-base-200 px-10 py-10" aria-labelledby="dump-h">
        <h2 id="dump-h" class="eyebrow">Your recall</h2>
        {#if outside || !dump.trim()}<p class="mt-4 text-muted">Done outside the app.</p>{:else}<div class="mt-4 max-w-prose whitespace-pre-line text-[15px] leading-relaxed">{dump}</div>{/if}
      </section>
      <section class="overflow-y-auto px-10 pb-10" aria-labelledby="check-h">
        <div class="sticky top-0 z-10 bg-base-100 pb-4 pt-10">
          <h2 id="check-h" class="text-xl font-semibold">Tick what you recalled</h2>
          <p class="mt-1 text-sm text-muted">Only tick a point if it was in your recall, not if you recognise it now.</p>
          {#if preview}
            <div class="panel mt-6 p-5" aria-label="Score so far">
              <div class="flex items-baseline justify-between">
                <p><span class="text-2xl font-semibold tabular-nums">{preview.score}%</span> <span class="text-sm tabular-nums text-muted">{ticked.size} of {view.points.length} recalled</span></p>
                <p class="flex items-center gap-1 text-sm font-semibold {preview.passed ? 'text-right-ink' : 'text-wrong-ink'}">{#if preview.passed}<ArrowUp class="size-4" />Passing{:else}<ArrowDown class="size-4" />Under the pass mark{/if}</p>
              </div>
              <div class="meter mt-3" role="img" aria-label="{preview.score}% with the pass mark at {schedule.pass_mark}%"><div class="fill" style="width: {preview.score}%"></div><div class="mark" style="left: {schedule.pass_mark}%"></div></div>
              <div class="mt-4 flex items-center justify-between gap-4">
                <p class="text-sm text-muted" aria-live="polite">{view.row.position === null ? "Starts on" : preview.passed ? "Moves up to" : preview.to === view.row.step ? "Stays on" : "Goes back to"} {preview.to}, next check {onDay(preview.next_due, today)}</p>
                <button class="btn btn-primary shrink-0" disabled={busy} onclick={finish}>Finish check <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">Enter</kbd></button>
              </div>
            </div>
          {/if}
          {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
        </div>
        <ol class="panel mt-2 divide-y" aria-label="Checklist">
          {#each view.points as point, index (index)}
            {@const on = ticked.has(index)}
            <li><div role="checkbox" aria-checked={on} tabindex="0" class="flex min-h-11 cursor-pointer items-center gap-4 px-4 py-2 hover:bg-base-200"
              onclick={() => toggle(index)} onkeydown={(event) => { if (event.key === " " || (event.key === "Enter" && !event.ctrlKey && !event.metaKey)) { event.preventDefault(); toggle(index); } }}>
              <span class="tick">{on ? "✓" : ""}</span>
              <span class="flex-1 text-[15px]"><Rich inline text={point.text} /></span>
              <span class="whitespace-nowrap text-xs {on ? 'font-semibold text-right-ink' : 'text-muted'}">{on ? "Recalled" : "Not yet"}</span>
            </div></li>
          {/each}
        </ol>
        <div class="mt-4 flex items-center justify-between gap-3">
          {#if addingPoint}
            <form class="flex flex-1 gap-2" onsubmit={addMissing}>
              <!-- svelte-ignore a11y_autofocus -->
              <input class="input input-sm flex-1" aria-label="Missing point" placeholder="A point the checklist should have…" bind:value={extra} autofocus />
              <button class="btn btn-ghost btn-sm" type="submit">Add</button>
              <button class="btn btn-ghost btn-sm btn-square" type="button" aria-label="Cancel" onclick={() => (addingPoint = false)}><X class="size-4" /></button>
            </form>
          {:else}
            <button class="btn btn-ghost btn-sm" onclick={() => (addingPoint = true)}><Plus class="size-4" />Add a point the list is missing</button>
          {/if}
          <p class="text-xs text-muted">Space ticks the focused point, Tab moves to the next</p>
        </div>
      </section>
    </div>
  {:else if result}
    <div class="mx-auto max-w-3xl px-12 pb-24">
      <section class="pt-12" aria-labelledby="score-h">
        <p class="text-sm text-muted">{view.row.subject_name}</p>
        <h1 id="score-h" class="mt-2 flex items-baseline gap-4"><span class="text-5xl font-semibold tabular-nums">{result.score}%</span><span class="text-lg tabular-nums text-muted">{result.recalled} of {result.total} recalled</span></h1>
        <p class="mt-2 text-sm tabular-nums text-muted">{result.previous_score === null ? "First check" : `Last check ${result.previous_score}%`} · pass mark {schedule.pass_mark}% · <span class="inline-flex items-center gap-1"><Timer class="size-3.5" aria-hidden="true" />Recalled in {clock(seconds)}</span></p>
      </section>
      <section class="panel mt-8 p-6" aria-labelledby="next-h">
        <h2 id="next-h" class="flex items-center gap-2 text-lg font-semibold">
          {#if result.from === null}Started on {result.to}{:else if result.from === result.to}Stayed on {result.to}{:else if result.passed}<ArrowUp class="size-5" />Up a step{:else}<ArrowDown class="size-5" />Back a step{/if}
        </h2>
        <p class="mt-1 text-muted">{result.from ? `${result.from} → ${result.to}. ` : ""}Next check <span class="font-semibold text-base-content">{onDay(result.next_due, today)}</span>.</p>
        <div class="mt-6"><Ladder steps={schedule.steps} position={result.position} step={result.to} next={result.next_step} /></div>
      </section>
      {#if result.missed.length}
        <section class="mt-12" aria-labelledby="missed-h">
          <h2 id="missed-h" class="eyebrow mb-3">Missed this time · {result.missed.length}</h2>
          <ul class="panel divide-y" aria-label="Missed points">
            {#each result.missed as missed (missed.text)}
              <li class="flex items-center gap-4 px-4 py-3"><span class="text-wrong-ink" aria-hidden="true"><X class="size-4" /></span><span class="flex-1 text-[15px]"><Rich inline text={missed.text} /></span>{#if missed.checks > 1}<span class="whitespace-nowrap text-xs text-muted">Missed {missed.missed} of {missed.checks} {missed.checks === 1 ? "check" : "checks"}</span>{/if}</li>
            {/each}
          </ul>
          <p class="mt-3 text-sm text-muted">These are what to look over before the next check.</p>
        </section>
      {:else}
        <p class="panel mt-12 p-6 font-semibold">Everything on the checklist, recalled.</p>
      {/if}
      <div class="mt-12 flex items-center justify-end gap-3">
        <button class="btn btn-ghost" onclick={() => onEdit(id)}><Pencil class="size-4" />Edit checklist</button>
        {#if nextDue}
          <button class="btn btn-primary" onclick={() => nextDue && onNext(nextDue.id)}><Play class="size-4" />Next: {nextDue.subject_name}</button>
        {:else}
          <button class="btn btn-primary" onclick={onLeave}>Back to Recall</button>
        {/if}
      </div>
    </div>
  {/if}
{/if}

{#if leaving}
  <dialog open class="modal modal-open" aria-labelledby="leave-h">
    <div class="modal-box max-w-md">
      <h3 id="leave-h" class="text-lg font-semibold">Leave without saving this recall?</h3>
      <p class="mt-2 text-muted">Nothing is saved until you finish the check, so what you've typed and ticked will be lost.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (leaving = false)}>Keep going</button>
        <button class="btn btn-error" onclick={onLeave}>Leave</button>
      </div>
    </div>
  </dialog>
{/if}

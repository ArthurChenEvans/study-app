<script lang="ts">
  // free-recall.feature (R53, R54, R59): points, adding from questions, and history.
  import { ArrowLeft, FileText, GripVertical, ListPlus, Play, Plus, Trash2, X } from "@lucide/svelte";
  import { api, errorMessage, type Candidate, type ChecklistView, type RecallHistory, type RecallItem, type RecallPoint, type Schedule } from "../lib/api";
  import Ladder from "../components/Ladder.svelte";
  import Rich from "../components/Rich.svelte";
  import ScoreChart from "../components/ScoreChart.svelte";
  import { dayLabel, onDay } from "../lib/dates";

  let { id, onBack, onStart }: { id: number; onBack: () => void; onStart: (id: number) => void } = $props();

  let view = $state<ChecklistView | null>(null);
  let schedule = $state<Schedule | null>(null);
  let history = $state<RecallHistory | null>(null);
  let today = $state(0);
  let tab = $state<"points" | "history">("points");
  let points = $state<RecallPoint[]>([]);
  let editingIndex = $state<number | null>(null);
  let newPoint = $state("");
  let status = $state("");
  let error = $state("");
  let dragFrom = $state<number | null>(null);

  let importing = $state(false);
  let candidates = $state<Candidate[]>([]);
  let filter = $state("");
  // Keyed "Question:12" or "Note:5", since a question and a card can share an id.
  let picked = $state<Set<string>>(new Set());
  let unpicked = $state<Set<string>>(new Set());
  let from = $state<"mcq" | "cards">("mcq");
  const keyOf = (item: RecallItem) => ("Question" in item ? `Question:${item.Question}` : `Note:${item.Note}`);
  const itemOf = (key: string): RecallItem => {
    const [kind, id] = key.split(":");
    return kind === "Question" ? { Question: Number(id) } : { Note: Number(id) };
  };
  const isCard = (candidate: Candidate) => "Note" in candidate.item;
  let deleting = $state(false);
  let openDump = $state<number | null>(null);

  async function load() {
    [view, schedule, today] = await Promise.all([api.checklist(id), api.recallSchedule(), api.today()]);
    points = view.points;
  }

  async function save(next: RecallPoint[], message = "Saved") {
    error = "";
    try {
      points = await api.savePoints(id, next);
      if (view) view.row.points = points.length;
      status = message;
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function add(event: SubmitEvent) {
    event.preventDefault();
    if (!newPoint.trim()) return;
    save([...points, { text: newPoint.trim(), question: null, note: null }], "Point added");
    newPoint = "";
  }

  function finishEdit(index: number, text: string) {
    editingIndex = null;
    if (text.trim() === points[index].text) return;
    save(points.map((point, position) => (position === index ? { ...point, text } : point)));
  }

  function move(from: number, to: number) {
    if (to < 0 || to >= points.length || from === to) return;
    const next = [...points];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    save(next, `Moved to position ${to + 1}`);
  }

  function pointKeys(event: KeyboardEvent, index: number) {
    if (!event.altKey || !["ArrowUp", "ArrowDown"].includes(event.key)) return;
    event.preventDefault();
    const to = index + (event.key === "ArrowUp" ? -1 : 1);
    move(index, to);
    queueMicrotask(() => document.querySelector<HTMLElement>(`[data-point="${to}"]`)?.focus());
  }

  async function openImport() {
    candidates = await api.recallCandidates(id);
    from = candidates.some((candidate) => !isCard(candidate)) || !candidates.length ? "mcq" : "cards";
    picked = new Set();
    unpicked = new Set();
    filter = "";
    importing = true;
  }

  const inTab = $derived(candidates.filter((c) => isCard(c) === (from === "cards")));
  const shown = $derived(inTab.filter((c) => `${c.text} ${c.answer} ${c.topic_name}`.toLowerCase().includes(filter.trim().toLowerCase())));
  const cardCount = $derived(candidates.filter(isCard).length);
  const adds = $derived(picked.size);
  const removes = $derived(unpicked.size);
  const importLabel = $derived(adds && removes ? `Add ${adds}, remove ${removes}` : adds ? `Add ${adds} ${adds === 1 ? "point" : "points"}` : removes ? `Remove ${removes} ${removes === 1 ? "point" : "points"}` : "No changes yet");

  function togglePick(candidate: Candidate) {
    const set = new Set(candidate.in_checklist ? unpicked : picked);
    const key = keyOf(candidate.item);
    if (set.has(key)) set.delete(key);
    else set.add(key);
    if (candidate.in_checklist) unpicked = set;
    else picked = set;
  }

  async function applyImport() {
    error = "";
    try {
      points = await api.addFromItems(id, [...picked].map(itemOf), [...unpicked].map(itemOf));
      if (view) view.row.points = points.length;
      importing = false;
      status = importLabel.replace(/^Add/, "Added").replace(/^Remove/, "Removed").replace(", remove", ", removed");
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function showHistory() {
    tab = "history";
    history ??= await api.recallHistory(id);
  }

  async function remove() {
    await api.deleteChecklist(id);
    onBack();
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <button class="mt-8 inline-flex items-center gap-1 py-1 text-sm text-muted hover:underline" onclick={onBack}><ArrowLeft class="size-4" />Recall</button>
  {#if view && schedule}
    <div class="flex items-end justify-between gap-6 pb-6 pt-4">
      <div>
        <p class="text-sm text-muted">{view.row.module ? "Module" : "Topic"}{view.row.subject_exists ? "" : " (deleted)"} · {points.length} {points.length === 1 ? "point" : "points"}</p>
        <h1 class="mt-1 text-3xl font-semibold">{view.row.subject_name}</h1>
      </div>
      <button class="btn btn-primary" disabled={points.length === 0} onclick={() => onStart(id)}><Play class="size-4" />Start recall</button>
    </div>

    <div class="flex items-center gap-1 border-b border-base-300" role="tablist" aria-label="Checklist views">
      <button role="tab" aria-selected={tab === "points"} class="px-3 py-2 text-sm {tab === 'points' ? 'border-b-2 border-primary font-semibold' : 'text-muted'}" onclick={() => (tab = "points")}>Checklist <span class="ml-1 text-xs font-normal text-muted">{points.length}</span></button>
      <button role="tab" aria-selected={tab === "history"} class="px-3 py-2 text-sm {tab === 'history' ? 'border-b-2 border-primary font-semibold' : 'text-muted'}" onclick={showHistory}>History</button>
    </div>

    {#if tab === "points"}
      <div class="mt-8 grid grid-cols-[1fr_300px] items-start gap-12">
        <div>
          <div class="mb-3 flex items-center justify-between">
            <h2 class="text-sm font-semibold">Points to recall</h2>
            <button class="btn btn-ghost btn-sm" onclick={openImport}><ListPlus class="size-4" />Add from questions or cards</button>
          </div>
          {#if points.length === 0}
            <div class="panel p-8 text-center">
              <p class="font-semibold">No points yet</p>
              <p class="mt-1 text-sm text-muted">Write the main things you should be able to recall, one per point, or add them from your questions and flashcards.</p>
            </div>
          {:else}
            <ol class="panel divide-y" aria-label="Points to recall">
              {#each points as point, index (index)}
                <li class="flex min-h-11 items-center gap-3 px-4 py-2 {dragFrom === index ? 'opacity-50' : ''}" draggable="true"
                  ondragstart={() => (dragFrom = index)} ondragend={() => (dragFrom = null)} ondragover={(event) => event.preventDefault()}
                  ondrop={() => { if (dragFrom !== null) move(dragFrom, index); dragFrom = null; }}>
                  <span class="cursor-grab text-muted" title="Drag to reorder, or Alt+Up and Alt+Down" aria-hidden="true"><GripVertical class="size-4" /></span>
                  <span class="w-6 text-right text-xs tabular-nums text-muted">{index + 1}</span>
                  {#if editingIndex === index}
                    <!-- svelte-ignore a11y_autofocus -->
                    <input class="input input-sm flex-1" aria-label="Point {index + 1}" value={point.text} autofocus
                      onblur={(event) => finishEdit(index, event.currentTarget.value)}
                      onkeydown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); if (event.key === "Escape") editingIndex = null; }} />
                  {:else}
                    <button class="min-w-0 flex-1 rounded px-2 py-1 text-left text-[15px] hover:bg-base-200" data-point={index} aria-label="Edit point {index + 1}: {point.text}" title="Click to edit, Alt+Up or Alt+Down to move"
                      onclick={() => (editingIndex = index)} onkeydown={(event) => pointKeys(event, index)}><Rich inline text={point.text} /></button>
                  {/if}
                  {#if point.question !== null}<span class="badge-src" title="Added from an MCQ. Removing it here doesn't change the question.">From MCQ</span>
                  {:else if point.note !== null}<span class="badge-src" title="Added from a flashcard. Removing it here doesn't change the card.">From card</span>{/if}
                  <button class="btn btn-ghost btn-sm btn-square" aria-label="Remove point {index + 1}" onclick={() => save(points.filter((_, position) => position !== index), "Point removed")}><X class="size-4" /></button>
                </li>
              {/each}
            </ol>
          {/if}
          <form class="mt-3 flex gap-2" onsubmit={add}>
            <input class="input flex-1" aria-label="New point" placeholder="e.g. Write-back vs write-through caches…" bind:value={newPoint} />
            <button class="btn btn-ghost" type="submit"><Plus class="size-4" />Add point</button>
          </form>
          <p class="mt-2 text-sm text-muted">Markdown and maths in <code>$…$</code> work. Keep each point to one idea, so ticking it is a clear yes or no.</p>
          {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
          <p role="status" class="mt-3 text-sm text-muted">{status}</p>
        </div>

        <aside class="panel sticky top-8 p-6" aria-label="Schedule">
          <p class="eyebrow">Schedule</p>
          <p class="mt-3 text-2xl font-semibold">{view.row.step ?? "Not started"}</p>
          <p class="text-sm text-muted">Next check {onDay(view.row.due, today)}</p>
          <div class="mt-4"><Ladder steps={schedule.steps} position={view.row.position} step={view.row.step} next={view.row.next_step} /></div>
          <dl class="mt-6 space-y-3 text-sm">
            <div class="flex justify-between"><dt class="text-muted">Last check</dt><dd class="tabular-nums">{view.row.last_score === null ? "None yet" : `${view.row.last_score}%`}</dd></div>
            <div class="flex justify-between gap-4"><dt class="text-muted">Covers</dt><dd class="text-right">{view.topics.join(", ") || "No topics now"}</dd></div>
          </dl>
          {#if points.length === 0}<p class="mt-6 text-sm text-muted">Needs at least one point to start.</p>{/if}
          <button class="btn btn-ghost btn-sm mt-6 w-full justify-start" onclick={() => (deleting = true)}><Trash2 class="size-4" />Delete checklist</button>
        </aside>
      </div>
    {:else if history}
      {#if history.checks.length === 0}
        <div class="panel mt-8 max-w-lg p-8"><p class="font-semibold">No checks yet</p><p class="mt-1 text-sm text-muted">Each recall you finish shows up here with its score and step change.</p></div>
      {:else}
        <div class="mt-8 grid grid-cols-[1fr_300px] items-start gap-12">
          <div class="space-y-8">
            <section class="panel p-6" aria-labelledby="trend-h">
              <div class="flex items-baseline justify-between"><h2 id="trend-h" class="text-sm font-semibold">Score per check</h2><span class="text-xs text-muted">Pass mark {history.pass_mark}%</span></div>
              <div class="mt-4"><ScoreChart points={[...history.checks].reverse().map((check) => ({ label: dayLabel(check.day, today), percent: check.score, retry: false, detail: check.from ? `${check.from} → ${check.to}` : `Started on ${check.to}` }))} detailLabel="Step" /></div>
            </section>
            <section aria-labelledby="checks-h">
              <h2 id="checks-h" class="eyebrow mb-3">Checks</h2>
              <div class="panel overflow-hidden">
                <table class="table" aria-label="Checks">
                  <thead><tr class="text-muted"><th>Date</th><th>Score</th><th>Step</th><th>Dump</th></tr></thead>
                  <tbody>
                    {#each history.checks as check, index (index)}
                      <tr>
                        <td class="tabular-nums">{dayLabel(check.day, today)}</td>
                        <td class="tabular-nums"><span class="font-semibold">{check.score}%</span> <span class="text-xs {check.passed ? 'text-right-ink' : 'text-wrong-ink'}">{check.passed ? "passed" : "under pass mark"}</span></td>
                        <td class="text-sm">{check.from ? `${check.from} → ${check.to}` : `Started on ${check.to}`}</td>
                        <td>{#if check.dump}<button class="btn btn-ghost btn-xs" aria-expanded={openDump === index} onclick={() => (openDump = openDump === index ? null : index)}><FileText class="size-3.5" />{openDump === index ? "Hide dump" : "View dump"}</button>{:else}<span class="text-sm text-muted">Outside the app</span>{/if}</td>
                      </tr>
                      {#if openDump === index && check.dump}<tr><td colspan="4" class="whitespace-pre-line bg-base-200 text-[15px] leading-relaxed">{check.dump}</td></tr>{/if}
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          </div>
          <aside class="panel p-6" aria-label="Missed most often">
            <p class="eyebrow">Missed most often</p>
            {#if history.most_missed.length}
              <ol class="mt-4 space-y-4 text-sm">
                {#each history.most_missed as missed (missed.text)}<li><p><Rich inline text={missed.text} /></p><p class="text-xs tabular-nums text-muted">Missed {missed.missed} of {missed.checks} {missed.checks === 1 ? "check" : "checks"}</p></li>{/each}
              </ol>
            {:else}<p class="mt-4 text-sm text-muted">Nothing missed yet.</p>{/if}
          </aside>
        </div>
      {/if}
    {/if}
  {/if}
</div>

{#if importing}
  <dialog open class="modal modal-open" aria-labelledby="import-h">
    <div class="modal-box max-w-3xl">
      <h3 id="import-h" class="text-lg font-semibold">Add points from questions and cards</h3>
      <p class="mt-2 text-sm text-muted">Each one becomes a point: a question and its answer, a card's front and back, or a cloze card's text with the answers in bold. You can edit the point after, and the question or card stays as it is. Untick one that's already in the checklist to remove its point.</p>
      <div class="mt-6 flex items-center justify-between gap-4">
        <div class="seg" role="radiogroup" aria-label="Add from">
          <button type="button" role="radio" aria-checked={from === "mcq"} onclick={() => (from = "mcq")}>MCQ <span class="text-xs tabular-nums">{candidates.length - cardCount}</span></button>
          <button type="button" role="radio" aria-checked="false" disabled title="Not built yet" class="opacity-60">Short answer</button>
          <button type="button" role="radio" aria-checked={from === "cards"} onclick={() => (from = "cards")}>Flashcards <span class="text-xs tabular-nums">{cardCount}</span></button>
        </div>
        <input type="search" class="input input-sm w-64" aria-label="Filter" placeholder="Filter…" bind:value={filter} />
      </div>
      <p class="mt-2 text-xs text-muted">From {view?.topics.length ? view.topics.join(", ") : "no topics"}. Archived cards aren't listed. Short answer appears here once it's built.</p>
      {#if inTab.length === 0}
        <p class="panel mt-4 p-6 text-sm text-muted">{from === "mcq" ? "There are no MCQs in these topics yet." : "There are no flashcards in these topics yet."}</p>
      {:else}
        <ul class="panel mt-4 max-h-80 divide-y overflow-y-auto" aria-label={from === "mcq" ? "Questions" : "Flashcards"}>
          {#each shown as candidate (keyOf(candidate.item))}
            {@const key = keyOf(candidate.item)}
            {@const on = candidate.in_checklist ? !unpicked.has(key) : picked.has(key)}
            <li>
              <label class="flex cursor-pointer items-start gap-4 px-4 py-3 hover:bg-base-100">
                <input type="checkbox" class="checkbox checkbox-sm mt-0.5" checked={on} onchange={() => togglePick(candidate)} />
                <span class="flex-1"><span class="text-sm"><Rich inline text={candidate.text} /></span>{#if candidate.answer}<span class="block text-sm text-muted">→ <Rich inline text={candidate.answer} /></span>{/if}</span>
                <span class="whitespace-nowrap text-right text-xs {candidate.in_checklist && !on ? 'font-semibold text-wrong-ink' : 'text-muted'}">{candidate.in_checklist ? (on ? "In the checklist" : "Will be removed") : candidate.topic_name}{#if isCard(candidate) && !candidate.in_checklist}<span class="block">{candidate.kind}</span>{/if}</span>
              </label>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (importing = false)}>Cancel</button>
        <button class="btn btn-primary" disabled={!adds && !removes} onclick={applyImport}>{importLabel}</button>
      </div>
    </div>
  </dialog>
{/if}

{#if deleting && view}
  <dialog open class="modal modal-open" aria-labelledby="delete-checklist-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-checklist-h" class="text-lg font-semibold">Delete the {view.row.subject_name} checklist?</h3>
      <p class="mt-2 text-muted">Its points and its check history are deleted too. Your questions aren't touched.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = false)}>Cancel</button>
        <button class="btn btn-error" onclick={remove}>Delete checklist</button>
      </div>
    </div>
  </dialog>
{/if}

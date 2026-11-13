<script lang="ts">
  // flashcards.feature (R62 to R64, R70, R71, R77): a topic's cards and the note editor.
  import { Archive, ArchiveRestore, ArrowLeft, Brackets, Flag, Play, Plus, Trash2, X } from "@lucide/svelte";
  import { tick, untrack } from "svelte";
  import { api, errorMessage, type Note, type NoteKind, type NoteView, type Subject, type TopicCards, type TopicRow } from "../lib/api";
  import { deleteWithUndo, toast } from "../lib/celebrations.svelte";
  import Rich from "../components/Rich.svelte";
  import { dueLabel } from "../lib/dates";

  let { topic, editNote = null, notice = "", onBack, onReview, onTopic = () => {} }: { topic: number; editNote?: number | "new" | null; notice?: string; onBack: () => void; onReview: (scope: Subject) => void; onTopic?: (topic: number, notice: string) => void } = $props();

  let data = $state<TopicCards | null>(null);
  let today = $state(0);
  let editing = $state<{ id: number | null; note: Note; view: NoteView | null } | null>(null);
  let error = $state("");
  let status = $state(untrack(() => notice)); // the route is keyed, so the first value is the one wanted
  let deleting = $state(false);
  let clozeBox = $state<HTMLTextAreaElement | null>(null);
  let topics = $state<TopicRow[]>([]);

  const blank = (kind: NoteKind = "Basic"): Note => ({ topic, kind, front: "", back: "", cues: [], text: "", extra: "" });
  const leech = $derived(editing?.view?.cards.find((card) => card.leech) ?? null);

  // The list: filter by what needs you, or type to find a card.
  // New and To review are separate: a topic's "due" count includes today's new cards.
  type Show = "all" | "new" | "due" | "tricky" | "archived";
  let show = $state<Show>("all");
  let query = $state("");
  const isDue = (card: TopicCards["cards"][number]) => !card.archived && !card.is_new && card.due <= today;
  const counts = $derived({
    all: data?.cards.length ?? 0,
    new: data?.cards.filter((card) => card.is_new && !card.archived).length ?? 0,
    due: data?.cards.filter(isDue).length ?? 0,
    tricky: data?.cards.filter((card) => card.leech && !card.archived).length ?? 0,
    archived: data?.cards.filter((card) => card.archived).length ?? 0,
  });
  const listed = $derived((data?.cards ?? []).filter((card) =>
    (show === "all" || (show === "new" && card.is_new && !card.archived) || (show === "due" && isDue(card)) || (show === "tricky" && card.leech && !card.archived) || (show === "archived" && card.archived))
    && card.front.toLowerCase().includes(query.trim().toLowerCase())));

  async function load() {
    const [loaded, loadedToday, loadedTopics] = await Promise.all([api.topicCards(topic), api.today(), api.topics()]);
    data = { ...loaded, cards: loaded.cards.filter((card) => !hiddenNotes.has(card.note)) };
    today = loadedToday;
    topics = loadedTopics;
  }

  async function open(id: number | null) {
    error = "";
    status = "";
    if (id === null) editing = { id: null, note: blank(), view: null };
    else {
      const view = await api.noteView(id);
      editing = { id, note: structuredClone(view.note), view };
    }
  }

  async function save(event?: Event) {
    event?.preventDefault();
    if (!editing) return;
    error = "";
    try {
      const note = $state.snapshot(editing.note);
      await api.saveNote(editing.id, { note, suggestion: null });
      status = editing.id === null ? `${count(note)} added` : "Changes saved";
      editing = null;
      if (note.topic !== topic) return onTopic(note.topic, `${status} to ${topics.find((row) => row.id === note.topic)?.name ?? "another topic"}`);
      await load();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function count(note: Note) {
    const cards = note.kind === "Basic" ? 1 + note.cues.filter((cue) => cue.trim()).length : note.kind === "Reversed" ? 2 : clozeNumbers(note.text).length;
    return `${cards} ${cards === 1 ? "card" : "cards"}`;
  }

  // ---- Cloze preview (the backend makes the real cards on save) ----
  const gapPattern = /\{\{c(\d+)::([^}]*?)(?:::([^}]*))?\}\}/g;
  function clozeNumbers(text: string): number[] {
    return [...new Set([...text.matchAll(gapPattern)].filter((m) => Number(m[1]) > 0 && m[2].trim()).map((m) => Number(m[1])))].sort((a, b) => a - b);
  }
  function clozeFront(text: string, number: number) {
    return text.replace(gapPattern, (_, n, answer, hint) => (Number(n) === number ? `<span class="cloze-gap">[${hint?.trim() || "..."}]</span>` : answer));
  }

  async function makeGap() {
    if (!editing || !clozeBox) return;
    const { selectionStart: start, selectionEnd: end, value } = clozeBox;
    const next = Math.max(0, ...clozeNumbers(value)) + 1;
    const chosen = value.slice(start, end) || "answer";
    editing.note.text = `${value.slice(0, start)}{{c${next}::${chosen}}}${value.slice(end)}`;
    await tick();
    clozeBox.focus();
    clozeBox.setSelectionRange(start + 6 + String(next).length, start + 6 + String(next).length + chosen.length);
  }

  async function toggleArchive(id: number, archived: boolean, quiet = false) {
    await api.archiveCard(id, archived);
    await load();
    if (editing?.id !== null && editing?.id !== undefined) editing.view = await api.noteView(editing.id);
    if (!quiet) toast({ text: archived ? "Card archived" : "Card back in reviews", icon: "check", action: { label: "Undo", run: () => toggleArchive(id, !archived, true) } });
  }

  // toasts.feature (R157): a deleted card goes at once, with a few seconds to undo.
  let hiddenNotes = $state<Set<number>>(new Set());

  function remove() {
    if (!editing?.id) return;
    const id = editing.id;
    deleteWithUndo({
      text: "Card deleted",
      hide: () => {
        hiddenNotes = new Set([...hiddenNotes, id]);
        deleting = false;
        editing = null;
        if (data) data.cards = data.cards.filter((card) => card.note !== id);
      },
      restore: () => {
        hiddenNotes = new Set([...hiddenNotes].filter((note) => note !== id));
        load();
      },
      commit: async () => {
        await api.deleteNote(id);
        hiddenNotes = new Set([...hiddenNotes].filter((note) => note !== id));
        await load();
      },
    });
  }

  function keys(event: KeyboardEvent) {
    if (!editing) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); save(); }
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "c" && editing.note.kind === "Cloze") { event.preventDefault(); makeGap(); }
  }

  load().then(() => { if (editNote !== null) open(editNote === "new" ? null : editNote); });
</script>

<svelte:window onkeydown={keys} />

<div class="mx-auto max-w-6xl px-12 pb-24">
  <button class="mt-8 inline-flex items-center gap-1 py-1 text-xs font-extrabold uppercase tracking-wider text-muted hover:underline" onclick={() => (editing ? (editing = null) : onBack())}><ArrowLeft class="size-4" />{editing ? data?.name : "Cards"}</button>
  {#if data}
    <div class="flex items-end justify-between gap-6 pb-6 pt-4">
      <div><p class="text-sm text-muted">{data.module ?? "No module"} · {data.cards.length} {data.cards.length === 1 ? "card" : "cards"} · {data.due} due</p><h1 class="mt-1 text-3xl font-semibold">{data.name}</h1></div>
      <div class="flex items-center gap-2">
        {#if !editing}<button class="btn btn-ghost" onclick={() => open(null)}><Plus class="size-4" />New card</button>{/if}
        <button class="btn {editing || !data.due ? 'btn-outline' : 'btn-primary'}" disabled={!data.due} onclick={() => onReview({ Topic: topic })}><Play class="size-4" />{data.due && !editing ? `Review the ${data.due} due` : `Review ${data.name}`}</button>
      </div>
    </div>
    {#if status && !editing}<p role="status" class="mb-4 text-sm text-muted">{status}</p>{/if}

    {#if !editing}
      {#if data.cards.length === 0}
        <div class="panel max-w-lg p-8"><p class="font-semibold">No cards in {data.name} yet</p><p class="mt-1 text-sm text-muted">Write a card for something you keep forgetting.</p><button class="btn btn-primary mt-4" onclick={() => open(null)}><Plus class="size-4" />New card</button></div>
      {:else}
        <div class="mb-4 flex flex-wrap items-center gap-3">
          <div class="seg" role="radiogroup" aria-label="Show">
            {#each [["all", "All"], ["new", "New"], ["due", "To review"], ["tricky", "Tricky"], ["archived", "Archived"]] as [key, label] (key)}
              <button type="button" role="radio" aria-checked={show === key} onclick={() => (show = key as Show)}>{label} <span class="tabular-nums">{counts[key as Show]}</span></button>
            {/each}
          </div>
          <span class="flex-1"></span>
          <input type="search" class="input input-sm w-72" aria-label="Filter cards" placeholder="Filter cards…" bind:value={query} />
        </div>
        <div class="panel overflow-hidden">
          <table class="table table-fixed" aria-label="Cards in {data.name}">
            <colgroup><col class="w-[40%]" /><col class="w-[12%]" /><col class="w-[16%]" /><col class="w-[12%]" /><col class="w-[10%]" /><col class="w-[10%]" /></colgroup>
            <thead><tr class="text-muted"><th>Front</th><th>Type</th><th>From</th><th>Next</th><th>Again</th><th><span class="sr-only">Archive</span></th></tr></thead>
            <tbody>
              {#each listed as card (card.id)}
                <tr class="hover:bg-base-100">
                  <td><button class="line-clamp-2 py-1 text-left hover:underline {card.archived ? 'text-muted' : ''}" onclick={() => open(card.note)}><Rich inline text={card.front} /></button></td>
                  <td><span class="chip">{card.kind}</span></td>
                  <td>{#if card.source && card.source !== "Written"}<span class="badge-src">{card.source}</span>{:else}<span class="text-sm text-muted">Written</span>{/if}</td>
                  <td class="text-sm tabular-nums">{#if card.archived}<span class="flex items-center gap-1 text-muted"><Archive class="size-3.5" />Archived</span>{:else if card.is_new}<span class="text-muted">New</span>{:else if isDue(card)}<span class="chip chip-due">Due today</span>{:else}{dueLabel(card.due, today)}{/if}</td>
                  <td class="text-sm tabular-nums">{#if card.leech}<span class="chip chip-wrong" title="Tricky card"><Flag class="size-3.5" />{card.lapses}×</span>{:else if card.lapses}{card.lapses}×{:else}<span class="text-muted">None</span>{/if}</td>
                  <td class="text-right"><button class="row-actions btn btn-ghost btn-sm btn-square" aria-label="{card.archived ? 'Bring back' : 'Archive'} card" title={card.archived ? "Bring back" : "Archive"} onclick={() => toggleArchive(card.id, !card.archived)}>{#if card.archived}<ArchiveRestore class="size-4" />{:else}<Archive class="size-4" />{/if}</button></td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if listed.length === 0}<p class="px-6 py-6 text-sm text-muted">No cards match.</p>{/if}
        </div>
      {/if}
    {:else}
      {@const note = editing.note}
      <div class="grid grid-cols-[minmax(0,1fr)] wide:grid-cols-[1fr_320px] items-start gap-10">
        <form class="space-y-8" onsubmit={save}>
          {#if leech}
            <div class="panel flex items-start gap-3 px-4 py-3" role="note"><Flag class="mt-0.5 size-4 shrink-0" />
              <p><span class="font-semibold">Again {leech.lapses} times.</span> <span class="text-muted">Ask why the answer wasn't obvious, and write that cue as the front. Saving clears this flag and keeps the schedule.</span></p></div>
          {/if}
          <div class="flex items-center justify-between gap-4"><span class="text-sm font-semibold">Card type</span>
            <div class="seg" role="radiogroup" aria-label="Card type">
              {#each [["Basic", "Basic"], ["Reversed", "Basic and reversed"], ["Cloze", "Cloze"]] as [kind, label] (kind)}
                <button type="button" role="radio" aria-checked={note.kind === kind} onclick={() => (note.kind = kind as NoteKind)}>{label}</button>
              {/each}
            </div>
          </div>

          {#if note.kind === "Cloze"}
            <div class="space-y-2">
              <div class="flex items-end justify-between"><label class="text-sm font-semibold" for="cloze-text">Text</label>
                <button type="button" class="btn btn-ghost btn-sm" onclick={makeGap}><Brackets class="size-4" />Make a gap <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">Shift</kbd><kbd class="kbd kbd-xs">C</kbd></button></div>
              <textarea id="cloze-text" class="textarea w-full font-mono text-sm" rows="4" bind:this={clozeBox} bind:value={note.text}></textarea>
              <p class="text-sm text-muted">Select a word and make a gap, or type <code>{"{{c1::answer}}"}</code>. Add a hint with <code>{"{{c1::answer::hint}}"}</code>. Gaps with the same number are hidden together.</p>
            </div>
            <div class="space-y-2">
              <label class="text-sm font-semibold" for="cloze-extra">Extra on the back <span class="font-normal text-muted">(optional)</span></label>
              <textarea id="cloze-extra" class="textarea w-full text-[15px]" rows="2" placeholder="Why it matters, or where it came from…" bind:value={note.extra}></textarea>
            </div>
          {:else}
            {#if note.kind === "Reversed"}<p class="-mt-4 text-sm text-muted">Makes two cards: front to back, and back to front. Each is scheduled on its own. Best when both directions are worth knowing, like a term and its meaning.</p>{/if}
            <div class="space-y-2">
              <label class="text-sm font-semibold" for="card-front">Front</label>
              <textarea id="card-front" class="textarea w-full text-[15px]" rows="3" placeholder="e.g. I want X to happen, what handles it?…" bind:value={note.front}></textarea>
              <p class="text-sm text-muted">Write it as the thought you'll have when you need this, not "what is the name of".</p>
            </div>
            <div class="space-y-2">
              <label class="text-sm font-semibold" for="card-back">Back</label>
              <textarea id="card-back" class="textarea w-full font-mono text-sm" rows="4" bind:value={note.back}></textarea>
              <p class="text-sm text-muted">Markdown, <code>code</code>, maths in <code>$…$</code> and images.</p>
            </div>
            {#if note.kind === "Basic"}
              <fieldset class="space-y-3">
                {#if note.cues.length}<legend class="text-sm font-semibold">More cues for the same answer <span class="font-normal text-muted">(optional)</span></legend>{/if}
                {#each note.cues as _, index (index)}
                  <div class="flex items-center gap-2"><input class="input input-sm flex-1" aria-label="Extra cue {index + 1}" bind:value={note.cues[index]} /><button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Remove extra cue {index + 1}" onclick={() => note.cues.splice(index, 1)}><X class="size-4" /></button></div>
                {/each}
                <div class="flex flex-wrap items-center gap-3"><button type="button" class="btn btn-ghost btn-sm -ml-3" onclick={() => note.cues.push("")}><Plus class="size-4" />Add another cue for the same answer</button><span class="text-sm text-muted">Each cue becomes its own card.</span></div>
              </fieldset>
            {/if}
          {/if}

          {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}
          <!-- The save bar stays in reach however long the card gets. -->
          <div class="sticky bottom-0 z-10 flex items-center gap-3 border-t border-base-300 bg-base-100 py-4">
            <button type="submit" class="btn btn-primary">{editing.id === null ? `Save ${count(note)}` : "Save changes"} <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">S</kbd></button>
            <button type="button" class="btn btn-ghost" onclick={() => (editing = null)}>Cancel</button>
            {#if editing.id !== null}<button type="button" class="btn btn-ghost ml-auto" onclick={() => (deleting = true)}><Trash2 class="size-4" />Delete…</button>{/if}
          </div>
        </form>
        <div class="sticky top-8 space-y-4">
        {#if note.kind === "Cloze"}
          {@const numbers = clozeNumbers(note.text)}
          <section class="panel p-6" aria-label="Preview">
            <div class="flex items-baseline justify-between"><p class="eyebrow">Preview</p><p class="text-sm text-muted">{numbers.length ? `Makes ${numbers.length} ${numbers.length === 1 ? "card" : "cards"}` : "No gaps yet"}</p></div>
            {#if numbers.length}
              <ol class="mt-4 space-y-4">{#each numbers as number (number)}<li class="flex gap-4"><span class="w-8 shrink-0 pt-0.5 text-xs tabular-nums text-muted">c{number}</span><div class="text-[15px]"><Rich inline text={clozeFront(note.text, number)} /></div></li>{/each}</ol>
            {/if}
          </section>
        {:else if note.front.trim() || note.back.trim()}
          <section class="panel p-6" aria-label="Preview">
            <p class="eyebrow">Preview</p>
            <div class="mt-3 text-[15px]"><Rich text={note.front} /></div>
            <hr class="my-4 border-base-300" />
            <div class="text-[15px]"><Rich text={note.back} /></div>
          </section>
        {/if}
        <aside class="panel p-6" aria-label="Card details">
          <p class="eyebrow">{editing.view && editing.view.cards.length > 1 ? "Cards" : "Card"}</p>
          <dl class="mt-4 space-y-3 text-sm">
            {#if editing.id === null}
              <div><dt><label class="text-muted" for="card-topic">Topic</label></dt><dd><select id="card-topic" class="select select-sm mt-1 w-full" bind:value={editing.note.topic}>{#each topics as row (row.id)}<option value={row.id}>{row.name}</option>{/each}</select></dd></div>
            {:else}
              <div class="flex justify-between gap-4"><dt class="text-muted">Topic</dt><dd>{data.name}</dd></div>
            {/if}
            <div class="flex justify-between gap-4"><dt class="text-muted">Made from</dt><dd>{editing.view?.source?.startsWith("quiz:") ? "Quiz mistake" : editing.view?.source?.startsWith("recall:") ? "Recall miss" : "Written here"}</dd></div>
            {#if editing.view}
              {#each editing.view.cards as card (card.id)}
                <div class="border-t border-base-300 pt-3">
                  <p class="font-semibold">{card.key === "main" ? "Front" : card.key.startsWith("cue:") ? `Cue ${Number(card.key.slice(4)) + 1}` : card.key === "forward" ? "Forward" : card.key === "reverse" ? "Reverse" : card.key}</p>
                  <div class="mt-1 flex justify-between gap-4"><dt class="text-muted">Next review</dt><dd>{card.archived ? "Archived" : card.is_new ? "New" : dueLabel(card.due, today)}</dd></div>
                  <div class="flex justify-between gap-4"><dt class="text-muted">Reviews · Again</dt><dd class="tabular-nums">{card.reps} · {card.lapses}</dd></div>
                  {#if card.remember_now !== null}<div class="flex justify-between gap-4"><dt class="text-muted">Chance you remember it now</dt><dd class="tabular-nums">{card.remember_now}%</dd></div>{/if}
                </div>
              {/each}
            {:else}
              <div class="flex justify-between gap-4"><dt class="text-muted">First review</dt><dd>With today's new cards</dd></div>
            {/if}
          </dl>
        </aside>
        </div>
      </div>
    {/if}
  {/if}
</div>

{#if deleting}
  <dialog open class="modal modal-open" aria-labelledby="delete-note-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-note-h" class="text-lg font-semibold">Delete this card?</h3>
      <p class="mt-2 text-muted">{editing?.view && editing.view.cards.length > 1 ? `All ${editing.view.cards.length} cards from this note and their review history are deleted.` : "The card and its review history are deleted."} If you might want it again, archive it instead.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = false)}>Cancel</button>
        <button class="btn btn-error" onclick={remove}>Delete</button>
      </div>
    </div>
  </dialog>
{/if}

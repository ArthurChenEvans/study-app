<script lang="ts">
  // R191: Browse all. Every card in every topic, narrowed by the search box and a filter panel
  // that folds away. A card due today keeps its answer hidden, so browsing never spoils a review.
  import { EyeOff, FolderInput, Pause, Play, Search, SlidersHorizontal, Trash2, X } from "@lucide/svelte";
  import { api, type BrowseRow, type Id, type ModuleRow, type TopicRow } from "../lib/api";
  import { filterCards, parseQuery, sortCards, type CardSort, type CardStatus } from "../lib/cardQuery";
  import { deleteWithUndo, toast } from "../lib/celebrations.svelte";
  import { dueLabel } from "../lib/dates";

  let { today, modules, onEdit }: { today: number; modules: ModuleRow[]; onEdit: (topic: Id, note: Id) => void } = $props();

  let rows = $state<BrowseRow[] | null>(null);
  let topics = $state<TopicRow[]>([]);
  let text = $state("");
  let sort = $state<CardSort>("due");
  let pickedTopics = $state<Id[]>([]);
  let status = $state<CardStatus[]>([]);
  let kinds = $state<BrowseRow["kind"][]>([]);
  let tags = $state<string[]>([]);
  let ticked = $state<Id[]>([]);
  let hidden = $state<Set<Id>>(new Set());

  // The panel shows or hides from the Filters button, and stays how you left it.
  const PANEL_KEY = "studyapp-card-filters";
  let panelOpen = $state((() => { try { return localStorage.getItem(PANEL_KEY) !== "shut"; } catch { return true; } })());
  function togglePanel() {
    panelOpen = !panelOpen;
    try { localStorage.setItem(PANEL_KEY, panelOpen ? "open" : "shut"); } catch { /* this session only */ }
  }

  async function load() {
    const [cards, allTopics] = await Promise.all([api.browseCards(), api.topics().catch(() => [])]);
    rows = cards.filter((row) => !hidden.has(row.note));
    topics = allTopics ?? [];
    ticked = ticked.filter((id) => rows!.some((row) => row.id === id));
  }

  const query = $derived(parseQuery(text));
  // A big collection shows 100 at a time, so the list stays quick. A new search starts again.
  const PAGE = 100;
  let limit = $state(PAGE);
  $effect(() => {
    void [text, sort, pickedTopics.length, status.length, kinds.length, tags.length];
    limit = PAGE;
  });
  const shown = $derived(rows ? sortCards(filterCards(rows, query, { topics: pickedTopics, status, kinds, tags }, today), sort) : []);
  const panelCount = $derived(pickedTopics.length + status.length + kinds.length + tags.length);
  const allTags = $derived([...new Set((rows ?? []).flatMap((row) => row.tags))].sort());
  const countIn = (test: (row: BrowseRow) => boolean) => (rows ?? []).filter(test).length;
  const STATUSES: [CardStatus, string][] = [["due", "Due today"], ["new", "New"], ["learning", "Learning"], ["paused", "Paused"]];
  const KINDS: BrowseRow["kind"][] = ["Basic", "Reversed", "Cloze"];
  const statusCount = (key: CardStatus) => countIn((row) => (key === "due" ? !row.archived && row.due <= today : key === "new" ? row.is_new && !row.archived : key === "learning" ? row.learning && !row.archived : row.archived));
  // The panel's topics are the ones that have cards, grouped by module.
  const cardTopics = $derived([...new Map((rows ?? []).map((row) => [row.topic, { id: row.topic, name: row.topic_name, module: row.module }])).values()].sort((x, y) => x.name.localeCompare(y.name)));
  const tree = $derived([
    ...modules.map((module) => ({ name: module.name, topics: cardTopics.filter((t) => t.module === module.id) })),
    { name: "No module", topics: cardTopics.filter((t) => t.module === null || !modules.some((m) => m.id === t.module)) },
  ].filter((group) => group.topics.length));
  const flip = <T,>(list: T[], value: T) => (list.includes(value) ? list.filter((item) => item !== value) : [...list, value]);
  const chips = $derived([
    ...pickedTopics.map((id) => ({ label: cardTopics.find((t) => t.id === id)?.name ?? "Topic", clear: () => (pickedTopics = pickedTopics.filter((t) => t !== id)) })),
    ...status.map((key) => ({ label: STATUSES.find(([k]) => k === key)![1], clear: () => (status = status.filter((s) => s !== key)) })),
    ...kinds.map((kind) => ({ label: kind, clear: () => (kinds = kinds.filter((k) => k !== kind)) })),
    ...tags.map((tag) => ({ label: `#${tag}`, clear: () => (tags = tags.filter((t) => t !== tag)) })),
  ]);
  const whenDue = (row: BrowseRow) => (row.archived ? "Paused" : row.is_new ? "New" : row.due <= today ? "Today" : dueLabel(row.due, today));
  const allTicked = $derived(shown.length > 0 && shown.slice(0, limit).every((row) => ticked.includes(row.id)));

  // ---------- Acting on ticked cards ----------
  const tickedRows = $derived((rows ?? []).filter((row) => ticked.includes(row.id)));
  const tickedPaused = $derived(tickedRows.length > 0 && tickedRows.every((row) => row.archived));
  let moving = $state(false);
  let moveTo = $state("");

  async function pause() {
    const archived = !tickedPaused;
    for (const row of tickedRows) await api.archiveCard(row.id, archived);
    toast({ text: `${archived ? "Paused" : "Unpaused"} ${tickedRows.length} ${tickedRows.length === 1 ? "card" : "cards"}`, icon: "check" });
    ticked = [];
    await load();
  }

  async function move(event: SubmitEvent) {
    event.preventDefault();
    const topic = Number(moveTo);
    if (!topic) return;
    for (const note of new Set(tickedRows.map((row) => row.note))) {
      const view = await api.noteView(note);
      await api.saveNote(note, { note: { ...view.note, topic }, suggestion: null });
    }
    toast({ text: `Moved to ${topics.find((t) => t.id === topic)?.name ?? "the topic"}`, icon: "move" });
    moving = false;
    ticked = [];
    await load();
  }

  /** A card belongs to its note, so deleting takes the note and every card made from it. */
  function remove() {
    const notes = new Set(tickedRows.map((row) => row.note));
    const count = (rows ?? []).filter((row) => notes.has(row.note)).length;
    ticked = [];
    deleteWithUndo({
      text: `Deleted ${count} ${count === 1 ? "card" : "cards"}`,
      hide: () => {
        hidden = new Set([...hidden, ...notes]);
        rows = (rows ?? []).filter((row) => !notes.has(row.note));
      },
      restore: () => {
        hidden = new Set([...hidden].filter((note) => !notes.has(note)));
        load();
      },
      commit: async () => {
        for (const note of notes) await api.deleteNote(note);
        hidden = new Set([...hidden].filter((note) => !notes.has(note)));
        await load();
      },
    });
  }

  load();
</script>

<div class="flex flex-col gap-4">
  <div class="flex flex-wrap gap-3">
    <button type="button" class="btn {panelOpen ? 'btn-outline nav-current' : 'btn-outline'}" aria-expanded={panelOpen} aria-controls="card-filters" onclick={togglePanel}>
      <SlidersHorizontal class="size-4" />Filters{#if panelCount}<span class="due-pill">{panelCount}</span>{/if}
    </button>
    <label class="input min-w-64 flex-1">
      <Search class="size-4 opacity-70" />
      <input type="search" aria-label="Search cards" aria-describedby="card-search-help" placeholder="Search cards, or try topic:DNS tag:exam is:due" bind:value={text} data-list-filter />
    </label>
  </div>
  <p id="card-search-help" class="-mt-2 text-xs text-muted">Words search fronts and answers you can see. Narrow it with <code>topic:</code> <code>module:</code> <code>tag:</code> <code>is:due</code> <code>is:new</code> <code>is:paused</code> <code>type:cloze</code>, and <code>-word</code> to leave a word out.</p>

  <div class="flex flex-col gap-5 tab:flex-row tab:items-start">
    {#if panelOpen}
      <section id="card-filters" class="panel flex w-full shrink-0 flex-col gap-5 p-4 tab:w-64" aria-labelledby="card-filters-h">
        <div class="flex items-center justify-between">
          <h2 id="card-filters-h" class="text-[16px] font-black">Filters</h2>
          {#if panelCount}<button class="link text-sm font-bold" onclick={() => { pickedTopics = []; status = []; kinds = []; tags = []; }}>Clear all</button>{/if}
        </div>
        <fieldset class="flex max-h-80 flex-col gap-1.5 overflow-y-auto pr-1">
          <legend class="eyebrow mb-1.5">Topics</legend>
          {#each tree as group (group.name)}
            <span class="mt-1 text-[13px] font-black text-muted">{group.name}</span>
            {#each group.topics as topic (topic.id)}
              <label class="flex items-center gap-2 pl-2 text-sm"><input type="checkbox" class="checkbox size-6" checked={pickedTopics.includes(topic.id)} onchange={() => (pickedTopics = flip(pickedTopics, topic.id))} /><span class="flex-1">{topic.name}</span><span class="text-xs text-muted tabular-nums">{countIn((row) => row.topic === topic.id)}</span></label>
            {/each}
          {/each}
        </fieldset>
        <fieldset class="flex flex-col gap-1.5">
          <legend class="eyebrow mb-1.5">Status</legend>
          {#each STATUSES as [key, label] (key)}
            <label class="flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox size-6" checked={status.includes(key)} onchange={() => (status = flip(status, key))} /><span class="flex-1">{label}</span><span class="text-xs text-muted tabular-nums">{statusCount(key)}</span></label>
          {/each}
        </fieldset>
        <fieldset class="flex flex-col gap-1.5">
          <legend class="eyebrow mb-1.5">Card type</legend>
          {#each KINDS as kind (kind)}
            <label class="flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox size-6" checked={kinds.includes(kind)} onchange={() => (kinds = flip(kinds, kind))} /><span class="flex-1">{kind}</span><span class="text-xs text-muted tabular-nums">{countIn((row) => row.kind === kind)}</span></label>
          {/each}
        </fieldset>
        {#if allTags.length}
          <fieldset>
            <legend class="eyebrow mb-1.5">Tags</legend>
            <div class="flex flex-wrap gap-1.5">
              {#each allTags as tag (tag)}<button type="button" class="kind-chip" aria-pressed={tags.includes(tag)} onclick={() => (tags = flip(tags, tag))}>{tag}</button>{/each}
            </div>
          </fieldset>
        {/if}
      </section>
    {/if}

    <section class="flex min-w-0 flex-1 flex-col gap-3" aria-label="Card list">
      <div class="flex flex-wrap items-center gap-2">
        {#each chips as chip (chip.label)}
          <button type="button" class="chip gap-1" aria-label="Remove filter: {chip.label}" onclick={chip.clear}>{chip.label}<X class="size-3.5" aria-hidden="true" /></button>
        {/each}
        <span class="flex-1"></span>
        {#if rows}<span class="text-sm text-muted">{shown.length} of {rows.length} {rows.length === 1 ? "card" : "cards"}</span>{/if}
        <label class="flex items-center gap-2 text-sm text-muted">Sort
          <select class="select select-sm w-40" bind:value={sort}><option value="due">Due soonest</option><option value="newest">Newest first</option><option value="hardest">Hardest first</option><option value="topic">Topic</option></select></label>
      </div>

      {#if ticked.length}
        <div class="selbar" role="region" aria-label="Ticked cards">
          <span class="text-sm font-extrabold">{ticked.length} ticked</span>
          <button class="btn btn-ghost btn-sm" onclick={() => { moveTo = ""; moving = true; }}><FolderInput class="size-4" />Move to topic</button>
          <button class="btn btn-ghost btn-sm" onclick={pause}>{#if tickedPaused}<Play class="size-4" />Unpause{:else}<Pause class="size-4" />Pause{/if}</button>
          <button class="btn btn-ghost btn-sm text-error" onclick={remove}><Trash2 class="size-4" />Delete</button>
          <button class="btn btn-quiet btn-sm ml-auto" onclick={() => (ticked = [])}>Untick all</button>
        </div>
      {/if}

      {#if rows && shown.length}
        <div class="panel pane">
          <table class="rows table-fixed" aria-label="All cards">
            <thead><tr>
              <th class="w-12"><span><input type="checkbox" class="checkbox size-6" aria-label="Tick all {Math.min(limit, shown.length)} shown" checked={allTicked} onchange={() => (ticked = allTicked ? [] : shown.slice(0, limit).map((row) => row.id))} /></span></th>
              <th><span>Front</span></th><th class="max-tab:hidden"><span>Answer</span></th><th class="w-44"><span>Topic</span></th><th class="w-28"><span>Due</span></th>
            </tr></thead>
            <tbody>
              {#each shown.slice(0, limit) as row (row.id)}
                <tr aria-label={row.front}>
                  <td><input type="checkbox" class="checkbox size-6" aria-label="Tick {row.front}" checked={ticked.includes(row.id)} onchange={() => (ticked = flip(ticked, row.id))} /></td>
                  <td class="max-w-0"><button type="button" class="block min-h-6 w-full truncate text-left font-extrabold" title={row.front} onclick={() => onEdit(row.topic, row.note)}>{row.front}</button><span class="block text-xs text-muted">{row.kind}</span></td>
                  <td class="max-w-0 max-tab:hidden">{#if row.back === null}<span class="inline-flex items-center gap-1.5 text-sm italic text-muted"><EyeOff class="size-3.5" aria-hidden="true" />Hidden until you review it</span>{:else}<span class="block truncate text-sm text-muted" title={row.back}>{row.back}</span>{/if}</td>
                  <td class="max-w-0"><span class="block truncate text-sm">{row.topic_name}</span><span class="block truncate text-xs text-muted">{row.module_name ?? "No module"}</span></td>
                  <td class="text-sm {whenDue(row) === 'Today' ? 'font-extrabold text-[var(--accent-ink)]' : 'text-muted'}">{whenDue(row)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if shown.length > limit}
          {@const left = shown.length - limit}
          <button class="btn btn-outline self-center" onclick={() => (limit += PAGE)}>Show {Math.min(PAGE, left)} more ({left} left)</button>
        {/if}
      {:else if rows}
        <div class="panel p-8">
          <p class="font-extrabold">{rows.length ? "No cards match" : "No cards yet"}</p>
          {#if rows.length && (text || panelCount)}<button class="btn btn-outline btn-sm mt-3" onclick={() => { text = ""; pickedTopics = []; status = []; kinds = []; tags = []; }}>Clear the search and filters</button>{/if}
        </div>
      {/if}
    </section>
  </div>
</div>

{#if moving}
  <dialog open class="modal modal-open" aria-labelledby="move-cards-h">
    <form class="modal-box max-w-md" onsubmit={move}>
      <h3 id="move-cards-h" class="text-lg font-semibold">Move {ticked.length} {ticked.length === 1 ? "card" : "cards"}</h3>
      <p class="mt-2 text-sm text-muted">A card moves with the rest of its note, so a reversed or cloze note moves all its cards.</p>
      <label class="mt-5 block"><span class="text-sm font-semibold">To topic</span>
        <select class="select mt-1 w-full" bind:value={moveTo}>
          <option value="" disabled>Pick a topic</option>
          {#each topics as topic (topic.id)}<option value={String(topic.id)}>{topic.name}</option>{/each}
        </select></label>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (moving = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary" disabled={!moveTo}>Move</button>
      </div>
    </form>
  </dialog>
{/if}

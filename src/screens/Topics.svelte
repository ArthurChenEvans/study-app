<script lang="ts">
  // topics.feature (R1, R2, R3, R30, R43, R51)
  // calendar.feature (R96, R97): each topic's quiz step and next quiz, and the schedule.
  // lists.feature (R138 to R144): a module rail, a filter, sortable columns in their own pane,
  // an item card for the picked topic, and ticks for acting on several at once.
  import { X, CalendarCog, FolderInput, FolderPlus, GraduationCap, Library, Pencil, Play, Plus, Search, SquareStack, Tag, Trash2 } from "@lucide/svelte";
  import { tick as settle } from "svelte";
  import RowMenu from "../components/RowMenu.svelte";
  import Highlight from "../components/Highlight.svelte";
  import Rail, { type RailItem } from "../components/list/Rail.svelte";
  import FilterBox from "../components/list/FilterBox.svelte";
  import SortHeader, { type Sort } from "../components/list/SortHeader.svelte";
  import ColumnsMenu from "../components/list/ColumnsMenu.svelte";
  import { api, errorMessage, type Id, type ModuleRow, type Schedule, type TopicQuizView, type TopicRow } from "../lib/api";
  import { deleteWithUndo, toast } from "../lib/celebrations.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ScheduleDialog from "../components/ScheduleDialog.svelte";
  import { dueLabel } from "../lib/dates";
  import { matchesFilter, saveColumns, savedColumns, sortBy } from "../lib/lists";

  let { onEdit = () => {}, onQuiz = () => {}, onQuizTopics = () => {}, onCards = () => {}, onSearch = () => {} }: {
    onEdit?: (topic: Id) => void; onQuiz?: (topic: Id) => void; onQuizTopics?: (topics: Id[]) => void; onCards?: (topic: Id) => void; onSearch?: (text: string) => void;
  } = $props();

  let quizzes = $state<TopicQuizView[]>([]);
  let today = $state(0);
  let editingSchedule = $state<Schedule | null>(null);
  let daily = $state<{ value: number; coming: number } | undefined>(undefined);
  const quizOf = (topic: Id) => quizzes.find((quiz) => quiz.topic === topic);

  let topics = $state<TopicRow[]>([]);
  const dueTopics = $derived(topics.filter((topic) => quizOf(topic.id)?.due_today && topic.question_count > 0).map((topic) => topic.id));
  let modules = $state<ModuleRow[]>([]);
  let loaded = $state(false);

  // ---------- The list (R138 to R144) ----------
  const COLUMNS_KEY = "studyapp-topic-columns";
  let rail = $state("all");
  let filter = $state("");
  let status = $state<"all" | "due" | "tight" | "new">("all");
  let sort = $state<Sort>({ key: "name", up: true });
  let columns = $state(savedColumns(COLUMNS_KEY, { tags: false, questions: true, short: false, step: true, next: true }));
  $effect(() => saveColumns(COLUMNS_KEY, columns));
  let selected = $state<Id | null>(null);
  let ticked = $state<Id[]>([]);
  let pane = $state<HTMLDivElement>();

  const moduleName = (id: Id | null) => (id === null ? null : (modules.find((m) => m.id === id)?.name ?? null));
  const inRail = (topic: TopicRow, key: string) => key === "all" || (key === "none" ? topic.module === null : topic.module === Number(key.slice(1)));
  const inStatus = (topic: TopicRow) => {
    const quiz = quizOf(topic.id);
    return status === "all" || (status === "due" ? !!quiz?.due_today : status === "tight" ? !!quiz?.tightened_for : !quiz);
  };
  const filtered = $derived(topics.filter((topic) => inStatus(topic) && matchesFilter(filter, topic.name, moduleName(topic.module), topic.tags.join(" "))));
  const railItems = $derived<RailItem[]>([
    { key: "all", label: "All modules", count: filtered.length },
    ...modules.map((m) => ({ key: `m${m.id}`, label: m.name, count: filtered.filter((t) => t.module === m.id).length })),
    ...(topics.some((t) => t.module === null) ? [{ key: "none", label: "No module", count: filtered.filter((t) => t.module === null).length }] : []),
  ]);
  const railModule = $derived(rail.startsWith("m") ? modules.find((m) => `m${m.id}` === rail) ?? null : null);
  const shown = $derived(sortBy(filtered.filter((topic) => inRail(topic, rail)), (topic) => {
    const quiz = quizOf(topic.id);
    switch (sort.key) {
      case "questions": return topic.question_count;
      case "short": return topic.short_count;
      case "step": return quiz ? quiz.position : null;
      case "next": return quiz ? quiz.due : null;
      default: return topic.name;
    }
  }, sort.up, (topic) => topic.name));
  const picked = $derived(topics.find((t) => t.id === selected) ?? null);
  const allTicked = $derived(shown.length > 0 && shown.every((t) => ticked.includes(t.id)));
  const counts = $derived({
    due: topics.filter((t) => quizOf(t.id)?.due_today).length,
    tight: topics.filter((t) => quizOf(t.id)?.tightened_for).length,
    new: topics.filter((t) => !quizOf(t.id)).length,
  });

  function toggle(id: Id) {
    ticked = ticked.includes(id) ? ticked.filter((t) => t !== id) : [...ticked, id];
  }

  async function focusRow(id: Id) {
    selected = id;
    await settle();
    pane?.querySelector<HTMLButtonElement>(`[data-topic="${id}"]`)?.focus();
  }

  // ↑ ↓ move, Enter edits, Space ticks (R143).
  function rowKeys(event: KeyboardEvent, topic: TopicRow) {
    const at = shown.findIndex((t) => t.id === topic.id);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const next = shown[at + (event.key === "ArrowDown" ? 1 : -1)];
      if (next) focusRow(next.id);
    } else if (event.key === "Enter") {
      event.preventDefault();
      onEdit(topic.id);
    } else if (event.key === " ") {
      event.preventDefault();
      selected = topic.id;
      toggle(topic.id);
    }
  }

  // ---------- Acting on topics ----------
  let creating = $state(false);
  let newName = $state("");
  let newModule = $state<string>("");
  let createError = $state("");

  let deleting = $state<TopicRow | null>(null);
  let deletingModule = $state<{ id: Id; name: string; count: number } | null>(null);
  let tagging = $state<TopicRow | null>(null);
  let tagText = $state("");
  /** Moving one topic, or the ticked ones. */
  let moving = $state<Id[] | null>(null);
  let moveTo = $state("");
  let bulkTagging = $state(false);
  let bulkTag = $state("");

  function openTags(topic: TopicRow) {
    tagging = topic;
    tagText = topic.tags.join(", ");
  }

  async function saveTags(event: SubmitEvent) {
    event.preventDefault();
    if (!tagging) return;
    const tags = tagText.split(",").map((tag) => tag.trim()).filter(Boolean);
    await api.setTopicTags(tagging.id, tags);
    tagging = null;
    await load();
  }

  function openMove(ids: Id[]) {
    const first = topics.find((t) => t.id === ids[0]);
    moveTo = first?.module === null || first === undefined ? "" : String(first.module);
    moving = ids;
  }

  async function move(event: SubmitEvent) {
    event.preventDefault();
    if (!moving) return;
    const target = moveTo ? Number(moveTo) : null;
    const before = moving.map((id) => ({ id, module: topics.find((t) => t.id === id)?.module ?? null })).filter((row) => row.module !== target);
    for (const id of moving) await api.moveTopic(id, target);
    const where = target === null ? "No module" : modules.find((m) => m.id === target)?.name ?? "the module";
    const one = moving.length === 1 ? topics.find((t) => t.id === moving![0])?.name : null;
    moving = null;
    await load();
    toast({
      text: one ? `${one} moved to ${where}` : `${before.length} topics moved to ${where}`,
      icon: "move",
      ms: 6000,
      drain: true,
      action: before.length ? { label: "Undo", run: async () => {
        for (const row of before) await api.moveTopic(row.id, row.module);
        await load();
        toast({ text: "Moved back", icon: "undo", ms: 2500 }, true);
      } } : undefined,
    });
  }

  async function addTag(event: SubmitEvent) {
    event.preventDefault();
    const tag = bulkTag.trim();
    if (!tag) return;
    let tagged = 0;
    for (const topic of topics.filter((t) => ticked.includes(t.id))) {
      if (!topic.tags.some((existing) => existing.toLowerCase() === tag.toLowerCase())) {
        await api.setTopicTags(topic.id, [...topic.tags, tag]);
        tagged += 1;
      }
    }
    toast({ text: `Tagged ${tagged} ${tagged === 1 ? "topic" : "topics"} "${tag}"`, icon: "check" });
    bulkTagging = false;
    bulkTag = "";
    await load();
  }

  // toasts.feature (R157): deletes go at once with a few seconds to undo, and only happen for
  // real once that's passed. Until then the item is just hidden here.
  let hiddenTopics = $state<Set<Id>>(new Set());
  let hiddenModules = $state<Set<Id>>(new Set());
  const without = (set: Set<Id>, id: Id) => new Set([...set].filter((value) => value !== id));

  function confirmDeleteModule() {
    if (!deletingModule) return;
    const { id, name, count } = deletingModule;
    deletingModule = null;
    deleteWithUndo({
      text: "Module deleted",
      sub: count ? `${name}. Its ${count === 1 ? "topic moves" : `${count} topics move`} to No module.` : name,
      hide: () => {
        hiddenModules = new Set([...hiddenModules, id]);
        rail = "all";
        modules = modules.filter((m) => m.id !== id);
        topics = topics.map((t) => (t.module === id ? { ...t, module: null } : t));
      },
      restore: () => {
        hiddenModules = without(hiddenModules, id);
        load();
      },
      commit: async () => {
        await api.deleteModule(id);
        hiddenModules = without(hiddenModules, id);
        await load();
      },
    });
  }

  let creatingModule = $state(false);
  let moduleNameInput = $state("");
  let moduleError = $state("");

  async function createModule(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.createModule(moduleNameInput.trim());
      toast({ text: "Module added", sub: moduleNameInput.trim(), icon: "plus" });
      creatingModule = false;
      moduleNameInput = "";
      await load();
    } catch (error) {
      moduleError = errorMessage(error);
    }
  }

  async function load() {
    const [allTopics, allModules, loadedQuizzes, loadedToday] = await Promise.all([api.topics(), api.modules(), api.topicQuizzes().catch(() => []), api.today().catch(() => 0)]);
    modules = allModules.filter((m) => !hiddenModules.has(m.id));
    topics = allTopics.filter((t) => !hiddenTopics.has(t.id)).map((t) => (t.module !== null && hiddenModules.has(t.module) ? { ...t, module: null } : t));
    quizzes = loadedQuizzes;
    today = loadedToday;
    ticked = ticked.filter((id) => topics.some((t) => t.id === id));
    loaded = true;
  }

  function openCreate() {
    newName = "";
    newModule = railModule ? String(railModule.id) : "";
    createError = "";
    creating = true;
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      selected = await api.createTopic(newName.trim(), newModule ? Number(newModule) : null);
      toast({ text: "Topic added", sub: newName.trim(), icon: "plus" });
      creating = false;
      await load();
    } catch (error) {
      createError = errorMessage(error);
    }
  }

  /** A topic with questions asks first (it's a big delete). An empty one goes straight away. */
  function requestDelete(topic: TopicRow) {
    if (topic.question_count + topic.short_count > 0) deleting = topic;
    else removeTopic(topic);
  }

  function confirmDelete() {
    if (!deleting) return;
    const topic = deleting;
    deleting = null;
    removeTopic(topic);
  }

  function removeTopic(topic: TopicRow) {
    const questions = topic.question_count + topic.short_count;
    deleteWithUndo({
      text: "Topic deleted",
      sub: questions ? `${topic.name} and its ${questions} ${questions === 1 ? "question" : "questions"}` : topic.name,
      hide: () => {
        hiddenTopics = new Set([...hiddenTopics, topic.id]);
        topics = topics.filter((t) => t.id !== topic.id);
        ticked = ticked.filter((id) => id !== topic.id);
        if (selected === topic.id) selected = null;
      },
      restore: () => {
        hiddenTopics = without(hiddenTopics, topic.id);
        load();
      },
      commit: async () => {
        await api.deleteTopic(topic.id);
        hiddenTopics = without(hiddenTopics, topic.id);
        await load();
      },
    });
  }

  load();
</script>

{#snippet pips(quiz: TopicQuizView)}<span class="pips" aria-hidden="true">{#each Array.from({ length: quiz.steps }) as _, step (step)}<i class:on={step <= quiz.position}></i>{/each}</span>{/snippet}
{#snippet next(quiz: TopicQuizView | undefined, topic: TopicRow)}
  {#if quiz}
    {#if quiz.due_today}<span class="font-extrabold text-[var(--accent-ink)]">Due today</span>{:else if quiz.waiting}<span class="font-bold">Waiting its turn</span>{:else}{dueLabel(quiz.due, today)}{/if}
    {#if quiz.tightened_for}<span class="ml-1 inline-flex align-middle" title="Tightened for {quiz.tightened_for}"><GraduationCap class="size-3.5" aria-hidden="true" /><span class="sr-only">Tightened for {quiz.tightened_for}</span></span>{/if}
  {:else}<span class="text-muted">{topic.question_count ? "Quiz it to start" : "No questions yet"}</span>{/if}
{/snippet}

<div class="mx-auto max-w-[96rem] px-12 pb-24">
  <PageHeader title="Topics" subtitle={loaded && topics.length ? `${topics.length} topics in ${modules.length} modules${dueTopics.length ? ` · ${dueTopics.length} ${dueTopics.length === 1 ? "quiz" : "quizzes"} due today` : ""}` : ""}>
    {#if topics.length}<button class="btn btn-ghost" onclick={async () => { const queue = await api.quizQueue(); daily = { value: queue.daily, coming: queue.next_four_weeks }; editingSchedule = await api.quizSchedule(); }}><CalendarCog class="size-4" />Quiz schedule</button>{/if}
    <button class="btn btn-ghost" onclick={() => { moduleError = ""; creatingModule = true; }}><FolderPlus class="size-4" />New module</button>
    <button class="btn {dueTopics.length || (loaded && topics.length === 0) ? 'btn-ghost' : 'btn-primary'}" onclick={openCreate}><Plus class="size-4" />New topic</button>
    {#if dueTopics.length}<button class="btn btn-primary" onclick={() => onQuizTopics(dueTopics)}><Play class="size-4" />Quiz the {dueTopics.length === 1 ? "due topic" : `${dueTopics.length} due topics`}</button>{/if}
  </PageHeader>

  {#if loaded && topics.length === 0}
    <div class="panel max-w-lg p-10">
      <span class="well" aria-hidden="true"><Library class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No topics yet</h2>
      <p class="mt-2 text-muted">A topic holds your questions on one subject, like "CPU". You can group topics into modules later.</p>
      <button class="btn btn-primary mt-6" onclick={openCreate}><Plus class="size-4" />Create your first topic</button>
    </div>
  {:else if loaded}
    <div class="flex gap-5">
      <Rail items={railItems} bind:value={rail} onPick={() => (selected = null)} />
      <section class="min-w-0 flex-1" aria-label="Topic list">
        <div class="mb-3 flex flex-wrap items-center gap-3">
          <FilterBox bind:value={filter} shown={shown.length} total={topics.length} placeholder="Filter by name, module or tag…" />
          <div class="seg" role="radiogroup" aria-label="Show">
            {#each [["all", "All", null], ["due", "Due", counts.due], ["tight", "Tightened", counts.tight], ["new", "Not started", counts.new]] as const as [key, label, count] (key)}
              <button type="button" role="radio" aria-checked={status === key} onclick={() => { status = key; selected = null; }}>{label}{#if count !== null}{" "}<span class="tabular-nums">{count}</span>{/if}</button>
            {/each}
          </div>
          <span class="flex-1"></span>
          {#if railModule}
            {@const id = railModule.id}
            {@const name = railModule.name}
            <RowMenu small label="{name} module actions" items={[{ label: "Delete module…", danger: true, onSelect: () => (deletingModule = { id, name, count: topics.filter((t) => t.module === id).length }) }]} />
          {/if}
          <ColumnsMenu options={[{ key: "tags", label: "Tags" }, { key: "questions", label: "Questions" }, { key: "short", label: "Short answers" }, { key: "step", label: "Quiz step" }, { key: "next", label: "Next quiz" }]} bind:value={columns} />
        </div>
        {#if shown.length}
          <div class="panel pane" bind:this={pane}>
            <table class="rows" aria-label="Topics">
              <thead><tr>
                <th class="w-12"><span><input type="checkbox" class="checkbox" aria-label="Tick all {shown.length} shown" checked={allTicked} onchange={() => (ticked = allTicked ? ticked.filter((id) => !shown.some((t) => t.id === id)) : [...new Set([...ticked, ...shown.map((t) => t.id)])])} /></span></th>
                <SortHeader key="name" label="Topic" bind:sort />
                {#if columns.tags}<th><span>Tags</span></th>{/if}
                {#if columns.questions}<SortHeader key="questions" label="Questions" numeric bind:sort />{/if}
                {#if columns.short}<SortHeader key="short" label="Short" numeric bind:sort />{/if}
                {#if columns.step}<SortHeader key="step" label="Quiz step" bind:sort />{/if}
                {#if columns.next}<SortHeader key="next" label="Next quiz" bind:sort />{/if}
              </tr></thead>
              <tbody>
                {#each shown as topic (topic.id)}
                  {@const quiz = quizOf(topic.id)}
                  <tr aria-selected={selected === topic.id} onclick={() => (selected = topic.id)} ondblclick={() => onEdit(topic.id)}>
                    <td><input type="checkbox" class="checkbox" aria-label="Tick {topic.name}" checked={ticked.includes(topic.id)} onclick={(event) => event.stopPropagation()} onchange={() => toggle(topic.id)} /></td>
                    <td class="w-full min-w-40 max-w-0">
                      <button type="button" class="block min-h-6 w-full truncate text-left text-[15px] font-extrabold" data-topic={topic.id} title={topic.name} onkeydown={(event) => rowKeys(event, topic)}><Highlight text={topic.name} term={filter} /></button>
                      {#if rail === "all"}<span class="block truncate text-xs text-muted">{#if topic.module !== null}<Highlight text={moduleName(topic.module) ?? ""} term={filter} />{:else}No module{/if}</span>{/if}
                    </td>
                    {#if columns.tags}<td data-label="Tags"><span class="flex flex-wrap gap-1">{#each topic.tags as tag (tag)}<span class="chip"><Highlight text={tag} term={filter} /></span>{/each}</span></td>{/if}
                    {#if columns.questions}<td class="num" data-label="Questions">{topic.question_count}</td>{/if}
                    {#if columns.short}<td class="num" data-label="Short answers">{topic.short_count}</td>{/if}
                    {#if columns.step}<td class="whitespace-nowrap" data-label="Quiz step">{#if quiz}<span class="text-[13px]">{quiz.step}</span>{:else}<span class="text-muted">Not started</span>{/if}</td>{/if}
                    {#if columns.next}<td class="whitespace-nowrap" data-label="Next quiz">{@render next(quiz, topic)}</td>{/if}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="mt-2 text-xs text-muted max-tab:hidden"><kbd class="kbd kbd-xs">↑</kbd> <kbd class="kbd kbd-xs">↓</kbd> to move, <kbd class="kbd kbd-xs">Enter</kbd> to edit, <kbd class="kbd kbd-xs">Space</kbd> to tick, <kbd class="kbd kbd-xs">/</kbd> to filter</p>
        {:else}
          <div class="panel p-8">
            <p class="font-extrabold">{filter.trim() ? `Nothing matches "${filter.trim()}"` : "No topics here"}{railModule ? ` in ${railModule.name}` : ""}</p>
            <p class="mt-1 text-sm text-muted">Filters look at names, modules and tags. To look inside questions, use Search.</p>
            <div class="mt-4 flex flex-wrap gap-2">
              {#if filter}<button class="btn btn-outline btn-sm" onclick={() => (filter = "")}>Clear the filter</button>{/if}
              {#if rail !== "all" || status !== "all"}<button class="btn btn-ghost btn-sm" onclick={() => { rail = "all"; status = "all"; }}>Look in all topics</button>{/if}
              {#if filter.trim()}<button class="btn btn-ghost btn-sm" onclick={() => onSearch(filter.trim())}><Search class="size-4" />Search questions</button>{/if}
            </div>
          </div>
        {/if}
        {#if ticked.length}
          <div class="selbar" role="region" aria-label="Ticked topics">
            <span class="text-sm font-extrabold">{ticked.length} ticked</span>
            <button class="btn btn-outline btn-sm" onclick={() => onQuizTopics(ticked)}><Play class="size-4" />Quiz them</button>
            <button class="btn btn-ghost btn-sm" onclick={() => openMove(ticked)}><FolderInput class="size-4" />Move to module</button>
            <button class="btn btn-ghost btn-sm" onclick={() => { bulkTag = ""; bulkTagging = true; }}><Tag class="size-4" />Tag</button>
            <button class="btn btn-quiet btn-sm ml-auto" onclick={() => (ticked = [])}>Untick all</button>
          </div>
        {/if}
      </section>
      <aside class="itemcard" aria-label="Picked topic" data-empty={!picked}>
        {#if picked}
          <button type="button" class="itemcard-close btn btn-quiet btn-sm btn-square" aria-label="Close" onclick={() => (selected = null)}><X class="size-4" /></button>
          {@const quiz = quizOf(picked.id)}
          <div class="panel p-5">
            <p class="text-xs font-bold text-muted">{moduleName(picked.module) ?? "No module"}</p>
            <h2 class="mt-1 text-lg font-black leading-snug">{picked.name}</h2>
            {#if picked.tags.length}<div class="mt-2 flex flex-wrap gap-1">{#each picked.tags as tag (tag)}<span class="chip">{tag}</span>{/each}</div>{/if}
            <dl class="mt-4 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-2.5 text-sm">
              <dt class="text-muted">Questions</dt><dd class="text-right font-extrabold tabular-nums">{picked.question_count}{picked.short_count ? `, ${picked.short_count} short answer` : ""}</dd>
              <dt class="text-muted">Quiz step</dt><dd class="flex items-center justify-end gap-2 font-extrabold">{#if quiz}{@render pips(quiz)}{quiz.step}{:else}Not started{/if}</dd>
              <dt class="text-muted">Next quiz</dt><dd class="text-right font-extrabold">{#if quiz}{quiz.due_today ? "Due today" : quiz.waiting ? "Waiting its turn" : dueLabel(quiz.due, today)}{:else}{picked.question_count ? "Quiz it to start" : "No questions yet"}{/if}</dd>
            </dl>
            {#if quiz?.tightened_for}<p class="note-box mt-3 flex items-start gap-2 p-2 text-xs"><GraduationCap class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />Tightened for {quiz.tightened_for}{quiz.normal_due !== quiz.due ? `, it would be ${dueLabel(quiz.normal_due, today)}` : ""}</p>{/if}
            <div class="mt-5 grid grid-cols-2 gap-2">
              <button class="btn btn-outline btn-sm {quiz?.due_today ? 'text-primary-ink' : ''}" aria-label="Quiz {picked.name}" disabled={!picked.question_count} onclick={() => onQuiz(picked!.id)}><Play class="size-4" />Quiz</button>
              <button class="btn btn-outline btn-sm" aria-label="Edit {picked.name}" onclick={() => onEdit(picked!.id)}><Pencil class="size-4" />Edit</button>
              <button class="btn btn-ghost btn-sm col-span-2" onclick={() => onCards(picked!.id)}><SquareStack class="size-4" />Cards</button>
            </div>
            <div class="action-row mt-3 border-t-2 border-base-300 pt-3">
              <button class="btn btn-quiet btn-xs" onclick={() => openMove([picked!.id])}><FolderInput class="size-3.5" />Move</button>
              <button class="btn btn-quiet btn-xs" onclick={() => openTags(picked!)}><Tag class="size-3.5" />Tags</button>
              <button class="btn btn-quiet btn-xs text-error" onclick={() => requestDelete(picked!)}><Trash2 class="size-3.5" />Delete</button>
            </div>
          </div>
        {:else}
          <div class="panel p-5 text-sm text-muted"><p class="font-extrabold text-base-content">Pick a topic to see it here</p><p class="mt-2">Click a row, or use the arrow keys. Double-click or press Enter to edit it.</p></div>
        {/if}
      </aside>
    </div>
  {/if}
</div>

{#if moving}
  <dialog open class="modal modal-open" aria-labelledby="move-title">
    <form class="modal-box max-w-md" onsubmit={move}>
      <h3 id="move-title" class="text-lg font-semibold">{moving.length === 1 ? `Move "${topics.find((t) => t.id === moving![0])?.name}"` : `Move ${moving.length} topics`}</h3>
      <label class="mt-6 block"><span class="text-sm font-semibold">To module</span>
        <select class="select mt-1 w-full" bind:value={moveTo}>
          <option value="">No module</option>
          {#each modules as module (module.id)}<option value={String(module.id)}>{module.name}</option>{/each}
        </select></label>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (moving = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Move</button>
      </div>
    </form>
  </dialog>
{/if}

{#if bulkTagging}
  <dialog open class="modal modal-open" aria-labelledby="bulk-tag-title">
    <form class="modal-box max-w-md" onsubmit={addTag}>
      <h3 id="bulk-tag-title" class="text-lg font-semibold">Tag {ticked.length} {ticked.length === 1 ? "topic" : "topics"}</h3>
      <label class="mt-6 block"><span class="text-sm font-semibold">Tag to add</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={bulkTag} autofocus placeholder="e.g. exam…" /></label>
      <p class="mt-1 text-xs text-muted">Their other tags stay as they are.</p>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (bulkTagging = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary" disabled={!bulkTag.trim()}>Add tag</button>
      </div>
    </form>
  </dialog>
{/if}

{#if creating}
  <dialog open class="modal modal-open" aria-labelledby="new-topic-title">
    <form class="modal-box max-w-md" onsubmit={create}>
      <h3 id="new-topic-title" class="text-lg font-semibold">New topic</h3>
      <label class="mt-6 block">
        <span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={newName} autofocus placeholder="e.g. CPU…" />
      </label>
      <label class="mt-4 block">
        <span class="text-sm font-semibold">Module</span>
        <select class="select mt-1 w-full" bind:value={newModule}>
          <option value="">No module</option>
          {#each modules as module (module.id)}<option value={String(module.id)}>{module.name}</option>{/each}
        </select>
      </label>
      {#if createError}<p role="alert" class="mt-4 text-sm text-error">{createError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create topic</button>
      </div>
    </form>
  </dialog>
{/if}

{#if deleting}
  <dialog open class="modal modal-open" aria-labelledby="delete-topic-title">
    <div class="modal-box max-w-md">
      <h3 id="delete-topic-title" class="text-lg font-semibold">Delete "{deleting.name}"?</h3>
      <p class="mt-2 text-muted">Its {deleting.question_count} questions will be removed from your topics. Past quiz attempts keep the questions as they were, so your history isn't affected.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = null)}>Cancel</button>
        <button class="btn btn-error" onclick={confirmDelete}>Delete topic</button>
      </div>
    </div>
  </dialog>
{/if}

{#if creatingModule}
  <dialog open class="modal modal-open" aria-labelledby="new-module-title">
    <form class="modal-box max-w-md" onsubmit={createModule}>
      <h3 id="new-module-title" class="text-lg font-semibold">New module</h3>
      <p class="mt-2 text-sm text-muted">A module groups related topics, like a course unit.</p>
      <label class="mt-6 block">
        <span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={moduleNameInput} autofocus placeholder="e.g. Computer Architecture…" />
      </label>
      {#if moduleError}<p role="alert" class="mt-4 text-sm text-error">{moduleError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creatingModule = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create module</button>
      </div>
    </form>
  </dialog>
{/if}

{#if deletingModule}
  <dialog open class="modal modal-open" aria-labelledby="delete-module-title">
    <div class="modal-box max-w-md">
      <h3 id="delete-module-title" class="text-lg font-semibold">Delete module "{deletingModule.name}"?</h3>
      <p class="mt-2 text-muted">{deletingModule.count === 1 ? "Its 1 topic is" : `Its ${deletingModule.count} topics are`} moved to "No module". No topics or questions are deleted.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deletingModule = null)}>Cancel</button>
        <button class="btn btn-error" onclick={confirmDeleteModule}>Delete module</button>
      </div>
    </div>
  </dialog>
{/if}

{#if tagging}
  <dialog open class="modal modal-open" aria-labelledby="tags-title">
    <form class="modal-box max-w-md" onsubmit={saveTags}>
      <h3 id="tags-title" class="text-lg font-semibold">Tags for "{tagging.name}"</h3>
      <label class="mt-6 block">
        <span class="text-sm font-semibold">Tags</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" aria-label="Tags" bind:value={tagText} autofocus placeholder="e.g. hardware, exam…" />
        <span class="mt-1 block text-xs text-muted">Separate tags with commas.</span>
      </label>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (tagging = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Save tags</button>
      </div>
    </form>
  </dialog>
{/if}

{#if editingSchedule}
  <ScheduleDialog title="Quiz schedule" intro="Every topic climbs these steps, starting the first time you quiz it. Each topic in a quiz is scored on its own questions, and the next date counts from the quiz day. Retries don't move it." passWhat="the topic's questions right" applies="Changes apply from each topic's next quiz." schedule={editingSchedule} daily={daily}
    onSave={async (next, perDay) => { await api.setQuizSchedule(next); if (perDay !== undefined) await api.setQuizzesADay(perDay); editingSchedule = null; await load(); }} onReset={() => api.resetQuizSchedule()} onClose={() => (editingSchedule = null)} />
{/if}

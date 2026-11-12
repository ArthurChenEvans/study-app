<script lang="ts">
  // flashcards.feature (R72, R73): suggestions from recall misses and quiz mistakes wait for you.
  import { ArrowLeft, Plus, X } from "@lucide/svelte";
  import { api, errorMessage, type Suggestion, type TopicRow } from "../lib/api";
  import Rich from "../components/Rich.svelte";
  import { ChevronRight } from "@lucide/svelte";
  import FilterBox from "../components/list/FilterBox.svelte";
  import { matchesFilter } from "../lib/lists";

  let { onBack, onChanged = () => {} }: { onBack: () => void; onChanged?: () => void } = $props();

  let items = $state<Suggestion[] | null>(null);
  let topics = $state<TopicRow[]>([]);
  let filter = $state<"all" | Suggestion["kind"]>("all");
  let making = $state<{ key: string; front: string; back: string; topic: number | null } | null>(null);
  let error = $state("");
  let status = $state("");

  // Ten at a time: a long backlog stays a short page, and keyboard users don't tab past hundreds of buttons.
  const PAGE = 10;
  let topicFilter = $state<number | "all">("all");
  let text = $state("");
  const matching = $derived((items ?? []).filter((item) => (filter === "all" || item.kind === filter) && (topicFilter === "all" || item.topic === topicFilter) && matchesFilter(text, item.text, item.answer, fromWhere(item))));
  // lists.feature (R149): grouped by topic, biggest first, folded after the first.
  const groups = $derived.by(() => {
    const byTopic = new Map<number | null, Suggestion[]>();
    for (const item of matching) byTopic.set(item.topic, [...(byTopic.get(item.topic) ?? []), item]);
    return [...byTopic.entries()].map(([topic, list]) => ({ topic, name: topic === null ? "No topic" : topicName(topic) || "No topic", list })).sort((a, b) => b.list.length - a.list.length || a.name.localeCompare(b.name));
  });
  let opened = $state<Record<string, boolean>>({});
  let limits = $state<Record<string, number>>({});
  const keyOf = (topic: number | null) => String(topic);
  const isOpen = (topic: number | null, index: number) => opened[keyOf(topic)] ?? (index === 0 || text.trim() !== "" || matching.length <= PAGE);
  const limitOf = (topic: number | null) => limits[keyOf(topic)] ?? PAGE;

  async function dismissAll(list: Suggestion[]) {
    for (const item of list) await api.dismissSuggestion(item.key);
    status = `${list.length} ${list.length === 1 ? "suggestion" : "suggestions"} dismissed`;
    await load();
    onChanged();
  }
  const usedTopics = $derived(topics.filter((topic) => (items ?? []).some((item) => item.topic === topic.id)));
  const kindLabel = { Recall: "Recall", Quiz: "Quiz", Struggle: "Project" } as const;
  const kindChip = { Recall: "chip-due", Quiz: "chip-primary", Struggle: "chip-info" } as const;
  /** "Quiz: Caches" reads as a chip and "Caches", so the kind isn't only words in a small line. */
  const fromWhere = (item: Suggestion) => item.from.replace(/^[^:]+:\s*/, "");
  const count = (kind: Suggestion["kind"]) => (items ?? []).filter((item) => item.kind === kind).length;
  const topicName = (id: number | null) => topics.find((topic) => topic.id === id)?.name ?? "";

  async function load() {
    [items, topics] = await Promise.all([api.cardSuggestions(), api.topics()]);
  }

  function make(item: Suggestion) {
    error = "";
    status = "";
    // R73, R95: a quiz mistake or a project struggle starts as the front. A recall miss is the back.
    making = item.kind === "Recall"
      ? { key: item.key, front: "", back: item.text, topic: item.topic }
      : { key: item.key, front: item.text, back: item.answer ?? "", topic: item.topic };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!making) return;
    if (making.topic === null) { error = "Choose a topic for the card"; return; }
    error = "";
    try {
      await api.saveNote(null, { note: { topic: making.topic, kind: "Basic", front: making.front, back: making.back, cues: [], text: "", extra: "" }, suggestion: making.key });
      status = `Card saved in ${topicName(making.topic)}`;
      making = null;
      await load();
      onChanged();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function dismiss(item: Suggestion) {
    await api.dismissSuggestion(item.key);
    status = "Suggestion dismissed";
    await load();
    onChanged();
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <button class="mt-8 inline-flex items-center gap-1 py-1 text-xs font-extrabold uppercase tracking-wider text-muted hover:underline" onclick={onBack}><ArrowLeft class="size-4" />Cards</button>
  <div class="pb-6 pt-4">
    <h1 class="text-3xl font-semibold">Suggestions</h1>
    <p class="mt-1 text-sm text-muted">Things you missed in a recall or got wrong in a quiz. Make a card for the ones that matter, dismiss the rest.</p>
  </div>
  {#if status}<p role="status" class="mb-3 text-sm text-muted">{status}</p>{/if}
  {#if items}
    {#if items.length === 0}
      <div class="panel max-w-lg p-8"><p class="font-semibold">No suggestions right now</p><p class="mt-1 text-sm text-muted">Points you miss in a recall check, and quiz questions you get wrong or guess, show up here.</p></div>
    {:else}
      <div class="mb-3 flex flex-wrap items-center gap-y-2">
        <div class="seg" role="radiogroup" aria-label="Show">
          <button type="button" role="radio" aria-checked={filter === "all"} onclick={() => (filter = "all")}>All {items.length}</button>
          <button type="button" role="radio" aria-checked={filter === "Recall"} onclick={() => (filter = "Recall")}>Recall misses {count("Recall")}</button>
          <button type="button" role="radio" aria-checked={filter === "Quiz"} onclick={() => (filter = "Quiz")}>Quiz mistakes {count("Quiz")}</button>
          {#if count("Struggle")}<button type="button" role="radio" aria-checked={filter === "Struggle"} onclick={() => (filter = "Struggle")}>Project struggles {count("Struggle")}</button>{/if}
        </div>
        {#if usedTopics.length > 1}
          <select class="select select-sm ml-3 w-56" aria-label="Topic" bind:value={topicFilter}>
            <option value="all">All topics</option>
            {#each usedTopics as topic (topic.id)}<option value={topic.id}>{topic.name}</option>{/each}
          </select>
        {/if}
        <span class="flex-1"></span>
        <span class="text-sm text-muted">Nothing becomes a card until you say so</span>
      </div>
      <div class="mb-4"><FilterBox bind:value={text} shown={matching.length} total={items.length} placeholder="Filter suggestions…" /></div>
      <div role="group" aria-label="Suggestions">
      {#each groups as group, index (group.topic)}
      {@const open = isOpen(group.topic, index)}
      <section class="mb-5" aria-label="{group.name} suggestions">
        <div class="mb-2 flex items-center gap-3">
          <button type="button" class="flex min-h-8 items-center gap-2 text-left" aria-expanded={open} onclick={() => (opened = { ...opened, [keyOf(group.topic)]: !open })}>
            <ChevronRight class="size-4 transition-transform {open ? 'rotate-90' : ''}" aria-hidden="true" /><span class="text-lg font-black">{group.name}</span><span class="chip">{group.list.length}</span>
          </button>
          <span class="flex-1"></span>
          <button class="btn btn-quiet btn-sm" onclick={() => dismissAll(group.list)}><X class="size-4" />Dismiss these {group.list.length}</button>
        </div>
        {#if open}
      <ul class="space-y-3" aria-label="{group.name}">
        {#each group.list.slice(0, limitOf(group.topic)) as item (item.key)}
          {@const open = making?.key === item.key}
          <li class="panel px-5 py-4">
            <div class="flex items-center justify-between gap-6">
              <div class="flex-1"><p class="flex flex-wrap items-center gap-2 text-xs text-muted"><span class="chip {kindChip[item.kind]}">{kindLabel[item.kind]}</span><span>{fromWhere(item)} · {item.note}</span></p>
                <p class="mt-1.5 text-[17px] font-extrabold"><Rich inline text={item.text} />{#if item.answer}{" "}<span class="font-bold text-muted">→ <Rich inline text={item.answer} /></span>{/if}</p></div>
              {#if !open}
                <div class="flex shrink-0 items-center gap-2">
                  <button class="btn btn-ghost btn-sm" aria-label="Dismiss: {item.text}" onclick={() => dismiss(item)}><X class="size-4" />Dismiss</button>
                  <button class="btn btn-outline btn-sm text-primary-ink" onclick={() => make(item)}><Plus class="size-4" />Make a card</button>
                </div>
              {/if}
            </div>
            {#if open && making}
              <form class="mt-5 grid grid-cols-2 gap-6" onsubmit={save}>
                <div>
                  <label class="block text-sm font-semibold" for="sugg-front">Front</label>
                  <!-- svelte-ignore a11y_autofocus -->
                  <textarea id="sugg-front" class="textarea mt-1 w-full text-[15px]" rows="3" aria-describedby="sugg-front-help" placeholder="What were you trying to remember when it didn't come?…" bind:value={making.front} autofocus={item.kind === "Recall"}></textarea>
                  <p id="sugg-front-help" class="mt-1 text-sm text-muted">{item.kind === "Recall" ? "You missed this in a recall, so the front is up to you: the cue that should have brought it to mind." : item.kind === "Struggle" ? "Starts as what tripped you up. Use your 'why' above to write the cue that was missing." : "Starts as the question. Rewrite it as the thought you'll actually have."}</p>
                </div>
                <div>
                  <label class="block text-sm font-semibold" for="sugg-back">Back</label>
                  <textarea id="sugg-back" class="textarea mt-1 w-full text-[15px]" rows="3" bind:value={making.back}></textarea>
                </div>
                <div>
                  <label class="block text-sm font-semibold" for="sugg-topic">Topic</label>
                  <select id="sugg-topic" class="select mt-1 w-full" bind:value={making.topic}>
                    <option value={null} disabled>Choose a topic</option>
                    {#each topics as topic (topic.id)}<option value={topic.id}>{topic.name}</option>{/each}
                  </select>
                </div>
                <div class="col-span-2 flex items-center gap-3">
                  <button class="btn btn-primary" type="submit">Save card</button>
                  <button class="btn btn-ghost" type="button" onclick={() => (making = null)}>Cancel</button>
                  {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}
                </div>
              </form>
            {/if}
          </li>
        {/each}
      </ul>
      {#if group.list.length > limitOf(group.topic)}
        <div class="mt-3 flex items-center gap-4">
          <button class="btn btn-outline btn-sm" onclick={() => (limits = { ...limits, [keyOf(group.topic)]: limitOf(group.topic) + PAGE })}>Show {Math.min(PAGE, group.list.length - limitOf(group.topic))} more</button>
          <span class="text-sm text-muted">Showing {limitOf(group.topic)} of {group.list.length}</span>
        </div>
      {/if}
        {/if}
      </section>
      {:else}
        <p class="panel p-6 text-sm text-muted">No suggestions match. {#if text}<button class="btn btn-link btn-sm" onclick={() => (text = "")}>Clear the filter</button>{/if}</p>
      {/each}
      </div>
    {/if}
  {/if}
</div>

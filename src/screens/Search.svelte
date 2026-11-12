<script lang="ts">
  // search.feature (R31, R50)
  import { ArrowDown, ArrowUp, Search as SearchIcon, X } from "@lucide/svelte";
  import { api, type Id, type Matched, type ModuleRow, type SearchHit, type SearchQuery } from "../lib/api";
  import Highlight from "../components/Highlight.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { dayFromUtc, dayLabel } from "../lib/dates";

  let { onOpenTopic }: { onOpenTopic: (topic: Id) => void } = $props();

  const tabs: SearchQuery["tab"][] = ["All", "Questions", "Modules", "Topics", "Tags", "Sources"];
  let text = $state("");
  let tab = $state<SearchQuery["tab"]>("All");
  let module = $state("");
  let tag = $state("");
  let sort = $state<SearchQuery["sort"]>("Edited");
  let descending = $state(true);
  let hits = $state<SearchHit[]>([]);
  let modules = $state<ModuleRow[]>([]);
  let request = 0;
  let today = $state(0);
  api.today().then((day) => (today = day));

  const questions = $derived(hits.flatMap((hit) => ("Question" in hit ? [hit.Question] : [])));
  const others = $derived(hits.filter((hit) => !("Question" in hit)));
  const filtered = $derived(module !== "" || tag.trim() !== "");

  $effect(() => {
    const query: SearchQuery = { text, tab, module: module ? Number(module) : null, tag: tag.trim() || null, sort, descending };
    const mine = ++request;
    api.search(query).then((result) => {
      if (mine === request) hits = result; // ignore answers to older keystrokes
    });
  });

  api.modules().then((rows) => (modules = rows));

  const sorts: { value: SearchQuery["sort"]; label: string }[] = [
    { value: "Question", label: "Question A–Z" },
    { value: "Topic", label: "Topic" },
    { value: "Edited", label: "Last edited" },
  ];

  function sortBy(column: SearchQuery["sort"]) {
    sort = column;
    descending = column === "Edited";
  }

  const matchLabel = (matched: Matched) => ("Option" in matched ? ["Option", matched.Option] : "Explanation" in matched ? ["Explanation", matched.Explanation] : ["Tag", matched.Tag]);

  // From the search box, Down goes to the first result; Up and Down move between them.
  let resultList = $state<HTMLOListElement>();
  function moveFocus(event: KeyboardEvent, from: number | null) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const buttons = [...(resultList?.querySelectorAll<HTMLButtonElement>("button[data-result]") ?? [])];
    if (!buttons.length) return;
    event.preventDefault();
    if (from === null) {
      if (event.key === "ArrowDown") buttons[0].focus();
      return;
    }
    const next = from + (event.key === "ArrowDown" ? 1 : -1);
    if (next < 0) document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
    else buttons[Math.min(next, buttons.length - 1)].focus();
  }

  function clear() {
    module = "";
    tag = "";
  }

  function otherLabel(hit: SearchHit): { kind: string; label: string } {
    if ("Module" in hit) return { kind: "Module", label: hit.Module.name };
    if ("Topic" in hit) return { kind: "Topic", label: hit.Topic.name };
    if ("Tag" in hit) return { kind: "Tag", label: hit.Tag.name };
    if ("Source" in hit) return { kind: "Source", label: hit.Source.apa.replaceAll("*", "") };
    return { kind: "", label: "" };
  }
</script>


<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Search" />
  <label class="input input-lg search-big flex w-full items-center gap-3">
    <SearchIcon class="size-5" />
    <!-- svelte-ignore a11y_autofocus -->
    <input type="search" class="grow" aria-label="Search" placeholder="Questions, options, explanations, tags, sources…" bind:value={text} autofocus onkeydown={(event) => moveFocus(event, null)} />
  </label>

  <div class="mt-6 flex items-center gap-1 border-b border-base-300" role="tablist" aria-label="Categories">
    {#each tabs as name (name)}
      <button role="tab" aria-selected={tab === name} onclick={() => (tab = name)}>{name}</button>
    {/each}
  </div>

  <div class="mt-4 flex flex-wrap items-center gap-3">
    <span class="text-sm text-muted">Filters</span>
    <label class="flex items-center gap-2 text-sm">Module
      <select class="select select-sm w-56" bind:value={module}>
        <option value="">Any module</option>
        {#each modules as row (row.id)}<option value={String(row.id)}>{row.name}</option>{/each}
      </select>
    </label>
    <label class="flex items-center gap-2 text-sm">Tag <input class="input input-sm w-40" placeholder="e.g. hardware…" bind:value={tag} /></label>
    {#if filtered}<button class="btn btn-ghost btn-sm" onclick={clear}><X class="size-4" />Clear filters</button>{/if}
  </div>

  {#if hits.length === 0}
    <div class="panel mt-6 max-w-lg p-10">
      <h2 class="text-lg font-semibold">{text.trim() ? `No matches for "${text.trim()}"` : "Nothing here yet"}</h2>
      <p class="mt-2 text-muted">{filtered ? "Try fewer words, or remove a filter." : "Try fewer words, or a different tab."}</p>
      {#if filtered}<button class="btn btn-ghost btn-sm mt-4" onclick={clear}>Clear filters</button>{/if}
    </div>
  {/if}

  {#if questions.length}
    <div class="mt-6 flex flex-wrap items-center justify-between gap-3">
      <p class="text-sm text-muted" aria-live="polite">{questions.length} {questions.length === 1 ? "question matches" : "questions match"}{text.trim() ? ` "${text.trim()}"` : ""}</p>
      <div class="flex items-center gap-2 text-sm">
        <span class="text-muted" id="sort-label">Sort</span>
        <div class="seg" role="radiogroup" aria-labelledby="sort-label">
          {#each sorts as option (option.value)}<button type="button" role="radio" aria-checked={sort === option.value} onclick={() => sortBy(option.value)}>{option.label}</button>{/each}
        </div>
        <button class="btn btn-ghost btn-sm btn-square" aria-label={descending ? "Descending, switch to ascending" : "Ascending, switch to descending"} onclick={() => (descending = !descending)}>
          {#if descending}<ArrowDown class="size-4" />{:else}<ArrowUp class="size-4" />{/if}
        </button>
      </div>
    </div>
    <ol bind:this={resultList} class="panel mt-3 divide-y" aria-label="Questions">
      {#each questions as question, index (question.id)}
        <li class="search-result">
          <button class="w-full px-6 py-3 text-left" data-result onclick={() => onOpenTopic(question.topic)} onkeydown={(event) => moveFocus(event, index)} aria-describedby="where-{question.id}">
            <span class="block text-[15px]"><Highlight text={question.text} term={text} /></span>
            {#if question.matched}{@const [kind, value] = matchLabel(question.matched)}<span class="mt-1 block text-sm text-muted"><span class="font-semibold">{kind}:</span> <Highlight text={value} term={text} /></span>{/if}
          </button>
          <span id="where-{question.id}" class="search-meta">
            <span class="chip">{question.topic_name}</span>
            {#each question.tags as name (name)}<span class="chip chip-info">{name}</span>{/each}
            {#if sort === "Edited" && today}<span class="font-bold">{dayLabel(dayFromUtc(question.edited), today)}</span>{/if}
          </span>
        </li>
      {/each}
    </ol>
    <p class="mt-2 text-xs text-muted"><kbd class="kbd kbd-xs">↑</kbd> <kbd class="kbd kbd-xs">↓</kbd> to move, <kbd class="kbd kbd-xs">Enter</kbd> to open</p>
  {/if}

  {#if others.length}
    <ul class="panel mt-6 divide-y" aria-label="Other matches">
      {#each others as hit, index (index)}
        {@const item = otherLabel(hit)}
        <li class="flex items-center gap-4 px-6 py-3 text-sm"><span class="chip w-20 justify-center">{item.kind}</span>{" "}<span>{item.label}</span></li>
      {/each}
    </ul>
  {/if}
</div>

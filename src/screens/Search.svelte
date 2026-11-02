<script lang="ts">
  // search.feature (R31). Which tab questions belong under is open (Q16): they show under All.
  import { ArrowDown, ArrowUp, ArrowUpDown, Search as SearchIcon, X } from "@lucide/svelte";
  import { api, type Id, type ModuleRow, type SearchHit, type SearchQuery } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import { dayFromUtc, dayLabel } from "../lib/dates";

  let { onOpenTopic }: { onOpenTopic: (topic: Id) => void } = $props();

  const tabs: SearchQuery["tab"][] = ["All", "Modules", "Topics", "Tags", "Sources"];
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

  function sortBy(column: SearchQuery["sort"]) {
    if (sort === column) descending = !descending;
    else {
      sort = column;
      descending = column === "Edited";
    }
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

{#snippet header(label: string, column: SearchQuery["sort"])}
  <th>
    <button class="flex items-center gap-1" aria-label="Sort by {label.toLowerCase()}" onclick={() => sortBy(column)}>
      {label}
      {#if sort !== column}<ArrowUpDown class="size-3" />{:else if descending}<ArrowDown class="size-3" />{:else}<ArrowUp class="size-3" />{/if}
    </button>
  </th>
{/snippet}

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Search" />
  <label class="input input-lg flex w-full items-center gap-3">
    <SearchIcon class="size-5" />
    <!-- svelte-ignore a11y_autofocus -->
    <input type="search" class="grow" aria-label="Search" placeholder="Questions, options, explanations, tags, sources…" bind:value={text} autofocus />
  </label>

  <div class="mt-6 flex items-center gap-1 border-b border-base-300" role="tablist" aria-label="Categories">
    {#each tabs as name (name)}
      <button role="tab" aria-selected={tab === name} class="px-3 py-2 text-sm {tab === name ? 'border-b-2 border-primary font-semibold' : 'text-muted'}" onclick={() => (tab = name)}>{name}</button>
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
    <div class="panel mt-6 overflow-hidden">
      <table class="table" aria-label="Questions">
        <thead><tr class="text-muted">{@render header("Question", "Question")}{@render header("Topic", "Topic")}<th>Tags</th>{@render header("Edited", "Edited")}</tr></thead>
        <tbody>
          {#each questions as question (question.id)}
            <tr class="hover:bg-base-100">
              <td class="max-w-md"><button class="text-left hover:underline" onclick={() => onOpenTopic(question.topic)}>{question.text}</button></td>
              <td>{question.topic_name}</td>
              <td>{#each question.tags as name (name)}<span class="badge badge-ghost badge-sm mr-1">{name}</span>{/each}</td>
              <td class="text-sm text-muted">{today ? dayLabel(dayFromUtc(question.edited), today) : ""}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if others.length}
    <ul class="panel mt-6 divide-y" aria-label="Other matches">
      {#each others as hit, index (index)}
        {@const item = otherLabel(hit)}
        <li class="flex items-center gap-4 px-6 py-3 text-sm"><span class="badge badge-ghost badge-sm w-16 justify-center">{item.kind}</span>{" "}<span>{item.label}</span></li>
      {/each}
    </ul>
  {/if}
</div>

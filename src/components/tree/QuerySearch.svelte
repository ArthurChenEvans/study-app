<script lang="ts">
  // R220: the query box with grouped results under it, for the Search screen and list filters.
  // Spec: search-query.feature, "Results are grouped by kind", "A broken query keeps the last
  // good results", "No results". Tests: QuerySearch.test.ts (T-PILL-21, T-PILL-26).
  import { api, type ItemKind, type QuerySearch, type ResultItem } from "../../lib/api";
  import QueryBox from "./QueryBox.svelte";

  let { value = "", onOpenTopic }: { value?: string; onOpenTopic?: (topic: number) => void } = $props();

  // svelte-ignore state_referenced_locally
  let current = $state(value);
  let box = $state(0);
  let good = $state<QuerySearch | null>(null);
  let searched = $state("");
  let tab = $state<ItemKind | "All">("All");

  const KINDS: [ItemKind, string][] = [["Module", "Modules"], ["Topic", "Topics"], ["Question", "Questions"], ["Flashcard", "Flashcards"], ["RecallPoint", "Free Recall"], ["PracticeEntry", "Practice Log"]];

  function results(found: QuerySearch, input: string) {
    if (found.error) return;
    good = found;
    searched = input;
  }

  type Row = { kind: ItemKind; id: number; title: string; path: string; topic: number | null };
  const rows = $derived.by((): Row[] => {
    if (!good) return [];
    const out: Row[] = [];
    for (const [kind] of KINDS) {
      if (tab !== "All" && tab !== kind) continue;
      for (const id of good.results.groups[kind] ?? []) {
        const item: ResultItem | undefined = good.items.find((entry) => entry.kind === kind && entry.id === id);
        const fallback = good.paths.find(([node]) => node.kind === kind && node.id === id)?.[1] ?? "";
        const path = item?.path || fallback;
        out.push({ kind, id, title: item?.title ?? path.split(" › ").at(-1) ?? "", path, topic: item?.topic ?? (kind === "Topic" ? id : null) });
      }
    }
    return out;
  });
  const total = $derived(good ? Object.values(good.results.groups).reduce((sum, ids) => sum + (ids?.length ?? 0), 0) : 0);

  function retype(text: string) {
    current = text;
    box++;
  }

  async function clear() {
    retype("");
    searched = "";
    results(await api.querySearch(""), "");
  }
</script>

<section aria-label="Search results" class="flex flex-col gap-4">
  {#key box}<QueryBox value={current} onResults={results} />{/key}

  {#each good?.unknown ?? [] as unknown (unknown.value)}
    <p class="text-sm">
      No {unknown.kind.toLowerCase()} called {unknown.value}.
      {#if unknown.suggestion}Did you mean <button type="button" class="btn btn-link btn-sm" onclick={() => retype(current.replace(new RegExp(`${unknown.kind.toLowerCase()}:${unknown.value}`, "i"), `${unknown.kind.toLowerCase()}:${unknown.suggestion!.includes(" ") ? `"${unknown.suggestion}"` : unknown.suggestion}`))}>{unknown.suggestion}</button>?{/if}
    </p>
  {/each}

  {#if good && total > 0}
    <div role="tablist" aria-label="Kinds" class="flex flex-wrap gap-1">
      <button type="button" role="tab" aria-selected={tab === "All"} class="tab" onclick={() => (tab = "All")}>All <span class="tabular-nums">{total}</span></button>
      {#each KINDS as [kind, name] (kind)}
        {@const count = good.results.groups[kind]?.length ?? 0}
        {#if count}<button type="button" role="tab" aria-selected={tab === kind} class="tab" onclick={() => (tab = kind)}>{name} <span class="tabular-nums">{count}</span></button>{/if}
      {/each}
    </div>
    <ul class="panel divide-y-2 divide-base-300" aria-label="Results">
      {#each rows as row (`${row.kind}-${row.id}`)}
        <li class="flex flex-col gap-0.5 px-4 py-3">
          <span class="text-[13px] font-bold text-muted">{row.path}</span>
          {#if row.topic != null && onOpenTopic}
            <button type="button" class="btn btn-link btn-sm justify-start p-0 text-left" onclick={() => onOpenTopic!(row.topic!)}>{row.title}</button>
          {:else}
            <span class="font-bold">{row.title}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {:else if good && searched.trim()}
    <div class="panel flex flex-col items-start gap-3 p-6">
      <p>Nothing matches {searched}</p>
      <button type="button" class="btn btn-ghost btn-sm" onclick={clear}>Clear search</button>
    </div>
  {/if}
</section>

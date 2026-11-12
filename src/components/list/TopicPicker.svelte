<script lang="ts">
  // lists.feature (R138, R139, R145): picking topics from a long list. A module rail, a
  // filter, "Tick all N in a module", and the list in its own scrolling pane. Used by New
  // quiz. Each row is one label, so clicking anywhere on it ticks it.
  import { SquareCheck, SquareMinus } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import Highlight from "../Highlight.svelte";
  import Rail, { type RailItem } from "./Rail.svelte";
  import FilterBox from "./FilterBox.svelte";
  import type { Id, ModuleRow, TopicRow } from "../../lib/api";
  import { matchesFilter, sortBy } from "../../lib/lists";

  let { topics, modules, chosen = $bindable([]), label, disabled = () => false, extra }: {
    topics: TopicRow[];
    modules: ModuleRow[];
    chosen?: Id[];
    label: string;
    /** Topics that can't be ticked (no questions of the picked type). */
    disabled?: (topic: TopicRow) => boolean;
    /** What goes at the end of each row. */
    extra?: Snippet<[TopicRow]>;
  } = $props();

  let rail = $state("all");
  let filter = $state("");

  const moduleName = (id: Id | null) => (id === null ? null : (modules.find((m) => m.id === id)?.name ?? null));
  const filtered = $derived(topics.filter((topic) => matchesFilter(filter, topic.name, moduleName(topic.module), topic.tags.join(" "))));
  const inRail = (topic: TopicRow) => rail === "all" || (rail === "none" ? topic.module === null : `m${topic.module}` === rail);
  const shown = $derived(sortBy(filtered.filter(inRail), (topic) => topic.name, true));
  const railItems = $derived<RailItem[]>([
    { key: "all", label: "All modules", count: filtered.length },
    ...modules.map((m) => ({ key: `m${m.id}`, label: m.name, count: filtered.filter((t) => t.module === m.id).length })),
    ...(topics.some((t) => t.module === null) ? [{ key: "none", label: "No module", count: filtered.filter((t) => t.module === null).length }] : []),
  ]);
  const railModule = $derived(rail.startsWith("m") ? modules.find((m) => `m${m.id}` === rail) ?? null : null);
  const tickable = $derived(railModule ? topics.filter((t) => t.module === railModule.id && !disabled(t)) : []);
  const allTicked = $derived(tickable.length > 0 && tickable.every((t) => chosen.includes(t.id)));

  function toggle(id: Id) {
    chosen = chosen.includes(id) ? chosen.filter((topic) => topic !== id) : [...chosen, id];
  }
</script>

<div class="flex gap-5">
  <Rail items={railItems} bind:value={rail} />
  <section class="min-w-0 flex-1" aria-label={label}>
    <div class="mb-3 flex flex-wrap items-center gap-3">
      <FilterBox bind:value={filter} shown={shown.length} total={topics.length} placeholder="Filter topics…" />
      {#if railModule && tickable.length}
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => (chosen = allTicked ? chosen.filter((id) => !tickable.some((t) => t.id === id)) : [...new Set([...chosen, ...tickable.map((t) => t.id)])])}>
          {#if allTicked}<SquareMinus class="size-4" />Untick all {tickable.length} in {railModule.name}{:else}<SquareCheck class="size-4" />Tick all {tickable.length} in {railModule.name}{/if}
        </button>
      {/if}
    </div>
    {#if shown.length}
      <ul class="panel pane divide-y" aria-label={label}>
        {#each shown as topic (topic.id)}
          {@const off = disabled(topic)}
          <li class={chosen.includes(topic.id) ? "bg-[color-mix(in_oklch,var(--primary-ink)_10%,var(--surface))]" : ""}>
            <label class="flex items-center gap-3 px-4 py-2.5 {off ? 'cursor-not-allowed opacity-60' : 'cursor-pointer hover:bg-base-200'}">
              <input type="checkbox" class="checkbox checkbox-sm" checked={chosen.includes(topic.id)} disabled={off} onchange={() => toggle(topic.id)} />
              <span class="min-w-0 flex-1"><span class="block truncate text-[15px] font-extrabold"><Highlight text={topic.name} term={filter} /></span>
                {#if rail === "all"}<span class="block truncate text-xs text-muted">{moduleName(topic.module) ?? "No module"}</span>{/if}</span>
              {@render extra?.(topic)}
            </label>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="panel p-6 text-sm text-muted">{topics.length ? `Nothing matches "${filter.trim()}".` : "Create a topic and write some questions first."}{#if filter}{" "}<button type="button" class="btn btn-link btn-sm" onclick={() => (filter = "")}>Clear the filter</button>{/if}</div>
    {/if}
  </section>
</div>

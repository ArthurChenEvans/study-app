<script lang="ts">
  // sources.feature (R13, R44 to R46)
  import { X, BookOpen, Plus } from "@lucide/svelte";
  import RowMenu from "../components/RowMenu.svelte";
  import { api, errorMessage, type SourceFields, type SourceRow, type TopicRow } from "../lib/api";
  import { deleteWithUndo, toast } from "../lib/celebrations.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Rich from "../components/Rich.svelte";
  import Highlight from "../components/Highlight.svelte";
  import FilterBox from "../components/list/FilterBox.svelte";
  import SortHeader, { type Sort } from "../components/list/SortHeader.svelte";
  import { matchesFilter, sortBy } from "../lib/lists";

  let sources = $state<SourceRow[]>([]);
  let topics = $state<TopicRow[]>([]);
  let loaded = $state(false);
  let adding = $state(false);
  let error = $state("");
  let draft = $state(blank());
  let editing = $state<number | null>(null);
  let deleting = $state<SourceRow | null>(null);

  function edit(source: SourceRow) {
    const f = source.fields;
    draft = { authors: f.authors, year: f.year ?? "", title: f.title, edition: f.edition ?? "", description: f.description ?? "", publisher: f.publisher ?? "", url: f.url ?? "" };
    error = "";
    editing = source.id;
    adding = true;
  }

  let hiddenSources = $state<Set<number>>(new Set());
  const unhide = (id: number) => (hiddenSources = new Set([...hiddenSources].filter((value) => value !== id)));

  function confirmDelete() {
    if (!deleting) return;
    const source = deleting;
    deleting = null;
    deleteWithUndo({
      text: "Source deleted",
      sub: source.fields.title,
      hide: () => {
        hiddenSources = new Set([...hiddenSources, source.id]);
        sources = sources.filter((row) => row.id !== source.id);
      },
      restore: () => { unhide(source.id); load(); },
      commit: async () => { await api.deleteSource(source.id); unhide(source.id); await load(); },
    });
  }

  function blank() {
    return { authors: "", year: "", title: "", edition: "", description: "", publisher: "", url: "" };
  }

  const fields: { key: keyof ReturnType<typeof blank>; label: string; wide?: boolean; hint?: string }[] = [
    { key: "authors", label: "Author(s)", wide: true, hint: "As in APA, e.g. Kirk, D. B., & Hwu, W. W." },
    { key: "year", label: "Year" },
    { key: "edition", label: "Edition", hint: "e.g. 4th" },
    { key: "title", label: "Title", wide: true },
    { key: "description", label: "Description", hint: "e.g. Lecture slides" },
    { key: "publisher", label: "Publisher" },
    { key: "url", label: "URL", wide: true },
  ];

  // lists.feature (R149): one compact sortable row per source, the full reference in the item card.
  let filter = $state("");
  let sort = $state<Sort>({ key: "authors", up: true });
  let picked = $state<number | null>(null);
  const shown = $derived(sortBy(sources.filter((s) => matchesFilter(filter, s.fields.authors, s.fields.title, s.fields.year, s.fields.publisher)), (s) => (sort.key === "year" ? (s.fields.year ?? null) : sort.key === "title" ? s.fields.title : sort.key === "cited" ? s.cited_by_questions : s.fields.authors || s.fields.title), sort.up, (s) => s.fields.title));
  const pickedSource = $derived(sources.find((s) => s.id === picked) ?? null);

  const topicName = (id: number) => topics.find((topic) => topic.id === id)?.name ?? "Deleted topic";

  async function load() {
    const [allSources, allTopics] = await Promise.all([api.sources(), api.topics()]);
    sources = allSources.filter((row) => !hiddenSources.has(row.id));
    topics = allTopics;
    loaded = true;
  }

  function open() {
    draft = blank();
    editing = null;
    error = "";
    adding = true;
  }

  async function add(event: SubmitEvent) {
    event.preventDefault();
    const optional = (text: string) => (text.trim() === "" ? null : text.trim());
    const fieldsToSave: SourceFields = {
      authors: draft.authors.trim(),
      year: optional(draft.year),
      title: draft.title.trim(),
      edition: optional(draft.edition),
      description: optional(draft.description),
      publisher: optional(draft.publisher),
      url: optional(draft.url),
    };
    try {
      if (editing === null) await api.addSource(fieldsToSave);
      else await api.updateSource(editing, fieldsToSave);
      toast({ text: editing === null ? "Source added" : "Source saved", sub: fieldsToSave.title, icon: editing === null ? "plus" : "check" });
      adding = false;
      await load();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Bibliography" subtitle={sources.length ? `${sources.length} sources, formatted in APA` : ""}>
    {#if sources.length}<button class="btn btn-primary" onclick={open}><Plus class="size-4" />Add source</button>{/if}
  </PageHeader>

  {#if loaded && sources.length === 0}
    <div class="panel max-w-lg p-10">
      <span class="well" aria-hidden="true"><BookOpen class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No sources yet</h2>
      <p class="mt-2 text-muted">Add the books, papers and lectures you write questions from, then cite them on each question.</p>
      <button class="btn btn-primary mt-6" onclick={open}><Plus class="size-4" />Add your first source</button>
    </div>
  {:else}
    <div class="mb-3"><FilterBox bind:value={filter} shown={shown.length} total={sources.length} placeholder="Filter by author, title or year…" /></div>
    <div class="flex gap-5">
      <div class="min-w-0 flex-1">
        {#if shown.length}
          <div class="panel pane">
            <table class="rows" aria-label="Sources">
              <thead><tr>
                <SortHeader key="authors" label="Author(s)" bind:sort />
                <SortHeader key="year" label="Year" bind:sort />
                <SortHeader key="title" label="Title" bind:sort />
                <SortHeader key="cited" label="Cited by" numeric bind:sort />
              </tr></thead>
              <tbody>
                {#each shown as source (source.id)}
                  <tr aria-selected={picked === source.id} onclick={() => (picked = source.id)}>
                    <td class="max-w-56"><button type="button" class="block min-h-6 w-full truncate text-left font-extrabold" onclick={() => (picked = source.id)}><Highlight text={source.fields.authors || "No author"} term={filter} /></button></td>
                    <td class="whitespace-nowrap tabular-nums" data-label="Year">{source.fields.year ?? "n.d."}</td>
                    <td class="max-w-0 w-full truncate italic" data-label="Title"><Highlight text={source.fields.title} term={filter} /></td>
                    <td class="num whitespace-nowrap" data-label="Cited by">{source.cited_by_questions} {source.cited_by_questions === 1 ? "question" : "questions"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="panel p-6 text-sm text-muted">Nothing matches "{filter.trim()}". <button class="btn btn-link btn-sm" onclick={() => (filter = "")}>Clear the filter</button></p>
        {/if}
      </div>
      <aside class="itemcard" aria-label="Picked source" data-empty={!pickedSource}>
        {#if pickedSource}
          <button type="button" class="itemcard-close btn btn-quiet btn-sm btn-square" aria-label="Close" onclick={() => (picked = null)}><X class="size-4" /></button>
          {@const source = pickedSource}
          <div class="panel p-5">
            <div class="flex items-start justify-between gap-2"><span class="well" aria-hidden="true"><BookOpen class="size-5" /></span>
              <RowMenu small label="Source actions" items={[{ label: "Edit source", onSelect: () => edit(source) }, { label: "Delete source…", danger: true, onSelect: () => (deleting = source) }]} /></div>
            <p class="mt-3 pl-6 -indent-6 text-[15px] leading-relaxed"><Rich inline text={source.apa} /></p>
            <p class="mt-3 text-sm text-muted">Cited by <span class="tabular-nums">{source.cited_by_questions} {source.cited_by_questions === 1 ? "question" : "questions"}</span></p>
            <div class="mt-2 flex flex-wrap gap-1.5">{#each source.used_by as id (id)}<span class="chip">{topicName(id)}</span>{:else}<span class="text-sm text-muted">Not cited yet</span>{/each}</div>
          </div>
        {:else}
          <div class="panel p-5 text-sm text-muted"><p class="font-extrabold text-base-content">Pick a source to see it here</p><p class="mt-2">The full APA reference and the topics citing it show here.</p></div>
        {/if}
      </aside>
    </div>
  {/if}
</div>

{#if adding}
  <dialog open class="modal modal-open" aria-labelledby="add-source-title">
    <form class="modal-box max-w-lg" onsubmit={add}>
      <h3 id="add-source-title" class="text-lg font-semibold">{editing === null ? "Add source" : "Edit source"}</h3>
      <div class="mt-6 grid grid-cols-2 gap-4">
        {#each fields as field (field.key)}
          <label class="block {field.wide ? 'col-span-2' : ''}">
            <span class="text-sm font-semibold">{field.label}</span>
            <input class="input mt-1 w-full" bind:value={draft[field.key]} placeholder={field.hint ? `${field.hint}…` : ""} />
          </label>
        {/each}
      </div>
      <p class="mt-4 text-sm text-muted">Shown in APA style. Only the title is needed. Leave anything you don't have blank.{editing === null ? "" : " Past quiz attempts keep the old details."}</p>
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (adding = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">{editing === null ? "Add source" : "Save source"}</button>
      </div>
    </form>
  </dialog>
{/if}

{#if deleting}
  <dialog open class="modal modal-open" aria-labelledby="delete-source-title">
    <div class="modal-box max-w-md">
      <h3 id="delete-source-title" class="text-lg font-semibold">Delete this source?</h3>
      <p class="mt-2 text-muted">
        {#if deleting.cited_by_questions === 0}It isn't cited by any questions.
        {:else}{deleting.cited_by_questions === 1 ? "1 question cites it" : `${deleting.cited_by_questions} questions cite it`}, and those citations will be removed.{/if}
        Past quiz attempts keep it as it was.
      </p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = null)}>Cancel</button>
        <button class="btn btn-error" onclick={confirmDelete}>Delete source</button>
      </div>
    </div>
  </dialog>
{/if}

<script lang="ts">
  // sources.feature (R13). Deleting or editing a cited source is open (Q10, Q11), so not offered yet.
  import { BookOpen, Plus } from "@lucide/svelte";
  import { api, errorMessage, type SourceFields, type SourceRow, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import Rich from "../components/Rich.svelte";

  let sources = $state<SourceRow[]>([]);
  let topics = $state<TopicRow[]>([]);
  let loaded = $state(false);
  let adding = $state(false);
  let error = $state("");
  let draft = $state(blank());

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

  const maxCited = $derived(Math.max(1, ...sources.map((source) => source.cited_by_questions)));
  const topicName = (id: number) => topics.find((topic) => topic.id === id)?.name ?? "Deleted topic";

  async function load() {
    [sources, topics] = await Promise.all([api.sources(), api.topics()]);
    loaded = true;
  }

  function open() {
    draft = blank();
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
      await api.addSource(fieldsToSave);
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
      <span class="grid size-10 place-items-center rounded-lg bg-base-200"><BookOpen class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No sources yet</h2>
      <p class="mt-2 text-muted">Add the books, papers and lectures you write questions from, then cite them on each question.</p>
      <button class="btn btn-primary mt-6" onclick={open}><Plus class="size-4" />Add your first source</button>
    </div>
  {:else}
    <ul class="panel divide-y">
      {#each sources as source (source.id)}
        <li class="flex gap-10 p-6" aria-label={source.fields.authors}>
          <p class="max-w-prose flex-1 pl-8 -indent-8 leading-relaxed"><Rich inline text={source.apa} /></p>
          <div class="w-64 shrink-0 text-sm">
            <p class="flex items-baseline justify-between"><span class="text-muted">Cited by</span>{" "}<span class="font-semibold tabular-nums">{source.cited_by_questions} questions</span></p>
            <div class="mt-2 h-1.5 rounded-full bg-base-200"><div class="h-full rounded-full" style="width: {(source.cited_by_questions / maxCited) * 100}%; background: var(--chart-ink)"></div></div>
            <p class="mt-3 flex flex-wrap gap-1">{#each source.used_by as id (id)}<span class="badge badge-ghost badge-sm">{topicName(id)}</span>{:else}<span class="text-muted">Not cited yet</span>{/each}</p>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if adding}
  <dialog open class="modal modal-open" aria-labelledby="add-source-title">
    <form class="modal-box max-w-lg" onsubmit={add}>
      <h3 id="add-source-title" class="text-lg font-semibold">Add source</h3>
      <div class="mt-6 grid grid-cols-2 gap-4">
        {#each fields as field (field.key)}
          <label class="block {field.wide ? 'col-span-2' : ''}">
            <span class="text-sm font-semibold">{field.label}</span>
            <input class="input mt-1 w-full" bind:value={draft[field.key]} placeholder={field.hint ? `${field.hint}…` : ""} />
          </label>
        {/each}
      </div>
      <p class="mt-4 text-sm text-muted">Shown in APA style once added. Leave anything you don't have blank.</p>
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (adding = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Add source</button>
      </div>
    </form>
  </dialog>
{/if}

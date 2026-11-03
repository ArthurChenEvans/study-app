<script lang="ts">
  // flashcards.feature (R72, R73): suggestions from recall misses and quiz mistakes wait for you.
  import { ArrowLeft, Plus, X } from "@lucide/svelte";
  import { api, errorMessage, type Suggestion, type TopicRow } from "../lib/api";
  import Rich from "../components/Rich.svelte";

  let { onBack, onChanged = () => {} }: { onBack: () => void; onChanged?: () => void } = $props();

  let items = $state<Suggestion[] | null>(null);
  let topics = $state<TopicRow[]>([]);
  let filter = $state<"all" | "Recall" | "Quiz">("all");
  let making = $state<{ key: string; front: string; back: string; topic: number | null } | null>(null);
  let error = $state("");
  let status = $state("");

  const shown = $derived((items ?? []).filter((item) => filter === "all" || item.kind === filter));
  const count = (kind: "Recall" | "Quiz") => (items ?? []).filter((item) => item.kind === kind).length;
  const topicName = (id: number | null) => topics.find((topic) => topic.id === id)?.name ?? "";

  async function load() {
    [items, topics] = await Promise.all([api.cardSuggestions(), api.topics()]);
  }

  function make(item: Suggestion) {
    error = "";
    status = "";
    making = item.kind === "Quiz"
      ? { key: item.key, front: item.text, back: item.answer ?? "", topic: item.topic }
      : { key: item.key, front: "", back: item.text, topic: item.topic };
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
  <button class="mt-8 inline-flex items-center gap-1 py-1 text-sm text-muted hover:underline" onclick={onBack}><ArrowLeft class="size-4" />Cards</button>
  <div class="pb-6 pt-4">
    <h1 class="text-3xl font-semibold">Suggestions</h1>
    <p class="mt-1 text-sm text-muted">Things you missed in a recall or got wrong in a quiz. Make a card for the ones that matter, dismiss the rest.</p>
  </div>
  {#if status}<p role="status" class="mb-3 text-sm text-muted">{status}</p>{/if}
  {#if items}
    {#if items.length === 0}
      <div class="panel max-w-lg p-8"><p class="font-semibold">No suggestions right now</p><p class="mt-1 text-sm text-muted">Points you miss in a recall check, and quiz questions you get wrong or guess, show up here.</p></div>
    {:else}
      <div class="mb-3 flex items-center justify-between">
        <div class="seg" role="radiogroup" aria-label="Show">
          <button type="button" role="radio" aria-checked={filter === "all"} onclick={() => (filter = "all")}>All {items.length}</button>
          <button type="button" role="radio" aria-checked={filter === "Recall"} onclick={() => (filter = "Recall")}>Recall misses {count("Recall")}</button>
          <button type="button" role="radio" aria-checked={filter === "Quiz"} onclick={() => (filter = "Quiz")}>Quiz mistakes {count("Quiz")}</button>
        </div>
        <span class="text-sm text-muted">Nothing becomes a card until you say so</span>
      </div>
      <ul class="panel divide-y" aria-label="Suggestions">
        {#each shown as item (item.key)}
          {@const open = making?.key === item.key}
          <li class="px-6 py-5">
            <div class="flex items-start justify-between gap-6">
              <div class="flex-1"><p class="text-xs text-muted">{item.from} · {item.note}</p>
                <p class="mt-1 text-[15px]"><Rich inline text={item.text} /></p>
                {#if item.answer}<p class="text-sm text-muted">→ <Rich inline text={item.answer} /></p>{/if}</div>
              {#if !open}
                <div class="flex shrink-0 items-center gap-2">
                  <button class="btn btn-ghost btn-sm" aria-label="Dismiss: {item.text}" onclick={() => dismiss(item)}><X class="size-4" />Dismiss</button>
                  <button class="btn btn-outline btn-sm border-field-border" onclick={() => make(item)}><Plus class="size-4" />Make a card</button>
                </div>
              {/if}
            </div>
            {#if open && making}
              <form class="mt-5 grid grid-cols-2 gap-6" onsubmit={save}>
                <div>
                  <label class="block text-sm font-semibold" for="sugg-front">Front</label>
                  <!-- svelte-ignore a11y_autofocus -->
                  <textarea id="sugg-front" class="textarea mt-1 w-full text-[15px]" rows="3" aria-describedby="sugg-front-help" placeholder="What were you trying to remember when it didn't come?…" bind:value={making.front} autofocus={item.kind === "Recall"}></textarea>
                  <p id="sugg-front-help" class="mt-1 text-sm text-muted">{item.kind === "Recall" ? "You missed this in a recall, so the front is up to you: the cue that should have brought it to mind." : "Starts as the question. Rewrite it as the thought you'll actually have."}</p>
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
    {/if}
  {/if}
</div>

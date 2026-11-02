<script lang="ts">
  // questions.feature, sources.feature (R4 to R10, R14, R48, R49)
  import { BookOpen, ImagePlus, Plus, X } from "@lucide/svelte";
  import Preview from "../components/Preview.svelte";
  import Rich from "../components/Rich.svelte";
  import { api, errorMessage, type Id, type QuestionInput, type SourceRow } from "../lib/api";
  import { imageMarkdown } from "../lib/images";

  let { topic }: { topic: Id } = $props();

  const MIN = 3;
  const MAX = 5;

  type Draft = { text: string; multiple: boolean; options: { text: string; correct: boolean }[]; explanation: string; tags: string; citations: { source: string; location: string }[] };
  const blank = (): Draft => ({ text: "", multiple: false, options: Array.from({ length: 4 }, () => ({ text: "", correct: false })), explanation: "", tags: "", citations: [] });

  let topicName = $state("");
  let questions = $state<[Id, string][]>([]);
  let sources = $state<SourceRow[]>([]);
  let editing = $state<Id | null>(null);
  let draft = $state<Draft>(blank());
  let error = $state("");
  let saved = $state("");

  async function load() {
    const [topics, list, sourceRows] = await Promise.all([api.topics(), api.questionsIn(topic), api.sources()]);
    topicName = topics.find((row) => row.id === topic)?.name ?? "";
    questions = list;
    sources = sourceRows;
  }

  function toggleCorrect(index: number) {
    const ticking = !draft.options[index].correct;
    draft.options = draft.options.map((option, position) => ({
      ...option,
      correct: position === index ? ticking : draft.multiple ? option.correct : false,
    }));
  }

  let switchNote = $state("");

  // R49: going from multiple to single keeps the first tick and clears the rest.
  function setMultiple(multiple: boolean) {
    switchNote = "";
    if (!multiple && draft.multiple) {
      const first = draft.options.findIndex((option) => option.correct);
      const ticks = draft.options.filter((option) => option.correct).length;
      draft.options = draft.options.map((option, index) => ({ ...option, correct: index === first }));
      if (ticks > 1) switchNote = `Kept option ${first + 1} as the correct answer and cleared the others.`;
    }
    draft.multiple = multiple;
  }

  function input(): QuestionInput {
    return {
      topic,
      text: draft.text.trim(),
      multiple: draft.multiple,
      options: draft.options.filter((option) => option.text.trim() !== "").map((option) => ({ text: option.text.trim(), correct: option.correct })),
      explanation: draft.explanation.trim(),
      tags: draft.tags.split(",").map((tag) => tag.trim()).filter(Boolean),
      citations: draft.citations.filter((citation) => citation.source !== "").map((citation) => ({ source: Number(citation.source), location: citation.location.trim() })),
    };
  }

  async function save(event?: Event) {
    event?.preventDefault();
    error = "";
    switchNote = "";
    try {
      if (editing === null) await api.saveQuestion(input());
      else await api.updateQuestion(editing, input());
      saved = editing === null ? "Question saved" : "Changes saved";
      editing = null;
      draft = blank();
      await load();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function edit(id: Id) {
    const question = await api.question(id);
    editing = id;
    error = "";
    saved = "";
    draft = {
      text: question.text,
      multiple: question.multiple,
      options: question.options.map((option) => ({ ...option })),
      explanation: question.explanation,
      tags: question.tags.join(", "),
      citations: question.citations.map((citation) => ({ source: String(citation.source), location: citation.location })),
    };
  }

  // R7 / questions.feature "Add an image by pasting or picking". Images go into the
  // database straight away and the field gets ![name](image:12) at the cursor.
  type RichField = HTMLInputElement | HTMLTextAreaElement;
  const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];
  let questionField = $state<HTMLTextAreaElement>();
  let lastField = $state<RichField>();
  let picker = $state<HTMLInputElement>();
  let imageNote = $state("");

  /** "the question", "the explanation" or "option 2", for the status message. */
  function fieldName(field: RichField): string {
    const option = field.getAttribute("aria-label");
    if (option) return option.toLowerCase();
    return `the ${(field.labels?.[0]?.textContent ?? "question").trim().toLowerCase()}`;
  }

  async function addImages(files: File[], field: RichField) {
    error = "";
    imageNote = "";
    for (const file of files) {
      try {
        const id = await api.addImage(file);
        const start = field.selectionStart ?? field.value.length;
        const end = field.selectionEnd ?? start;
        const spacer = start > 0 && !/\s$/.test(field.value.slice(0, start)) ? " " : "";
        field.setRangeText(spacer + imageMarkdown(id, file.name), start, end, "end");
        field.dispatchEvent(new Event("input", { bubbles: true }));
        imageNote = `Image added to ${fieldName(field)}`;
      } catch (failure) {
        error = errorMessage(failure);
      }
    }
  }

  function onPaste(event: ClipboardEvent) {
    const field = event.target;
    if (!(field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) || !field.dataset.rich) return;
    const images = [...(event.clipboardData?.files ?? [])].filter((file) => IMAGE_TYPES.includes(file.type));
    if (!images.length) return;
    event.preventDefault();
    addImages(images, field);
  }

  function onFocus(event: FocusEvent) {
    const field = event.target;
    if ((field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) && field.dataset.rich) lastField = field;
  }

  function onPicked() {
    const files = [...(picker?.files ?? [])];
    const field = lastField?.isConnected ? lastField : questionField;
    if (files.length && field) addImages(files, field).then(() => field.focus());
    if (picker) picker.value = "";
  }

  /** Only preview when there's something Markdown can't show as plain text. */
  const worthPreviewing = (text: string) => /image:\d|```|\$[^$]+\$|!\[/.test(text);

  function startNew() {
    editing = null;
    draft = blank();
    error = "";
    imageNote = "";
  }

  load();
</script>

<svelte:window
  onkeydown={(event) => {
    // R48: Ctrl+S saves the question.
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") save(event);
  }}
/>

<div class="flex h-screen">
  <section class="flex w-1/2 min-w-0 flex-col border-r border-base-300 bg-base-200" aria-label="Source preview">
    <Preview />
  </section>

  <section class="w-1/2 overflow-auto" aria-label="Question editor">
    <div class="flex items-end justify-between px-10 pb-6 pt-10">
      <div>
        <p class="text-sm text-muted">{topicName}</p>
        <h1 class="mt-1 text-2xl font-semibold">{editing === null ? "New question" : "Edit question"}</h1>
      </div>
      {#if editing !== null}<button class="btn btn-ghost btn-sm" onclick={startNew}><Plus class="size-4" />New question</button>{/if}
    </div>

    {#if questions.length}
      <details class="mx-10 mb-6 max-w-2xl panel">
        <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">{questions.length} questions in {topicName}</summary>
        <ul class="divide-y border-t border-base-300">
          {#each questions as [id, text] (id)}
            <li><button class="w-full px-4 py-2 text-left text-sm hover:bg-base-100 {editing === id ? 'font-semibold' : ''}" onclick={() => edit(id)}>{text}</button></li>
          {/each}
        </ul>
      </details>
    {/if}

    <form class="max-w-2xl space-y-8 px-10 pb-16" onsubmit={save} onpaste={onPaste} onfocusin={onFocus}>
      <div class="space-y-2">
        <label class="text-sm font-semibold" for="question-text">Question</label>
        <textarea id="question-text" class="textarea w-full font-mono text-sm" rows="3" data-rich="true" bind:this={questionField} bind:value={draft.text}></textarea>
        <div class="flex flex-wrap items-center justify-between gap-2">
          <p class="text-sm text-muted">Markdown, `code`, $LaTeX$ and images. Paste an image into any field, or add one from a file.</p>
          <button type="button" class="btn btn-ghost btn-sm" title="Adds the image where your cursor was last, in the question, an option or the explanation" onclick={() => picker?.click()}><ImagePlus class="size-4" />Add image</button>
          <input bind:this={picker} type="file" class="hidden" accept={IMAGE_TYPES.join(",")} multiple aria-label="Image file" onchange={onPicked} />
        </div>
        {#if imageNote}<p role="status" class="text-sm text-muted">{imageNote}</p>{/if}
        {#if worthPreviewing(draft.text)}<div class="panel p-4" aria-label="Question preview"><Rich text={draft.text} class="max-w-prose" /></div>{/if}
      </div>

      <fieldset class="space-y-3">
        <div class="flex items-center justify-between">
          <legend class="text-sm font-semibold">Answers</legend>
          <div class="seg" role="radiogroup" aria-label="Answer type">
            <button type="button" role="radio" aria-checked={!draft.multiple} onclick={() => setMultiple(false)}>Single</button>
            <button type="button" role="radio" aria-checked={draft.multiple} onclick={() => setMultiple(true)}>Multiple</button>
          </div>
        </div>
        {#each draft.options as option, index (index)}
          <div class="flex items-center gap-3">
            <input type="checkbox" class="checkbox checkbox-sm" aria-label="Option {index + 1} is correct" checked={option.correct} onchange={() => toggleCorrect(index)} />
            <input class="input input-sm flex-1 {option.correct ? 'is-right' : ''}" aria-label="Option {index + 1}" data-rich="true" bind:value={option.text} />
            <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Remove option {index + 1}" disabled={draft.options.length <= MIN} onclick={() => (draft.options = draft.options.filter((_, position) => position !== index))}><X class="size-4" /></button>
          </div>
        {/each}
        <div class="flex items-center gap-3">
          <button type="button" class="btn btn-ghost btn-sm" disabled={draft.options.length >= MAX} onclick={() => (draft.options = [...draft.options, { text: "", correct: false }])}><Plus class="size-4" />Add option</button>
          {#if switchNote}<span role="status" class="block text-sm text-muted">{switchNote}</span>{/if}
          <span class="text-sm text-muted tabular-nums">{draft.options.length} of {MAX} options. {draft.multiple ? "Tick every correct answer." : "Tick the correct answer."}</span>
        </div>
      </fieldset>

      <div class="space-y-2">
        <label class="text-sm font-semibold" for="question-explanation">Explanation</label>
        <textarea id="question-explanation" class="textarea w-full font-mono text-sm" rows="3" data-rich="true" bind:value={draft.explanation}></textarea>
        {#if worthPreviewing(draft.explanation)}<div class="panel p-4" aria-label="Explanation preview"><Rich text={draft.explanation} class="max-w-prose" /></div>{/if}
      </div>

      <fieldset class="space-y-3">
        <legend class="text-sm font-semibold">Sources <span class="font-normal text-muted">(optional)</span></legend>
        {#each draft.citations as citation, index (index)}
          <div class="flex items-center gap-3" role="group" aria-label="Citation {index + 1}">
            <BookOpen class="size-4 shrink-0" />
            <select class="select select-sm flex-1" aria-label="Source" bind:value={citation.source}>
              <option value="">Pick a source…</option>
              {#each sources as source (source.id)}<option value={String(source.id)}>{source.apa}</option>{/each}
            </select>
            <input class="input input-sm w-32" aria-label="Page, slide or line" placeholder="p. 42…" bind:value={citation.location} />
            <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Remove citation {index + 1}" onclick={() => (draft.citations = draft.citations.filter((_, position) => position !== index))}><X class="size-4" /></button>
          </div>
        {/each}
        <button type="button" class="btn btn-ghost btn-sm" onclick={() => (draft.citations = [...draft.citations, { source: "", location: "" }])}><Plus class="size-4" />Cite a source</button>
        {#if sources.length === 0}<p class="text-sm text-muted">Add sources in the Bibliography first.</p>{/if}
      </fieldset>

      <div class="space-y-2">
        <label class="text-sm font-semibold" for="question-tags">Tags</label>
        <input id="question-tags" class="input input-sm w-full" placeholder="pipeline, registers…" bind:value={draft.tags} />
        <p class="text-sm text-muted">Separate tags with commas.</p>
      </div>

      {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}
      {#if saved && !error}<p role="status" class="text-sm text-muted">{saved}</p>{/if}
      <div class="flex items-center gap-3 border-t border-base-300 pt-6">
        <button type="submit" class="btn btn-primary">Save question</button>
        {#if editing !== null}<button type="button" class="btn btn-ghost" onclick={startNew}>Cancel</button>{/if}
      </div>
    </form>
  </section>
</div>

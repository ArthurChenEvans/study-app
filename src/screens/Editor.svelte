<script lang="ts">
  // questions.feature, sources.feature, short-answer.feature (R4 to R10, R14, R48, R49, R78 to R82)
  import { BookOpen, Check, CircleCheck, CircleX, Eye, ImagePlus, Plus, Trash2, X } from "@lucide/svelte";
  import Preview from "../components/Preview.svelte";
  import { showCitation } from "../lib/sourceFiles.svelte";
  import { locationTarget } from "../lib/toc";
  import Rich from "../components/Rich.svelte";
  import { api, errorMessage, type Id, type Marking, type QuestionInput, type ShortAnswer, type SourceRow } from "../lib/api";
  import { toast } from "../lib/celebrations.svelte";
  import { IMAGE_TYPES, imageInput, listenForDrops } from "../lib/imageInput.svelte";

  let { topic }: { topic: Id } = $props();

  const MIN = 3;
  const MAX = 5;

  type Draft = {
    text: string; multiple: boolean; options: { text: string; correct: boolean }[]; explanation: string; tags: string; citations: { source: string; location: string }[];
    // R78 to R81. Kept while switching type, so the options come back before saving.
    kind: "mcq" | "short"; checkBy: "Accepted" | "Keywords"; accepted: string[]; typos: boolean; keywords: { word: string; also: string }[]; needed: number | null; model: string;
  };
  const blank = (): Draft => ({
    text: "", multiple: false, options: Array.from({ length: 4 }, () => ({ text: "", correct: false })), explanation: "", tags: "", citations: [],
    kind: "mcq", checkBy: "Accepted", accepted: [""], typos: true, keywords: [{ word: "", also: "" }], needed: null, model: "",
  });

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

  function shortAnswer(): ShortAnswer {
    const check: ShortAnswer["check"] = draft.checkBy === "Accepted"
      ? { Accepted: { answers: draft.accepted.map((answer) => answer.trim()).filter(Boolean), typos: draft.typos } }
      : { Keywords: { keywords: draft.keywords.filter((keyword) => keyword.word.trim()).map((keyword) => ({ word: keyword.word.trim(), also: keyword.also.split(",").map((form) => form.trim()).filter(Boolean) })), needed: draft.needed } };
    return { check, model_answer: draft.model.trim() };
  }

  // R82: "Try an answer" asks the backend, so it's marked exactly as a quiz would.
  let tryText = $state("");
  let tried = $state<Marking | null>(null);
  $effect(() => {
    const text = tryText.trim();
    if (draft.kind !== "short" || !text) { tried = null; return; }
    const check = shortAnswer().check;
    api.tryShortAnswer(check, text).then((marking) => { if (tryText.trim() === text) tried = marking; }).catch(() => (tried = null));
  });
  const keywordCount = $derived(draft.keywords.filter((keyword) => keyword.word.trim()).length);

  function input(): QuestionInput {
    const short = draft.kind === "short";
    return {
      topic,
      text: draft.text.trim(),
      multiple: short ? false : draft.multiple,
      options: short ? [] : draft.options.filter((option) => option.text.trim() !== "").map((option) => ({ text: option.text.trim(), correct: option.correct })),
      explanation: draft.explanation.trim(),
      tags: draft.tags.split(",").map((tag) => tag.trim()).filter(Boolean),
      citations: draft.citations.filter((citation) => citation.source !== "").map((citation) => ({ source: Number(citation.source), location: citation.location.trim() })),
      short: short ? shortAnswer() : null,
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
      const kind = draft.kind;
      draft = { ...blank(), kind };
      tryText = "";
      await load();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  // R185: deleting the question being edited, after asking. Past quiz results keep their own copy.
  let deletingQuestion = $state(false);

  async function removeQuestion() {
    if (editing === null) return;
    const id = editing;
    deletingQuestion = false;
    try {
      await api.deleteQuestion(id);
      questions = await api.questionsIn(topic);
      startNew();
      toast({ text: "Question deleted", icon: "trash" });
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
      ...shortFields(question.short),
    };
    if (question.short) draft.options = blank().options;
    tryText = "";
  }

  function shortFields(short: ShortAnswer | null): Pick<Draft, "kind" | "checkBy" | "accepted" | "typos" | "keywords" | "needed" | "model"> {
    const empty = blank();
    if (!short) return { kind: "mcq", checkBy: empty.checkBy, accepted: empty.accepted, typos: empty.typos, keywords: empty.keywords, needed: empty.needed, model: empty.model };
    if ("Accepted" in short.check) {
      return { kind: "short", checkBy: "Accepted", accepted: [...short.check.Accepted.answers], typos: short.check.Accepted.typos, keywords: empty.keywords, needed: null, model: short.model_answer };
    }
    const { keywords, needed } = short.check.Keywords;
    return { kind: "short", checkBy: "Keywords", accepted: empty.accepted, typos: true, keywords: keywords.map((keyword) => ({ word: keyword.word, also: keyword.also.join(", ") })), needed, model: short.model_answer };
  }

  // R7 / questions.feature "Add an image by pasting, dropping or picking" (shared with cards).
  let questionField = $state<HTMLTextAreaElement>();
  let picker = $state<HTMLInputElement>();
  const images = imageInput(() => questionField);
  $effect(() => listenForDrops(images));

  function onPicked() {
    images.add([...(picker?.files ?? [])]);
    if (picker) picker.value = "";
  }

  /** Only preview when there's something Markdown can't show as plain text. */
  // "Kirk (2022)": the first author's surname and the year, for the preview to name the source.
  const shortCite = (source: SourceRow) => {
    const first = source.fields.authors.split(/[,;&]| and /)[0].trim() || source.fields.title;
    return source.fields.year ? `${first} (${source.fields.year})` : first;
  };
  const worthPreviewing = (text: string) => /image:\d|```|\$[^$]+\$|!\[/.test(text);

  function startNew() {
    editing = null;
    draft = { ...blank(), kind: draft.kind };
    tryText = "";
    error = "";
    images.clear();
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

    <form class="max-w-2xl space-y-8 px-10 pb-16" onsubmit={save} onpaste={images.onPaste} onfocusin={images.onFocus} ondragover={images.onDragOver} ondrop={images.onDrop}>
      <div class="flex items-center justify-between gap-4">
        <span class="text-sm font-semibold" id="type-label">Question type</span>
        <div class="seg" role="radiogroup" aria-labelledby="type-label">
          <button type="button" role="radio" aria-checked={draft.kind === "mcq"} onclick={() => (draft.kind = "mcq")}>Multiple choice</button>
          <button type="button" role="radio" aria-checked={draft.kind === "short"} onclick={() => (draft.kind = "short")}>Short answer</button>
        </div>
      </div>
      <div class="space-y-2">
        <label class="text-sm font-semibold" for="question-text">Question</label>
        <textarea id="question-text" class="textarea w-full font-mono text-sm" rows="3" data-rich="true" bind:this={questionField} bind:value={draft.text}></textarea>
        <div class="flex flex-wrap items-center justify-between gap-2">
          <p class="text-sm text-muted">Markdown, `code`, $LaTeX$ and images. Paste or drop an image into any field, or add one from a file.</p>
          <button type="button" class="btn btn-ghost btn-sm" title="Adds the image where your cursor was last, in the question, an option or the explanation" onclick={() => picker?.click()}><ImagePlus class="size-4" />Add image</button>
          <input bind:this={picker} type="file" class="hidden" accept={IMAGE_TYPES.join(",")} multiple aria-label="Image file" onchange={onPicked} />
        </div>
        {#if images.note}<p role="status" class="text-sm text-muted">{images.note}</p>{/if}{#if images.error}<p role="alert" class="text-sm text-error">{images.error}</p>{/if}
        {#if worthPreviewing(draft.text)}<div class="panel p-4" aria-label="Question preview"><Rich text={draft.text} class="max-w-prose" /></div>{/if}
      </div>

      {#if draft.kind === "mcq"}
      <fieldset class="space-y-3">
        <div class="flex items-center justify-between">
          <legend class="text-sm font-semibold">Answers</legend>
          <div class="seg" role="radiogroup" aria-label="Answer type">
            <button type="button" role="radio" aria-checked={!draft.multiple} onclick={() => setMultiple(false)}>Single</button>
            <button type="button" role="radio" aria-checked={draft.multiple} onclick={() => setMultiple(true)}>Multiple</button>
          </div>
        </div>
        <div class="option-grid text-xs text-muted" aria-hidden="true"><span>Answer</span><span class="text-center">Correct</span><span></span></div>
        {#each draft.options as option, index (index)}
          <div class="option-grid">
            <input class="input w-full {option.correct ? 'is-right' : ''}" aria-label="Option {index + 1}" placeholder="Option {index + 1}" data-rich="true" bind:value={option.text} />
            <input type="checkbox" class="checkbox checkbox-correct justify-self-center" aria-label="Option {index + 1} is correct" checked={option.correct} onchange={() => toggleCorrect(index)} />
            <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Remove option {index + 1}" disabled={draft.options.length <= MIN} onclick={() => (draft.options = draft.options.filter((_, position) => position !== index))}><X class="size-4" /></button>
          </div>
        {/each}
        <div class="flex items-center gap-3">
          <button type="button" class="btn btn-ghost btn-sm" disabled={draft.options.length >= MAX} onclick={() => (draft.options = [...draft.options, { text: "", correct: false }])}><Plus class="size-4" />Add option</button>
          {#if switchNote}<span role="status" class="block text-sm text-muted">{switchNote}</span>{/if}
          <span class="text-sm text-muted tabular-nums">{draft.options.length} of {MAX} options. {draft.multiple ? "Tick every correct answer." : "Tick the correct answer."}</span>
        </div>
      </fieldset>
      {:else}
        <fieldset class="space-y-3">
          <div class="flex items-center justify-between gap-4">
            <legend class="text-sm font-semibold">How it's checked</legend>
            <div class="seg" role="radiogroup" aria-label="How it's checked">
              <button type="button" role="radio" aria-checked={draft.checkBy === "Accepted"} onclick={() => (draft.checkBy = "Accepted")}>Accepted answers</button>
              <button type="button" role="radio" aria-checked={draft.checkBy === "Keywords"} onclick={() => (draft.checkBy = "Keywords")}>Keywords</button>
            </div>
          </div>
          {#if draft.checkBy === "Accepted"}
            <p class="text-sm text-muted">Right when your answer matches any of these. Case, extra spaces, punctuation and "the" or "a" at the start don't matter.</p>
            {#each draft.accepted as _, index (index)}
              <div class="flex items-center gap-3">
                <input class="input input-sm flex-1" aria-label="Accepted answer {index + 1}" placeholder="e.g. Transport layer…" bind:value={draft.accepted[index]} />
                <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Remove accepted answer {index + 1}" disabled={draft.accepted.length <= 1} onclick={() => draft.accepted.splice(index, 1)}><X class="size-4" /></button>
              </div>
            {/each}
            <div class="flex items-center justify-between gap-3">
              <button type="button" class="btn btn-ghost btn-sm" onclick={() => draft.accepted.push("")}><Plus class="size-4" />Add another accepted answer</button>
              <label class="flex min-h-6 items-center gap-2 text-sm"><input type="checkbox" class="toggle toggle-sm" aria-describedby="typo-help" bind:checked={draft.typos} />Allow small typos</label>
            </div>
            <p id="typo-help" class="text-sm text-muted">One wrong letter in words of 4 to 7 letters, two from 8. Never in numbers or words of 3 letters or fewer.</p>
          {:else}
            <p class="text-sm text-muted">Right when your answer contains enough of these, in any order. Put other ways of writing a keyword after a comma. Each word you write counts once, so "SYN-ACK" doesn't also count as "SYN" and "ACK".</p>
            {#each draft.keywords as keyword, index (index)}
              <div class="flex items-center gap-3">
                <input class="input input-sm w-40" aria-label="Keyword {index + 1}" bind:value={keyword.word} />
                <input class="input input-sm flex-1" aria-label="Other ways to write keyword {index + 1}" placeholder="Also accept…" bind:value={keyword.also} />
                <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Remove keyword {index + 1}" disabled={draft.keywords.length <= 1} onclick={() => draft.keywords.splice(index, 1)}><X class="size-4" /></button>
              </div>
            {/each}
            <div class="flex items-center justify-between gap-3">
              <button type="button" class="btn btn-ghost btn-sm" onclick={() => draft.keywords.push({ word: "", also: "" })}><Plus class="size-4" />Add keyword</button>
              <label class="flex items-center gap-2 text-sm"><span class="text-muted">Right with</span>
                <select class="select select-sm w-36" aria-label="How many keywords are needed" value={draft.needed === null ? "" : String(draft.needed)} onchange={(event) => (draft.needed = event.currentTarget.value ? Number(event.currentTarget.value) : null)}>
                  <option value="">All {keywordCount || ""}</option>
                  {#each Array.from({ length: Math.max(0, keywordCount - 1) }, (_, i) => keywordCount - 1 - i) as count (count)}<option value={String(count)}>At least {count}</option>{/each}
                </select></label>
            </div>
          {/if}
        </fieldset>
        <div class="space-y-2">
          <label class="text-sm font-semibold" for="model-answer">Model answer <span class="font-normal text-muted">(optional)</span></label>
          <textarea id="model-answer" class="textarea w-full font-mono text-sm" rows="2" data-rich="true" placeholder="Shown after you answer…" bind:value={draft.model}></textarea>
          <p class="text-sm text-muted">What you see after answering, next to what you wrote. {draft.checkBy === "Accepted" ? "Leave it blank to show the first accepted answer." : "Keywords only decide right or wrong."}</p>
          {#if worthPreviewing(draft.model)}<div class="panel p-4" aria-label="Model answer preview"><Rich text={draft.model} class="max-w-prose" /></div>{/if}
        </div>
        <section class="panel p-5" aria-label="Try an answer">
          <p class="text-sm font-semibold">Try an answer</p>
          <p class="mt-1 text-sm text-muted">See how the app would mark something before you save. Nothing is recorded.</p>
          <input class="input input-sm mt-3 w-full" aria-label="A test answer" placeholder="Type an answer to test…" bind:value={tryText} />
          {#if tried}
            <div class="mt-3 flex flex-wrap items-center gap-2 text-sm" aria-live="polite">
              <span class="flex items-center gap-1 font-semibold {tried.right ? 'text-right-ink' : 'text-wrong-ink'}">{#if tried.right}<CircleCheck class="size-4" />{:else}<CircleX class="size-4" />{/if}{tried.right ? (typeof tried.detail === "object" && "Matched" in tried.detail && tried.detail.Matched.typo ? "Right, with a typo" : "Right") : "Wrong"}</span>
              {#if typeof tried.detail === "object" && "Matched" in tried.detail}<span class="text-muted">matches "{tried.detail.Matched.answer}"</span>
              {:else if typeof tried.detail === "object" && "Keywords" in tried.detail}
                <span class="text-muted">{tried.detail.Keywords.hit.length} of {tried.detail.Keywords.hit.length + tried.detail.Keywords.missed.length} keywords:</span>
                {#each tried.detail.Keywords.hit as word (word)}<span class="kw kw-hit"><Check class="size-3" />{word}</span>{/each}
                {#each tried.detail.Keywords.missed as word (word)}<span class="kw kw-miss"><X class="size-3" />{word}</span>{/each}
              {/if}
            </div>
          {/if}
        </section>
      {/if}

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
            {#if citation.source && locationTarget(citation.location)}
              {@const source = sources.find((row) => String(row.id) === citation.source)}
              {@const target = locationTarget(citation.location)!}
              <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Show {"line" in target ? `line ${target.line}` : `page ${target.page}`} in the preview" title="Show in the preview" onclick={() => source && showCitation(source.id, shortCite(source), citation.location)}><Eye class="size-4" /></button>
            {/if}
            <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Remove citation {index + 1}" onclick={() => (draft.citations = draft.citations.filter((_, position) => position !== index))}><X class="size-4" /></button>
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
        {#if editing !== null}<button type="button" class="btn btn-ghost ml-auto text-error" onclick={() => (deletingQuestion = true)}><Trash2 class="size-4" />Delete question</button>{/if}
      </div>
    </form>
  </section>
</div>

{#if deletingQuestion}
  <dialog open class="modal modal-open" aria-labelledby="delete-question-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-question-h" class="text-lg font-semibold">Delete this question?</h3>
      <p class="mt-2 text-sm">Past quiz results keep their answers. Recall points made from it stay, without the link.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deletingQuestion = false)}>Keep it</button>
        <button class="btn btn-error" onclick={removeQuestion}>Delete question</button>
      </div>
    </div>
  </dialog>
{/if}

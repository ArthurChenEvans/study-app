<script lang="ts">
  // transfer.feature (R112, R113). The dialog's goal: get the right things into one file.
  // Most important: what's going in, said in words before you press Export.
  // import-export.feature (R173 to R176): the kind of file, from a .studyapp to Anki or Moodle.
  import { FileOutput } from "@lucide/svelte";
  import { api, errorMessage, type ExportFile, type ExportFormat, type ExportSummary, type Id, type ModuleRow, type TopicRow } from "../lib/api";
  import { countList } from "../lib/transfer";

  const FORMATS: { id: ExportFormat; name: string; file: string; for: string }[] = [
    { id: "StudyApp", name: "Shudhana", file: ".studyapp", for: "Everything, to import here again or on another computer." },
    { id: "Anki", name: "Anki deck", file: ".apkg", for: "Cards and cloze, with images and maths. Opens in Anki and AnkiDroid." },
    { id: "QuestionsCsv", name: "Questions spreadsheet", file: ".csv", for: "Multiple choice and short answer questions, one per row." },
    { id: "CardsCsv", name: "Cards spreadsheet", file: ".csv", for: "Front, back and cloze cards, one per row." },
    { id: "Json", name: "JSON", file: ".json", for: "Questions, cards and checklists, for scripts and other apps." },
    { id: "Markdown", name: "Markdown checklists", file: ".md", for: "Recall checklists as ticked lists. Opens in Obsidian or any notes app." },
    { id: "Gift", name: "Moodle GIFT", file: ".txt", for: "Questions for a Moodle quiz, with feedback." },
    { id: "Aiken", name: "Moodle Aiken", file: ".txt", for: "Single answer questions only, the simplest Moodle format." },
  ];

  let { onClose, onDone }: { onClose: () => void; onDone: (file: ExportFile) => void } = $props();

  let everything = $state(true);
  let history = $state(true);
  let ticked = $state<Id[]>([]);
  let filter = $state("");
  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  let summary = $state<ExportSummary | null>(null);
  let busy = $state(false);
  let format = $state<ExportFormat>("StudyApp");
  let quizzesAsCards = $state(false);
  const keepsHistory = $derived(format === "StudyApp" || format === "Anki");
  let error = $state("");

  Promise.all([api.topics(), api.modules()]).then(([allTopics, allModules]) => {
    topics = allTopics;
    modules = allModules;
  });

  // A .studyapp request stays as it always was, so older versions read the same thing.
  const request = $derived({
    topics: everything ? null : [...ticked].sort((a, b) => a - b),
    history: keepsHistory && history,
    ...(format === "StudyApp" ? {} : { format }),
    ...(format === "Anki" ? { quizzes_as_cards: quizzesAsCards } : {}),
  });
  const ready = $derived(everything || ticked.length > 0);

  // Says what's going in whenever the choice changes.
  let asked = 0;
  $effect(() => {
    const wanted = request;
    if (!ready) {
      summary = null;
      return;
    }
    const mine = ++asked;
    api.exportSummary(wanted).then((next) => { if (mine === asked) summary = next; }, () => {});
  });

  function choose(all: boolean) {
    everything = all;
    // Sharing leaves your history out unless you say otherwise.
    history = all;
  }

  const shown = $derived(topics.filter((topic) => topic.name.toLowerCase().includes(filter.trim().toLowerCase())));
  const groups = $derived([...modules.map((module) => ({ module: module as ModuleRow | null, topics: shown.filter((topic) => topic.module === module.id) })), { module: null, topics: shown.filter((topic) => topic.module === null) }].filter((group) => group.topics.length));
  const inModule = (module: ModuleRow | null) => topics.filter((topic) => topic.module === (module?.id ?? null)).map((topic) => topic.id);
  const allTicked = (ids: Id[]) => ids.length > 0 && ids.every((id) => ticked.includes(id));

  function toggle(id: Id) {
    ticked = ticked.includes(id) ? ticked.filter((other) => other !== id) : [...ticked, id];
  }
  function toggleModule(module: ModuleRow | null) {
    const ids = inModule(module);
    ticked = allTicked(ids) ? ticked.filter((id) => !ids.includes(id)) : [...new Set([...ticked, ...ids])];
  }

  async function save() {
    busy = true;
    error = "";
    try {
      onDone(await api.exportFile(request));
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  const label = $derived(everything ? "Export everything" : `Export ${ticked.length || ""} ${ticked.length === 1 ? "topic" : "topics"}`.replace("  ", " "));
</script>

<dialog open class="modal modal-open" aria-labelledby="export-h">
  <div class="modal-box max-w-xl">
    <h3 id="export-h" class="text-lg font-semibold">Export</h3>
    <label class="mt-5 block">
      <span class="text-sm font-semibold">Kind of file</span>
      <select class="select mt-2 w-full" bind:value={format} aria-label="Kind of file">
        {#each FORMATS as option (option.id)}<option value={option.id}>{option.name} ({option.file})</option>{/each}
      </select>
      <span class="mt-1 block text-sm text-muted">{FORMATS.find((option) => option.id === format)?.for}</span>
    </label>
    <fieldset class="mt-5">
      <legend class="text-sm font-semibold">What to export</legend>
      <label class="mt-2 flex items-start gap-3"><input type="radio" name="export-what" class="radio mt-0.5" checked={everything} onchange={() => choose(true)} /><span><span class="block text-[15px]">Everything</span><span class="block text-sm text-muted">To move to another computer, or keep a copy somewhere else.</span></span></label>
      <label class="mt-3 flex items-start gap-3"><input type="radio" name="export-what" class="radio mt-0.5" checked={!everything} onchange={() => choose(false)} /><span><span class="block text-[15px]">Some topics, to share</span><span class="block text-sm text-muted">Their questions, cards, checklists, sources and images.</span></span></label>
    </fieldset>

    {#if !everything}
      <div class="mt-4 ml-8">
        <input type="search" class="input input-sm w-full" placeholder="Filter topics…" aria-label="Filter topics" bind:value={filter} />
        <div class="mt-2 max-h-64 overflow-y-auto rounded-lg border border-base-300">
          {#each groups as group (group.module?.id ?? "none")}
            {@const ids = inModule(group.module)}
            <label class="flex items-center gap-3 border-b border-base-300 bg-base-200 px-3 py-2 text-sm font-semibold">
              <input type="checkbox" class="checkbox" aria-label="All of {group.module?.name ?? 'No module'}" checked={allTicked(ids)} onchange={() => toggleModule(group.module)} />
              {group.module?.name ?? "No module"}<span class="font-normal text-muted">{ids.filter((id) => ticked.includes(id)).length} of {ids.length}</span>
            </label>
            {#each group.topics as topic (topic.id)}
              <label class="flex items-center gap-3 px-3 py-1.5 pl-9 text-[15px] hover:bg-base-200">
                <input type="checkbox" class="checkbox" aria-label={topic.name} checked={ticked.includes(topic.id)} onchange={() => toggle(topic.id)} />
                <span class="flex-1">{topic.name}</span><span class="text-xs text-muted tabular-nums">{topic.question_count} questions</span>
              </label>
            {/each}
          {:else}
            <p class="px-3 py-4 text-sm text-muted">No topics match.</p>
          {/each}
        </div>
        <p class="mt-1 text-sm text-muted">{ticked.length} {ticked.length === 1 ? "topic" : "topics"} ticked</p>
      </div>
    {/if}

    {#if format === "Anki"}
      <label class="mt-5 flex items-start gap-3"><input type="checkbox" class="checkbox mt-0.5" bind:checked={quizzesAsCards} /><span><span class="block text-sm font-semibold">Quiz questions as cards too</span><span class="block text-sm text-muted">Each question becomes a card with its answer on the back.</span></span></label>
      <label class="mt-3 flex items-start gap-3"><input type="checkbox" class="checkbox mt-0.5" bind:checked={history} /><span><span class="block text-sm font-semibold">Keep my progress</span><span class="block text-sm text-muted">Cards keep when they're due. Leave it off when sharing, so your friend starts fresh.</span></span></label>
    {:else if format === "StudyApp"}
      <label class="mt-5 flex items-start gap-3"><input type="checkbox" class="checkbox mt-0.5" bind:checked={history} /><span><span class="block text-sm font-semibold">Include my history</span><span class="block text-sm text-muted">{everything ? "Reviews, quiz results, recall checks, practice logs and schedules." : "Card reviews, recall checks and quiz steps. Leave it off when sharing, so your friend starts fresh."}</span></span></label>
    {:else}
      <p class="mt-5 text-sm text-muted">Your history stays here. Only {format === "Markdown" ? "checklists" : format.includes("Cards") ? "cards" : format === "Json" ? "questions, cards and checklists" : "questions"} go in.</p>
    {/if}

    <p class="mt-5 min-h-6 text-sm text-muted" aria-live="polite">
      {#if !ready}Tick the topics to share.{:else if summary}Saves {summary.name}: {countList(summary.counts)}.{/if}
    </p>
    {#if error}<p role="alert" class="mt-2 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      <button class="btn btn-ghost" onclick={onClose}>Cancel</button>
      <button class="btn btn-primary" disabled={!ready || busy} onclick={save}><FileOutput class="size-4" />{busy ? "Exporting…" : label}</button>
    </div>
  </div>
</dialog>

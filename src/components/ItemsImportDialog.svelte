<script lang="ts">
  // import-export.feature: Spreadsheets, JSON, Markdown and Moodle files (R173, R174).
  import { onMount } from "svelte";
  import { api, type CsvKind, type CsvRole, type ItemsPreview, type ModuleRow, type Target, type TopicRow } from "../lib/api";
  import { cancelImport, importer, remap, runImport } from "../lib/importer.svelte";
  import TargetSelect from "./TargetSelect.svelte";

  let { preview }: { preview: ItemsPreview } = $props();

  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  let showProblems = $state(false);
  let targets = $state<Target[]>([]);
  $effect.pre(() => {
    targets = preview.topics.map((row) => row.target);
  });

  onMount(() => {
    Promise.resolve(api.topics()).then((rows) => (topics = rows ?? [])).catch(() => {});
    Promise.resolve(api.modules()).then((rows) => (modules = rows ?? [])).catch(() => {});
  });

  const KINDS: [CsvKind, string][] = [["Mcq", "A multiple choice question"], ["Short", "A short answer question"], ["Card", "A flashcard"]];
  const SHARED: [CsvRole, string][] = [["Topic", "Topic"], ["Module", "Module"], ["Tags", "Tags"], ["Ignore", "Ignore it"]];
  const ROLES: Record<CsvKind, [CsvRole, string][]> = {
    Mcq: [["Question", "Question"], ["Option", "An option"], ["Correct", "Correct answer"], ["Explanation", "Explanation"], ["Type", "Single or multiple"]],
    Short: [["Question", "Question"], ["ModelAnswer", "Model answer"], ["Keywords", "Keywords"], ["Explanation", "Explanation"]],
    Card: [["Front", "Front"], ["Back", "Back"], ["Text", "Cloze text"], ["Extra", "Extra"], ["CardKind", "Card type"]],
  };
  const FORMATS = { csv: "A spreadsheet", json: "A JSON file", markdown: "Markdown checklists", gift: "Moodle GIFT questions", aiken: "Moodle Aiken questions" };

  const plural = (count: number, word: string) => `${count.toLocaleString("en-GB")} ${word}${count === 1 ? "" : "s"}`;
  const csv = $derived(preview.csv);
  const chosen = $derived(preview.topics.filter((_, index) => targets[index]?.kind !== "Skip"));
  const counts = $derived(chosen.reduce((sum, row) => ({ questions: sum.questions + row.questions, cards: sum.cards + row.cards, checklists: sum.checklists + row.checklists }), { questions: 0, cards: 0, checklists: 0 }));
  const what = $derived.by(() => {
    const parts = [counts.questions ? plural(counts.questions, "question") : "", counts.cards ? plural(counts.cards, "card") : "", counts.checklists ? plural(counts.checklists, "recall checklist") : ""].filter(Boolean);
    return parts.length === 0 ? "" : parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(", ")} and ${parts.at(-1)}`;
  });
  const summary = $derived(csv ? `A spreadsheet with ${plural(csv.total_rows, "row")} and ${plural(csv.columns.length, "column")}.` : `${FORMATS[preview.format]} with ${what || "nothing to import"}.`);

  function setRole(index: number, role: CsvRole) {
    if (!csv) return;
    void remap({ kind: csv.kind, header: csv.header, roles: csv.roles.map((old, at) => (at === index ? role : old)) });
  }

  function go() {
    const choice = { targets: preview.topics.map((row, index): [string, Target] => [row.key, $state.snapshot(targets[index])]) };
    void runImport(preview.file_name, () => api.itemsApply(choice));
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="items-h">
  <div class="modal-box max-w-3xl">
    <h2 id="items-h" class="text-xl font-black">Import <span class="path break-all">{preview.file_name}</span></h2>
    <p class="mt-1 text-[15px] text-muted">{summary}</p>

    {#if showProblems}
      <h3 class="mt-5 text-lg font-bold">{preview.problems.length === 1 ? "1 row has problems" : `${preview.problems.length} rows have problems`}</h3>
      <p class="mt-1 text-sm text-muted">These are left out. Everything else imports.</p>
      <ul class="mt-3 space-y-3">
        {#each preview.problems as problem, index (index)}
          <li class="rounded-box border-2 border-base-300 p-3">
            <div class="flex flex-wrap gap-2"><span class="font-bold">{problem.at}</span><span class="min-w-0 truncate text-muted">{problem.text}</span></div>
            <p class="mt-1 text-[15px]">{problem.why}</p>
          </li>
        {/each}
      </ul>
      <div class="modal-action"><button type="button" class="btn btn-ghost" onclick={() => (showProblems = false)}>Back to the columns</button></div>
    {:else}
      {#if csv}
        <fieldset class="mt-5">
          <legend class="text-sm font-bold">Each row is</legend>
          <div class="mt-2 flex flex-wrap gap-x-5 gap-y-1">
            {#each KINDS as [kind, label] (kind)}
              <label class="flex items-center gap-2"><input type="radio" class="radio radio-sm" name="csv-kind" checked={csv.kind === kind} onchange={() => void remap({ kind, header: csv.header, roles: csv.roles })} /> {label}</label>
            {/each}
          </div>
        </fieldset>
        <label class="mt-3 flex items-center gap-2 text-[15px]"><input type="checkbox" class="checkbox checkbox-sm" checked={csv.header} onchange={(event) => void remap({ kind: csv.kind, header: (event.target as HTMLInputElement).checked, roles: csv.roles })} /> The first row is column names</label>

        <div class="mt-4 overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                {#each csv.columns as column, index (index)}
                  <th class="min-w-36 align-top">
                    <span class="block truncate font-semibold normal-case">{column}</span>
                    <select class="select select-sm mt-1 w-full" aria-label="Column {index + 1} is" value={csv.roles[index]} onchange={(event) => setRole(index, (event.target as HTMLSelectElement).value as CsvRole)}>
                      {#each [...ROLES[csv.kind], ...SHARED] as [role, label] (role)}<option value={role}>{label}</option>{/each}
                    </select>
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each csv.sample as row, index (index)}
                <tr>{#each row as cell, at (at)}<td class="max-w-48 truncate">{cell}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if preview.problems.length}
        <div class="mt-4 flex flex-wrap items-center gap-3 rounded-box bg-warning/15 p-3 text-[15px]">
          <span class="flex-1">{preview.problems.length === 1 ? "1 row has a problem and is left out." : `${preview.problems.length} rows have problems and are left out.`}</span>
          <button type="button" class="btn btn-sm" onclick={() => (showProblems = true)}>See which</button>
        </div>
      {/if}

      <ul class="mt-5 space-y-2">
        {#each preview.topics as row, index (row.key)}
          <li class="grid grid-cols-1 items-center gap-2 sm:grid-cols-[1fr_minmax(0,16rem)]">
            <span class="min-w-0 truncate font-semibold">{preview.topics.length === 1 ? "Into" : row.name}</span>
            {#if targets[index]}
              <TargetSelect label={preview.topics.length === 1 ? "Into" : `Where ${row.name} goes`} bind:target={targets[index]} fresh={row.target} {topics} {modules} skip={preview.topics.length > 1} />
            {/if}
          </li>
        {/each}
      </ul>

      {#if preview.already_here}
        <p class="mt-4 text-sm text-muted">{preview.already_here === 1 ? "1 is" : `${preview.already_here} are`} already here from an earlier import, and stay as they are.</p>
      {/if}

      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={cancelImport} disabled={importer.busy}>Cancel</button>
        <button type="button" class="btn btn-primary" onclick={go} disabled={importer.busy || !what}>Import {what || "nothing"}</button>
      </div>
    {/if}
  </div>
</dialog>

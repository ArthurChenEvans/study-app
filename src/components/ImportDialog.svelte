<script lang="ts">
  // transfer.feature (R114 to R117). The dialog's goal: decide what comes in before
  // anything changes. Most important: how much is new and what clashes, in a table a
  // screen reader can read cell by cell.
  import { FileInput, ShieldCheck } from "@lucide/svelte";
  import { api, type ClashRule, type ImportPreview } from "../lib/api";
  import { runImport } from "../lib/importer.svelte";
  import { fullLabel } from "../lib/dates";

  let { preview, today, onClose, onDone }: { preview: ImportPreview; today: number; onClose: () => void; onDone: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let rule = $state<ClashRule>(preview.suggested);
  // svelte-ignore state_referenced_locally
  let theirs = $state<string[]>(preview.clashes.filter((clash) => clash.theirs_newer).map((clash) => clash.uid));
  let busy = $state(false);
  let error = $state("");

  const nothingNew = $derived(preview.rows.every((row) => row.new === 0) && preview.clashes.length === 0);
  const what = $derived(`${preview.everything ? "Everything" : "Some topics"}, ${preview.history ? "with history" : "no history"} · exported ${fullLabel(preview.exported, today)}${preview.from_here ? " from this computer" : ""}`);
  const count = (n: number) => `${n} ${n === 1 ? "thing" : "things"}`;

  function pick(uid: string, take: boolean) {
    theirs = take ? [...new Set([...theirs, uid])] : theirs.filter((other) => other !== uid);
  }

  async function bring() {
    busy = true;
    error = "";
    // The same ending as every import: a toast with Undo (R177).
    if (await runImport(preview.file_name, () => api.importApply({ rule, theirs: rule === "EachOne" ? theirs : [] }))) onDone();
    else busy = false;
  }

  const RULES: [ClashRule, string, string][] = [
    ["Newer", "Use whichever changed last", "Here or in the file, the more recent edit wins."],
    ["KeepMine", "Keep mine", "Only new things come in."],
    ["TakeTheirs", "Take the file's", "Your history on them stays."],
    ["EachOne", "Look at each one", ""],
  ];
</script>

<dialog open class="modal modal-open" aria-labelledby="import-h">
  <div class="modal-box max-w-2xl">
    <h3 id="import-h" class="text-lg font-semibold">Import {preview.file_name}</h3>
    <p class="mt-1 text-sm text-muted">{what}</p>

    <!-- A full export has a row for every kind of record, so the table scrolls on its own
         and the Import button stays in view. -->
    <div class="mt-5 max-h-[40vh] overflow-y-auto rounded-lg border border-base-300">
      <table class="table table-sm">
        <thead class="sticky top-0 bg-[var(--surface)]"><tr class="text-muted"><th>What</th><th class="text-right">New</th><th class="text-right">Already here</th><th class="text-right">Changed since</th></tr></thead>
        <tbody>
          {#each preview.rows as row (row.what)}
            <tr><td>{row.what}</td><td class="text-right tabular-nums {row.new ? 'font-semibold' : ''}">{row.new}</td><td class="text-right tabular-nums text-muted">{row.same}</td><td class="text-right tabular-nums">{row.changed}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if nothingNew}
      <p class="mt-4 text-[15px]">Everything in this file is here already.</p>
    {:else if preview.clashes.length}
      <fieldset class="mt-5">
        <legend class="text-sm font-semibold">For the {count(preview.clashes.length)} changed in both places</legend>
        {#each RULES as [value, label, hint] (value)}
          <label class="mt-2 flex items-start gap-3"><input type="radio" name="import-rule" class="radio mt-0.5" checked={rule === value} onchange={() => (rule = value)} /><span><span class="block text-[15px]">{label}</span>{#if hint}<span class="block text-sm text-muted">{hint}</span>{/if}</span></label>
        {/each}
      </fieldset>
      {#if rule === "EachOne"}
        <ul class="mt-3 max-h-72 divide-y overflow-y-auto rounded-lg border border-base-300" aria-label="Changed in both places">
          {#each preview.clashes as clash (clash.uid)}
            <li class="px-4 py-3">
              <p class="text-xs text-muted">{clash.what}</p>
              <p class="text-[15px]">{clash.label || "(untitled)"}</p>
              <p class="text-sm text-muted">Changed here {fullLabel(clash.mine_day, today)}, in the file {fullLabel(clash.theirs_day, today)}</p>
              <div class="mt-2 flex gap-4 text-sm" role="radiogroup" aria-label="Which version of {clash.label}">
                <label class="flex items-center gap-2"><input type="radio" class="radio" name="clash-{clash.uid}" checked={!theirs.includes(clash.uid)} onchange={() => pick(clash.uid, false)} />{clash.theirs_newer ? "Mine" : "Mine, newer"}</label>
                <label class="flex items-center gap-2"><input type="radio" class="radio" name="clash-{clash.uid}" checked={theirs.includes(clash.uid)} onchange={() => pick(clash.uid, true)} />{clash.theirs_newer ? "The file's, newer" : "The file's"}</label>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    {#if !nothingNew}
      <p class="mt-4 flex gap-2 text-sm text-muted"><ShieldCheck class="mt-0.5 size-4 shrink-0" /><span>A backup is made first.</span></p>
      <p class="mt-1 text-sm text-muted">Reviews and quizzes are added, never replaced. Nothing here is deleted.</p>
    {/if}
    {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      {#if nothingNew}
        <button class="btn btn-primary" onclick={onClose}>Close</button>
      {:else}
        <button class="btn btn-ghost" onclick={onClose}>Cancel</button>
        <button class="btn btn-primary" disabled={busy} onclick={bring}><FileInput class="size-4" />{busy ? "Importing…" : "Import"}</button>
      {/if}
    </div>
  </div>
</dialog>

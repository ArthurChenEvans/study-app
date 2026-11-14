<script lang="ts">
  // import-export.feature: Anki decks (R171, R172). Each deck becomes a topic, or goes into one.
  import { onMount } from "svelte";
  import { api, type AnkiPreview, type ModuleRow, type Target, type TopicRow } from "../lib/api";
  import { cancelImport, importer, runImport } from "../lib/importer.svelte";
  import TargetSelect from "./TargetSelect.svelte";

  let { preview }: { preview: AnkiPreview } = $props();

  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  // svelte-ignore state_referenced_locally
  let targets = $state<Target[]>(preview.decks.map((deck) => deck.target));
  let keep = $state(false);
  // R188: a deck kept the Michael Nielsen way is one deck, filtered by tags.
  let byTag = $state(false);
  let picked = $state<number[]>([]);
  let find = $state("");
  let keepSuspended = $state(false);
  const shownTags = $derived(preview.tags.map((tag, index) => ({ ...tag, index })).filter((tag) => tag.name.toLowerCase().includes(find.trim().toLowerCase())));
  const toggle = (index: number) => (picked = picked.includes(index) ? picked.filter((one) => one !== index) : [...picked, index]);

  onMount(() => {
    Promise.resolve(api.topics()).then((rows) => (topics = rows ?? [])).catch(() => {});
    Promise.resolve(api.modules()).then((rows) => (modules = rows ?? [])).catch(() => {});
  });

  const plural = (count: number, word: string) => `${count.toLocaleString("en-GB")} ${word}${count === 1 ? "" : "s"}`;
  const kind = $derived(preview.file_name.toLowerCase().endsWith(".colpkg") ? "An Anki collection" : "An Anki deck");
  const extras = $derived([preview.images ? plural(preview.images, "image") : ""].filter(Boolean));
  const chosen = $derived(
    preview.rows.filter((row) => targets[row.deck]?.kind !== "Skip" && !row.here && (keepSuspended || !row.suspended) && (!byTag || row.tags.some((tag) => picked.includes(tag)))).length,
  );

  function go() {
    const choice = { decks: preview.decks.map((deck, index): [string, Target] => [deck.name, $state.snapshot(targets[index])]), keep_progress: keep, tags: byTag ? picked.map((index) => preview.tags[index].name) : [], keep_suspended: keepSuspended };
    void runImport(preview.file_name, () => api.ankiApply(choice));
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="anki-h">
  <div class="modal-box max-w-2xl">
    <h2 id="anki-h" class="text-xl font-black">Import <span class="path break-all">{preview.file_name}</span></h2>
    <p class="mt-1 text-[15px] text-muted">{kind}: {plural(preview.notes, "note")} in {plural(preview.decks.length, "deck")}{#if extras.length}, with {extras.join(" and ")}{/if}.</p>

    <h3 class="mt-5 text-sm font-bold">Where each deck goes</h3>
    <ul class="mt-2 space-y-2">
      {#each preview.decks as deck, index (deck.name)}
        <li class="grid grid-cols-1 items-center gap-2 sm:grid-cols-[1fr_minmax(0,16rem)]">
          <div class="min-w-0"><span class="block truncate font-semibold">{deck.name}</span><span class="text-sm text-muted">{plural(deck.notes, "note")}</span></div>
          <TargetSelect label="Where {deck.name} goes" bind:target={targets[index]} fresh={deck.target} {topics} {modules} />
        </li>
      {/each}
    </ul>

    {#if preview.tags.length}
      <fieldset class="mt-5">
        <legend class="text-sm font-bold">Tags</legend>
        <label class="mt-2 flex items-center gap-2"><input type="radio" class="radio" name="anki-tags" checked={!byTag} onchange={() => (byTag = false)} /> Every note</label>
        <label class="mt-1 flex items-center gap-2"><input type="radio" class="radio" name="anki-tags" checked={byTag} onchange={() => (byTag = true)} /> Only notes with these tags</label>
        {#if byTag}
          {#if preview.tags.length > 8}
            <input type="search" class="input mt-2 w-full" aria-label="Find a tag" placeholder="Find a tag..." bind:value={find} />
          {/if}
          <ul class="mt-2 max-h-56 space-y-1 overflow-y-auto">
            {#each shownTags as tag (tag.index)}
              {@const depth = tag.name.split("::").length - 1}
              <li style:padding-left="{depth * 1.25}rem">
                <label class="flex items-center gap-2">
                  <input type="checkbox" class="checkbox checkbox-sm" aria-label="{tag.name}, {plural(tag.notes, 'note')}" checked={picked.includes(tag.index)} onchange={() => toggle(tag.index)} />
                  <span class="font-semibold">{tag.name.split("::").at(-1)}</span><span class="text-sm text-muted">{plural(tag.notes, "note")}</span>
                </label>
              </li>
            {/each}
          </ul>
          <p class="mt-1 text-sm text-muted">A tag brings the tags under it too.</p>
        {/if}
      </fieldset>
    {/if}

    {#if preview.suspended}
      <fieldset class="mt-5">
        <legend class="text-sm font-bold">Suspended cards</legend>
        <p class="mt-1 text-sm text-muted">{plural(preview.suspended, "card")} {preview.suspended === 1 ? "is" : "are"} suspended in Anki.</p>
        <label class="mt-2 flex items-center gap-2"><input type="radio" class="radio" name="anki-suspended" checked={!keepSuspended} onchange={() => (keepSuspended = false)} /> Leave them out</label>
        <label class="mt-1 flex items-center gap-2"><input type="radio" class="radio" name="anki-suspended" checked={keepSuspended} onchange={() => (keepSuspended = true)} /> Keep them, paused</label>
      </fieldset>
    {/if}

    <h3 class="mt-5 text-sm font-bold">Card types</h3>
    <ul class="mt-2 space-y-1 text-[15px]">
      {#each preview.types as type (type.name)}
        <li class="flex flex-wrap items-center gap-2">
          <span class="font-semibold">{type.name}</span><span class="text-muted">{plural(type.notes, "note")}, become {type.becomes} cards</span>
          {#if type.check}<span class="badge badge-warning badge-sm">Check these</span>{/if}
        </li>
      {/each}
    </ul>

    {#if preview.sounds}
      <p class="mt-3 text-sm text-muted">{plural(preview.sounds, "sound")} {preview.sounds === 1 ? "is" : "are"} left out. Shudhana doesn't play sounds yet.</p>
    {/if}

    {#if preview.has_progress}
      <fieldset class="mt-5">
        <legend class="text-sm font-bold">Progress</legend>
        <label class="mt-2 flex items-center gap-2"><input type="radio" class="radio" name="anki-progress" checked={!keep} onchange={() => (keep = false)} /> Start fresh</label>
        <label class="mt-1 flex items-center gap-2"><input type="radio" class="radio" name="anki-progress" checked={keep} onchange={() => (keep = true)} /> Keep Anki's progress</label>
      </fieldset>
    {/if}

    {#if preview.already_here}
      <p class="mt-4 text-sm text-muted">{preview.already_here === 1 ? "1 note is" : `${preview.already_here} notes are`} already here from an earlier import, and stay as they are.</p>
    {/if}

    <div class="modal-action">
      <button type="button" class="btn btn-ghost" onclick={cancelImport} disabled={importer.busy}>Cancel</button>
      <button type="button" class="btn btn-primary" onclick={go} disabled={importer.busy || chosen === 0}>Import {plural(chosen, "note")}</button>
    </div>
  </div>
</dialog>

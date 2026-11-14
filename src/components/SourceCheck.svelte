<script lang="ts">
  // sources.feature (R202): check a source filled in from a file or page. Each field says
  // whether it came from the file, so you know which ones to look at.
  import type { SourceFields } from "../lib/api";
  import { apaPreview, type FieldName, type FoundDetails } from "../lib/fileDetails";

  let { found, keepLabel = "Save source", discardLabel = "Don't keep it", onSave, onDiscard, onClose }: {
    found: FoundDetails;
    keepLabel?: string;
    discardLabel?: string;
    onSave: (fields: SourceFields) => void;
    onDiscard: () => void;
    onClose: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let fields = $state<SourceFields>({ ...found.fields });
  const rows: { name: FieldName; label: string; placeholder: string }[] = [
    { name: "authors", label: "Authors", placeholder: "Surname, I. and Surname, I." },
    { name: "year", label: "Year", placeholder: "e.g. 2026" },
    { name: "title", label: "Title", placeholder: "" },
    { name: "description", label: "Kind", placeholder: "e.g. Lecture slides" },
    { name: "publisher", label: "Publisher or site", placeholder: "" },
    { name: "url", label: "Web address", placeholder: "https://…" },
  ];
  const text = (name: FieldName) => fields[name] ?? "";
  function set(name: FieldName, value: string) {
    (fields as unknown as Record<string, string | null>)[name] = name === "authors" || name === "title" ? value : value.trim() ? value : null;
  }
  function save(event: SubmitEvent) {
    event.preventDefault();
    if (!fields.title.trim()) return;
    onSave({ ...fields, authors: fields.authors.trim(), title: fields.title.trim() });
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="check-source-h" onkeydown={(event) => { if (event.key === "Escape") onClose(); }}>
  <form class="modal-box max-w-lg" onsubmit={save}>
    <h3 id="check-source-h" class="text-lg font-semibold">Check this source</h3>
    <p class="mt-1 text-sm text-muted">Filled in from {found.from}. Fix anything that's wrong, then save.</p>
    <div class="mt-4 space-y-3">
      {#each rows as row (row.name)}
        {@const fromFile = found.fromFile.includes(row.name)}
        <label class="block" data-field={row.name}>
          <span class="flex items-center gap-2 text-sm font-semibold">{row.label}<span class="chip {fromFile ? 'chip-primary' : ''} text-xs">{fromFile ? "From the file" : "Not in the file"}</span></span>
          <input class="input mt-1 w-full" placeholder={row.placeholder} value={text(row.name)} required={row.name === "title"} oninput={(event) => set(row.name, event.currentTarget.value)} />
        </label>
      {/each}
    </div>
    <p class="mt-4 text-sm"><span class="text-muted">APA:</span> <span>{#each apaPreview(fields).split("*") as part, index (index)}{#if index % 2}<i>{part}</i>{:else}{part}{/if}{/each}</span></p>
    <div class="modal-action">
      <button type="button" class="btn btn-ghost" onclick={onDiscard}>{discardLabel}</button>
      <button type="submit" class="btn btn-primary">{keepLabel}</button>
    </div>
  </form>
</dialog>

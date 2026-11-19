<script lang="ts">
  // R217: deleting a module names everything it removes, offers "Move contents up", lists
  // clashes and needs the module's name typed. Spec: nested-modules.feature, "Deleting".
  // Tests: DeleteModuleDialog.test.ts (T-DEL-01, T-DEL-04, T-DEL-07).
  import { AlertTriangle } from "@lucide/svelte";
  import type { DeleteChoice, DeletePreview } from "../../lib/api";

  let { name, preview, onConfirm, onCancel }: {
    name: string;
    preview: DeletePreview;
    onConfirm: (choice: DeleteChoice) => void;
    onCancel: () => void;
  } = $props();

  let choice = $state<DeleteChoice>("MoveContentsUp");
  let typed = $state("");
  const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? "" : "s"}`;
  const blocked = $derived(choice === "MoveContentsUp" && preview.clashes.length > 0);
  const ready = $derived(typed.trim() === name && !blocked);
  const topics = $derived(preview.topics.length ? preview.topics.join(", ") : "no topics");
</script>

<dialog open class="modal modal-open" aria-label="Delete {name}?" onkeydown={(event) => { if (event.key === "Escape") { event.preventDefault(); onCancel(); } }}>
  <div class="modal-box flex max-w-lg flex-col gap-4">
    <h3 class="text-lg font-black">Delete {name}?</h3>
    <p>
      {name} holds {plural(preview.topics.length, "topic")} ({topics}){#if preview.modules.length}, {plural(preview.modules.length, "submodule")} ({preview.modules.join(", ")}){/if},
      {plural(preview.questions, "question")}, {plural(preview.cards, "card")} and {plural(preview.checklists, "recall checklist")}.
    </p>
    <fieldset class="flex flex-col gap-2">
      <legend class="sr-only">What happens to what's inside</legend>
      <label class="flex cursor-pointer items-start gap-3 rounded-xl border-2 border-base-300 p-3">
        <input type="radio" class="radio mt-0.5" name="delete-choice" value="MoveContentsUp" bind:group={choice} />
        <span><b>Move contents up</b><br /><span class="text-sm text-muted">Its topics and submodules move up a level</span></span>
      </label>
      <label class="flex cursor-pointer items-start gap-3 rounded-xl border-2 border-base-300 p-3">
        <input type="radio" class="radio mt-0.5" name="delete-choice" value="Everything" bind:group={choice} />
        <span><b>Delete everything</b><br /><span class="text-sm text-muted">Removes it with every topic and submodule in it</span></span>
      </label>
    </fieldset>
    {#if blocked}
      <p role="alert" class="flex items-start gap-2 rounded-xl bg-error/15 p-3 text-sm font-bold text-error">
        <AlertTriangle class="mt-0.5 size-4 shrink-0" strokeWidth={2.4} />
        The level above already has {preview.clashes.join(", ")}. Rename {preview.clashes.length === 1 ? "it" : "them"} first.
      </p>
    {/if}
    <label class="flex flex-col gap-1">
      <span class="text-sm font-bold">Type {name} to confirm</span>
      <input class="input w-full" bind:value={typed} autocomplete="off" />
    </label>
    <div class="modal-action">
      <button type="button" class="btn btn-ghost" onclick={onCancel}>Cancel</button>
      <button type="button" class="btn btn-error" disabled={!ready} onclick={() => onConfirm(choice)}>Delete {name}</button>
    </div>
  </div>
</dialog>

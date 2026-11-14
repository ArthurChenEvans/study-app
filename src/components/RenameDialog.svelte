<script lang="ts">
  // R185: one dialog for renaming anything with a name (a module, a topic, a project).
  // The backend's message shows if the name isn't accepted, and the dialog stays open.
  import { onMount } from "svelte";
  import { errorMessage } from "../lib/api";

  let { title, value, label = "Name", extra = null, onSave, onClose }: {
    title: string;
    label?: string;
    value: string;
    /** A second, optional field, like a project's kind. */
    extra?: { label: string; value: string; placeholder?: string } | null;
    onSave: (name: string, extra: string) => Promise<void>;
    onClose: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let name = $state(value);
  // svelte-ignore state_referenced_locally
  let second = $state(extra?.value ?? "");
  let error = $state("");
  let busy = $state(false);
  let input = $state<HTMLInputElement>();

  onMount(() => {
    input?.focus();
    input?.select();
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = "";
    try {
      await onSave(name.trim(), second.trim());
    } catch (failure) {
      const message = errorMessage(failure);
      error = message.charAt(0).toUpperCase() + message.slice(1) + (message.endsWith(".") ? "" : ".");
    } finally {
      busy = false;
    }
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="rename-h" onkeydown={(event) => { if (event.key === "Escape") { event.preventDefault(); onClose(); } }}>
  <form class="modal-box max-w-md" onsubmit={save}>
    <h3 id="rename-h" class="text-lg font-semibold">{title}</h3>
    <label class="mt-4 block">
      <span class="text-sm font-bold">{label}</span>
      <input bind:this={input} class="input mt-1 w-full" bind:value={name} aria-invalid={error ? "true" : undefined} aria-describedby={error ? "rename-error" : undefined} />
    </label>
    {#if extra}
      <label class="mt-3 block">
        <span class="text-sm font-bold">{extra.label}</span>
        <input class="input mt-1 w-full" bind:value={second} placeholder={extra.placeholder ?? ""} />
      </label>
    {/if}
    {#if error}<p id="rename-error" role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      <button type="button" class="btn btn-ghost" onclick={onClose}>Cancel</button>
      <button type="submit" class="btn btn-primary" disabled={busy}>Save</button>
    </div>
  </form>
</dialog>

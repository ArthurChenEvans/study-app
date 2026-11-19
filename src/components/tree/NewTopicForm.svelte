<script lang="ts">
  // R217: making a topic. With no module picked, the Default warning asks first.
  // Spec: nested-modules.feature, "Default module". Tests: NewTopicForm.test.ts (T-DEF-09 to T-DEF-11).
  import { tick } from "svelte";
  import { api, errorMessage, type TreeView } from "../../lib/api";

  let { tree, module: preset = null, onCreated, onCancel }: { tree: TreeView; module?: number | null; onCreated?: (topic: number) => void; onCancel?: () => void } = $props();

  let name = $state("");
  // svelte-ignore state_referenced_locally
  let chosen = $state(preset == null ? "" : String(preset));
  let warning = $state(false);
  let error = $state("");
  let picker = $state<HTMLSelectElement>();
  const modules = $derived(tree.nodes.filter((node) => node.node.kind === "Module" && !node.is_default));

  async function create(target: number | null) {
    error = "";
    try {
      // Created first: an optional call skips its arguments when there's no handler.
      const topic = await api.createTopic(name.trim(), target);
      onCreated?.(topic);
      name = "";
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!name.trim()) {
      error = "Give the topic a name";
      return;
    }
    if (chosen === "") warning = true;
    else void create(Number(chosen));
  }

  async function no() {
    warning = false;
    await tick();
    picker?.focus();
  }
</script>

<form aria-label="New topic" class="flex flex-col gap-4" onsubmit={submit}>
  <label class="flex flex-col gap-1">
    <span class="text-sm font-bold">Topic name</span>
    <input class="input w-full" bind:value={name} aria-invalid={error ? "true" : undefined} />
  </label>
  <div class="flex flex-col gap-1">
    <label class="flex flex-col gap-1">
      <span class="text-sm font-bold">Module</span>
      <select class="select w-full" bind:value={chosen} bind:this={picker} aria-describedby="new-topic-module-help">
        <option value="">No module</option>
        {#each modules as node (node.node.id)}<option value={String(node.node.id)}>{node.path.join(" › ")}</option>{/each}
      </select>
    </label>
    <span id="new-topic-module-help" class="text-sm text-muted">Modules show their full path, so ones with the same name are told apart.</span>
  </div>
  {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}
  <div class="flex justify-end gap-3">
    {#if onCancel}<button type="button" class="btn btn-ghost" onclick={onCancel}>Cancel</button>{/if}
    <button type="submit" class="btn btn-primary">Create topic</button>
  </div>
</form>

{#if warning}
  <dialog open class="modal modal-open" aria-labelledby="default-h" onkeydown={(event) => { if (event.key === "Escape") { event.preventDefault(); void no(); } }}>
    <div class="modal-box max-w-md">
      <h3 id="default-h" class="text-lg font-black">Put this topic in Default?</h3>
      <p class="mt-3">Creating this topic without selecting a module will put this under Default. Are you sure you want this?</p>
      <p class="mt-2 text-sm text-muted">Default is not the fundamental module. It is a root module like all your other first layer modules. It is just where topics with unnamed modules go. Your root modules are not children of default</p>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={no}>No</button>
        <button type="button" class="btn btn-primary" onclick={() => { warning = false; void create(null); }}>Yes</button>
      </div>
    </div>
  </dialog>
{/if}

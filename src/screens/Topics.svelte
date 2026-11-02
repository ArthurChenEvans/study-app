<script lang="ts">
  // topics.feature (R1, R2, R3, R30)
  import { Folder, FolderOpen, Library, Pencil, Play, Plus, Trash2 } from "@lucide/svelte";
  import { api, errorMessage, type Id, type ModuleRow, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";

  let { onEdit = (_topic: Id) => {}, onQuiz = (_topic: Id) => {} } = $props();

  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  let loaded = $state(false);

  let creating = $state(false);
  let newName = $state("");
  let newModule = $state<string>("");
  let createError = $state("");

  let deleting = $state<TopicRow | null>(null);

  const groups = $derived.by(() => {
    const byModule = modules
      .map((module) => ({ key: `m${module.id}`, name: module.name, topics: topics.filter((topic) => topic.module === module.id) }))
      .filter((group) => group.topics.length > 0);
    const loose = topics.filter((topic) => topic.module === null);
    return loose.length ? [...byModule, { key: "none", name: "No module", topics: loose }] : byModule;
  });

  async function load() {
    [topics, modules] = await Promise.all([api.topics(), api.modules()]);
    loaded = true;
  }

  function openCreate() {
    newName = "";
    newModule = "";
    createError = "";
    creating = true;
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.createTopic(newName.trim(), newModule ? Number(newModule) : null);
      creating = false;
      await load();
    } catch (error) {
      createError = errorMessage(error);
    }
  }

  async function confirmDelete() {
    if (!deleting) return;
    await api.deleteTopic(deleting.id);
    deleting = null;
    await load();
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Topics" subtitle={loaded && topics.length ? `${topics.length} topics in ${modules.length} modules` : ""}>
    <button class="btn btn-primary" onclick={openCreate}><Plus class="size-4" />New topic</button>
  </PageHeader>

  {#if loaded && topics.length === 0}
    <div class="panel max-w-lg p-10">
      <span class="grid size-10 place-items-center rounded-lg bg-base-200"><Library class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No topics yet</h2>
      <p class="mt-2 text-muted">A topic holds your questions on one subject, like "CPU". You can group topics into modules later.</p>
      <button class="btn btn-primary mt-6" onclick={openCreate}><Plus class="size-4" />Create your first topic</button>
    </div>
  {/if}

  {#each groups as group (group.key)}
    <section class="mt-8" aria-labelledby="group-{group.key}">
      <h2 id="group-{group.key}" class="mb-2 flex items-center gap-2 text-sm font-semibold text-muted">
        {#if group.key === "none"}<FolderOpen class="size-4" />{:else}<Folder class="size-4" />{/if}{group.name}
      </h2>
      <div class="panel overflow-hidden">
        <table class="table table-fixed">
          <colgroup><col class="w-[30%]" /><col class="w-[30%]" /><col class="w-[12%]" /><col class="w-[28%]" /></colgroup>
          <thead><tr class="text-muted"><th>Topic</th><th>Tags</th><th class="text-right">Questions</th><th></th></tr></thead>
          <tbody>
            {#each group.topics as topic (topic.id)}
              <tr class="hover:bg-base-100">
                <td class="font-semibold">{topic.name}</td>
                <td>
                  {#each topic.tags as tag (tag)}<span class="badge badge-ghost mr-1 rounded-full text-xs">{tag}</span>{:else}<span class="text-sm text-muted">None</span>{/each}
                </td>
                <td class="text-right tabular-nums">{topic.question_count}</td>
                <td class="whitespace-nowrap text-right">
                  <button class="btn btn-ghost btn-sm" onclick={() => onQuiz(topic.id)}><Play class="size-4" />Quiz</button>
                  <button class="btn btn-ghost btn-sm" onclick={() => onEdit(topic.id)}><Pencil class="size-4" />Edit</button>
                  <button class="btn btn-ghost btn-sm btn-square" aria-label="Delete {topic.name}" onclick={() => (deleting = topic)}><Trash2 class="size-4" /></button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/each}
</div>

{#if creating}
  <dialog open class="modal modal-open" aria-labelledby="new-topic-title">
    <form class="modal-box max-w-md" onsubmit={create}>
      <h3 id="new-topic-title" class="text-lg font-semibold">New topic</h3>
      <label class="mt-6 block">
        <span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={newName} autofocus placeholder="e.g. CPU…" />
      </label>
      <label class="mt-4 block">
        <span class="text-sm font-semibold">Module</span>
        <select class="select mt-1 w-full" bind:value={newModule}>
          <option value="">No module</option>
          {#each modules as module (module.id)}<option value={String(module.id)}>{module.name}</option>{/each}
        </select>
      </label>
      {#if createError}<p role="alert" class="mt-4 text-sm text-error">{createError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create topic</button>
      </div>
    </form>
  </dialog>
{/if}

{#if deleting}
  <dialog open class="modal modal-open" aria-labelledby="delete-topic-title">
    <div class="modal-box max-w-md">
      <h3 id="delete-topic-title" class="text-lg font-semibold">Delete "{deleting.name}"?</h3>
      <p class="mt-2 text-muted">Its {deleting.question_count} questions will be removed from your topics. Past quiz attempts keep the questions as they were, so your history isn't affected.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = null)}>Cancel</button>
        <button class="btn btn-error" onclick={confirmDelete}>Delete topic</button>
      </div>
    </div>
  </dialog>
{/if}

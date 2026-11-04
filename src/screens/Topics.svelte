<script lang="ts">
  // topics.feature (R1, R2, R3, R30, R43, R51)
  // calendar.feature (R96, R97): each topic's quiz step and next quiz, and the schedule.
  import { CalendarCog, Folder, FolderOpen, FolderPlus, Library, Pencil, Play, Plus } from "@lucide/svelte";
  import RowMenu from "../components/RowMenu.svelte";
  import { api, errorMessage, type Id, type ModuleRow, type Schedule, type TopicQuizView, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import ScheduleDialog from "../components/ScheduleDialog.svelte";
  import { dueLabel } from "../lib/dates";

  let { onEdit = () => {}, onQuiz = () => {}, onQuizTopics = () => {} }: { onEdit?: (topic: Id) => void; onQuiz?: (topic: Id) => void; onQuizTopics?: (topics: Id[]) => void } = $props();

  let quizzes = $state<TopicQuizView[]>([]);
  let today = $state(0);
  let editingSchedule = $state<Schedule | null>(null);
  const quizOf = (topic: Id) => quizzes.find((quiz) => quiz.topic === topic);

  let topics = $state<TopicRow[]>([]);
  const dueTopics = $derived(topics.filter((topic) => quizOf(topic.id)?.due_today && topic.question_count > 0).map((topic) => topic.id));
  let modules = $state<ModuleRow[]>([]);
  let loaded = $state(false);

  let creating = $state(false);
  let newName = $state("");
  let newModule = $state<string>("");
  let createError = $state("");

  let deleting = $state<TopicRow | null>(null);
  let deletingModule = $state<{ id: Id; name: string; count: number } | null>(null);
  let tagging = $state<TopicRow | null>(null);
  let tagText = $state("");

  function openTags(topic: TopicRow) {
    tagging = topic;
    tagText = topic.tags.join(", ");
  }

  async function saveTags(event: SubmitEvent) {
    event.preventDefault();
    if (!tagging) return;
    const tags = tagText.split(",").map((tag) => tag.trim()).filter(Boolean);
    await api.setTopicTags(tagging.id, tags);
    tagging = null;
    await load();
  }

  async function confirmDeleteModule() {
    if (!deletingModule) return;
    await api.deleteModule(deletingModule.id);
    deletingModule = null;
    await load();
  }

  let creatingModule = $state(false);
  let moduleName = $state("");
  let moduleError = $state("");

  async function createModule(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.createModule(moduleName.trim());
      creatingModule = false;
      moduleName = "";
      await load();
    } catch (error) {
      moduleError = errorMessage(error);
    }
  }

  const groups = $derived.by(() => {
    const byModule = modules
      .map((module) => ({ key: `m${module.id}`, id: module.id as Id | null, name: module.name, topics: topics.filter((topic) => topic.module === module.id) }));
    const loose = topics.filter((topic) => topic.module === null);
    return loose.length ? [...byModule, { key: "none", id: null, name: "No module", topics: loose }] : byModule;
  });

  async function load() {
    [topics, modules, quizzes, today] = await Promise.all([api.topics(), api.modules(), api.topicQuizzes().catch(() => []), api.today().catch(() => 0)]);
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
  <PageHeader title="Topics" subtitle={loaded && topics.length ? `${topics.length} topics in ${modules.length} modules${dueTopics.length ? ` · ${dueTopics.length} ${dueTopics.length === 1 ? "quiz" : "quizzes"} due today` : ""}` : ""}>
    {#if topics.length}<button class="btn btn-ghost" onclick={async () => (editingSchedule = await api.quizSchedule())}><CalendarCog class="size-4" />Quiz schedule</button>{/if}
    <button class="btn btn-ghost" onclick={() => { moduleError = ""; creatingModule = true; }}><FolderPlus class="size-4" />New module</button>
    <button class="btn {dueTopics.length ? 'btn-ghost' : 'btn-primary'}" onclick={openCreate}><Plus class="size-4" />New topic</button>
    {#if dueTopics.length}<button class="btn btn-primary" onclick={() => onQuizTopics(dueTopics)}><Play class="size-4" />Quiz the {dueTopics.length === 1 ? "due topic" : `${dueTopics.length} due topics`}</button>{/if}
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
        {#if group.id !== null}
          {@const id = group.id}
          <RowMenu small label="{group.name} module actions" items={[{ label: "Delete module…", danger: true, onSelect: () => (deletingModule = { id, name: group.name, count: group.topics.length }) }]} />
        {/if}
      </h2>
      {#if group.topics.length === 0}
        <p class="panel px-6 py-4 text-sm text-muted">No topics in this module yet.</p>
      {:else}
      <div class="panel overflow-hidden">
        <table class="table table-fixed">
          <colgroup><col class="w-[24%]" /><col class="w-[22%]" /><col class="w-[10%]" /><col class="w-[18%]" /><col class="w-[26%]" /></colgroup>
          <thead><tr class="text-muted"><th>Topic</th><th>Tags</th><th class="text-right">Questions</th><th>Next quiz</th><th><span class="sr-only">Actions</span></th></tr></thead>
          <tbody>
            {#each group.topics as topic (topic.id)}
              <tr class="hover:bg-base-100">
                <td class="font-semibold"><button type="button" class="text-left hover:underline" onclick={() => onEdit(topic.id)}>{topic.name}</button></td>
                <td>
                  {#each topic.tags as tag (tag)}<span class="badge badge-ghost mr-1 rounded-full text-xs">{tag}</span>{:else}<span class="text-sm text-muted">None</span>{/each}
                </td>
                <td class="text-right tabular-nums">{topic.question_count}</td>
                <td>
                  {#if quizOf(topic.id)}{@const quiz = quizOf(topic.id)!}{#if quiz.due_today}<span class="flex items-center gap-1.5 text-sm font-semibold"><span class="due-dot" aria-hidden="true"></span>Due today</span>{:else}<span class="block text-sm">{dueLabel(quiz.due, today)}</span>{/if}<span class="block text-xs text-muted">{quiz.step} step</span>
                  {:else}<span class="text-sm text-muted">{topic.question_count ? "Quiz it to start" : "No questions yet"}</span>{/if}
                </td>
                <td class="whitespace-nowrap text-right">
                  <span class="row-actions inline-flex items-center gap-1">
                    <button class="btn btn-ghost btn-sm" aria-label="Quiz {topic.name}" onclick={() => onQuiz(topic.id)}><Play class="size-4" />Quiz</button>
                    <button class="btn btn-ghost btn-sm" aria-label="Edit {topic.name}" onclick={() => onEdit(topic.id)}><Pencil class="size-4" />Edit</button>
                    <RowMenu small label="More for {topic.name}" items={[
                      { label: "Edit tags", onSelect: () => openTags(topic) },
                      { label: "Delete topic…", danger: true, onSelect: () => (deleting = topic) },
                    ]} />
                  </span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {/if}
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

{#if creatingModule}
  <dialog open class="modal modal-open" aria-labelledby="new-module-title">
    <form class="modal-box max-w-md" onsubmit={createModule}>
      <h3 id="new-module-title" class="text-lg font-semibold">New module</h3>
      <p class="mt-2 text-sm text-muted">A module groups related topics, like a course unit.</p>
      <label class="mt-6 block">
        <span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={moduleName} autofocus placeholder="e.g. Computer Architecture…" />
      </label>
      {#if moduleError}<p role="alert" class="mt-4 text-sm text-error">{moduleError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creatingModule = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create module</button>
      </div>
    </form>
  </dialog>
{/if}

{#if deletingModule}
  <dialog open class="modal modal-open" aria-labelledby="delete-module-title">
    <div class="modal-box max-w-md">
      <h3 id="delete-module-title" class="text-lg font-semibold">Delete module "{deletingModule.name}"?</h3>
      <p class="mt-2 text-muted">{deletingModule.count === 1 ? "Its 1 topic is" : `Its ${deletingModule.count} topics are`} moved to "No module". No topics or questions are deleted.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deletingModule = null)}>Cancel</button>
        <button class="btn btn-error" onclick={confirmDeleteModule}>Delete module</button>
      </div>
    </div>
  </dialog>
{/if}

{#if tagging}
  <dialog open class="modal modal-open" aria-labelledby="tags-title">
    <form class="modal-box max-w-md" onsubmit={saveTags}>
      <h3 id="tags-title" class="text-lg font-semibold">Tags for "{tagging.name}"</h3>
      <label class="mt-6 block">
        <span class="text-sm font-semibold">Tags</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" aria-label="Tags" bind:value={tagText} autofocus placeholder="e.g. hardware, exam…" />
        <span class="mt-1 block text-xs text-muted">Separate tags with commas.</span>
      </label>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (tagging = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Save tags</button>
      </div>
    </form>
  </dialog>
{/if}

{#if editingSchedule}
  <ScheduleDialog title="Quiz schedule" intro="Every topic climbs these steps, starting the first time you quiz it. Each topic in a quiz is scored on its own questions, and the next date counts from the quiz day. Retries don't move it." passWhat="the topic's questions right" applies="Changes apply from each topic's next quiz." schedule={editingSchedule}
    onSave={async (next) => { await api.setQuizSchedule(next); editingSchedule = null; await load(); }} onReset={() => api.resetQuizSchedule()} onClose={() => (editingSchedule = null)} />
{/if}

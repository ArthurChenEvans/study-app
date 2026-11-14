<script lang="ts">
  // R189: Topics & Modules. A module groups topics, and every quiz, card and checklist lives in a
  // topic, so this is where topics and modules are made, renamed, moved, tagged and deleted.
  // topics.feature (R1, R2, R3, R43, R51)
  import { Brain, FolderPlus, ListChecks, PencilLine, Plus, Search, SquareStack, Trash2 } from "@lucide/svelte";
  import RowMenu from "../components/RowMenu.svelte";
  import RenameDialog from "../components/RenameDialog.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { api, errorMessage, type Id, type ModuleRow, type RecallRow, type TopicRow } from "../lib/api";
  import { deleteWithUndo, toast } from "../lib/celebrations.svelte";
  import { matchesFilter } from "../lib/lists";

  let { onEdit = () => {}, onCards = () => {}, onRecall = () => {} }: { onEdit?: (topic: Id) => void; onCards?: (topic: Id) => void; onRecall?: () => void } = $props();

  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  let cardCounts = $state<Map<Id, number>>(new Map());
  let checklists = $state<RecallRow[]>([]);
  let loaded = $state(false);
  let selected = $state<Id | null>(null);
  let find = $state("");

  // Deletes wait out their undo window here before they happen for real (R157).
  let hiddenTopics = $state<Set<Id>>(new Set());
  let hiddenModules = $state<Set<Id>>(new Set());
  const without = (set: Set<Id>, id: Id) => new Set([...set].filter((value) => value !== id));

  async function load() {
    const [allTopics, allModules, home, recall] = await Promise.all([api.topics(), api.modules(), api.cardsHome().catch(() => null), api.recallList().catch(() => [])]);
    modules = allModules.filter((m) => !hiddenModules.has(m.id));
    topics = allTopics.filter((t) => !hiddenTopics.has(t.id)).map((t) => (t.module !== null && hiddenModules.has(t.module) ? { ...t, module: null } : t));
    cardCounts = new Map((home?.per_topic ?? []).map((row) => [row.topic, row.total]));
    checklists = recall ?? [];
    loaded = true;
  }

  const moduleName = (id: Id | null) => (id === null ? null : (modules.find((m) => m.id === id)?.name ?? null));
  const matches = (topic: TopicRow) => matchesFilter(find, topic.name, moduleName(topic.module), topic.tags.join(" "));
  const groups = $derived([
    ...modules.map((module) => ({ module, topics: topics.filter((t) => t.module === module.id && (matches(t) || matchesFilter(find, module.name))) })),
    ...(topics.some((t) => t.module === null) ? [{ module: null, topics: topics.filter((t) => t.module === null && matches(t)) }] : []),
  ].filter((group) => !find.trim() || group.topics.length || (group.module && matchesFilter(find, group.module.name))));
  const picked = $derived(topics.find((t) => t.id === selected) ?? null);
  const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;
  const cardsIn = (topic: TopicRow) => cardCounts.get(topic.id) ?? 0;
  const checklistsIn = (topic: TopicRow) => checklists.filter((row) => !row.module && row.subject_name === topic.name).length;
  const questionsIn = (topic: TopicRow) => topic.question_count + topic.short_count;

  // ---------- Making ----------
  let creating = $state(false);
  let newName = $state("");
  let newModule = $state("");
  let createError = $state("");
  let creatingModule = $state(false);
  let moduleInput = $state("");
  let moduleError = $state("");

  function openCreate() {
    newName = "";
    newModule = picked?.module ? String(picked.module) : "";
    createError = "";
    creating = true;
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      selected = await api.createTopic(newName.trim(), newModule ? Number(newModule) : null);
      toast({ text: "Topic added", sub: newName.trim(), icon: "plus" });
      creating = false;
      await load();
    } catch (error) {
      createError = errorMessage(error);
    }
  }

  async function createModule(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.createModule(moduleInput.trim());
      toast({ text: "Module added", sub: moduleInput.trim(), icon: "plus" });
      creatingModule = false;
      moduleInput = "";
      await load();
    } catch (error) {
      moduleError = errorMessage(error);
    }
  }

  // ---------- Changing ----------
  let renaming = $state<{ kind: "module" | "topic"; id: Id; name: string } | null>(null);

  async function rename(name: string) {
    if (!renaming) return;
    const { kind, id } = renaming;
    if (kind === "module") await api.renameModule(id, name);
    else await api.renameTopic(id, name);
    renaming = null;
    toast({ text: kind === "module" ? "Module renamed" : "Topic renamed", sub: name, icon: "check" });
    await load();
  }

  async function move(topic: TopicRow, value: string) {
    const target = value ? Number(value) : null;
    await api.moveTopic(topic.id, target);
    toast({ text: `${topic.name} moved to ${moduleName(target) ?? "No module"}`, icon: "move" });
    await load();
  }

  async function saveTags(topic: TopicRow, text: string) {
    const tags = text.split(",").map((tag) => tag.trim()).filter(Boolean);
    if (tags.join(",") === topic.tags.join(",")) return;
    await api.setTopicTags(topic.id, tags);
    await load();
  }

  // ---------- Deleting ----------
  let deleting = $state<TopicRow | null>(null);
  let deletingModule = $state<{ id: Id; name: string; count: number } | null>(null);
  const goesWith = $derived(deleting ? [questionsIn(deleting) && plural(questionsIn(deleting), "question"), cardsIn(deleting) && plural(cardsIn(deleting), "card"), checklistsIn(deleting) && plural(checklistsIn(deleting), "checklist")].filter(Boolean) as string[] : []);
  const listed = (parts: string[]) => (parts.length > 1 ? `${parts.slice(0, -1).join(", ")} and ${parts.at(-1)}` : parts[0] ?? "");

  /** A topic with anything in it asks first. An empty one goes straight away, with Undo. */
  function requestDelete(topic: TopicRow) {
    if (questionsIn(topic) || cardsIn(topic) || checklistsIn(topic)) deleting = topic;
    else removeTopic(topic);
  }

  function removeTopic(topic: TopicRow) {
    deleting = null;
    deleteWithUndo({
      text: "Topic deleted",
      sub: topic.name,
      hide: () => {
        hiddenTopics = new Set([...hiddenTopics, topic.id]);
        topics = topics.filter((t) => t.id !== topic.id);
        if (selected === topic.id) selected = null;
      },
      restore: () => {
        hiddenTopics = without(hiddenTopics, topic.id);
        load();
      },
      commit: async () => {
        await api.deleteTopic(topic.id);
        hiddenTopics = without(hiddenTopics, topic.id);
        await load();
      },
    });
  }

  function confirmDeleteModule() {
    if (!deletingModule) return;
    const { id, name, count } = deletingModule;
    deletingModule = null;
    deleteWithUndo({
      text: "Module deleted",
      sub: count ? `${name}. Its ${count === 1 ? "topic moves" : `${count} topics move`} to No module.` : name,
      hide: () => {
        hiddenModules = new Set([...hiddenModules, id]);
        modules = modules.filter((m) => m.id !== id);
        topics = topics.map((t) => (t.module === id ? { ...t, module: null } : t));
      },
      restore: () => {
        hiddenModules = without(hiddenModules, id);
        load();
      },
      commit: async () => {
        await api.deleteModule(id);
        hiddenModules = without(hiddenModules, id);
        await load();
      },
    });
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Topics & Modules" subtitle="A module groups topics. Every quiz, card and checklist lives in a topic.">
    <button class="btn btn-ghost" onclick={() => { moduleError = ""; moduleInput = ""; creatingModule = true; }}><FolderPlus class="size-4" />New module</button>
    <button class="btn btn-primary" onclick={openCreate}><Plus class="size-4" />New topic</button>
  </PageHeader>

  {#if loaded}
    <div class="grid grid-cols-1 items-start gap-5 tab:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
      <nav class="flex flex-col gap-3" aria-label="Topics by module">
        <label class="input w-full">
          <Search class="size-4 opacity-70" />
          <input type="search" aria-label="Find a topic or module" placeholder="Find a topic or module…" bind:value={find} />
        </label>
        {#each groups as group (group.module?.id ?? "none")}
          <section class="panel p-2" aria-labelledby="module-{group.module?.id ?? 'none'}">
            <div class="flex items-center gap-2 px-2 py-1">
              <h2 id="module-{group.module?.id ?? 'none'}" class="flex-1 text-[15px] font-black">{group.module?.name ?? "No module"}</h2>
              <span class="text-xs text-muted">{plural(group.topics.length, "topic")}</span>
              {#if group.module}
                {@const id = group.module.id}
                {@const name = group.module.name}
                <RowMenu small label="{name} module actions" items={[{ label: "Rename module…", onSelect: () => (renaming = { kind: "module", id, name }) }, { label: "Delete module…", danger: true, onSelect: () => (deletingModule = { id, name, count: topics.filter((t) => t.module === id).length }) }]} />
              {/if}
            </div>
            {#each group.topics as topic (topic.id)}
              <button type="button" class="nav-link flex min-h-10 w-full items-center gap-2 px-3 text-left" aria-current={selected === topic.id ? "true" : undefined} onclick={() => (selected = topic.id)}>
                <span class="flex-1 truncate">{topic.name}</span>
                <span class="text-xs font-bold text-muted" aria-hidden="true">{questionsIn(topic)} q · {cardsIn(topic)} cards</span>
              </button>
            {:else}
              <p class="px-3 py-2 text-sm text-muted">No topics in it yet.</p>
            {/each}
          </section>
        {:else}
          <div class="panel p-6 text-sm text-muted">{find.trim() ? `Nothing matches "${find.trim()}".` : "No topics yet. Make one with New topic."}</div>
        {/each}
      </nav>

      {#if picked}
        {@const topic = picked}
        <section class="panel flex flex-col gap-5 p-6" aria-labelledby="picked-h">
          <div class="flex flex-wrap items-start gap-2">
            <div class="flex-1">
              <p class="eyebrow">Topic in {moduleName(topic.module) ?? "no module"}</p>
              <h2 id="picked-h" class="mt-1 text-2xl font-black">{topic.name}</h2>
            </div>
            <button class="btn btn-outline btn-sm" onclick={() => (renaming = { kind: "topic", id: topic.id, name: topic.name })}><PencilLine class="size-4" />Rename</button>
            <button class="btn btn-outline btn-sm text-error" onclick={() => requestDelete(topic)}><Trash2 class="size-4" />Delete</button>
          </div>
          <div class="grid grid-cols-1 gap-4 tab:grid-cols-2">
            <label class="block"><span class="text-sm font-bold">Module</span>
              <select class="select mt-1 w-full" value={topic.module === null ? "" : String(topic.module)} onchange={(event) => move(topic, event.currentTarget.value)}>
                <option value="">No module</option>
                {#each modules as module (module.id)}<option value={String(module.id)}>{module.name}</option>{/each}
              </select></label>
            <div><label class="block"><span class="text-sm font-bold">Tags</span>
              <input class="input mt-1 w-full" value={topic.tags.join(", ")} placeholder="e.g. exam, lecture-5" aria-describedby="tags-help" onchange={(event) => saveTags(topic, event.currentTarget.value)} /></label>
              <span id="tags-help" class="mt-1 block text-xs text-muted">Separate tags with commas.</span></div>
          </div>
          <div>
            <h3 class="eyebrow">What's in it</h3>
            <div class="mt-2 grid grid-cols-1 gap-3 tab:grid-cols-3">
              <div class="panel flex flex-col gap-1 p-4"><ListChecks class="size-5" aria-hidden="true" /><span class="font-extrabold">{plural(questionsIn(topic), "quiz question")}</span><button class="btn btn-ghost btn-sm mt-1 justify-start px-0" onclick={() => onEdit(topic.id)}>Open in Quizzes</button></div>
              <div class="panel flex flex-col gap-1 p-4"><SquareStack class="size-5" aria-hidden="true" /><span class="font-extrabold">{plural(cardsIn(topic), "card")}</span><button class="btn btn-ghost btn-sm mt-1 justify-start px-0" onclick={() => onCards(topic.id)}>Open in Cards</button></div>
              <div class="panel flex flex-col gap-1 p-4"><Brain class="size-5" aria-hidden="true" /><span class="font-extrabold">{plural(checklistsIn(topic), "free recall checklist")}</span><button class="btn btn-ghost btn-sm mt-1 justify-start px-0" onclick={onRecall}>Open in Free Recall</button></div>
            </div>
          </div>
        </section>
      {:else}
        <div class="panel p-6 text-sm text-muted max-tab:hidden"><p class="font-extrabold text-base-content">Pick a topic to see it here</p><p class="mt-2">You can rename it, move it to another module, tag it, or delete it.</p></div>
      {/if}
    </div>
  {/if}
</div>

{#if creating}
  <dialog open class="modal modal-open" aria-labelledby="new-topic-title">
    <form class="modal-box max-w-md" onsubmit={create}>
      <h3 id="new-topic-title" class="text-lg font-semibold">New topic</h3>
      <label class="mt-6 block"><span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={newName} autofocus placeholder="e.g. CPU…" /></label>
      <label class="mt-4 block"><span class="text-sm font-semibold">Module</span>
        <select class="select mt-1 w-full" bind:value={newModule}>
          <option value="">No module</option>
          {#each modules as module (module.id)}<option value={String(module.id)}>{module.name}</option>{/each}
        </select></label>
      {#if createError}<p role="alert" class="mt-4 text-sm text-error">{createError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create topic</button>
      </div>
    </form>
  </dialog>
{/if}

{#if creatingModule}
  <dialog open class="modal modal-open" aria-labelledby="new-module-title">
    <form class="modal-box max-w-md" onsubmit={createModule}>
      <h3 id="new-module-title" class="text-lg font-semibold">New module</h3>
      <p class="mt-2 text-sm text-muted">A module groups related topics, like a course unit.</p>
      <label class="mt-6 block"><span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={moduleInput} autofocus placeholder="e.g. Computer Architecture…" /></label>
      {#if moduleError}<p role="alert" class="mt-4 text-sm text-error">{moduleError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creatingModule = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create module</button>
      </div>
    </form>
  </dialog>
{/if}

{#if deleting}
  <dialog open class="modal modal-open" aria-labelledby="delete-topic-title">
    <div class="modal-box max-w-md">
      <h3 id="delete-topic-title" class="text-lg font-semibold">Delete "{deleting.name}"?</h3>
      <p class="mt-2 text-muted">Its {listed(goesWith)} go with it. Past quiz attempts keep the questions as they were, so your history isn't affected.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deleting = null)}>Cancel</button>
        <button class="btn btn-error" onclick={() => deleting && removeTopic(deleting)}>Delete topic</button>
      </div>
    </div>
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

{#if renaming}
  <RenameDialog title={renaming.kind === "module" ? "Rename module" : "Rename topic"} value={renaming.name} onSave={rename} onClose={() => (renaming = null)} />
{/if}

<script lang="ts">
  // R189: Topics & Modules. A module groups topics, and every quiz, card and checklist lives in a
  // topic, so this is where topics and modules are made, renamed, moved, tagged and deleted.
  // topics.feature (R1, R2, R3, R43, R51), nested-modules.feature (R217 to R219)
  import { Brain, FolderPlus, Link2, ListChecks, PencilLine, Plus, Search, SquareStack, Trash2, X } from "@lucide/svelte";
  import DeleteModuleDialog from "../components/tree/DeleteModuleDialog.svelte";
  import GraphView from "../components/tree/GraphView.svelte";
  import RowMenu from "../components/RowMenu.svelte";
  import { rowMenu } from "../lib/contextMenu.svelte";
  import { tick } from "svelte";
  import RenameDialog from "../components/RenameDialog.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { api, errorMessage, type DeleteChoice, type DeletePreview, type Id, type ModuleRow, type NodeRef, type RecallRow, type SourceRow, type TopicRow, type TreeView } from "../lib/api";
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

  // R217: modules nest, so each knows its parent and depth. R218: related links per topic.
  let tree = $state<TreeView>({ nodes: [] });
  let view = $state<"tree" | "graph">("tree");
  let sourceRows = $state<SourceRow[]>([]);
  const MAX_DEPTH = 4;

  async function load() {
    const [allTopics, allModules, home, recall, nodes] = await Promise.all([api.topics(), api.modules(), api.cardsHome().catch(() => null), api.recallList().catch(() => []), api.moduleTree().catch(() => ({ nodes: [] }))]);
    tree = nodes ?? { nodes: [] };
    modules = allModules.filter((m) => !hiddenModules.has(m.id));
    topics = allTopics.filter((t) => !hiddenTopics.has(t.id)).map((t) => (t.module !== null && hiddenModules.has(t.module) ? { ...t, module: null } : t));
    cardCounts = new Map((home?.per_topic ?? []).map((row) => [row.topic, row.total]));
    checklists = recall ?? [];
    loaded = true;
  }

  const moduleName = (id: Id | null) => (id === null ? null : (modules.find((m) => m.id === id)?.name ?? null));
  /** "Computer Architecture › N2T", so modules with the same name are told apart. */
  const modulePath = (id: Id | null): string | null => {
    if (id === null) return null;
    const names: string[] = [];
    let current = modules.find((m) => m.id === id);
    while (current && names.length <= MAX_DEPTH) {
      names.unshift(current.name);
      current = current.parent == null ? undefined : modules.find((m) => m.id === current!.parent);
    }
    return names.join(" › ") || null;
  };
  const depthOf = (id: Id) => (modulePath(id) ?? "").split(" › ").length;
  /** Modules parents first, each followed by its submodules, Default last. */
  const ordered = $derived.by(() => {
    const out: { module: ModuleRow; depth: number }[] = [];
    const visit = (parent: Id | null, depth: number) => {
      const kids = modules.filter((m) => (m.parent ?? null) === parent).sort((a, b) => Number(!!a.is_default) - Number(!!b.is_default) || a.name.localeCompare(b.name));
      for (const module of kids) {
        out.push({ module, depth });
        visit(module.id, depth + 1);
      }
    };
    visit(null, 0);
    // Anything whose parent is hidden (waiting out an undo) shows at the top.
    for (const module of modules) if (!out.some((row) => row.module.id === module.id)) out.push({ module, depth: 0 });
    return out;
  });
  const matches = (topic: TopicRow) => matchesFilter(find, topic.name, modulePath(topic.module), topic.tags.join(" "));
  const groups = $derived([
    ...ordered.map(({ module, depth }) => ({ module, depth, topics: topics.filter((t) => t.module === module.id && (matches(t) || matchesFilter(find, module.name))) })),
    ...(topics.some((t) => t.module === null) ? [{ module: null, depth: 0, topics: topics.filter((t) => t.module === null && matches(t)) }] : []),
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
  let moduleParent = $state("");
  let moduleInput = $state("");
  let moduleError = $state("");

  function openCreate(module: Id | null = picked?.module ?? null) {
    newName = "";
    newModule = module ? String(module) : "";
    createError = "";
    creating = true;
  }

  // R217: a topic made with no module goes to Default, after saying so.
  let defaultWarning = $state(false);

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (!newModule && !defaultWarning) {
      if (!newName.trim()) {
        createError = "Give the topic a name";
        return;
      }
      defaultWarning = true;
      return;
    }
    defaultWarning = false;
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
      await api.createSubmodule(moduleInput.trim(), moduleParent ? Number(moduleParent) : null);
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
    try {
      await api.moveTopic(topic.id, target);
      toast({ text: `${topic.name} moved to ${modulePath(target) ?? "Default"}`, icon: "move" });
    } catch (error) {
      toast({ text: "Couldn't move it", sub: errorMessage(error), icon: "alert", error: true });
    }
    await load();
  }

  // R217: modules move under another module, or to the top level.
  let movingModule = $state<{ id: Id; name: string; parent: string } | null>(null);
  let moveModuleError = $state("");
  async function moveModule(event: SubmitEvent) {
    event.preventDefault();
    if (!movingModule) return;
    try {
      await api.moveModule(movingModule.id, movingModule.parent ? Number(movingModule.parent) : null);
      toast({ text: `${movingModule.name} moved`, sub: movingModule.parent ? modulePath(Number(movingModule.parent)) ?? "" : "To the top level", icon: "move" });
      movingModule = null;
      await load();
    } catch (error) {
      moveModuleError = errorMessage(error);
    }
  }
  /** Modules a module can go under: not itself, nothing inside it, not Default. */
  const homesFor = (id: Id) => ordered.filter(({ module }) => !module.is_default && module.id !== id && !(modulePath(module.id) ?? "").startsWith(`${modulePath(id)} › `));

  // R218: related topics, linked but never merged.
  const relatedOf = (topic: TopicRow) => tree.nodes.find((node) => node.node.kind === "Topic" && node.node.id === topic.id)?.related ?? [];
  const topicPath = (id: Id) => {
    const topic = topics.find((t) => t.id === id);
    return topic ? [modulePath(topic.module), topic.name].filter(Boolean).join(" › ") : "";
  };
  async function link(topic: TopicRow, value: string) {
    if (!value) return;
    await api.linkRelated(topic.id, Number(value));
    await load();
  }
  async function unlink(topic: TopicRow, other: Id) {
    await api.unlinkRelated(topic.id, other);
    await load();
  }

  async function openGraph() {
    view = "graph";
    sourceRows = await api.sources().catch(() => []);
  }
  function openNode(node: NodeRef) {
    view = "tree";
    if (node.kind === "Topic") selected = node.id;
  }

  async function saveTags(topic: TopicRow, text: string) {
    const tags = text.split(",").map((tag) => tag.trim()).filter(Boolean);
    if (tags.join(",") === topic.tags.join(",")) return;
    await api.setTopicTags(topic.id, tags);
    await load();
  }

  // ---------- Deleting ----------
  let deleting = $state<TopicRow | null>(null);
  let deletingModule = $state<{ id: Id; name: string; preview: DeletePreview } | null>(null);
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

  async function requestDeleteModule(id: Id, name: string) {
    try {
      deletingModule = { id, name, preview: await api.moduleDeletePreview(id) };
    } catch (error) {
      toast({ text: "Couldn't delete it", sub: errorMessage(error), icon: "alert", error: true });
    }
  }

  /** R217: the dialog names what goes; moving contents up keeps everything, a level higher. */
  function confirmDeleteModule(choice: DeleteChoice) {
    if (!deletingModule) return;
    const { id, name, preview } = deletingModule;
    deletingModule = null;
    const kept = choice === "MoveContentsUp";
    deleteWithUndo({
      text: "Module deleted",
      sub: kept && preview.topics.length ? `${name}. What was in it moved up a level.` : name,
      hide: () => {
        hiddenModules = new Set([...hiddenModules, id]);
        modules = modules.filter((m) => m.id !== id);
        if (!kept) topics = topics.filter((t) => t.module !== id);
      },
      restore: () => {
        hiddenModules = without(hiddenModules, id);
        load();
      },
      commit: async () => {
        await api.deleteModuleWith(id, choice);
        hiddenModules = without(hiddenModules, id);
        await load();
      },
    });
  }

  // R215: the same actions as the panel and the "…" button, a right-click or long-press away.
  function topicMenu(topic: TopicRow) {
    return () => ({ label: topic.name, items: [
      { label: "Edit questions", onSelect: () => onEdit(topic.id) },
      { label: "Cards", onSelect: () => onCards(topic.id) },
      { label: "Rename…", onSelect: () => (renaming = { kind: "topic", id: topic.id, name: topic.name }) },
      { label: "Move to module…", onSelect: async () => { selected = topic.id; await tick(); document.getElementById("topic-module")?.focus(); } },
      { label: "Delete…", danger: true, onSelect: () => requestDelete(topic) },
    ] });
  }
  function moduleItems(id: Id, name: string) {
    const module = modules.find((m) => m.id === id);
    const isDefault = !!module?.is_default;
    return [
      { label: "New topic in it…", onSelect: () => openCreate(id) },
      ...(!isDefault && depthOf(id) < MAX_DEPTH ? [{ label: "New submodule…", onSelect: () => { moduleError = ""; moduleInput = ""; moduleParent = String(id); creatingModule = true; } }] : []),
      ...(!isDefault ? [{ label: "Rename module…", onSelect: () => (renaming = { kind: "module" as const, id, name }) }] : []),
      ...(!isDefault ? [{ label: "Move module…", onSelect: () => { moveModuleError = ""; movingModule = { id, name, parent: module?.parent == null ? "" : String(module.parent) }; } }] : []),
      { label: "Delete module…", danger: true, onSelect: () => requestDeleteModule(id, name) },
    ];
  }

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Topics & Modules" subtitle="A module groups topics. Every quiz, card and checklist lives in a topic.">
    <div class="join" role="group" aria-label="View">
      <button class="btn btn-sm join-item {view === 'tree' ? 'btn-neutral' : 'btn-ghost'}" aria-pressed={view === "tree"} onclick={() => (view = "tree")}>Tree</button>
      <button class="btn btn-sm join-item {view === 'graph' ? 'btn-neutral' : 'btn-ghost'}" aria-pressed={view === "graph"} onclick={openGraph}>Graph</button>
    </div>
    <button class="btn btn-ghost" onclick={() => { moduleError = ""; moduleInput = ""; moduleParent = ""; creatingModule = true; }}><FolderPlus class="size-4" />New module</button>
    <button class="btn btn-primary" onclick={() => openCreate()}><Plus class="size-4" />New topic</button>
  </PageHeader>

  {#if loaded && view === "graph"}
    <GraphView {tree} sources={sourceRows} onOpen={openNode} onCreateModule={() => { moduleError = ""; moduleInput = ""; moduleParent = ""; creatingModule = true; }} />
  {:else if loaded}
    <div class="grid grid-cols-1 items-start gap-5 tab:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
      <nav class="flex flex-col gap-3" aria-label="Topics by module">
        <label class="input w-full">
          <Search class="size-4 opacity-70" />
          <input type="search" aria-label="Find a topic or module" placeholder="Find a topic or module…" bind:value={find} />
        </label>
        {#each groups as group (group.module?.id ?? "none")}
          <section class="panel p-2" aria-labelledby="module-{group.module?.id ?? 'none'}" style="margin-left: {group.depth * 16}px">
            <div class="flex items-center gap-2 px-2 py-1" use:rowMenu={() => (group.module ? { label: group.module.name, items: moduleItems(group.module.id, group.module.name) } : { label: "No module", items: [{ label: "New topic in it…", onSelect: () => openCreate(null) }] })}>
              <h2 id="module-{group.module?.id ?? 'none'}" class="flex-1 text-[15px] font-black" class:text-muted={group.module?.is_default} title={group.module ? (modulePath(group.module.id) ?? undefined) : undefined}>{group.module?.name ?? "No module"}</h2>
              <span class="text-xs text-muted">{plural(group.topics.length, "topic")}</span>
              {#if group.module}
                {@const id = group.module.id}
                {@const name = group.module.name}
                <RowMenu small label="{name} module actions" items={moduleItems(id, name)} />
              {/if}
            </div>
            {#each group.topics as topic (topic.id)}
              <button type="button" class="nav-link flex min-h-10 w-full items-center gap-2 px-3 text-left" aria-current={selected === topic.id ? "true" : undefined} use:rowMenu={topicMenu(topic)} onclick={() => (selected = topic.id)}>
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
              <p class="eyebrow">Topic in {modulePath(topic.module) ?? "no module"}</p>
              <h2 id="picked-h" class="mt-1 text-2xl font-black">{topic.name}</h2>
            </div>
            <button class="btn btn-outline btn-sm" onclick={() => (renaming = { kind: "topic", id: topic.id, name: topic.name })}><PencilLine class="size-4" />Rename</button>
            <button class="btn btn-outline btn-sm text-error" onclick={() => requestDelete(topic)}><Trash2 class="size-4" />Delete</button>
          </div>
          <div class="grid grid-cols-1 gap-4 tab:grid-cols-2">
            <label class="block"><span class="text-sm font-bold">Module</span>
              <select id="topic-module" class="select mt-1 w-full" value={topic.module === null ? "" : String(topic.module)} onchange={(event) => move(topic, event.currentTarget.value)}>
                {#if topic.module === null}<option value="">No module</option>{/if}
                {#each ordered as row (row.module.id)}<option value={String(row.module.id)}>{modulePath(row.module.id)}</option>{/each}
              </select></label>
            <div><label class="block"><span class="text-sm font-bold">Tags</span>
              <input class="input mt-1 w-full" value={topic.tags.join(", ")} placeholder="e.g. exam, lecture-5" aria-describedby="tags-help" onchange={(event) => saveTags(topic, event.currentTarget.value)} /></label>
              <span id="tags-help" class="mt-1 block text-xs text-muted">Separate tags with commas.</span></div>
          </div>
          <div>
            <h3 class="eyebrow">Related topics</h3>
            <p class="mt-1 text-sm text-muted">Linked, never merged. A quiz can add related topics when you ask it to.</p>
            <ul class="mt-2 flex flex-col gap-1">
              {#each relatedOf(topic) as other (other)}
                <li class="flex items-center gap-2"><Link2 class="size-4 text-info" aria-hidden="true" /><span class="flex-1">{topicPath(other)}</span>
                  <button class="btn btn-ghost btn-sm btn-square" aria-label="Unlink {topicPath(other)}" onclick={() => unlink(topic, other)}><X class="size-4" /></button></li>
              {/each}
            </ul>
            <label class="mt-2 block"><span class="sr-only">Link a related topic</span>
              <select class="select select-sm w-full" aria-label="Link a related topic" value="" onchange={(event) => { void link(topic, event.currentTarget.value); event.currentTarget.value = ""; }}>
                <option value="">Link a related topic…</option>
                {#each topics.filter((t) => t.id !== topic.id && !relatedOf(topic).includes(t.id)) as other (other.id)}<option value={String(other.id)}>{topicPath(other.id)}</option>{/each}
              </select></label>
          </div>
          <div>
            <h3 class="eyebrow">What's in it</h3>
            <div class="mt-2 grid grid-cols-1 gap-3 tab:grid-cols-3">
              <div class="panel flex flex-col gap-1 p-4">
                <span class="flex items-center gap-2 text-sm font-bold text-muted"><ListChecks class="size-4" aria-hidden="true" />Questions</span>
                <span class="text-3xl font-black tabular-nums">{questionsIn(topic)}</span>
                <button class="link mt-auto min-h-6 self-start text-sm font-bold" aria-label="Open in Quizzes" onclick={() => onEdit(topic.id)}>Open</button>
              </div>
              <div class="panel flex flex-col gap-1 p-4">
                <span class="flex items-center gap-2 text-sm font-bold text-muted"><SquareStack class="size-4" aria-hidden="true" />Cards</span>
                <span class="text-3xl font-black tabular-nums">{cardsIn(topic)}</span>
                <button class="link mt-auto min-h-6 self-start text-sm font-bold" aria-label="Open in Cards" onclick={() => onCards(topic.id)}>Open</button>
              </div>
              <div class="panel flex flex-col gap-1 p-4">
                <span class="flex items-center gap-2 text-sm font-bold text-muted"><Brain class="size-4" aria-hidden="true" />Checklists</span>
                <span class="text-3xl font-black tabular-nums">{checklistsIn(topic)}</span>
                <button class="link mt-auto min-h-6 self-start text-sm font-bold" aria-label="Open in Free Recall" onclick={onRecall}>Open</button>
              </div>
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
          {#each ordered.filter((row) => !row.module.is_default) as row (row.module.id)}<option value={String(row.module.id)}>{modulePath(row.module.id)}</option>{/each}
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
      <p class="mt-2 text-sm text-muted">A module groups related topics, like a course unit. Modules go up to {MAX_DEPTH} levels deep.</p>
      <label class="mt-4 block"><span class="text-sm font-semibold">Inside</span>
        <select class="select mt-1 w-full" bind:value={moduleParent}>
          <option value="">The top level</option>
          {#each ordered.filter((row) => !row.module.is_default && row.depth < MAX_DEPTH - 1) as row (row.module.id)}<option value={String(row.module.id)}>{modulePath(row.module.id)}</option>{/each}
        </select></label>
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
  <DeleteModuleDialog name={deletingModule.name} preview={deletingModule.preview} onConfirm={confirmDeleteModule} onCancel={() => (deletingModule = null)} />
{/if}

{#if defaultWarning}
  <dialog open class="modal modal-open" aria-labelledby="default-h">
    <div class="modal-box max-w-md">
      <h3 id="default-h" class="text-lg font-black">Put this topic in Default?</h3>
      <p class="mt-3">Creating this topic without selecting a module will put this under Default. Are you sure you want this?</p>
      <p class="mt-2 text-sm text-muted">Default is not the fundamental module. It is a root module like all your other first layer modules. It is just where topics with unnamed modules go. Your root modules are not children of default</p>
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (defaultWarning = false)}>No</button>
        <button type="button" class="btn btn-primary" onclick={() => create(new SubmitEvent("submit"))}>Yes</button>
      </div>
    </div>
  </dialog>
{/if}

{#if movingModule}
  <dialog open class="modal modal-open" aria-labelledby="move-module-h">
    <form class="modal-box max-w-md" onsubmit={moveModule}>
      <h3 id="move-module-h" class="text-lg font-semibold">Move {movingModule.name}</h3>
      <label class="mt-4 block"><span class="text-sm font-semibold">Inside</span>
        <select class="select mt-1 w-full" bind:value={movingModule.parent}>
          <option value="">The top level</option>
          {#each homesFor(movingModule.id) as row (row.module.id)}<option value={String(row.module.id)}>{modulePath(row.module.id)}</option>{/each}
        </select></label>
      {#if moveModuleError}<p role="alert" class="mt-4 text-sm text-error">{moveModuleError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (movingModule = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Move module</button>
      </div>
    </form>
  </dialog>
{/if}

{#if renaming}
  <RenameDialog title={renaming.kind === "module" ? "Rename module" : "Rename topic"} value={renaming.name} onSave={rename} onClose={() => (renaming = null)} />
{/if}

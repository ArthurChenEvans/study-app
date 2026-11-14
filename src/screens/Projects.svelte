<script lang="ts">
  import { stackTable } from "../lib/stack";
  // projects.feature (R87, R94): projects, and the issues every project shares.
  import { Box, Clock, Code, Dumbbell, FileUp, Hammer, Play, Plus } from "@lucide/svelte";
  import { api, errorMessage, type Id, type IssueView, type ProjectRow } from "../lib/api";
  import { toast } from "../lib/celebrations.svelte";
  import RowMenu from "../components/RowMenu.svelte";
  import RenameDialog from "../components/RenameDialog.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { dayLabel, dueLabel, pastOn } from "../lib/dates";

  let { onOpen = () => {}, onDrills = () => {} }: { onOpen?: (project: Id) => void; onDrills?: (ids: Id[]) => void } = $props();

  let projects = $state<ProjectRow[] | null>(null);
  let issues = $state<IssueView[]>([]);
  let today = $state(0);
  let tab = $state<"projects" | "issues">("projects");
  let creating = $state(false);
  let name = $state("");
  let kind = $state("");
  let error = $state("");
  let query = $state("");
  let kindFilter = $state("all");
  let issueQuery = $state("");
  let issueFilter = $state<"all" | "due" | "drilling" | "solid" | "none">("all");
  const KINDS = ["Coding", "Blender", "Past paper", "Writing"];

  // R185: editing and deleting an issue and its drill
  let editingDrill = $state<IssueView | null>(null);
  let renamingIssue = $state<IssueView | null>(null);
  let deletingDrill = $state<IssueView | null>(null);
  let deletingIssue = $state<IssueView | null>(null);

  const issueMenu = (issue: IssueView) => [
    ...(issue.drill ? [{ label: "Edit drill…", onSelect: () => (editingDrill = issue) }] : []),
    { label: "Rename issue…", onSelect: () => (renamingIssue = issue) },
    ...(issue.drill ? [{ label: "Delete drill…", danger: true, onSelect: () => (deletingDrill = issue) }] : []),
    { label: "Delete issue…", danger: true, onSelect: () => (deletingIssue = issue) },
  ];

  async function saveDrill(task: string, hints: string) {
    if (!editingDrill?.drill) return;
    await api.updateDrill(editingDrill.drill.id, task, hints);
    editingDrill = null;
    toast({ text: "Drill saved", sub: task, icon: "check" });
    await load();
  }

  async function saveIssue(name: string) {
    if (!renamingIssue) return;
    await api.renameIssue(renamingIssue.id, name);
    renamingIssue = null;
    toast({ text: "Issue renamed", sub: name, icon: "check" });
    await load();
  }

  async function removeDrill() {
    const drillId = deletingDrill?.drill?.id;
    deletingDrill = null;
    if (drillId === undefined) return;
    try {
      await api.deleteDrill(drillId);
      toast({ text: "Drill deleted", icon: "trash" });
      await load();
    } catch (failure) {
      toast({ text: "Couldn't delete the drill", sub: errorMessage(failure), icon: "alert", error: true });
    }
  }

  async function removeIssue() {
    if (!deletingIssue) return;
    const { id: issueId, name } = deletingIssue;
    deletingIssue = null;
    try {
      await api.deleteIssue(issueId);
      toast({ text: "Issue deleted", sub: name, icon: "trash" });
      await load();
    } catch (failure) {
      toast({ text: "Couldn't delete the issue", sub: errorMessage(failure), icon: "alert", error: true });
    }
  }

  async function load() {
    [projects, issues, today] = await Promise.all([api.projects(), api.issues(), api.today()]);
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      const id = await api.createProject(name, kind);
      toast({ text: "Log created", sub: name, icon: "plus" });
      creating = false;
      onOpen(id);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function openCreate() {
    name = "";
    kind = "";
    error = "";
    creating = true;
  }

  const drillState = (issue: IssueView) => {
    const drill = issue.drill;
    if (!drill) return "None";
    if (drill.due === null) return drill.last ? `Solid ${pastOn(drill.last.day, today)}` : "Ticked off";
    if (drill.due <= today) return drill.last && drill.last.rating !== "Solid" ? `Again today, was ${drill.last.rating}` : "Due today";
    return `Due ${dueLabel(drill.due, today)}`;
  };
  // Projects: the one you're in or worked on last comes first.
  // R190: a kind can nest with ::, like Coding::Rust, and the filter goes by the top of it.
  const topKind = (kind: string | null) => (kind ?? "").split("::")[0].trim();
  const kinds = $derived([...new Set((projects ?? []).map((project) => topKind(project.kind)).filter(Boolean))].sort());
  const sortedProjects = $derived([...(projects ?? [])].sort((a, b) => Number(!!b.session) - Number(!!a.session) || (b.last_day ?? -Infinity) - (a.last_day ?? -Infinity) || a.name.localeCompare(b.name)));
  const shownProjects = $derived(sortedProjects.filter((project) => (kindFilter === "all" || topKind(project.kind) === kindFilter) && project.name.toLowerCase().includes(query.trim().toLowerCase())));

  // Issues: a drill due today first, then the ones that keep coming back.
  const issueState = (issue: IssueView) => (!issue.drill ? "none" : issue.drill.due === null ? "solid" : issue.drill.due <= today ? "due" : "drilling");
  const issueCounts = $derived(issues.reduce((counts, issue) => ({ ...counts, [issueState(issue)]: (counts[issueState(issue)] ?? 0) + 1 }), {} as Record<string, number>));
  const issueFilters = [["due", "Drill due"], ["drilling", "Still drilling"], ["solid", "Solid"], ["none", "No drill"]] as const;
  const shownIssues = $derived([...issues]
    .sort((a, b) => Number(issueState(b) === "due") - Number(issueState(a) === "due") || b.seen - a.seen || (b.last_seen ?? -Infinity) - (a.last_seen ?? -Infinity))
    .filter((issue) => (issueFilter === "all" || issueState(issue) === issueFilter) && `${issue.name} ${issue.drill?.task ?? ""}`.toLowerCase().includes(issueQuery.trim().toLowerCase())));
  const dueDrills = $derived(issues.flatMap((issue) => (issue.drill && issue.drill.due !== null && issue.drill.due <= today ? [issue.drill.id] : [])));

  load();
  /** An icon for a project's kind, so the list reads at a glance. */
  function kindIcon(kind: string | null) {
    const name = (kind ?? "").toLowerCase();
    if (name.includes("paper") || name.includes("exam")) return FileUp;
    if (name.includes("cod") || name.includes("app") || name.includes("program")) return Code;
    if (name.includes("blender") || name.includes("model") || name.includes("3d")) return Box;
    return Hammer;
  }
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Practice Log" subtitle={projects?.length ? "Log what trips you up while you work, then review it at the end of the session." : ""}>
    {#if projects?.length}
      <button class="btn {dueDrills.length ? 'btn-ghost' : 'btn-primary'}" onclick={openCreate}><Plus class="size-4" />New log</button>
      {#if dueDrills.length}<button class="btn btn-primary" onclick={() => onDrills(dueDrills)}><Play class="size-4 fill-current" />Do {dueDrills.length === 1 ? "today's drill" : `today's ${dueDrills.length} drills`}</button>{/if}
    {/if}
  </PageHeader>

  {#if projects && projects.length === 0}
    <div class="panel max-w-xl p-10">
      <span class="well" aria-hidden="true"><Hammer class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">Log what trips you up while you work</h2>
      <p class="mt-2 max-w-prose text-muted">A practice log is for anything hands-on: code, a Blender scene, a past paper. While you work, note each struggle and why it wasn't obvious. At the end of the session, review them: a missed fact can become a card, and something you couldn't apply can become a drill.</p>
      <button class="btn btn-primary mt-6" onclick={openCreate}><Plus class="size-4" />New log</button>
    </div>
  {:else if projects}
    <div role="tablist" class="mb-6 flex gap-1 border-b border-base-300">
      {#each [["projects", "Logs", projects.length], ["issues", "Issues", issues.length]] as [key, label, count] (key)}
        <button role="tab" aria-selected={tab === key} class="px-3 py-2 text-sm {tab === key ? 'border-b-2 border-base-content font-semibold' : 'text-muted'}" onclick={() => (tab = key as typeof tab)}>{label}<span class="ml-1 text-xs font-normal text-muted">{count}</span></button>
      {/each}
    </div>
    {#if tab === "projects"}
      {#if projects.length > 5}
        <div class="mb-4 flex flex-wrap items-center gap-3">
          <input type="search" class="input input-sm w-80" aria-label="Find a log" placeholder="Find a log…" bind:value={query} />
          {#if kinds.length > 1}
            <div class="seg" role="radiogroup" aria-label="Kind">
              <button type="button" role="radio" aria-checked={kindFilter === "all"} onclick={() => (kindFilter = "all")}>All <span class="tabular-nums">{projects.length}</span></button>
              {#each kinds as name (name)}<button type="button" role="radio" aria-checked={kindFilter === name} onclick={() => (kindFilter = name)}>{name} <span class="tabular-nums">{projects.filter((project) => topKind(project.kind) === name).length}</span></button>{/each}
            </div>
          {/if}
        </div>
      {/if}
      <div class="panel overflow-hidden">
        <table class="table" aria-label="Practice logs" use:stackTable>
          <thead><tr class="text-muted"><th>Log</th><th aria-sort="descending">Last worked on</th><th>Struggles</th><th>To review</th><th>Drills due</th></tr></thead>
          <tbody>
            {#each shownProjects as project (project.id)}
              {@const KindIcon = kindIcon(project.kind)}
              <!-- The whole row opens the project: one button, stretched over the row. -->
              <tr class="link-row hover:bg-base-100">
                <td><span class="flex items-center gap-3"><span class="well" aria-hidden="true" style="--well: var(--color-info); --well-ink: var(--color-info)"><KindIcon class="size-5" /></span><span><button class="row-link py-1 text-[15.5px] font-extrabold" onclick={() => onOpen(project.id)}>{project.name}</button>{#if project.kind}<span class="block text-[13px] text-muted">{project.kind}</span>{/if}</span></span></td>
                <td class="text-sm">{project.session ? "Session running" : project.last_day === null ? "Not yet" : dayLabel(project.last_day, today)}</td>
                <td class="tabular-nums">{project.struggles}</td>
                <td>{#if project.to_review}<span class="todo-chip"><Clock class="size-3.5" aria-hidden="true" />{project.to_review} to review</span>{:else}<span class="text-muted">None</span>{/if}</td>
                <td>{#if project.drills_due}<span class="chip chip-due">{project.drills_due} today</span>{:else}<span class="text-muted">None</span>{/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if shownProjects.length === 0}<p class="px-6 py-6 text-sm text-muted">No logs match.</p>{/if}
      </div>
    {:else}
      <p class="mb-4 text-sm text-muted">Issues are shared by every log. When the same one comes up again in a review, its drill is offered.</p>
      {#if issues.length === 0}
        <p class="panel p-8 text-muted">No issues yet. They're named when you review a struggle you couldn't apply.</p>
      {:else}
        {#if issues.length > 5}
          <div class="mb-4 flex flex-wrap items-center gap-3">
            <input type="search" class="input input-sm w-80" aria-label="Find an issue" placeholder="Find an issue…" bind:value={issueQuery} />
            <div class="seg" role="radiogroup" aria-label="Show">
              <button type="button" role="radio" aria-checked={issueFilter === "all"} onclick={() => (issueFilter = "all")}>All <span class="tabular-nums">{issues.length}</span></button>
              {#each issueFilters as [key, label] (key)}{#if issueCounts[key]}<button type="button" role="radio" aria-checked={issueFilter === key} onclick={() => (issueFilter = key)}>{label} <span class="tabular-nums">{issueCounts[key]}</span></button>{/if}{/each}
            </div>
          </div>
        {/if}
        <div class="panel overflow-hidden">
          <table class="table" aria-label="Issues" use:stackTable>
            <thead><tr class="text-muted"><th>Issue</th><th aria-sort="descending">Seen</th><th>Last seen</th><th>Drill</th><th><span class="sr-only">Actions</span></th></tr></thead>
            <tbody>
              {#each shownIssues as issue (issue.id)}
                <tr class="hover:bg-base-100">
                  <td><span class="font-semibold">{issue.name}</span>{#if issue.drill}<div class="text-xs text-muted">{issue.drill.task}</div>{/if}</td>
                  <td class="text-sm"><b class="font-semibold tabular-nums">{issue.seen} {issue.seen === 1 ? "time" : "times"}</b> <span class="text-muted">in {issue.projects.length === 1 ? issue.projects[0] : `${issue.projects.length} logs`}</span></td>
                  <td class="text-sm">{issue.last_seen === null ? "Never" : dayLabel(issue.last_seen, today)}</td>
                  <td class="text-sm">
                    {#if issueState(issue) === "due"}
                      <span class="flex items-center gap-3"><span class="flex items-center gap-1.5 font-semibold"><span class="due-dot" aria-hidden="true"></span>{drillState(issue)}</span><button class="btn btn-outline btn-sm" aria-label="Do the drill for {issue.name}" onclick={() => onDrills([issue.drill!.id])}>Do drill</button></span>
                    {:else}<span class={issue.drill ? "" : "text-muted"}>{drillState(issue)}</span>{/if}
                  </td>
                  <td class="w-14 text-right"><RowMenu small label="{issue.name} actions" items={issueMenu(issue)} /></td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if shownIssues.length === 0}<p class="px-6 py-6 text-sm text-muted">No issues match.</p>{/if}
        </div>
      {/if}
    {/if}
  {/if}
</div>

{#if creating}
  <dialog open class="modal modal-open" aria-labelledby="new-project-h">
    <form class="modal-box max-w-md" onsubmit={create}>
      <h3 id="new-project-h" class="text-lg font-semibold">New practice log</h3>
      <p class="mt-1 text-sm text-muted">One log per piece of hands-on work, like a coding project, a Blender scene or a set of past papers.</p>
      <label class="mt-5 block"><span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={name} autofocus placeholder="e.g. Study app…" /></label>
      <!-- R190: the kind box always shows. A quick pick fills it, and :: nests a kind. -->
      <div class="mt-4">
        <label class="block" for="project-kind"><span class="text-sm font-semibold">Kind</span> <span class="text-sm font-normal text-muted">(optional)</span></label>
        <div class="mt-2 flex flex-wrap gap-2" role="group" aria-label="Common kinds">
          {#each KINDS as option (option)}
            <button type="button" class="kind-chip" aria-pressed={kind.trim() === option} onclick={() => (kind = kind.trim() === option ? "" : option)}>{option}</button>
          {/each}
        </div>
        <input id="project-kind" class="input mt-3 w-full font-mono" bind:value={kind} placeholder="e.g. Coding::Rust" aria-describedby="project-kind-help" />
        <p id="project-kind-help" class="mt-1.5 text-xs text-muted">Tap one above or type your own. Put :: between levels to nest it, like Coding::Rust. Then you can filter by all Coding logs or just Rust ones.</p>
      </div>
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create log</button>
      </div>
    </form>
  </dialog>
{/if}

{#if editingDrill?.drill}
  <RenameDialog title="Edit drill" label="Task" value={editingDrill.drill.task} extra={{ label: "Hints", value: editingDrill.drill.hints, placeholder: "Optional, shown if you get stuck" }} onSave={saveDrill} onClose={() => (editingDrill = null)} />
{/if}

{#if renamingIssue}
  <RenameDialog title="Rename issue" value={renamingIssue.name} onSave={saveIssue} onClose={() => (renamingIssue = null)} />
{/if}

{#if deletingDrill}
  <dialog open class="modal modal-open" aria-labelledby="delete-drill-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-drill-h" class="text-lg font-semibold">Delete this drill?</h3>
      <p class="mt-2 text-sm">"{deletingDrill.drill?.task}" and its past runs go. The issue stays, so a later review can make a new drill.</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deletingDrill = null)}>Keep it</button>
        <button class="btn btn-error" onclick={removeDrill}>Delete drill</button>
      </div>
    </div>
  </dialog>
{/if}

{#if deletingIssue}
  <dialog open class="modal modal-open" aria-labelledby="delete-issue-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-issue-h" class="text-lg font-semibold">Delete the issue {deletingIssue.name}?</h3>
      <p class="mt-2 text-sm">Struggles that named it keep going, without an issue.{deletingIssue.drill ? " Its drill and the drill's past runs go too." : ""}</p>
      <div class="modal-action">
        <button class="btn btn-ghost" onclick={() => (deletingIssue = null)}>Keep it</button>
        <button class="btn btn-error" onclick={removeIssue}>Delete issue</button>
      </div>
    </div>
  </dialog>
{/if}

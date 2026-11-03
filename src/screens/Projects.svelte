<script lang="ts">
  // projects.feature (R87, R94): projects, and the issues every project shares.
  import { Hammer, Plus } from "@lucide/svelte";
  import { api, errorMessage, type Id, type IssueView, type ProjectRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import { dayLabel, dueLabel } from "../lib/dates";

  let { onOpen = () => {}, onDrills = () => {} }: { onOpen?: (project: Id) => void; onDrills?: (ids: Id[]) => void } = $props();

  let projects = $state<ProjectRow[] | null>(null);
  let issues = $state<IssueView[]>([]);
  let today = $state(0);
  let tab = $state<"projects" | "issues">("projects");
  let creating = $state(false);
  let name = $state("");
  let kind = $state("");
  let error = $state("");

  async function load() {
    [projects, issues, today] = await Promise.all([api.projects(), api.issues(), api.today()]);
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      const id = await api.createProject(name, kind);
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
    if (drill.due === null) return drill.last ? `Solid on ${dayLabel(drill.last.day, today)}` : "Ticked off";
    if (drill.due <= today) return drill.last && drill.last.rating !== "Solid" ? `Again today, was ${drill.last.rating}` : "Due today";
    return `Due ${dueLabel(drill.due, today)}`;
  };
  const dueDrills = $derived(issues.flatMap((issue) => (issue.drill && issue.drill.due !== null && issue.drill.due <= today ? [issue.drill.id] : [])));

  load();
</script>

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Projects" subtitle={projects?.length ? "Log what trips you up while you work, then review it at the end of the session." : ""}>
    {#if projects?.length}
      {#if dueDrills.length}<button class="btn btn-ghost" onclick={() => onDrills(dueDrills)}>Do {dueDrills.length === 1 ? "today's drill" : `today's ${dueDrills.length} drills`}</button>{/if}
      <button class="btn btn-primary" onclick={openCreate}><Plus class="size-4" />New project</button>
    {/if}
  </PageHeader>

  {#if projects && projects.length === 0}
    <div class="panel max-w-xl p-10">
      <span class="grid size-10 place-items-center rounded-lg bg-base-200"><Hammer class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">Log what trips you up while you work</h2>
      <p class="mt-2 max-w-prose text-muted">A project is anything applied: code, a Blender scene, a past paper. While you work, note each struggle and why it wasn't obvious. At the end of the session, review them: a missed fact can become a card, and something you couldn't apply can become a drill.</p>
      <button class="btn btn-primary mt-6" onclick={openCreate}><Plus class="size-4" />New project</button>
    </div>
  {:else if projects}
    <div role="tablist" class="mb-6 flex gap-1 border-b border-base-300">
      {#each [["projects", "Projects", projects.length], ["issues", "Issues", issues.length]] as [key, label, count] (key)}
        <button role="tab" aria-selected={tab === key} class="px-3 py-2 text-sm {tab === key ? 'border-b-2 border-base-content font-semibold' : 'text-muted'}" onclick={() => (tab = key as typeof tab)}>{label}<span class="ml-1 text-xs font-normal text-muted">{count}</span></button>
      {/each}
    </div>
    {#if tab === "projects"}
      <div class="panel overflow-hidden">
        <table class="table" aria-label="Projects">
          <thead><tr class="text-muted"><th>Project</th><th>Last worked on</th><th>Struggles</th><th>To review</th><th>Drills due</th></tr></thead>
          <tbody>
            {#each projects as project (project.id)}
              <tr class="hover:bg-base-100">
                <td><button class="py-1 text-left font-semibold hover:underline" onclick={() => onOpen(project.id)}>{project.name}</button>{#if project.kind}<div class="text-xs text-muted">{project.kind}</div>{/if}</td>
                <td class="text-sm">{project.session ? "Session running" : project.last_day === null ? "Not yet" : dayLabel(project.last_day, today)}</td>
                <td class="tabular-nums">{project.struggles}</td>
                <td class="tabular-nums {project.to_review ? 'font-semibold' : 'text-muted'}">{project.to_review || "None"}</td>
                <td class="tabular-nums {project.drills_due ? '' : 'text-muted'}">{project.drills_due || "None"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="mb-3 text-sm text-muted">Issues are shared by every project. When the same one comes up again in a review, its drill is offered.</p>
      {#if issues.length === 0}
        <p class="panel p-8 text-muted">No issues yet. They're named when you review a struggle you couldn't apply.</p>
      {:else}
        <div class="panel overflow-hidden">
          <table class="table" aria-label="Issues">
            <thead><tr class="text-muted"><th>Issue</th><th>Seen</th><th>Last seen</th><th>Drill</th></tr></thead>
            <tbody>
              {#each issues as issue (issue.id)}
                <tr class="hover:bg-base-100">
                  <td><span class="font-semibold">{issue.name}</span>{#if issue.drill}<div class="text-xs text-muted">{issue.drill.task}</div>{/if}</td>
                  <td class="text-sm">{issue.seen} {issue.seen === 1 ? "time" : "times"} in {issue.projects.length === 1 ? issue.projects[0] : `${issue.projects.length} projects`}</td>
                  <td class="text-sm">{issue.last_seen === null ? "Never" : dayLabel(issue.last_seen, today)}</td>
                  <td class="text-sm {issue.drill ? '' : 'text-muted'}">{drillState(issue)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  {/if}
</div>

{#if creating}
  <dialog open class="modal modal-open" aria-labelledby="new-project-h">
    <form class="modal-box max-w-md" onsubmit={create}>
      <h3 id="new-project-h" class="text-lg font-semibold">New project</h3>
      <label class="mt-6 block"><span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={name} autofocus placeholder="e.g. Study app…" /></label>
      <label class="mt-4 block"><span class="text-sm font-semibold">Kind <span class="font-normal text-muted">(optional)</span></span>
        <input class="input mt-1 w-full" list="project-kinds" bind:value={kind} placeholder="Coding, Blender, Past paper…" /></label>
      <datalist id="project-kinds"><option value="Coding"></option><option value="Blender"></option><option value="Past paper"></option><option value="Writing"></option></datalist>
      {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Create project</button>
      </div>
    </form>
  </dialog>
{/if}

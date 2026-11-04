<script lang="ts">
  // projects.feature (R90 to R92, R95): review a session's struggles one at a time.
  import { CalendarDays, CircleCheck, Clock, Dumbbell, ListChecks, Minus, Plus, Repeat, SquareStack } from "@lucide/svelte";
  import { api, errorMessage, type DrillView, type Id, type IssueView, type RecallRow, type ReviewChoice, type StruggleView, type TopicRow } from "../lib/api";
  import { dayFromIso, dueLabel, fullLabel, isoOf, onDay, pastOn } from "../lib/dates";

  let { project, onDone = () => {}, onSuggestions = () => {}, onCalendar = () => {} }: { project: Id; onDone?: () => void; onSuggestions?: () => void; onCalendar?: () => void } = $props();

  type Action = "card" | "checklist" | "drill" | "nothing";
  let queue = $state<StruggleView[] | null>(null);
  let index = $state(0);
  let checklists = $state<RecallRow[]>([]);
  let topics = $state<TopicRow[]>([]);
  let issues = $state<IssueView[]>([]);
  let today = $state(0);
  let projectName = $state("");
  let done = $state<StruggleView[]>([]);

  let action = $state<Action | null>(null);
  let checklist = $state("");
  let issue = $state("");
  let known = $state<DrillView | null>(null);
  let drillMode = $state<"again" | "new">("new");
  let task = $state("");
  let hints = $state("");
  let topic = $state("");
  let asking = $state(false);
  let when = $state<"tomorrow" | "other">("tomorrow");
  let otherDay = $state("");
  let error = $state("");

  const current = $derived(queue?.[index] ?? null);
  const kindLabel = { Recall: "Couldn't recall it", Apply: "Couldn't apply it", Unsure: "Not sure yet" } as const;
  const time = (minute: number) => `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;

  async function load() {
    const [rows, recall, topicRows, issueRows, day, view] = await Promise.all([api.strugglesToReview(project), api.recallList(), api.topics(), api.issues(), api.today(), api.project(project)]);
    [checklists, topics, issues, today, projectName] = [recall, topicRows, issueRows, day, view.row.name];
    queue = rows;
    prepare();
  }

  function prepare() {
    error = "";
    asking = false;
    const struggle = queue?.[index];
    if (!struggle) return;
    action = struggle.kind === "Recall" ? "card" : struggle.kind === "Apply" ? "drill" : null;
    checklist = checklists.find((c) => c.subject_name === struggle.topic_name)?.id.toString() ?? checklists[0]?.id.toString() ?? "";
    issue = struggle.issue ?? "";
    known = null;
    drillMode = "new";
    task = "";
    hints = "";
    topic = struggle.topic?.toString() ?? "";
    when = "tomorrow";
    otherDay = isoOf(today + 2);
  }

  let lookup = 0;
  async function findDrill() {
    const asked = ++lookup;
    const found = issue.trim() ? await api.knownDrill(issue) : null;
    if (asked !== lookup) return;
    known = found;
    drillMode = found ? "again" : "new";
  }
  const seenBefore = $derived(issues.find((row) => row.name.toLowerCase().replace(/\s+/g, " ") === issue.trim().toLowerCase().replace(/\s+/g, " ")));

  function choice(day: number): ReviewChoice | null {
    if (action === "card") return "Card";
    if (action === "nothing") return "Nothing";
    if (action === "checklist") return checklist ? { Checklist: { checklist: Number(checklist) } } : null;
    if (action === "drill") {
      if (drillMode === "again" && known) return { DrillAgain: { issue, drill: known.id, day } };
      return { NewDrill: { issue, task, hints, topic: topic ? Number(topic) : null, day } };
    }
    return null;
  }

  async function next(event?: Event) {
    event?.preventDefault();
    if (!current || !action) return;
    if (action === "drill" && !asking) {
      if (!issue.trim()) return (error = "Name the issue, so you'd recognise it next time");
      if (drillMode === "new" && !task.trim()) return (error = "Write the drill: a small task that practises this");
      error = "";
      asking = true;
      return;
    }
    const day = when === "tomorrow" ? today + 1 : dayFromIso(otherDay);
    const picked = choice(day);
    if (!picked) return (error = action === "checklist" ? "Pick a checklist" : "Choose what should happen with it");
    try {
      done = [...done, await api.reviewStruggle(current.id, picked)];
      issues = await api.issues();
      index += 1;
      prepare();
    } catch (failure) {
      error = errorMessage(failure);
      asking = false;
    }
  }

  const choices = [
    ["card", SquareStack, "Suggest a card", "Waits in Suggestions for you to accept"],
    ["checklist", ListChecks, "Add to a recall checklist", "It becomes a point to recall"],
    ["drill", Dumbbell, "Schedule a drill", "For something you couldn't apply"],
    ["nothing", Minus, "Nothing", "Just keep it logged"],
  ] as const;

  function keys(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter" && current && !asking) {
      event.preventDefault();
      next();
      return;
    }
    // 1 to 4 pick a choice, unless you're typing.
    const typing = (event.target as HTMLElement | null)?.closest("input, textarea, select");
    if (!typing && !asking && current && /^[1-4]$/.test(event.key) && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      action = choices[Number(event.key) - 1][0];
      error = "";
    }
  }

  const drills = $derived(done.filter((s) => s.outcome !== null && typeof s.outcome === "object" && "Drill" in s.outcome));
  const cards = $derived(done.filter((s) => s.outcome === "Card").length);
  const points = $derived(done.filter((s) => s.outcome !== null && typeof s.outcome === "object" && "Checklist" in s.outcome).length);

  load();
</script>

<svelte:window onkeydown={keys} />

{#if queue}
  <div class="sticky top-0 z-10 flex items-center justify-between gap-6 border-b border-base-300 bg-base-100 px-12 py-4">
    <div class="flex items-center gap-3 text-sm"><span class="text-muted">{projectName}</span><span class="text-muted" aria-hidden="true">›</span><span class="font-semibold">Session review</span></div>
    {#if current}<p class="text-sm tabular-nums" aria-live="polite">Struggle <b>{index + 1}</b> <span class="text-muted">of {queue.length}</span></p>{/if}
    {#if current}<button class="btn btn-ghost btn-sm font-normal" onclick={onDone}><Clock class="size-4" />Finish later</button>{:else}<span></span>{/if}
  </div>

  {#if current}
    <form class="mx-auto max-w-3xl px-12 pb-24 pt-8" onsubmit={next}>
      <div class="space-y-6">
        <section class="panel p-6" aria-label="The struggle">
          <div class="flex items-center gap-2"><span class="kind">{kindLabel[current.kind]}</span><span class="text-sm text-muted">Logged {current.day === today ? "" : `${pastOn(current.day, today).replace(/^on /, "")} `}at {time(current.minute)}</span>{#if current.topic_name}<span class="badge-src">{current.topic_name}</span>{/if}</div>
          <p class="mt-3 text-lg">{current.what}</p>
          {#if current.why}<p class="mt-1 text-muted">Why it wasn't obvious: {current.why}</p>{/if}
        </section>

        <fieldset>
          <legend class="mb-3 text-sm font-semibold">What should happen with it?</legend>
          <div class="grid grid-cols-2 gap-3">
            {#each choices as [key, Icon, title, sub], position (key)}
              <label class="flex min-h-12 cursor-pointer items-start gap-3 rounded-lg border bg-surface px-4 py-3 {action === key ? 'border-2 border-primary bg-base-200' : 'border-field-border'}">
                <input type="radio" name="action" class="radio radio-sm mt-0.5" checked={action === key} onchange={() => { action = key; error = ""; }} />
                <span class="flex-1"><span class="flex items-center gap-2 font-semibold"><Icon class="size-4" />{title}</span><span class="mt-0.5 block text-sm text-muted">{sub}</span></span>
                <kbd class="kbd kbd-xs" aria-hidden="true">{position + 1}</kbd>
              </label>
            {/each}
          </div>
        </fieldset>

        {#if action === "checklist"}
          <label class="block"><span class="block text-sm font-semibold">Checklist</span>
            {#if checklists.length}<select class="select mt-1 w-full" bind:value={checklist}>{#each checklists as row (row.id)}<option value={String(row.id)}>{row.subject_name}</option>{/each}</select>
            {:else}<span class="mt-1 block text-sm text-muted">You don't have any checklists yet. Make one in Recall first.</span>{/if}</label>
        {/if}

        {#if action === "drill"}
          <div>
            <label class="block text-sm font-semibold" for="review-issue">Issue</label>
            <input id="review-issue" class="input mt-1 w-full" list="known-issues" placeholder="Name the issue, or pick one you've had before…" bind:value={issue} onchange={findDrill} onblur={findDrill} />
            <datalist id="known-issues">{#each issues as row (row.id)}<option value={row.name}></option>{/each}</datalist>
            <p class="mt-1 text-sm text-muted">{seenBefore ? `Seen ${seenBefore.seen} ${seenBefore.seen === 1 ? "time" : "times"} before, in ${seenBefore.projects.join(" and ")}.` : "Issues are shared by every project, so name it the way you'd recognise it again."}</p>
          </div>
          {#if known}
            <div class="panel flex items-start gap-4 p-5" role="note"><Dumbbell class="mt-0.5 size-5 shrink-0" />
              <div class="flex-1"><p class="font-semibold">You have a drill for this issue</p>
                <p class="mt-1 text-sm text-muted">"{known.task}". {known.last ? `Last done ${pastOn(known.last.day, today)}, rated ${known.last.rating}.` : known.due !== null ? `Already due ${dueLabel(known.due, today)}.` : "Not done yet."}</p>
                <div class="mt-4 flex flex-wrap gap-2">
                  <button type="button" class="btn btn-sm {drillMode === 'again' ? 'btn-outline border-primary' : 'btn-ghost'}" aria-pressed={drillMode === "again"} onclick={() => (drillMode = "again")}><Repeat class="size-4" />Schedule this drill again</button>
                  <button type="button" class="btn btn-sm {drillMode === 'new' ? 'btn-outline border-primary' : 'btn-ghost'}" aria-pressed={drillMode === "new"} onclick={() => (drillMode = "new")}><Plus class="size-4" />Do a different drill</button>
                </div></div></div>
          {/if}
          {#if drillMode === "new"}
            <div class="space-y-4">
              <div><label class="block text-sm font-semibold" for="review-task">The drill</label>
                <textarea id="review-task" class="textarea mt-1 w-full text-[15px]" rows="2" placeholder="A small task that practises exactly this…" bind:value={task}></textarea>
                <p class="mt-1 text-sm text-muted">Small enough for one sitting. Solid ticks it off, Struggled or Shaky brings it back tomorrow.</p></div>
              <div class="grid grid-cols-2 gap-6">
                <div><label class="block text-sm font-semibold" for="review-topic">Topic <span class="font-normal text-muted">(optional)</span></label>
                  <select id="review-topic" class="select mt-1 w-full" bind:value={topic}><option value="">No topic</option>{#each topics as row (row.id)}<option value={String(row.id)}>{row.name}</option>{/each}</select></div>
                <div><label class="block text-sm font-semibold" for="review-hints">Hints <span class="font-normal text-muted">(optional)</span></label>
                  <input id="review-hints" class="input mt-1 w-full" placeholder="Shown only if you ask while drilling…" bind:value={hints} /></div>
              </div>
            </div>
          {/if}
        {/if}
      </div>
      {#if error}<p role="alert" class="mt-6 text-sm text-error">{error}</p>{/if}
      <div class="mt-10 flex items-center justify-between gap-3 border-t border-base-300 pt-6">
        <p class="text-sm text-muted">Each one is saved as you go.</p>
        <button type="submit" class="btn btn-primary" disabled={!action}>{action === "drill" ? "Schedule the drill" : index + 1 === queue.length ? "Finish review" : "Next struggle"} <kbd class="kbd kbd-xs">Ctrl</kbd><kbd class="kbd kbd-xs">Enter</kbd></button>
      </div>
    </form>

    {#if asking}
      <dialog open class="modal modal-open" aria-labelledby="when-h">
        <form class="modal-box max-w-md" onsubmit={next}>
          <h3 id="when-h" class="text-lg font-semibold">When do you want to do it?</h3>
          <p class="mt-2 text-sm text-muted">{drillMode === "again" && known ? known.task : task}</p>
          <div class="mt-5 space-y-3">
            <label class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg border px-4 py-2 {when === 'tomorrow' ? 'border-primary' : 'border-field-border'}"><input type="radio" name="when" class="radio radio-sm" checked={when === "tomorrow"} onchange={() => (when = "tomorrow")} /><span class="flex-1">Tomorrow, {fullLabel(today + 1, today)}</span></label>
            <label class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg border px-4 py-2 {when === 'other' ? 'border-primary' : 'border-field-border'}"><input type="radio" name="when" class="radio radio-sm" checked={when === "other"} onchange={() => (when = "other")} /><span>Another day</span>
              <input type="date" class="input input-sm ml-auto w-40" aria-label="Pick a day" min={isoOf(today)} bind:value={otherDay} onfocus={() => (when = "other")} /></label>
          </div>
          <div class="modal-action">
            <button type="button" class="btn btn-ghost" onclick={() => (asking = false)}>Back</button>
            <button type="submit" class="btn btn-primary">Schedule for {when === "tomorrow" ? "tomorrow" : otherDay ? fullLabel(dayFromIso(otherDay), today) : "that day"}</button>
          </div>
        </form>
      </dialog>
    {/if}
  {:else}
    <div class="mx-auto max-w-3xl px-12 pb-24 pt-12">
      <h1 class="flex items-center gap-3 text-3xl font-semibold"><CircleCheck class="size-7" />{done.length ? "Session reviewed" : "Nothing to review"}</h1>
      {#if done.length}
        <ul class="panel mt-8 divide-y" aria-label="What happens next">
          {#if cards}<li class="flex items-center gap-4 px-6 py-4"><SquareStack class="size-4" /><span class="flex-1">{cards} card {cards === 1 ? "suggestion" : "suggestions"}, waiting in Suggestions</span><button class="btn btn-ghost btn-sm" onclick={onSuggestions}>Open Suggestions</button></li>{/if}
          {#each drills as struggle (struggle.id)}
            {@const outcome = struggle.outcome as { Drill: { id: Id; due: number | null } }}
            <li class="flex items-center gap-4 px-6 py-4"><Dumbbell class="size-4" /><span class="flex-1">Drill for "{struggle.issue}" {outcome.Drill.due === null ? "" : onDay(outcome.Drill.due, today)}</span><button class="btn btn-ghost btn-sm" onclick={onCalendar}><CalendarDays class="size-4" />See in calendar</button></li>
          {/each}
          {#if points}<li class="flex items-center gap-4 px-6 py-4"><ListChecks class="size-4" /><span class="flex-1">{points} {points === 1 ? "point" : "points"} added to recall checklists</span></li>{/if}
          {#if done.length - cards - drills.length - points > 0}<li class="flex items-center gap-4 px-6 py-4"><Minus class="size-4" /><span class="flex-1">{done.length - cards - drills.length - points} kept as notes</span></li>{/if}
        </ul>
      {/if}
      <div class="mt-8 flex justify-end"><button class="btn btn-primary" onclick={onDone}>Back to {projectName}</button></div>
    </div>
  {/if}
{/if}

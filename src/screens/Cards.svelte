<script lang="ts">
  // flashcards.feature (R68, R74, R75): what's due today, the forecast, retention, per topic.
  import { CircleCheck, Inbox, Play, Plus, RefreshCw, RotateCcw, Settings2, SquareStack } from "@lucide/svelte";
  import { api, errorMessage, type CardSettings, type CardsHome, type ModuleRow, type Subject, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import { dueLabel } from "../lib/dates";

  let { onReview, onTopic, onSuggestions, onNewCard }: { onReview: (scope: Subject | null) => void; onTopic: (topic: number) => void; onSuggestions: () => void; onNewCard: () => void } = $props();

  let home = $state<CardsHome | null>(null);
  let today = $state(0);
  let modules = $state<ModuleRow[]>([]);
  let topics = $state<TopicRow[]>([]);
  let scope = $state("");
  let editing = $state<CardSettings | null>(null);
  let noLimit = $state(false);
  let settingsError = $state("");
  let fitStatus = $state("");
  let fitting = $state(false);

  const total = $derived(home ? home.due.new + home.due.learning + home.due.review : 0);
  const peak = $derived(home ? Math.max(1, ...home.forecast) : 1);
  const peakDay = $derived(home ? home.forecast.indexOf(Math.max(...home.forecast.slice(1))) : -1);

  async function load() {
    [home, today, modules, topics] = await Promise.all([api.cardsHome(), api.today(), api.modules(), api.topics()]);
  }

  function start() {
    if (!scope) return onReview(null);
    const [kind, id] = scope.split(":");
    onReview(kind === "module" ? { Module: Number(id) } : { Topic: Number(id) });
  }

  async function openSettings() {
    editing = await api.cardSettings();
    noLimit = editing.max_reviews === null;
    settingsError = "";
    fitStatus = "";
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!editing) return;
    try {
      await api.setCardSettings({ ...$state.snapshot(editing), max_reviews: noLimit ? null : Number(editing.max_reviews ?? 0), new_per_day: Number(editing.new_per_day), leech_after: Number(editing.leech_after) });
      editing = null;
      await load();
    } catch (failure) {
      settingsError = errorMessage(failure);
    }
  }

  async function reset() {
    editing = await api.resetCardSettings();
    noLimit = editing.max_reviews === null;
  }

  async function fit() {
    fitting = true;
    fitStatus = "";
    try {
      const used = await api.fitScheduler();
      fitStatus = `Fitted to ${used.toLocaleString()} reviews.`;
      if (editing) editing.parameters = (await api.cardSettings()).parameters;
    } catch (failure) {
      fitStatus = errorMessage(failure);
    } finally {
      fitting = false;
    }
  }

  load();
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Enter" && !editing && total > 0 && (event.target as HTMLElement).tagName === "BODY") start(); }} />

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="Cards" subtitle={home && home.total_cards ? (total ? `${total} due today` : "All reviewed for today") : ""}>
    {#if home?.total_cards}
      <button class="btn btn-ghost" onclick={openSettings}><Settings2 class="size-4" />Settings</button>
      <button class="btn btn-ghost" onclick={onNewCard}><Plus class="size-4" />New card</button>
    {/if}
  </PageHeader>

  {#if home && home.total_cards === 0}
    <div class="panel max-w-xl p-10">
      <span class="grid size-10 place-items-center rounded-lg bg-base-200"><SquareStack class="size-5" /></span>
      <h2 class="mt-6 text-xl font-semibold">No cards yet</h2>
      <p class="mt-2 max-w-prose text-muted">Cards are for the things you keep forgetting. Write them in a topic, or make them from what you missed in a recall or got wrong in a quiz. The app picks which ones to show each day, so you only see a card just before you'd forget it.</p>
      <div class="mt-6 flex gap-3">
        <button class="btn btn-primary" onclick={onNewCard}><Plus class="size-4" />Write your first card</button>
        {#if home.suggestions}<button class="btn btn-outline border-field-border" onclick={onSuggestions}><Inbox class="size-4" />{home.suggestions} {home.suggestions === 1 ? "suggestion" : "suggestions"}</button>{/if}
      </div>
    </div>
  {:else if home}
    <div class="grid grid-cols-[1fr_340px] items-start gap-8">
      <section class="panel p-8" aria-labelledby="today-h">
        <h2 id="today-h" class="eyebrow">Today</h2>
        {#if total === 0}
          <div class="mt-4 flex items-center gap-4"><CircleCheck class="size-6" /><div><p class="text-2xl font-semibold">Done for today</p><p class="mt-1 text-muted">{home.forecast[1]} due tomorrow.</p></div></div>
        {:else}
          <p class="mt-4 text-5xl font-semibold leading-none tabular-nums">{total} <span class="text-lg font-normal text-muted">{total === 1 ? "card" : "cards"}</span></p>
          <p class="mt-4 flex flex-wrap items-center gap-3 text-sm" aria-label="{home.due.new} new, {home.due.learning} learning, {home.due.review} to review">
            <span class="tabular-nums"><span class="font-semibold">{home.due.new}</span> <span class="text-muted">new</span></span><span class="text-muted" aria-hidden="true">·</span>
            <span class="tabular-nums"><span class="font-semibold">{home.due.learning}</span> <span class="text-muted">learning</span></span><span class="text-muted" aria-hidden="true">·</span>
            <span class="tabular-nums"><span class="font-semibold">{home.due.review}</span> <span class="text-muted">to review</span></span>
          </p>
          <p class="mt-2 text-sm text-muted">About {home.minutes} {home.minutes === 1 ? "minute" : "minutes"}.</p>
          <div class="mt-8 flex items-center gap-3">
            <button class="btn btn-primary btn-lg" onclick={start}><Play class="size-4" />Start review <kbd class="kbd kbd-xs">Enter</kbd></button>
            <select class="select w-56" aria-label="Review which topics" bind:value={scope}>
              <option value="">All topics</option>
              {#if modules.length}<optgroup label="Modules">{#each modules as module (module.id)}<option value="module:{module.id}">{module.name}</option>{/each}</optgroup>{/if}
              <optgroup label="Topics">{#each home.per_topic as row (row.topic)}<option value="topic:{row.topic}">{row.name}</option>{/each}</optgroup>
            </select>
          </div>
        {/if}
        <div class="mt-10">
          <div class="flex items-baseline justify-between"><h3 class="text-sm font-semibold">Next 14 days</h3><span class="text-xs text-muted">cards due per day</span></div>
          <div class="forecast mt-3" role="img" aria-label="Cards due over the next 14 days: {home.forecast.join(', ')}">
            {#each home.forecast as count, index (index)}<span style="height: {(count / peak) * 100}%" title="{dueLabel(today + index, today)}: {count} {count === 1 ? 'card' : 'cards'}"></span>{/each}
          </div>
          <div class="mt-1 grid text-xs text-muted" style="grid-template-columns: repeat(14, 1fr)">
            <span class="col-span-3">Today</span>
            {#if peakDay > 2 && peakDay < 11 && home.forecast[peakDay] > 0}<span class="col-span-4 tabular-nums" style="grid-column-start: {peakDay + 1}">{dueLabel(today + peakDay, today)} · {home.forecast[peakDay]}</span>{/if}
            <span class="col-span-3 text-right" style="grid-column-start: 12">{dueLabel(today + 13, today)}</span>
          </div>
        </div>
      </section>
      <aside class="space-y-6">
        <section class="panel p-6" aria-labelledby="sugg-h">
          <div class="flex items-center justify-between"><h2 id="sugg-h" class="flex items-center gap-2 text-sm font-semibold"><Inbox class="size-4" />Suggestions</h2><span class="text-sm tabular-nums text-muted">{home.suggestions}</span></div>
          <p class="mt-2 text-sm text-muted">{home.suggestions ? "Points you missed in recalls and questions you got wrong or guessed could become cards." : "Nothing to suggest. Misses from recalls and quizzes show up here."}</p>
          {#if home.suggestions}<button class="btn btn-outline btn-sm mt-4 border-field-border" onclick={onSuggestions}>Look through them</button>{/if}
        </section>
        <section class="panel p-6" aria-labelledby="ret-h">
          <h2 id="ret-h" class="text-sm font-semibold">Retention, last 30 days</h2>
          <p class="mt-2"><span class="text-3xl font-semibold tabular-nums">{home.retention === null ? "None yet" : `${home.retention}%`}</span> <span class="text-sm text-muted">aiming for {home.target}%</span></p>
          <p class="mt-1 text-sm text-muted">Share of reviews you got without pressing Again.</p>
        </section>
      </aside>
    </div>

    <section class="mt-12" aria-labelledby="by-topic-h">
      <h2 id="by-topic-h" class="eyebrow mb-3">By topic</h2>
      <div class="panel overflow-hidden">
        <table class="table table-fixed" aria-label="Cards by topic">
          <colgroup><col class="w-[30%]" /><col class="w-[14%]" /><col class="w-[14%]" /><col class="w-[14%]" /><col class="w-[14%]" /><col class="w-[14%]" /></colgroup>
          <thead><tr class="text-muted"><th>Topic</th><th>New</th><th>Learning</th><th>To review</th><th>Cards</th><th>Retention</th></tr></thead>
          <tbody>
            {#each home.per_topic as row (row.topic)}
              <tr class="hover:bg-base-100">
                <td><button class="py-0.5 text-left font-semibold hover:underline" onclick={() => onTopic(row.topic)}>{row.name}</button><div class="text-xs text-muted">{row.module ?? "No module"}</div></td>
                <td class="tabular-nums">{row.due.new}</td><td class="tabular-nums">{row.due.learning}</td><td class="tabular-nums">{row.due.review}</td>
                <td class="tabular-nums">{row.total}</td><td class="tabular-nums">{row.retention === null ? "None yet" : `${row.retention}%`}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if topics.length > home.per_topic.length}<p class="mt-3 text-sm text-muted">Topics with no cards yet aren't listed. Open a topic from Topics to add cards to it.</p>{/if}
    </section>
  {/if}
</div>

{#if editing}
  <dialog open class="modal modal-open" aria-labelledby="settings-h">
    <form class="modal-box max-w-xl" onsubmit={save}>
      <h3 id="settings-h" class="text-lg font-semibold">Card settings</h3>
      <div class="mt-6 space-y-6">
        <div>
          <label class="block text-sm font-semibold" for="retention">How much to remember</label>
          <div class="mt-2 flex items-center gap-4">
            <input id="retention" type="range" min="0.7" max="0.99" step="0.01" class="slider flex-1" style="--fill: {((editing.desired_retention - 0.7) / 0.29) * 100}%" bind:value={editing.desired_retention} />
            <span class="w-12 text-right font-semibold tabular-nums">{Math.round(editing.desired_retention * 100)}%</span>
          </div>
          <p class="mt-1 text-sm text-muted">The chance you still remember a card when it comes back. Higher means more reviews a day. 90% is the usual choice.</p>
        </div>
        <div class="grid grid-cols-2 gap-6">
          <label class="block"><span class="block text-sm font-semibold">New cards a day</span><input class="input mt-1 w-24 tabular-nums" type="number" min="0" bind:value={editing.new_per_day} /></label>
          <div>
            <label class="block"><span class="block text-sm font-semibold">Most reviews a day</span><input class="input mt-1 w-24 tabular-nums" type="number" min="0" aria-label="Most reviews a day" bind:value={editing.max_reviews} disabled={noLimit} /></label>
            <label class="mt-2 flex items-center gap-2 text-sm"><input type="checkbox" class="checkbox checkbox-sm" bind:checked={noLimit} />No limit</label>
          </div>
        </div>
        <label class="block"><span class="block text-sm font-semibold">Ask me to rewrite a card after</span>
          <span class="mt-1 flex items-center gap-2"><input class="input w-20 tabular-nums" type="number" min="2" aria-label="Ask me to rewrite a card after" bind:value={editing.leech_after} /><span class="text-sm text-muted">times pressing Again</span></span></label>
        <div class="panel p-4">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-sm font-semibold">Fit the scheduler to you</p>
              <p class="mt-1 text-sm text-muted">Tunes FSRS to your own review history. Works better the more you've reviewed. {editing.parameters.length ? "Using your fitted settings." : "Using the defaults for now."}</p>
              {#if fitStatus}<p class="mt-2 text-sm" role="status">{fitStatus}</p>{/if}
            </div>
            <button type="button" class="btn btn-outline btn-sm shrink-0 border-field-border" disabled={fitting} onclick={fit}><RefreshCw class="size-4" />{fitting ? "Fitting…" : "Run now"}</button>
          </div>
        </div>
      </div>
      {#if settingsError}<p role="alert" class="mt-4 text-sm text-error">{settingsError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost mr-auto" onclick={reset}><RotateCcw class="size-4" />Reset to default</button>
        <button type="button" class="btn btn-ghost" onclick={() => (editing = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Save settings</button>
      </div>
    </form>
  </dialog>
{/if}

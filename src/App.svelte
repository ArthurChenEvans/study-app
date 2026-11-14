<script lang="ts">
  import { onMount, tick } from "svelte";
  import Sidebar, { type NavMode } from "./components/Sidebar.svelte";
  import Topics from "./screens/Topics.svelte";
  import Editor from "./screens/Editor.svelte";
  import Setup from "./screens/Setup.svelte";
  import Quiz from "./screens/Quiz.svelte";
  import Results from "./screens/Results.svelte";
  import Overview from "./screens/Overview.svelte";
  import Bibliography from "./screens/Bibliography.svelte";
  import History from "./screens/History.svelte";
  import Search from "./screens/Search.svelte";
  import Recall from "./screens/Recall.svelte";
  import Checklist from "./screens/Checklist.svelte";
  import RecallSession from "./screens/RecallSession.svelte";
  import Cards from "./screens/Cards.svelte";
  import TopicCards from "./screens/TopicCards.svelte";
  import CardReview from "./screens/CardReview.svelte";
  import Suggestions from "./screens/Suggestions.svelte";
  import Calendar from "./screens/Calendar.svelte";
  import Projects from "./screens/Projects.svelte";
  import Project from "./screens/Project.svelte";
  import SessionReview from "./screens/SessionReview.svelte";
  import DrillSession from "./screens/DrillSession.svelte";
  import YourData from "./screens/YourData.svelte";
  import Settings from "./screens/Settings.svelte";
  import Celebration from "./components/Celebration.svelte";
  import Pops from "./components/Pops.svelte";
  import Toasts from "./components/Toasts.svelte";
  import ImportHost from "./components/ImportHost.svelte";
  import { onImportChanged } from "./lib/importer.svelte";
  import { flushPendingDeletes, loadPrefs } from "./lib/celebrations.svelte";
  import { startUpdates } from "./lib/updates.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { Download } from "@lucide/svelte";
  import { statusLine } from "./lib/backup";
  import { syncLine } from "./lib/sync";
  import { screenFromHash, type Screen } from "./lib/router";
  import { api, type Id, type QuizView, type Subject, type SyncStatus } from "./lib/api";

  type Route =
    | { screen: Screen }
    | { screen: "editor"; topic: Id }
    | { screen: "setup-for"; topics: Id[] }
    | { screen: "quiz"; view: QuizView; run: number }
    | { screen: "results"; attempt: Id; fresh?: boolean }
    | { screen: "checklist"; id: Id }
    | { screen: "recall-session"; id: Id; run: number }
    | { screen: "topic-cards"; topic: Id; note: Id | "new" | null; notice?: string }
    | { screen: "card-review"; scope: Subject | null; run: number }
    | { screen: "suggestions" }
    | { screen: "calendar-day"; day: number }
    | { screen: "project"; id: Id }
    | { screen: "session-review"; project: Id; run: number }
    | { screen: "drills"; ids: Id[]; back: Route; run: number }
    | { screen: "search"; text: string };

  let route = $state<Route>({ screen: screenFromHash(location.hash) });

  // navigation.feature (R159, R160): the full sidebar when there's room, an icon rail on
  // tablets, a dock on phones. Breakpoints follow Material's window size classes, with the
  // full sidebar held back until 1100px so a portrait tablet keeps its width for content.
  const PHONE = "(max-width: 599px)";
  const TABLET = "(max-width: 1099px)";
  const navMode = (): NavMode => (typeof matchMedia !== "function" ? "full" : matchMedia(PHONE).matches ? "dock" : matchMedia(TABLET).matches ? "rail" : "full");
  let mode = $state<NavMode>(navMode());
  $effect(() => {
    if (typeof matchMedia !== "function") return;
    const queries = [matchMedia(PHONE), matchMedia(TABLET)];
    const update = () => (mode = navMode());
    queries.forEach((query) => query.addEventListener?.("change", update));
    return () => queries.forEach((query) => query.removeEventListener?.("change", update));
  });
  let runs = 0;

  const navScreen = $derived<Screen>(
    route.screen === "calendar-day" ? "calendar" : route.screen === "project" || route.screen === "session-review" || route.screen === "drills" ? "projects" : route.screen === "editor" ? "topics" : route.screen === "topic-cards" || route.screen === "card-review" || route.screen === "suggestions" ? "cards" : route.screen === "checklist" || route.screen === "recall-session" ? "recall" : route.screen === "setup-for" || route.screen === "quiz" || route.screen === "results" ? "setup" : route.screen,
  );

  function go(next: Route) {
    route = next;
    if (["overview", "calendar", "topics", "cards", "recall", "projects", "bibliography", "history", "search", "setup", "data", "settings"].includes(next.screen)) history.replaceState(null, "", `#${next.screen}`);
    window.scrollTo?.(0, 0);
    refreshDue();
  }

  // R58, R68, R100: the sidebar shows what's due today.
  type DueCounts = Partial<Record<Screen, number>>;
  let due = $state<DueCounts>({});
  // R102, R108: the day's backup is made when you move between screens, and the sidebar says how it's going.
  let backup = $state<{ safe: boolean; short: string } | null>(null);
  async function refreshBackup() {
    try {
      await api.dailyBackup();
    } catch { /* shown on Your data */ }
    try {
      const [status, today] = await Promise.all([api.backupStatus(), api.today()]);
      backup = status && typeof status === "object" ? statusLine(status, today) : null;
    } catch {
      backup = null;
    }
  }
  // R121, R128: sync when the app starts and every 30 seconds while it's open. The backend
  // waits while a quiz or review is in progress. Screens that only show things reload when
  // changes come in, and screens being edited are left alone.
  let sync = $state<{ ok: boolean; short: string; icon: string } | null>(null);
  const showSync = (status: SyncStatus | null) => (sync = status?.on ? syncLine(status, Math.floor(Date.now() / 1000)) : null);
  let dataVersion = $state(0);
  const RELOADS = ["overview", "calendar", "topics", "cards", "recall", "projects", "bibliography", "history"];
  async function syncTick(auto = true) {
    try {
      let status = await api.syncStatus();
      if (status?.on && !status.paused) {
        const result = await api.syncNow(auto);
        status = result.status;
        if (result.received && RELOADS.includes(route.screen)) dataVersion++;
        if (result.received) loadPrefs();
        if (result.received) refreshCounts();
      }
      showSync(status);
    } catch {
      /* shown on Your data */
    }
  }
  syncTick();
  const timer = setInterval(() => syncTick(), 30_000);
  $effect(() => () => clearInterval(timer));

  async function refreshDue() {
    refreshBackup();
    refreshCounts();
  }
  async function refreshCounts() {
    try {
      const today = await api.dueToday();
      due = today && typeof today === "object" ? { calendar: today.total, topics: today.quizzes, cards: today.cards, recall: today.checklists, projects: today.drills } : {};
    } catch {
      due = {};
    }
  }
  refreshDue();
  // R177: an import or its undo changes what every screen shows.
  $effect(() => onImportChanged(() => {
    if (RELOADS.includes(route.screen)) dataVersion++;
    refreshDue();
    syncTick();
  }));
  // The review's top bar names what is being reviewed.
  let scopeName = $state("All topics");
  async function startCards(scope: Subject | null) {
    scopeName = "All topics";
    try {
      if (scope && "Topic" in scope) scopeName = (await api.topics()).find((topic) => topic.id === scope.Topic)?.name ?? "Topic";
      if (scope && "Module" in scope) scopeName = (await api.modules()).find((module) => module.id === scope.Module)?.name ?? "Module";
    } catch { /* keep the default name */ }
    go({ screen: "card-review", scope, run: ++runs });
  }
  // A new card from the Cards home starts in the busiest topic, and the editor lets you change it.
  async function newCard() {
    const [home, topics] = await Promise.all([api.cardsHome(), api.topics()]);
    const topic = home.per_topic[0]?.topic ?? topics[0]?.id;
    go(topic === undefined ? { screen: "topics" } : { screen: "topic-cards", topic, note: "new" });
  }
  const startRecall = (id: Id) => go({ screen: "recall-session", id, run: ++runs });
  const startDrills = (ids: Id[]) => go({ screen: "drills", ids, back: route, run: ++runs });

  // R48: "/" focuses search from anywhere, except while typing or in a quiz.
  async function shortcut(event: KeyboardEvent) {
    if (event.key !== "/" || event.ctrlKey || event.metaKey || event.altKey || ["quiz", "recall-session", "card-review", "session-review", "drills"].includes(route.screen)) return;
    const target = event.target as HTMLElement | null;
    if (target && (target.closest("input, textarea, select, [contenteditable='true']"))) return;
    event.preventDefault();
    // A long list's own filter comes first (R139).
    const filter = document.querySelector<HTMLInputElement>("input[data-list-filter]");
    if (filter && !document.querySelector("dialog[open]")) return filter.focus();
    if (route.screen !== "search") go({ screen: "search" });
    await tick();
    document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
  }

  const startQuiz = (view: QuizView) => go({ screen: "quiz", view, run: ++runs });

  // A quiz is saved after every answer. If one was running when the app last closed (or
  // crashed), open straight back onto it at the question it stopped on.
  // celebrations.feature (R150): load which moments you want, and finish any deletes still
  // waiting on their undo window if the app closes.
  // updates.feature (R166): check for a new release on open, and say so plainly if one is
  // being installed as the app closes. The event says whether the system asks for a password.
  let installingOnClose = $state<{ asksPassword: boolean } | null>(null);
  onMount(() => {
    loadPrefs();
    startUpdates();
    let stop: (() => void) | undefined;
    listen<boolean>("updating-on-close", (event) => (installingOnClose = { asksPassword: event.payload !== false })).then((unlisten) => (stop = unlisten)).catch(() => {});
    return () => stop?.();
  });
  onMount(() => {
    api.currentQuestion().then(
      (view) => { if (view && typeof view === "object" && typeof view.position === "number" && route.screen !== "quiz") startQuiz(view); },
      () => { /* no quiz is running */ },
    );
  });
</script>

<svelte:window onhashchange={() => (route = { screen: screenFromHash(location.hash) })} onkeydown={shortcut} onbeforeunload={() => { flushPendingDeletes(); }} />

<div class="flex min-h-screen">
  {#if !["quiz", "recall-session", "card-review", "session-review", "drills"].includes(route.screen)}<Sidebar current={navScreen} {due} {backup} {sync} {mode} onNavigate={(screen) => go({ screen })} />{/if}
  <main class="min-w-0 flex-1" class:pb-24={mode === "dock"}>
    {#key dataVersion}
    {#if route.screen === "topics"}
      <Topics onCards={(topic) => go({ screen: "topic-cards", topic, note: null })} onSearch={(text) => go({ screen: "search", text })} onEdit={(topic) => go({ screen: "editor", topic })} onQuiz={(topic) => go({ screen: "setup-for", topics: [topic] })} onQuizTopics={(topics) => go({ screen: "setup-for", topics })} />
    {:else if route.screen === "editor"}
      {#key route.topic}<Editor topic={route.topic} />{/key}
    {:else if route.screen === "setup" || route.screen === "setup-for"}
      <Setup preselected={route.screen === "setup-for" ? route.topics : []} onStarted={startQuiz} />
    {:else if route.screen === "quiz"}
      {#key route.run}<Quiz initial={route.view} onFinished={(attempt) => go({ screen: "results", attempt, fresh: true })} />{/key}
    {:else if route.screen === "results"}
      {#key route.attempt}<Results attempt={route.attempt} fresh={route.fresh} onRetry={startQuiz} />{/key}
    {:else if route.screen === "recall"}
      <Recall onOpen={(id) => go({ screen: "checklist", id })} onStart={startRecall} onChanged={refreshDue} />
    {:else if route.screen === "checklist"}
      {#key route.id}<Checklist id={route.id} onBack={() => go({ screen: "recall" })} onStart={startRecall} />{/key}
    {:else if route.screen === "recall-session"}
      {#key route.run}<RecallSession id={route.id} onLeave={() => go({ screen: "recall" })} onEdit={(id) => go({ screen: "checklist", id })} onNext={startRecall} />{/key}
    {:else if route.screen === "cards"}
      <Cards onReview={startCards} onTopic={(topic) => go({ screen: "topic-cards", topic, note: null })} onSuggestions={() => go({ screen: "suggestions" })} onNewCard={newCard} />
    {:else if route.screen === "topic-cards"}
      {#key `${route.topic}:${route.note}`}<TopicCards topic={route.topic} editNote={route.note} notice={route.notice} onTopic={(topic, notice) => go({ screen: "topic-cards", topic, note: null, notice })} onBack={() => go({ screen: "cards" })} onReview={startCards} />{/key}
    {:else if route.screen === "card-review"}
      {#key route.run}<CardReview scope={route.scope} {scopeName} onLeave={() => go({ screen: "cards" })} onEdit={(topic, note) => go({ screen: "topic-cards", topic, note })} />{/key}
    {:else if route.screen === "suggestions"}
      <Suggestions onBack={() => go({ screen: "cards" })} onChanged={refreshDue} />
    {:else if route.screen === "calendar" || route.screen === "calendar-day"}
      <Calendar initialDay={route.screen === "calendar-day" ? route.day : null} onOpenDay={(day) => (route = day === null ? { screen: "calendar" } : { screen: "calendar-day", day })} onCards={() => startCards(null)} onRecall={startRecall} onQuiz={(topics) => go({ screen: "setup-for", topics })} onDrills={startDrills} onResults={(attempt) => go({ screen: "results", attempt })} onProject={(id) => go({ screen: "project", id })} />
    {:else if route.screen === "projects"}
      <Projects onOpen={(id) => go({ screen: "project", id })} onDrills={startDrills} />
    {:else if route.screen === "project"}
      {#key route.id}<Project id={route.id} onBack={() => go({ screen: "projects" })} onReview={(project) => go({ screen: "session-review", project, run: ++runs })} />{/key}
    {:else if route.screen === "session-review"}
      {@const project = route.project}
      {#key route.run}<SessionReview {project} onDone={() => go({ screen: "project", id: project })} onSuggestions={() => go({ screen: "suggestions" })} onCalendar={() => go({ screen: "calendar" })} />{/key}
    {:else if route.screen === "drills"}
      {@const back = route.back}
      {#key route.run}<DrillSession ids={route.ids} onLeave={() => go(back)} />{/key}
    {:else if route.screen === "settings"}
      <Settings onNavigate={(screen) => go({ screen })} />
    {:else if route.screen === "data"}
      <YourData onRestored={() => { refreshDue(); syncTick(); }} onSync={showSync} />
    {:else if route.screen === "bibliography"}
      <Bibliography />
    {:else if route.screen === "history"}
      <History onView={(attempt) => go({ screen: "results", attempt })} onNewQuiz={() => go({ screen: "setup" })} />
    {:else if route.screen === "search"}
      <Search initialText={"text" in route ? route.text : ""} onOpenTopic={(topic) => go({ screen: "editor", topic })} />
    {:else}
      <Overview onNavigate={(screen) => go({ screen })} onQuizTopics={(topics) => go({ screen: "setup-for", topics })} />
    {/if}
    {/key}
  </main>
</div>

<Celebration />
<Pops />
<ImportHost />
<Toasts />

{#if installingOnClose}
  <div class="modal modal-open" role="dialog" aria-modal="true" aria-labelledby="closing-h">
    <div class="modal-box max-w-md text-center">
      <Download class="mx-auto size-8" />
      <h2 id="closing-h" class="mt-3 text-lg font-black">Installing the update</h2>
      <p class="mt-2 text-muted">{installingOnClose.asksPassword ? "Your computer will ask for your password. " : ""}The app closes when it's done, and your study data was backed up first.</p>
    </div>
  </div>
{/if}

<script lang="ts">
  import { tick } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
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
  import { screenFromHash, type Screen } from "./lib/router";
  import { api, type Id, type QuizView, type Subject } from "./lib/api";

  type Route =
    | { screen: Screen }
    | { screen: "editor"; topic: Id }
    | { screen: "setup-for"; topics: Id[] }
    | { screen: "quiz"; view: QuizView; run: number }
    | { screen: "results"; attempt: Id }
    | { screen: "checklist"; id: Id }
    | { screen: "recall-session"; id: Id; run: number }
    | { screen: "topic-cards"; topic: Id; note: Id | "new" | null; notice?: string }
    | { screen: "card-review"; scope: Subject | null; run: number }
    | { screen: "suggestions" }
    | { screen: "project"; id: Id }
    | { screen: "session-review"; project: Id; run: number }
    | { screen: "drills"; ids: Id[]; back: Route; run: number };

  let route = $state<Route>({ screen: screenFromHash(location.hash) });
  let runs = 0;

  const navScreen = $derived<Screen>(
    route.screen === "project" || route.screen === "session-review" || route.screen === "drills" ? "projects" : route.screen === "editor" ? "topics" : route.screen === "topic-cards" || route.screen === "card-review" || route.screen === "suggestions" ? "cards" : route.screen === "checklist" || route.screen === "recall-session" ? "recall" : route.screen === "setup-for" || route.screen === "quiz" || route.screen === "results" ? "setup" : route.screen,
  );

  function go(next: Route) {
    route = next;
    if (["overview", "calendar", "topics", "cards", "recall", "projects", "bibliography", "history", "search", "setup"].includes(next.screen)) history.replaceState(null, "", `#${next.screen}`);
    window.scrollTo?.(0, 0);
    refreshDue();
  }

  // R58, R68, R100: the sidebar shows what's due today.
  type DueCounts = Partial<Record<Screen, number>>;
  let due = $state<DueCounts>({});
  async function refreshDue() {
    try {
      const today = await api.dueToday();
      due = today && typeof today === "object" ? { calendar: today.total, topics: today.quizzes, cards: today.cards, recall: today.checklists, projects: today.drills } : {};
    } catch {
      due = {};
    }
  }
  refreshDue();
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
    if (route.screen !== "search") go({ screen: "search" });
    await tick();
    document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
  }

  const startQuiz = (view: QuizView) => go({ screen: "quiz", view, run: ++runs });
</script>

<svelte:window onhashchange={() => (route = { screen: screenFromHash(location.hash) })} onkeydown={shortcut} />

<div class="flex min-h-screen">
  {#if !["quiz", "recall-session", "card-review", "session-review", "drills"].includes(route.screen)}<Sidebar current={navScreen} {due} onNavigate={(screen) => go({ screen })} />{/if}
  <main class="min-w-0 flex-1">
    {#if route.screen === "topics"}
      <Topics onEdit={(topic) => go({ screen: "editor", topic })} onQuiz={(topic) => go({ screen: "setup-for", topics: [topic] })} onQuizTopics={(topics) => go({ screen: "setup-for", topics })} />
    {:else if route.screen === "editor"}
      {#key route.topic}<Editor topic={route.topic} />{/key}
    {:else if route.screen === "setup" || route.screen === "setup-for"}
      <Setup preselected={route.screen === "setup-for" ? route.topics : []} onStarted={startQuiz} />
    {:else if route.screen === "quiz"}
      {#key route.run}<Quiz initial={route.view} onFinished={(attempt) => go({ screen: "results", attempt })} />{/key}
    {:else if route.screen === "results"}
      {#key route.attempt}<Results attempt={route.attempt} onRetry={startQuiz} />{/key}
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
    {:else if route.screen === "calendar"}
      <Calendar onCards={() => startCards(null)} onRecall={startRecall} onQuiz={(topics) => go({ screen: "setup-for", topics })} onDrills={startDrills} onResults={(attempt) => go({ screen: "results", attempt })} onProject={(id) => go({ screen: "project", id })} />
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
    {:else if route.screen === "bibliography"}
      <Bibliography />
    {:else if route.screen === "history"}
      <History onView={(attempt) => go({ screen: "results", attempt })} onNewQuiz={() => go({ screen: "setup" })} />
    {:else if route.screen === "search"}
      <Search onOpenTopic={(topic) => go({ screen: "editor", topic })} />
    {:else}
      <Overview onNavigate={(screen) => go({ screen })} />
    {/if}
  </main>
</div>

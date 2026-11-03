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
  import { screenFromHash, type Screen } from "./lib/router";
  import { api, type Id, type QuizView } from "./lib/api";

  type Route =
    | { screen: Screen }
    | { screen: "editor"; topic: Id }
    | { screen: "setup-for"; topics: Id[] }
    | { screen: "quiz"; view: QuizView; run: number }
    | { screen: "results"; attempt: Id }
    | { screen: "checklist"; id: Id }
    | { screen: "recall-session"; id: Id; run: number };

  let route = $state<Route>({ screen: screenFromHash(location.hash) });
  let runs = 0;

  const navScreen = $derived<Screen>(
    route.screen === "editor" ? "topics" : route.screen === "checklist" || route.screen === "recall-session" ? "recall" : route.screen === "setup-for" || route.screen === "quiz" || route.screen === "results" ? "setup" : route.screen,
  );

  function go(next: Route) {
    route = next;
    if (["overview", "topics", "recall", "bibliography", "history", "search", "setup"].includes(next.screen)) history.replaceState(null, "", `#${next.screen}`);
    window.scrollTo?.(0, 0);
    refreshDue();
  }

  // R58: the sidebar shows how many checklists are due.
  let recallDue = $state(0);
  async function refreshDue() {
    try {
      const count = await api.recallDueCount();
      recallDue = typeof count === "number" ? count : 0;
    } catch {
      recallDue = 0;
    }
  }
  refreshDue();
  const startRecall = (id: Id) => go({ screen: "recall-session", id, run: ++runs });

  // R48: "/" focuses search from anywhere, except while typing or in a quiz.
  async function shortcut(event: KeyboardEvent) {
    if (event.key !== "/" || event.ctrlKey || event.metaKey || event.altKey || route.screen === "quiz" || route.screen === "recall-session") return;
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
  {#if route.screen !== "quiz" && route.screen !== "recall-session"}<Sidebar current={navScreen} {recallDue} onNavigate={(screen) => go({ screen })} />{/if}
  <main class="min-w-0 flex-1">
    {#if route.screen === "topics"}
      <Topics onEdit={(topic) => go({ screen: "editor", topic })} onQuiz={(topic) => go({ screen: "setup-for", topics: [topic] })} />
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

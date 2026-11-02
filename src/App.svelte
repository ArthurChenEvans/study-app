<script lang="ts">
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
  import { screenFromHash, type Screen } from "./lib/router";
  import type { Id, QuizView } from "./lib/api";

  type Route =
    | { screen: Screen }
    | { screen: "editor"; topic: Id }
    | { screen: "setup-for"; topics: Id[] }
    | { screen: "quiz"; view: QuizView; run: number }
    | { screen: "results"; attempt: Id };

  let route = $state<Route>({ screen: screenFromHash(location.hash) });
  let runs = 0;

  const navScreen = $derived<Screen>(
    route.screen === "editor" ? "topics" : route.screen === "setup-for" || route.screen === "quiz" || route.screen === "results" ? "setup" : route.screen,
  );

  function go(next: Route) {
    route = next;
    if (["overview", "topics", "bibliography", "history", "search", "setup"].includes(next.screen)) history.replaceState(null, "", `#${next.screen}`);
    window.scrollTo?.(0, 0);
  }

  const startQuiz = (view: QuizView) => go({ screen: "quiz", view, run: ++runs });
</script>

<svelte:window onhashchange={() => (route = { screen: screenFromHash(location.hash) })} />

<div class="flex min-h-screen">
  {#if route.screen !== "quiz"}<Sidebar current={navScreen} onNavigate={(screen) => go({ screen })} />{/if}
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

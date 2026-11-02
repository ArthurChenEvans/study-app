<script lang="ts">
  import Sidebar from "./components/Sidebar.svelte";
  import Topics from "./screens/Topics.svelte";
  import { screenFromHash, type Screen } from "./lib/router";

  let screen = $state<Screen>(screenFromHash(location.hash));

  function navigate(next: Screen) {
    screen = next;
    history.replaceState(null, "", `#${next}`);
  }
</script>

<svelte:window onhashchange={() => (screen = screenFromHash(location.hash))} />

<div class="flex min-h-screen">
  <Sidebar current={screen} onNavigate={navigate} />
  <main class="min-w-0 flex-1">
    {#if screen === "topics"}
      <Topics onQuiz={() => navigate("setup")} />
    {:else}
      <div class="mx-auto max-w-6xl px-12 pt-12">
        <h1 class="text-3xl font-semibold">Coming next</h1>
        <p class="mt-2 text-muted">This screen is being built test-first. Topics is ready to use.</p>
      </div>
    {/if}
  </main>
</div>

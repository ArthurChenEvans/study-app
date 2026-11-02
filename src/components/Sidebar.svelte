<script lang="ts">
  import { BookOpen, History, Layers, LayoutDashboard, Library, Moon, Play, Search, Sun } from "@lucide/svelte";
  import { DARK, LIGHT, applyTheme, type Theme } from "../lib/theme";
  import type { Screen } from "../lib/router";

  let { current, onNavigate }: { current: Screen; onNavigate: (screen: Screen) => void } = $props();

  let theme = $state<Theme>((document.documentElement.dataset.theme as Theme) ?? DARK);

  const links: { screen: Screen; label: string; icon: typeof Library }[] = [
    { screen: "overview", label: "Overview", icon: LayoutDashboard },
    { screen: "topics", label: "Topics", icon: Library },
    { screen: "bibliography", label: "Bibliography", icon: BookOpen },
    { screen: "history", label: "History", icon: History },
    { screen: "search", label: "Search", icon: Search },
  ];

  function choose(next: Theme) {
    theme = next;
    applyTheme(next);
  }

  function go(event: MouseEvent, screen: Screen) {
    event.preventDefault();
    onNavigate(screen);
  }
</script>

<nav class="sticky top-0 flex h-screen w-60 shrink-0 flex-col gap-1 border-r border-base-300 bg-base-200 p-3" aria-label="Main">
  <div class="flex items-center gap-2 px-3 py-4">
    <span class="grid size-8 place-items-center rounded-lg bg-primary text-primary-content"><Layers class="size-4" /></span>
    <span class="font-semibold">Study App</span>
  </div>
  {#each links as link (link.screen)}
    <a
      href="#{link.screen}"
      class="nav-link flex items-center gap-3 rounded-lg px-3 py-2 text-sm"
      aria-current={current === link.screen ? "page" : undefined}
      onclick={(event) => go(event, link.screen)}
    >
      <link.icon class="size-4" />{link.label}
    </a>
  {/each}
  <div class="mt-4 px-3">
    <a href="#setup" class="btn btn-outline btn-sm w-full justify-start border-field-border" onclick={(event) => go(event, "setup")}><Play class="size-4" />New quiz</a>
  </div>
  <div class="mt-auto px-3 pb-2">
    <div class="seg w-full" role="radiogroup" aria-label="Theme">
      <button type="button" role="radio" class="flex flex-1 items-center justify-center gap-1.5" aria-checked={theme === DARK} onclick={() => choose(DARK)}><Moon class="size-3.5" />Dark</button>
      <button type="button" role="radio" class="flex flex-1 items-center justify-center gap-1.5" aria-checked={theme === LIGHT} onclick={() => choose(LIGHT)}><Sun class="size-3.5" />Light</button>
    </div>
  </div>
</nav>

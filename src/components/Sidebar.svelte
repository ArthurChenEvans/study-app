<script lang="ts">
  import { BookOpen, Brain, CalendarDays, Database, Hammer, SquareStack, History, Layers, LayoutDashboard, Library, Moon, Play, Search, ShieldAlert, ShieldCheck, Sun, Volume2 } from "@lucide/svelte";
  import { DARK, LIGHT, applyTheme, type Theme } from "../lib/theme";
  import { isSoundOn, setSoundOn } from "../lib/sound";
  import type { Screen } from "../lib/router";

  let { current, onNavigate, due = {}, backup = null }: { current: Screen; onNavigate: (screen: Screen) => void; due?: Partial<Record<Screen, number>>; backup?: { safe: boolean; short: string } | null } = $props();

  let theme = $state<Theme>((document.documentElement.dataset.theme as Theme) ?? DARK);
  let sound = $state(isSoundOn());

  const links: { screen: Screen; label: string; icon: typeof Library }[] = [
    { screen: "overview", label: "Overview", icon: LayoutDashboard },
    { screen: "calendar", label: "Calendar", icon: CalendarDays },
    { screen: "topics", label: "Topics", icon: Library },
    { screen: "cards", label: "Cards", icon: SquareStack },
    { screen: "recall", label: "Recall", icon: Brain },
    { screen: "projects", label: "Projects", icon: Hammer },
    { screen: "bibliography", label: "Bibliography", icon: BookOpen },
    { screen: "history", label: "History", icon: History },
    { screen: "search", label: "Search", icon: Search },
    { screen: "data", label: "Your data", icon: Database },
  ];

  /** What a count means, said the same way on hover and to a screen reader. */
  const dueLabel = (screen: Screen, count: number) => (screen === "projects" ? `${count} ${count === 1 ? "drill" : "drills"} due today` : `${count} due today`);

  function choose(next: Theme) {
    theme = next;
    applyTheme(next);
  }

  function toggleSound() {
    sound = !sound;
    setSoundOn(sound);
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
    {@const count = due[link.screen] ?? 0}
    <a
      href="#{link.screen}"
      class="nav-link flex items-center gap-3 rounded-lg px-3 py-2 text-sm"
      aria-current={current === link.screen ? "page" : undefined}
      onclick={(event) => go(event, link.screen)}
    >
      <link.icon class="size-4" /><span class="flex-1">{link.label}</span>
      {#if count > 0}<span class="text-xs tabular-nums" aria-label={dueLabel(link.screen, count)} title={dueLabel(link.screen, count)}>{count}</span>{/if}
      {#if link.screen === "search"}<kbd class="kbd kbd-sm" title="Press / to search">/</kbd>{/if}
    </a>
  {/each}
  <div class="mt-4 px-3">
    <a
      href="#setup"
      class="btn btn-outline btn-sm w-full justify-start border-field-border"
      class:nav-current={current === "setup"}
      aria-current={current === "setup" ? "page" : undefined}
      onclick={(event) => go(event, "setup")}><Play class="size-4" />New quiz</a
    >
  </div>
  <div class="mt-auto flex flex-col gap-3 px-3 pb-2">
    {#if backup}
      <!-- R108: the backup status in one line, linking to Your data -->
      <a href="#data" class="-mx-2 flex items-center gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-base-300 {backup.safe ? 'text-muted' : 'font-semibold'}" onclick={(event) => go(event, "data")}>
        {#if backup.safe}<ShieldCheck class="size-4" />{:else}<ShieldAlert class="size-4" />{/if}<span>{backup.short}</span>
      </a>
    {/if}
    <label class="flex cursor-pointer items-center gap-3 text-sm text-muted">
      <Volume2 class="size-4" /><span class="flex-1">Sound</span>
      <input type="checkbox" role="switch" class="toggle" checked={sound} onchange={toggleSound} />
    </label>
    <div class="seg w-full" role="radiogroup" aria-label="Theme">
      <button type="button" role="radio" class="flex flex-1 items-center justify-center gap-1.5" aria-checked={theme === DARK} onclick={() => choose(DARK)}><Moon class="size-3.5" />Dark</button>
      <button type="button" role="radio" class="flex flex-1 items-center justify-center gap-1.5" aria-checked={theme === LIGHT} onclick={() => choose(LIGHT)}><Sun class="size-3.5" />Light</button>
    </div>
  </div>
</nav>

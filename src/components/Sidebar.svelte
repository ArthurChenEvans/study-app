<script lang="ts">
  import { BookOpen, Brain, CalendarDays, Database, Hammer, SquareStack, History, Layers, FolderX, GitCompare, Eye, LayoutDashboard, Library, Moon, Play, RefreshCw, RefreshCwOff, Search, ShieldAlert, ShieldCheck, Sun, Volume2 } from "@lucide/svelte";
  import { DARK, LIGHT, applyPalette, applyTheme, initialPalette, type Theme } from "../lib/theme";
  import { isSoundOn, setSoundOn } from "../lib/sound";
  import type { Screen } from "../lib/router";

  let { current, onNavigate, due = {}, backup = null, sync = null }: { current: Screen; onNavigate: (screen: Screen) => void; due?: Partial<Record<Screen, number>>; backup?: { safe: boolean; short: string } | null; sync?: { ok: boolean; short: string; icon?: string } | null } = $props();
  // One line: the backup when it needs attention, otherwise sync once it's on (R108, R128).
  const showSync = $derived(sync !== null && (backup === null || backup.safe));

  let theme = $state<Theme>((document.documentElement.dataset.theme as Theme) ?? DARK);
  let sound = $state(isSoundOn());
  let colourBlind = $state(initialPalette() === "colour-blind");

  // Each section's icon takes a colour (Rust orange or Rust blue), so the sidebar reads at a glance.
  const links: { screen: Screen; label: string; icon: typeof Library; tone: "orange" | "blue" }[] = [
    { screen: "overview", label: "Overview", icon: LayoutDashboard, tone: "orange" },
    { screen: "calendar", label: "Calendar", icon: CalendarDays, tone: "blue" },
    { screen: "topics", label: "Topics", icon: Library, tone: "orange" },
    { screen: "cards", label: "Cards", icon: SquareStack, tone: "blue" },
    { screen: "recall", label: "Recall", icon: Brain, tone: "orange" },
    { screen: "projects", label: "Projects", icon: Hammer, tone: "blue" },
    { screen: "bibliography", label: "Bibliography", icon: BookOpen, tone: "orange" },
    { screen: "history", label: "History", icon: History, tone: "blue" },
    { screen: "search", label: "Search", icon: Search, tone: "orange" },
    { screen: "data", label: "Your data", icon: Database, tone: "blue" },
  ];

  /** What a count means, said the same way on hover and to a screen reader. */
  const dueLabel = (screen: Screen, count: number) => (screen === "projects" ? `${count} ${count === 1 ? "drill" : "drills"} due today` : `${count} due today`);

  function choose(next: Theme) {
    theme = next;
    applyTheme(next);
  }

  function toggleColours() {
    colourBlind = !colourBlind;
    applyPalette(colourBlind ? "colour-blind" : "standard");
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

<nav class="sticky top-0 flex h-screen w-60 shrink-0 flex-col gap-1 overflow-y-auto border-r-2 border-base-300 bg-base-200 p-3" aria-label="Main">
  <div class="flex items-center gap-2.5 px-2.5 pt-2 pb-4">
    <span class="logo-mark"><Layers class="size-5" /></span>
    <span class="text-[22px] font-black">Study App</span>
  </div>
  {#each links as link (link.screen)}
    {@const count = due[link.screen] ?? 0}
    <a
      href="#{link.screen}"
      class="nav-link flex h-11 items-center gap-3 px-3"
      aria-current={current === link.screen ? "page" : undefined}
      onclick={(event) => go(event, link.screen)}
    >
      <link.icon class="size-5 nav-icon-{link.tone}" strokeWidth={2.4} /><span class="flex-1">{link.label}</span>
      {#if count > 0}<span class="due-pill" aria-label={dueLabel(link.screen, count)} title={dueLabel(link.screen, count)}>{count}</span>{/if}
      {#if link.screen === "search"}<kbd class="kbd kbd-sm" title="Press / to search">/</kbd>{/if}
    </a>
  {/each}
  <div class="mt-4 px-3">
    <a
      href="#setup"
      class="btn btn-outline w-full"
      class:nav-current={current === "setup"}
      aria-current={current === "setup" ? "page" : undefined}
      onclick={(event) => go(event, "setup")}><Play class="size-4" />New quiz</a
    >
  </div>
  <div class="mt-auto flex flex-col gap-3 px-3 pb-2">
    {#if showSync && sync}
      <a href="#data" class="-mx-2 flex items-center gap-2 rounded-lg px-2 py-1.5 text-[13px] font-bold hover:bg-base-300 {sync.ok ? 'text-muted' : 'font-extrabold'}" onclick={(event) => go(event, "data")}>
        {#if sync.icon === "check"}<GitCompare class="size-4" />{:else if sync.icon === "missing"}<FolderX class="size-4" />{:else if sync.ok}<RefreshCw class="size-4" />{:else}<RefreshCwOff class="size-4" />{/if}<span>{sync.short}</span>
      </a>
    {:else if backup}
      <!-- R108: the backup status in one line, linking to Your data -->
      <a href="#data" class="-mx-2 flex items-center gap-2 rounded-lg px-2 py-1.5 text-[13px] font-bold hover:bg-base-300 {backup.safe ? 'text-muted' : 'font-extrabold'}" onclick={(event) => go(event, "data")}>
        {#if backup.safe}<ShieldCheck class="size-4" />{:else}<ShieldAlert class="size-4" />{/if}<span>{backup.short}</span>
      </a>
    {/if}
    <label class="flex cursor-pointer items-center gap-3 text-sm font-bold text-muted">
      <Volume2 class="size-4" /><span class="flex-1">Sound</span>
      <input type="checkbox" role="switch" class="toggle" checked={sound} onchange={toggleSound} />
    </label>
    <label class="flex cursor-pointer items-center gap-3 text-sm font-bold text-muted">
      <Eye class="size-4" /><span class="flex-1">Colour-blind safe</span>
      <input type="checkbox" role="switch" class="toggle" checked={colourBlind} onchange={toggleColours} />
    </label>
    <div class="seg w-full" role="radiogroup" aria-label="Theme">
      <button type="button" role="radio" class="flex flex-1 items-center justify-center gap-1.5" aria-checked={theme === DARK} onclick={() => choose(DARK)}><Moon class="size-3.5" />Dark</button>
      <button type="button" role="radio" class="flex flex-1 items-center justify-center gap-1.5" aria-checked={theme === LIGHT} onclick={() => choose(LIGHT)}><Sun class="size-3.5" />Light</button>
    </div>
  </div>
</nav>

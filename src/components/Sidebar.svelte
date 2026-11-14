<script lang="ts" module>
  export type NavMode = "full" | "rail" | "dock";
</script>

<script lang="ts">
  // navigation.feature (R158 to R160): the main navigation in three sizes. A full sidebar
  // with the links in three groups when there's room, an icon rail on tablets, and a dock
  // of five on phones. Anything that doesn't fit lives under More. Preferences live in
  // Settings, so the navigation only holds places you go.
  import { BookOpen, Brain, CalendarDays, Database, Ellipsis, FolderX, GitCompare, Hammer, History, LayoutDashboard, Layers, Library, Play, RefreshCw, RefreshCwOff, Search, Settings2, ShieldAlert, ShieldCheck, SquareStack, X } from "@lucide/svelte";
  import type { Screen } from "../lib/router";


  let { current, onNavigate, due = {}, backup = null, sync = null, mode = "full" }: {
    current: Screen;
    onNavigate: (screen: Screen) => void;
    due?: Partial<Record<Screen, number>>;
    backup?: { safe: boolean; short: string } | null;
    sync?: { ok: boolean; short: string; icon?: string } | null;
    mode?: NavMode;
  } = $props();
  // One line: the backup when it needs attention, otherwise sync once it's on (R108, R128).
  const showSync = $derived(sync !== null && (backup === null || backup.safe));

  type Item = { screen: Screen; label: string; short?: string; icon: typeof Library; tone: "orange" | "blue" | "plain" };
  const item = (screen: Screen, label: string, icon: typeof Library, tone: Item["tone"], short?: string): Item => ({ screen, label, icon, tone, short });

  // Each section's icon takes a colour (Rust orange or Rust blue), so the links read at a glance.
  const GROUPS: { name: string; items: Item[] }[] = [
    { name: "Today", items: [item("overview", "Overview", LayoutDashboard, "orange", "Today"), item("calendar", "Calendar", CalendarDays, "blue")] },
    { name: "Study", items: [item("topics", "Topics", Library, "orange"), item("cards", "Cards", SquareStack, "blue"), item("recall", "Recall", Brain, "orange"), item("projects", "Projects", Hammer, "blue")] },
    { name: "Library", items: [item("bibliography", "Bibliography", BookOpen, "orange"), item("history", "History", History, "blue")] },
  ];
  const ALL = GROUPS.flatMap((group) => group.items);
  const find = (screen: Screen) => ALL.find((entry) => entry.screen === screen)!;
  const SEARCH = item("search", "Search", Search, "plain");
  const NEW_QUIZ = item("setup", "New quiz", Play, "plain");
  const DATA = item("data", "Your data", Database, "plain");
  const SETTINGS = item("settings", "Settings", Settings2, "plain");

  const RAIL = ["overview", "calendar", "topics", "cards", "recall", "projects"].map((screen) => find(screen as Screen));
  const RAIL_MORE = [find("bibliography"), find("history"), NEW_QUIZ, DATA];
  const DOCK_LEFT = [find("overview"), find("calendar")];
  const DOCK_RIGHT = [find("topics"), find("cards")];
  const DOCK_MORE = [find("recall"), find("projects"), find("bibliography"), find("history"), SEARCH, NEW_QUIZ, DATA, SETTINGS];

  let moreOpen = $state(false);
  const moreItems = $derived(mode === "dock" ? DOCK_MORE : RAIL_MORE);
  const moreCurrent = $derived(moreItems.some((entry) => entry.screen === current));
  const moreDue = $derived(moreItems.reduce((sum, entry) => sum + (due[entry.screen] ?? 0), 0));

  /** What a count means, said the same way on hover and to a screen reader. */
  const dueLabel = (screen: Screen, count: number) => (screen === "projects" ? `${count} ${count === 1 ? "drill" : "drills"} due today` : `${count} due today`);
  const named = (entry: Item, label = entry.label) => {
    const count = due[entry.screen] ?? 0;
    return count > 0 ? `${label}, ${dueLabel(entry.screen, count)}` : label;
  };

  function go(event: MouseEvent, screen: Screen) {
    event.preventDefault();
    moreOpen = false;
    onNavigate(screen);
  }

  function keys(event: KeyboardEvent) {
    if (moreOpen && event.key === "Escape") moreOpen = false;
  }
</script>

<svelte:window onkeydown={keys} />

{#snippet pill(entry: Item)}
  {@const count = due[entry.screen] ?? 0}
  {#if count > 0}<span class="due-pill" aria-label={dueLabel(entry.screen, count)} title={dueLabel(entry.screen, count)}>{count}</span>{/if}
{/snippet}

{#snippet status()}
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
{/snippet}

{#snippet sheet()}
  {#if moreOpen}
    <button type="button" class="nav-scrim" aria-label="Close More" tabindex="-1" onclick={() => (moreOpen = false)}></button>
    <div class="nav-sheet {mode === 'dock' ? 'nav-sheet-dock' : 'nav-sheet-rail'}" role="dialog" aria-modal="true" aria-label="More">
      <div class="mb-1 flex items-center justify-between">
        <span class="eyebrow">More</span>
        <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Close" onclick={() => (moreOpen = false)}><X class="size-4" /></button>
      </div>
      {#each moreItems as entry (entry.screen)}
        <a href="#{entry.screen}" class="nav-link flex h-11 items-center gap-3 px-3" aria-current={current === entry.screen ? "page" : undefined} onclick={(event) => go(event, entry.screen)}>
          <entry.icon class="size-5 nav-icon-{entry.tone}" strokeWidth={2.4} /><span class="flex-1">{entry.label}</span>{@render pill(entry)}
        </a>
      {/each}
      {#if mode === "dock" && (backup || sync)}<div class="mt-2 px-3">{@render status()}</div>{/if}
    </div>
  {/if}
{/snippet}

{#if mode === "dock"}
  <nav class="nav-dock" aria-label="Main">
    {#each DOCK_LEFT as entry (entry.screen)}
      <a href="#{entry.screen}" class="dock-tab" aria-current={current === entry.screen ? "page" : undefined} onclick={(event) => go(event, entry.screen)}>
        <span class="dock-icon"><entry.icon class="size-5" strokeWidth={2.4} />{@render pill(entry)}</span><span>{entry.short ?? entry.label}</span>
      </a>
    {/each}
    <button type="button" class="dock-tab" aria-expanded={moreOpen} aria-current={moreCurrent ? "page" : undefined} onclick={() => (moreOpen = !moreOpen)}>
      <span class="dock-icon"><Ellipsis class="size-5" strokeWidth={2.4} />{#if moreDue}<span class="due-dot" aria-hidden="true"></span>{/if}</span><span>More</span>
    </button>
    {#each DOCK_RIGHT as entry (entry.screen)}
      <a href="#{entry.screen}" class="dock-tab" aria-current={current === entry.screen ? "page" : undefined} onclick={(event) => go(event, entry.screen)}>
        <span class="dock-icon"><entry.icon class="size-5" strokeWidth={2.4} />{@render pill(entry)}</span><span>{entry.short ?? entry.label}</span>
      </a>
    {/each}
  </nav>
  {@render sheet()}
{:else if mode === "rail"}
  <nav class="nav-rail" aria-label="Main">
    <span class="logo-mark mb-1" aria-hidden="true"><Layers class="size-5" /></span>
    <a href="#search" class="rail-item" aria-label="Search" title="Search (/)" aria-current={current === "search" ? "page" : undefined} onclick={(event) => go(event, "search")}><Search class="size-5" strokeWidth={2.4} /><span aria-hidden="true">Search</span></a>
    <hr class="rail-sep" />
    {#each RAIL as entry (entry.screen)}
      {@const count = due[entry.screen] ?? 0}
      <a href="#{entry.screen}" class="rail-item" aria-label={named(entry)} title={named(entry)} aria-current={current === entry.screen ? "page" : undefined} onclick={(event) => go(event, entry.screen)}>
        <span class="relative"><entry.icon class="size-5 nav-icon-{entry.tone}" strokeWidth={2.4} />{#if count > 0}<span class="rail-count" aria-hidden="true">{count}</span>{/if}</span><span aria-hidden="true">{entry.label}</span>
      </a>
    {/each}
    <button type="button" class="rail-item" aria-expanded={moreOpen} aria-current={moreCurrent ? "page" : undefined} onclick={() => (moreOpen = !moreOpen)}><Ellipsis class="size-5" strokeWidth={2.4} /><span>More</span></button>
    <div class="mt-auto flex flex-col items-center gap-1">
      <a href="#settings" class="rail-item" aria-label="Settings" title="Settings" aria-current={current === "settings" ? "page" : undefined} onclick={(event) => go(event, "settings")}><Settings2 class="size-5" strokeWidth={2.4} /><span aria-hidden="true">Settings</span></a>
    </div>
  </nav>
  {@render sheet()}
{:else}
  <nav class="sticky top-0 flex h-screen w-60 shrink-0 flex-col gap-1 overflow-y-auto border-r-2 border-base-300 bg-base-200 p-3" aria-label="Main">
    <div class="flex items-center gap-2.5 px-2.5 pt-2 pb-3">
      <span class="logo-mark"><Layers class="size-5" /></span>
      <span class="text-[22px] font-black">Shudhana</span>
    </div>
    <a href="#search" class="nav-search" aria-current={current === "search" ? "page" : undefined} onclick={(event) => go(event, "search")}>
      <Search class="size-4" /><span class="flex-1">Search</span><kbd class="kbd kbd-sm" title="Press / to search">/</kbd>
    </a>
    {#each GROUPS as group (group.name)}
      <div role="group" aria-label={group.name} class="mt-2 flex flex-col gap-0.5">
        <span class="nav-group" aria-hidden="true">{group.name}</span>
        {#each group.items as entry (entry.screen)}
          <a href="#{entry.screen}" class="nav-link flex h-10 items-center gap-3 px-3" aria-current={current === entry.screen ? "page" : undefined} onclick={(event) => go(event, entry.screen)}>
            <entry.icon class="size-5 nav-icon-{entry.tone}" strokeWidth={2.4} /><span class="flex-1">{entry.label}</span>{@render pill(entry)}
          </a>
        {/each}
      </div>
    {/each}
    <div class="mt-4 px-3">
      <a href="#setup" class="btn btn-outline w-full" class:nav-current={current === "setup"} aria-current={current === "setup" ? "page" : undefined} onclick={(event) => go(event, "setup")}><Play class="size-4" />New quiz</a>
    </div>
    <div class="mt-auto flex flex-col gap-1 px-3 pb-1 pt-4">
      {@render status()}
      <a href="#settings" class="-mx-2 flex min-h-9 items-center gap-2 rounded-lg px-2 text-[13px] font-bold text-muted hover:bg-base-300" class:nav-current={current === "settings"} aria-current={current === "settings" ? "page" : undefined} onclick={(event) => go(event, "settings")}>
        <Settings2 class="size-4" /><span>Settings</span>
      </a>
    </div>
  </nav>
{/if}

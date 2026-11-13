<script lang="ts">
  // navigation.feature (R158): every preference in one place, so the navigation only holds
  // places you go. Appearance and sound stay on this computer. Celebrations sync.
  import { ChevronRight, CircleCheck, Database, Download, Eye, Moon, PartyPopper, RefreshCw, Sun, TriangleAlert, Volume2 } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import CelebrationsDialog from "../components/CelebrationsDialog.svelte";
  import { DARK, LIGHT, applyPalette, applyTheme, initialPalette, type Theme } from "../lib/theme";
  import { isSoundOn, setSoundOn } from "../lib/sound";
  import type { Screen } from "../lib/router";
  import { checkNow, installNow, setAuto, updates } from "../lib/updates.svelte";

  let { onNavigate }: { onNavigate: (screen: Screen) => void } = $props();

  let theme = $state<Theme>((document.documentElement.dataset.theme as Theme) ?? DARK);
  let sound = $state(isSoundOn());
  let colourBlind = $state(initialPalette() === "colour-blind");
  let celebrations = $state(false);

  function choose(next: Theme) {
    theme = next;
    applyTheme(next);
  }
</script>

<div class="mx-auto max-w-3xl px-12 pb-24">
  <PageHeader title="Settings" subtitle="How the app looks and sounds. Your study data is under Your data." />

  <section class="panel p-5" aria-labelledby="look-h">
    <h2 id="look-h" class="eyebrow">Appearance</h2>
    <div class="mt-3 flex flex-wrap items-center gap-3">
      <span class="flex flex-1 items-center gap-3 font-bold"><Moon class="size-5 text-muted" />Theme</span>
      <div class="seg" role="radiogroup" aria-label="Theme">
        <button type="button" role="radio" class="flex items-center gap-1.5" aria-checked={theme === DARK} onclick={() => choose(DARK)}><Moon class="size-3.5" />Dark</button>
        <button type="button" role="radio" class="flex items-center gap-1.5" aria-checked={theme === LIGHT} onclick={() => choose(LIGHT)}><Sun class="size-3.5" />Light</button>
      </div>
    </div>
    <label class="mt-3 flex min-h-11 cursor-pointer items-center gap-3">
      <Eye class="size-5 text-muted" />
      <span class="flex-1"><span class="block font-bold">Colour-blind safe</span><span id="cb-help" class="block text-sm text-muted">Right and wrong in blue and orange instead of green and red. Icons and words always go with them.</span></span>
      <input type="checkbox" role="switch" class="toggle" aria-label="Colour-blind safe" aria-describedby="cb-help" checked={colourBlind} onchange={() => { colourBlind = !colourBlind; applyPalette(colourBlind ? "colour-blind" : "standard"); }} />
    </label>
  </section>

  <section class="panel mt-5 p-5" aria-labelledby="sound-h">
    <h2 id="sound-h" class="eyebrow">Sound and celebrations</h2>
    <label class="mt-3 flex min-h-11 cursor-pointer items-center gap-3">
      <Volume2 class="size-5 text-muted" />
      <span class="flex-1"><span class="block font-bold">Sound</span><span id="sound-help" class="block text-sm text-muted">Answer cues and celebration jingles. Everything they say is on screen too.</span></span>
      <input type="checkbox" role="switch" class="toggle" aria-label="Sound" aria-describedby="sound-help" checked={sound} onchange={() => { sound = !sound; setSoundOn(sound); }} />
    </label>
    <button type="button" class="settings-row mt-2" onclick={() => (celebrations = true)}>
      <PartyPopper class="size-5 text-muted" />
      <span class="flex-1 text-left"><span class="block font-bold">Celebrations</span><span class="block text-sm font-semibold text-muted">Big moments, small pops along the way, and toasts</span></span>
      <ChevronRight class="size-5 text-muted" />
    </button>
  </section>

  <section class="panel mt-5 p-5" aria-labelledby="updates-h">
    <h2 id="updates-h" class="eyebrow">Updates</h2>
    <p class="mt-3 font-bold">Shudhana {updates.current || "(desktop app only)"}</p>
    <label class="mt-2 flex min-h-11 cursor-pointer items-center gap-3">
      <RefreshCw class="size-5 text-muted" />
      <span class="flex-1"><span class="block font-bold">Auto update</span><span id="auto-help" class="block text-sm text-muted">Checks for a new release when the app opens and closes, downloads it in the background, and installs it as you close the app.</span></span>
      <input type="checkbox" role="switch" class="toggle" aria-label="Auto update" aria-describedby="auto-help" checked={updates.auto} onchange={() => setAuto(!updates.auto)} />
    </label>
    <div class="mt-3 flex flex-wrap items-center gap-3">
      <button type="button" class="btn btn-outline btn-sm" disabled={updates.status === "checking" || updates.status === "downloading" || updates.status === "installing"} onclick={checkNow}>
        <RefreshCw class="size-4 {updates.status === 'checking' ? 'animate-spin' : ''}" />Check for updates
      </button>
      <span class="text-sm" aria-live="polite">
        {#if updates.status === "checking"}Checking…{:else if updates.status === "uptodate"}<span class="inline-flex items-center gap-1.5 font-bold"><CircleCheck class="size-4" />You have the newest version</span>{:else if updates.status === "downloading"}Downloading…{:else if updates.status === "installing"}Installing. Your computer will ask for your password.{/if}
      </span>
    </div>
    {#if updates.info && (updates.status === "available" || updates.status === "ready")}
      <div class="note-box mt-4 p-4">
        <p class="font-extrabold">Version {updates.info.version} is available</p>
        {#if updates.info.notes}<p class="mt-1 whitespace-pre-line text-sm">{updates.info.notes}</p>{/if}
        <p class="mt-2 text-sm text-muted">{updates.info.asks_password ? "Installing asks for your password, because it updates the app for everyone on this computer. " : ""}Your study data is backed up first.</p>
        <button type="button" class="btn btn-primary btn-sm mt-3" onclick={installNow}><Download class="size-4" />Update to {updates.info.version}</button>
      </div>
    {/if}
    {#if updates.status === "failed" && updates.error}<p role="alert" class="mt-3 flex items-start gap-2 text-sm font-bold"><TriangleAlert class="mt-0.5 size-4 shrink-0" />{updates.error}</p>{/if}
  </section>

  <section class="panel mt-5 p-5" aria-labelledby="data-h">
    <h2 id="data-h" class="eyebrow">Your study data</h2>
    <a href="#data" class="settings-row mt-2" onclick={(event) => { event.preventDefault(); onNavigate("data"); }}>
      <Database class="size-5 text-muted" />
      <span class="flex-1"><span class="block font-bold">Your data</span><span class="block text-sm font-semibold text-muted">Backups, sync, and export and import</span></span>
      <ChevronRight class="size-5 text-muted" />
    </a>
  </section>
</div>

{#if celebrations}<CelebrationsDialog onClose={() => (celebrations = false)} />{/if}

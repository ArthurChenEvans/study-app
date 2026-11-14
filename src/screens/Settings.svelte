<script lang="ts">
  // navigation.feature (R158): every preference in one place, so the navigation only holds
  // places you go. Appearance and sound stay on this computer. Celebrations sync.
  import { Check, ChevronRight, CircleCheck, Database, Download, Eye, Moon, Palette, PartyPopper, RefreshCw, Sun, TriangleAlert, Volume2 } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import CelebrationsDialog from "../components/CelebrationsDialog.svelte";
  import { DARK, LIGHT, applyPalette, applyTheme, initialPalette, type Theme } from "../lib/theme";
  import { COLOURS, applyColours, buildColours, initialColours, type ColoursId } from "../lib/colours";
  import { isSoundOn, setSoundOn } from "../lib/sound";
  import type { Screen } from "../lib/router";
  import { checkNow, installNow, setAuto, updates } from "../lib/updates.svelte";

  let { onNavigate }: { onNavigate: (screen: Screen) => void } = $props();

  let theme = $state<Theme>((document.documentElement.dataset.theme as Theme) ?? DARK);
  let sound = $state(isSoundOn());
  let colourBlind = $state(initialPalette() === "colour-blind");
  let celebrations = $state(false);
  let colours = $state<ColoursId>(initialColours());
  // Each option previews itself in the current mode and palette (R184).
  const previews = $derived(COLOURS.map((option) => ({ ...option, c: buildColours(option.id, theme !== LIGHT, colourBlind) })));

  function pick(id: ColoursId) {
    colours = id;
    applyColours(id, theme !== LIGHT, colourBlind);
  }

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
    <div class="mt-5">
      <p id="colours-h" class="flex items-center gap-3 font-bold"><Palette class="size-5 text-muted" />Colours</p>
      <div class="colour-grid mt-3" role="radiogroup" aria-labelledby="colours-h">
        {#each previews as option (option.id)}
          <button type="button" role="radio" aria-checked={colours === option.id} aria-label={option.name} class="colour-option" style="background: {option.c['--color-base-100']}; color: {option.c['--color-base-content']}; border-color: {colours === option.id ? option.c['--color-primary'] : option.c['--color-base-300']}" onclick={() => pick(option.id)}>
            <span class="flex items-center justify-between gap-2">
              <span class="font-extrabold">{option.name}</span>
              {#if colours === option.id}<Check class="size-4" />{/if}
            </span>
            <span class="colour-option-card" style="background: {option.c['--surface']}; border-color: {option.c['--color-base-300']}" aria-hidden="true">
              <span class="colour-option-bar" style="background: {option.c['--chart-right']}; flex: 3"></span>
              <span class="colour-option-bar" style="background: {option.c['--chart-wrong']}; flex: 1"></span>
            </span>
            <span class="colour-option-button" style="background: {option.c['--color-primary']}; color: {option.c['--color-primary-content']}; border-color: {option.c['--primary-edge']}" aria-hidden="true">Start</span>
            <span class="text-xs font-semibold" style="color: {option.c['--muted']}">{option.blurb}</span>
          </button>
        {/each}
      </div>
    </div>
    <label class="mt-4 flex min-h-11 cursor-pointer items-center gap-3">
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
      <span class="flex-1"><span class="block font-bold">Your data</span><span class="block text-sm font-semibold text-muted">Backups and sync</span></span>
      <ChevronRight class="size-5 text-muted" />
    </a>
  </section>
</div>

{#if celebrations}<CelebrationsDialog onClose={() => (celebrations = false)} />{/if}

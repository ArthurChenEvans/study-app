<script lang="ts">
  // celebrations.feature (R152): turn everything off, or tune each moment. Which moments you
  // want syncs to your other computers. Sound and motion stay on this one.
  import { Play, RotateCcw } from "@lucide/svelte";
  import type { Big, CelebrationPrefs, Frequency, Mode } from "../lib/celebrate";
  import { device, live, previewBig, previewSmall, previewToast, resetPrefs, saveDevice, savePrefs, type DevicePrefs, type Motion } from "../lib/celebrations.svelte";
  import { errorMessage } from "../lib/api";
  import type { SmallEvent } from "../lib/celebrate";

  let { onClose }: { onClose: () => void } = $props();
  let error = $state("");

  const prefs = $derived(live.prefs);
  const off = $derived(!prefs.on);

  async function change(patch: Partial<CelebrationPrefs>) {
    error = "";
    try {
      await savePrefs({ ...JSON.parse(JSON.stringify(live.prefs)), ...patch });
    } catch (failure) {
      error = errorMessage(failure);
    }
  }
  const setMoment = (kind: Big, mode: Mode) => change({ moments: { ...live.prefs.moments, [kind]: mode } });
  const changeDevice = (patch: Partial<DevicePrefs>) => saveDevice({ ...device, ...patch });

  async function reset() {
    error = "";
    try { await resetPrefs(); } catch (failure) { error = errorMessage(failure); }
  }

  const BIG: { kind: Big; label: string }[] = [
    { kind: "reviews", label: "Reviews done" },
    { kind: "quiz", label: "Quiz complete" },
    { kind: "recall", label: "Free recall done" },
  ];
  const MODES: { value: Mode; label: string }[] = [{ value: "full", label: "Full screen" }, { value: "pop", label: "Small pop" }, { value: "off", label: "Off" }];
  const sample = (event: SmallEvent) => () => previewSmall(event);
</script>

{#snippet seg<T>(label: string, options: { value: T; label: string }[], value: T, pick: (value: T) => void, disabled = false)}
  <div class="seg" role="radiogroup" aria-label={label}>
    {#each options as option (option.label)}
      <button type="button" role="radio" aria-checked={value === option.value} {disabled} onclick={() => pick(option.value)}>{option.label}</button>
    {/each}
  </div>
{/snippet}

{#snippet hear(label: string, run: () => void)}
  <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Play {label}" title="Play it" onclick={run}><Play class="size-4" /></button>
{/snippet}

{#snippet toggle(label: string, checked: boolean, flip: () => void, disabled = false)}
  <label class="flex min-h-11 cursor-pointer items-center gap-3 font-bold {disabled ? 'opacity-60' : ''}">
    <span class="flex-1">{label}</span>
    <input type="checkbox" role="switch" class="toggle" {checked} {disabled} onchange={flip} />
  </label>
{/snippet}

<dialog open class="modal modal-open" aria-labelledby="cele-h">
  <div class="modal-box max-w-2xl">
    <h3 id="cele-h" class="text-lg font-semibold">Celebrations</h3>
    <p class="mt-1 text-sm text-muted">Big moments, small pops along the way, and notes about what just happened.</p>

    <div class="note-box mt-5 p-4">
      {@render toggle("Celebrations", prefs.on, () => change({ on: !prefs.on }))}
      {#if off}<p class="text-sm text-muted">The end-of-session summary still shows, without the confetti, music or animation. Error messages always show.</p>{/if}
    </div>

    <section class="mt-6" aria-labelledby="cele-big-h">
      <h4 id="cele-big-h" class="eyebrow">Big moments</h4>
      <ul class="mt-2 space-y-3">
        {#each BIG as item (item.kind)}
          <li class="flex flex-wrap items-center gap-3">
            <span class="min-w-40 flex-1 font-bold">{item.label}</span>
            {@render seg(item.label, MODES, prefs.moments[item.kind], (mode) => setMoment(item.kind, mode), off)}
            {@render hear(item.label, () => previewBig(item.kind))}
          </li>
        {/each}
      </ul>
    </section>

    <section class="mt-6" aria-labelledby="cele-small-h">
      <h4 id="cele-small-h" class="eyebrow">Small moments</h4>
      <div class="mt-2 space-y-2">
        <div class="flex items-center gap-2">
          <div class="flex-1">{@render toggle("Card count pops", prefs.counts, () => change({ counts: !prefs.counts }), off)}</div>
          {@render hear("Card count pops", sample({ kind: "count", done: 20 }))}
        </div>
        <div class="flex flex-wrap items-center gap-3 pl-1">
          <span class="flex-1 text-sm text-muted">Sessions under 15 cards only get the finish screen.</span>
          {@render seg("How often", [{ value: "fewer", label: "Fewer" }, { value: "normal", label: "Normal" }, { value: "more", label: "More" }], prefs.frequency, (frequency) => change({ frequency: frequency as Frequency }), off || !prefs.counts)}
        </div>
        <div class="flex items-center gap-2">
          <div class="flex-1">{@render toggle("Halfway and Last stretch", prefs.markers, () => change({ markers: !prefs.markers }), off)}</div>
          {@render hear("Halfway and Last stretch", sample({ kind: "halfway", done: 32 }))}
        </div>
        <div class="flex items-center gap-2">
          <div class="flex-1">{@render toggle("Answer streaks", prefs.streaks, () => change({ streaks: !prefs.streaks }), off)}</div>
          {@render hear("Answer streaks", sample({ kind: "streak", streak: 5 }))}
        </div>
        <div class="flex flex-wrap items-center gap-3 pl-1">
          <span class="flex-1 text-sm text-muted">Right answers in a row</span>
          {@render seg("Streak length", [{ value: 5, label: "5" }, { value: 10, label: "10" }], prefs.streakLength, (streakLength) => change({ streakLength: streakLength as 5 | 10 }), off || !prefs.streaks)}
        </div>
        <div class="flex items-center gap-2">
          <div class="flex-1">{@render toggle("Keep going nudge", prefs.nudge, () => change({ nudge: !prefs.nudge }), off)}</div>
          {@render hear("Keep going nudge", sample({ kind: "nudge" }))}
        </div>
        <div class="flex flex-wrap items-center gap-3 pl-1">
          <span class="flex-1 text-sm text-muted">When a while goes by with no pop</span>
          {@render seg("Nudge after", [{ value: 3, label: "3 min" }, { value: 5, label: "5 min" }, { value: 10, label: "10 min" }], prefs.nudgeMinutes, (nudgeMinutes) => change({ nudgeMinutes: nudgeMinutes as 3 | 5 | 10 }), off || !prefs.nudge)}
        </div>
      </div>
    </section>

    <section class="mt-6" aria-labelledby="cele-toast-h">
      <h4 id="cele-toast-h" class="eyebrow">Notes</h4>
      <div class="mt-2 flex items-center gap-2">
        <div class="flex-1">{@render toggle("Toasts after adding, saving and deleting", prefs.toasts, () => change({ toasts: !prefs.toasts }))}</div>
        {@render hear("Toasts", previewToast)}
      </div>
      <p class="text-sm text-muted">Deleting always shows its Undo, and errors always show.</p>
    </section>

    <section class="mt-6" aria-labelledby="cele-device-h">
      <h4 id="cele-device-h" class="eyebrow">On this computer</h4>
      <div class="mt-2 space-y-2">
        <div class="flex flex-wrap items-center gap-3">
          <span class="flex-1 font-bold">Motion</span>
          {@render seg("Motion", [{ value: "system", label: "Follow my system" }, { value: "full", label: "Full" }, { value: "calm", label: "Calm" }], device.motion, (motion) => changeDevice({ motion: motion as Motion }))}
        </div>
        <label class="flex min-h-11 items-center gap-3 font-bold">
          <span class="flex-1">Volume</span>
          <input type="range" class="range range-sm w-48" min="0" max="1" step="0.05" value={device.volume} oninput={(event) => changeDevice({ volume: Number(event.currentTarget.value) })} />
        </label>
        {@render toggle("Sound for big moments only", device.bigSoundOnly, () => changeDevice({ bigSoundOnly: !device.bigSoundOnly }))}
        <!-- R193 -->
        {@render seg("Confetti colours", [{ value: "theme", label: "My theme's" }, { value: "rainbow", label: "Rainbow" }], device.confetti, (value) => changeDevice({ confetti: value as "theme" | "rainbow" }))}
        <p class="text-sm text-muted">The Sound switch in the sidebar turns every sound on or off.</p>
      </div>
    </section>

    {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      <button type="button" class="btn btn-ghost mr-auto" onclick={reset}><RotateCcw class="size-4" />Reset to defaults</button>
      <button type="button" class="btn btn-primary" onclick={onClose}>Done</button>
    </div>
  </div>
</dialog>

<script lang="ts">
  // celebrations.feature (R154): a big moment in beats. The backdrop and rays come in, the
  // badge crouches, jumps and lands on the chord (with confetti), then the title, each number
  // and Continue. Calm mode, or reduced motion, keeps it to fades with no confetti.
  import { Check, ListChecks, Trophy } from "@lucide/svelte";
  import confetti from "canvas-confetti";
  import { onDestroy } from "svelte";
  import { calm, closeBig, live, type BigMoment } from "../lib/celebrations.svelte";
  import { playTick } from "../lib/jingles";

  const COLOURS = ["#d34516", "#f2c14e", "#f7915f", "#8cc4e2", "#7fd1ae", "#3d4775"];
  const moment = $derived<BigMoment | undefined>(live.big[0]);

  let shown = $state<number[]>([]);
  let canClose = $state(false);
  let quiet = $state(false);
  let continueButton = $state<HTMLButtonElement | null>(null);
  let timers: ReturnType<typeof setTimeout>[] = [];
  let opener: Element | null = null;

  const later = (ms: number, run: () => void) => timers.push(setTimeout(run, ms));
  function clear() {
    timers.forEach(clearTimeout);
    timers = [];
  }

  function fire(options: confetti.Options) {
    confetti({ colors: COLOURS, disableForReducedMotion: true, zIndex: 1200, ...options });
  }
  function landing(current: BigMoment) {
    if (current.kind === "recall") {
      fire({ particleCount: 45, spread: 100, startVelocity: 30, origin: { x: 0.5, y: 0.4 } });
      const end = Date.now() + 2500;
      const drift = () => {
        fire({ particleCount: 3, angle: 270, spread: 80, startVelocity: 4, gravity: 0.45, ticks: 400, origin: { x: Math.random(), y: -0.05 } });
        if (Date.now() < end && live.big[0]?.id === current.id) later(60, drift);
      };
      drift();
      return;
    }
    const tier = current.kind === "quiz" ? (current.score === 100 ? 3 : (current.score ?? 0) >= 80 ? 2 : 1) : 2;
    if (tier === 1) return fire({ particleCount: 35, spread: 100, startVelocity: 30, origin: { x: 0.5, y: 0.4 } });
    const power = tier === 3 ? 1.6 : 1;
    const count = Math.round(70 * power);
    fire({ particleCount: count, angle: 60, spread: 55, startVelocity: 55 * Math.sqrt(power), origin: { x: 0, y: 0.75 } });
    fire({ particleCount: count, angle: 120, spread: 55, startVelocity: 55 * Math.sqrt(power), origin: { x: 1, y: 0.75 } });
    if (tier === 3) later(300, () => fire({ particleCount: 110, spread: 100, startVelocity: 38, origin: { x: 0.5, y: 0.4 } }));
  }

  function countUp(index: number, target: number) {
    const started = Date.now();
    const step = () => {
      const progress = Math.min(1, (Date.now() - started) / 650);
      shown[index] = Math.round(target * (1 - Math.pow(1 - progress, 3)));
      if (progress < 1) later(16, step);
    };
    step();
  }

  function start(current: BigMoment) {
    clear();
    quiet = calm();
    canClose = false;
    opener ??= document.activeElement;
    shown = current.stats.map((stat) => (quiet ? stat.value : 0));
    if (!quiet) later(780, () => landing(current));
    current.stats.forEach((stat, index) => {
      const at = 1350 + index * 220;
      if (!quiet) later(at + 80, () => countUp(index, stat.value));
      later(at + 700, () => playTick(index));
    });
    later(quiet ? 400 : 600, () => {
      canClose = true;
      continueButton?.focus({ preventScroll: true });
    });
  }

  $effect(() => {
    if (moment) start(moment);
  });

  function close() {
    if (!canClose) return;
    clear();
    confetti.reset();
    closeBig();
    if (!live.big.length && opener instanceof HTMLElement) {
      opener.focus();
      opener = null;
    }
  }

  function keys(event: KeyboardEvent) {
    if (!moment || (event.key !== "Escape" && event.key !== "Enter")) return;
    event.preventDefault();
    close();
  }

  onDestroy(clear);

  const tone = { primary: "bg-primary text-primary-content", accent: "bg-accent text-accent-content", success: "bg-success text-success-content", info: "bg-info text-info-content" };
</script>

<svelte:window onkeydown={keys} />

{#if moment}
  {#key moment.id}
    <div class="celebrate" class:quiet role="dialog" aria-modal="true" aria-labelledby="celebrate-title">
      <div class="rays" aria-hidden="true"></div>
      <div class="cele-card">
        <div class="cele-badge" class:cele-gold={moment.kind === "quiz" && moment.score === 100} class:cele-sky={moment.kind === "recall"} aria-hidden="true">
          <span class="disc">
            {#if moment.kind === "reviews"}<Check class="size-16" strokeWidth={2.8} />{:else if moment.kind === "quiz"}<Trophy class="size-16" strokeWidth={2.4} />{:else}<ListChecks class="size-16" strokeWidth={2.6} />{/if}
          </span>
        </div>
        <h2 id="celebrate-title" class="cele-title">
          {#each moment.title.split(" ") as word, index (index)}<span style="animation-delay: {850 + index * 90}ms">{word}</span>{" "}{/each}
        </h2>
        {#if moment.line}<p class="cele-line">{moment.line}</p>{/if}
        {#if moment.stats.length}
          <div class="cele-stats">
            {#each moment.stats as stat, index (stat.label)}
              <div class="cele-stat" style="animation-delay: {1350 + index * 220}ms">
                <span class="cele-label {tone[stat.tone]}">{stat.label}</span>
                <strong class="tabular-nums">{shown[index] ?? 0}{stat.suffix ?? ""}</strong>
              </div>
            {/each}
          </div>
        {/if}
        <button bind:this={continueButton} type="button" class="btn btn-primary btn-lg w-full cele-go" aria-disabled={!canClose} onclick={close}>Continue</button>
      </div>
    </div>
  {/key}
{/if}

<style>
  .celebrate {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: grid;
    place-items: center;
    overflow: hidden;
    padding: 24px 16px;
    background: var(--color-base-100);
    animation: fade 0.25s both;
  }
  .rays {
    position: absolute;
    left: 50%;
    top: 38%;
    width: 160vmax;
    height: 160vmax;
    margin: -80vmax 0 0 -80vmax;
    border-radius: 50%;
    background: repeating-conic-gradient(color-mix(in oklab, var(--color-primary) 12%, transparent) 0 10deg, transparent 10deg 20deg);
    mask: radial-gradient(circle, #000 0, transparent 55%);
    animation: fade 0.6s 0.1s both, spin 40s linear infinite;
  }
  .cele-card { position: relative; display: grid; justify-items: center; gap: 16px; width: 100%; max-width: 470px; text-align: center; }
  .cele-badge { width: 150px; height: 150px; display: grid; place-items: center; transform-origin: 50% 100%; animation: jump 0.9s 0.12s ease-out both, bob 1.5s 1.05s ease-in-out infinite; }
  .disc {
    width: 128px; height: 128px; border-radius: 50%; display: grid; place-items: center;
    background: var(--color-primary); color: var(--color-primary-content);
    border: 4px solid var(--color-base-content); box-shadow: 0 6px 0 var(--color-base-content);
  }
  .cele-gold .disc { background: #f2c14e; color: #1e2650; }
  .cele-sky .disc { background: #8cc4e2; color: #1e2650; }
  .cele-title { display: flex; flex-wrap: wrap; justify-content: center; gap: 0 0.28em; margin: 0; font-size: clamp(2.1rem, 6vw, 3rem); font-weight: 900; line-height: 1.05; color: var(--primary-ink); }
  .cele-title span { display: inline-block; animation: pop 0.42s cubic-bezier(0.22, 1, 0.36, 1) both; }
  .cele-line { font-weight: 700; color: var(--muted); animation: rise 0.38s 1.15s cubic-bezier(0.34, 1.56, 0.64, 1) both; }
  .cele-stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(0, 1fr)); grid-auto-flow: column; gap: 10px; width: 100%; }
  .cele-stat { overflow: hidden; border: 2px solid var(--color-base-content); border-radius: 16px; background: var(--surface); animation: pop 0.42s cubic-bezier(0.22, 1, 0.36, 1) both; }
  .cele-label { display: block; padding: 4px; font-size: 0.7rem; font-weight: 900; letter-spacing: 0.08em; text-transform: uppercase; }
  .cele-stat strong { display: block; padding: 8px 4px 10px; font-size: 1.55rem; font-weight: 900; }
  .cele-go { animation: rise 0.38s 2.2s cubic-bezier(0.34, 1.56, 0.64, 1) both; }
  .cele-go[aria-disabled="true"] { cursor: default; }

  .quiet *, .quiet { animation-duration: 0.25s !important; animation-iteration-count: 1 !important; }
  .quiet .rays { animation: fade 0.25s both; }
  .quiet .cele-badge, .quiet .cele-title span, .quiet .cele-stat, .quiet .cele-line, .quiet .cele-go { animation-name: fade !important; animation-delay: 0s !important; }

  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes pop { 0% { opacity: 0; transform: scale(0.4); } 70% { opacity: 1; transform: scale(1.08); } 100% { opacity: 1; transform: none; } }
  @keyframes rise { from { opacity: 0; transform: translateY(24px); } to { opacity: 1; transform: none; } }
  @keyframes bob { 50% { transform: translateY(-5px) scale(0.98, 1.02); } }
  @keyframes jump {
    0% { opacity: 0; transform: translateY(40px) scale(0.6, 0.6); }
    22% { opacity: 1; transform: translateY(0) scale(1.15, 0.8); }
    50% { transform: translateY(-80px) scale(0.88, 1.14); }
    74% { transform: translateY(0) scale(1.22, 0.78); }
    86% { transform: translateY(0) scale(0.95, 1.06); }
    100% { opacity: 1; transform: none; }
  }
</style>

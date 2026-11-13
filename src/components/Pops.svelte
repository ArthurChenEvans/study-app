<script lang="ts">
  // celebrations.feature (R153): small moments pop up in the corner, never block anything,
  // and get a little bigger the further you go. Each one is said in words.
  import { Check, Flag, Flame, Layers, ListChecks, Sparkles, Trophy } from "@lucide/svelte";
  import confetti from "canvas-confetti";
  import { calm, live, type PopItem } from "../lib/celebrations.svelte";

  const ICONS = { check: Check, trophy: Trophy, list: ListChecks, layers: Layers, flame: Flame, flag: Flag, sparkles: Sparkles };
  const quiet = $derived(calm());

  function burst(node: HTMLElement, pop: PopItem) {
    if (pop.burst && !calm()) {
      setTimeout(() => {
        const box = node.getBoundingClientRect();
        confetti({ particleCount: 30 * pop.burst, spread: 70, startVelocity: 25, scalar: 0.8, zIndex: 1200, colors: ["#d34516", "#f2c14e", "#f7915f", "#8cc4e2", "#7fd1ae"], origin: { x: (box.left + 30) / innerWidth, y: box.top / innerHeight } });
      }, 250);
    }
  }
</script>

<div class="pops" class:quiet role="status" aria-live="polite">
  {#each live.pops as pop (pop.id)}
    {@const Icon = ICONS[pop.icon]}
    <div class="pop" use:burst={pop}>
      <span class="disc disc-{pop.icon}" aria-hidden="true"><Icon class="size-6" strokeWidth={2.6} /></span>
      <span><span class="block {pop.step >= 3 ? 'text-[18px] font-black' : 'font-extrabold'}">{pop.text}</span>{#if pop.sub}<span class="block text-sm font-bold text-muted">{pop.sub}</span>{/if}</span>
    </div>
  {/each}
</div>

<style>
  .pops { position: fixed; z-index: 1050; right: 16px; bottom: 16px; display: grid; justify-items: end; gap: 10px; max-width: calc(100vw - 32px); pointer-events: none; }
  .pop {
    display: flex; align-items: center; gap: 12px; padding: 10px 18px 10px 10px;
    background: var(--surface); border: 2px solid var(--color-base-content); border-radius: 18px; box-shadow: 0 5px 0 var(--color-base-content);
    transform-origin: 100% 100%; animation: in 0.42s cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .disc { width: 44px; height: 44px; flex: none; border-radius: 50%; display: grid; place-items: center; background: var(--color-primary); color: var(--color-primary-content); border: 3px solid var(--color-base-content); animation: hop 0.6s 0.15s both; }
  .disc-layers, .disc-sparkles { background: #f2c14e; color: #1e2650; }
  .disc-flame { background: #7fd1ae; color: #1e2650; }
  .disc-flag, .disc-list { background: #8cc4e2; color: #1e2650; }
  .quiet .pop, .quiet .disc { animation: fade 0.2s both; }
  @keyframes in { 0% { opacity: 0; transform: translateY(30px) scale(0.6); } 70% { opacity: 1; transform: translateY(-4px) scale(1.04); } 100% { opacity: 1; transform: none; } }
  @keyframes hop { 30% { transform: scale(1.2, 0.8); } 60% { transform: translateY(-14px) scale(0.9, 1.12); } 85% { transform: scale(1.1, 0.9); } }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
</style>

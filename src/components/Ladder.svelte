<script lang="ts">
  // Schedule track (design/free-recall.html): one segment per step, repeats as sub-ticks,
  // and a caption with the current step and what a pass leads to. Names aren't drawn per
  // segment because they don't fit.
  import type { ScheduleStep } from "../lib/api";

  let { steps, position, step, next, compact = false }: { steps: ScheduleStep[]; position: [number, number] | null; step?: string | null; next?: string; compact?: boolean } = $props();

  const here = $derived(position ?? [-1, 0]);
  const state = (index: number, sub: number, subs: number) => {
    const [at, rep] = here;
    if (index === at && (subs === 1 || sub === rep - 1)) return "here";
    if (index < at || (index === at && subs > 1 && sub < rep - 1)) return "done";
    return "";
  };
</script>

<div class="ladder" role="img" aria-label={position ? `Schedule step: ${step}, then ${next}` : `Not started, first step ${next}`}>
  {#each steps as item, index (index)}
    {@const subs = Math.max(1, item.repeats)}
    <div style="flex: {subs > 1 ? 2 : 1}">
      <div class="bar">{#each Array(subs) as _, sub (sub)}<span class={state(index, sub, subs)}></span>{/each}</div>
    </div>
  {/each}
</div>
{#if !compact}
  {#if position}
    <p class="mt-2 flex justify-between gap-4 text-sm"><span class="font-semibold">{step}</span><span class="text-muted">then {next}</span></p>
  {:else}
    <p class="mt-2 text-sm text-muted">First step: {next}</p>
  {/if}
{/if}

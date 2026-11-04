<script lang="ts">
  // Counts a number up from 0 over 600ms with a decelerating curve, then settles.
  // Reduced motion (or no animation frames) shows the final number straight away.
  // Screen readers always get the final number, never the counting.
  import { onDestroy } from "svelte";

  let { value, duration = 600, class: className = "" }: { value: number; duration?: number; class?: string } = $props();

  const still = typeof window === "undefined" || typeof window.requestAnimationFrame !== "function" || !window.matchMedia || window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let shown = $state(still ? value : 0);
  let frame = 0;

  if (!still) {
    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      shown = Math.round(value * (1 - Math.pow(1 - t, 3)));
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
  }

  onDestroy(() => cancelAnimationFrame(frame));
</script>

<span class="tabular-nums {still ? '' : 'settle'} {className}" aria-label={String(value)}><span aria-hidden="true">{shown}</span></span>

<script lang="ts">
  // Right (sure) / right (guessed) / wrong, always in that order with 2px gaps (design rules section 9).
  import type { Tally } from "../lib/api";
  let { tally, height = 8 }: { tally: Tally; height?: number } = $props();
  const wrong = $derived(tally.wrong + tally.unanswered);
  const total = $derived(tally.right_sure + tally.right_guessed + wrong);
  const share = (part: number) => (total === 0 ? 0 : (part / total) * 100);
</script>

<div class="seg-bar" style="height: {height}px" role="img" aria-label="{tally.right_sure} right and sure, {tally.right_guessed} guessed, {wrong} wrong or unanswered">
  {#if tally.right_sure}<span title="Right, sure: {tally.right_sure}" style="width: {share(tally.right_sure)}%; background: var(--chart-right)"></span>{/if}
  {#if tally.right_guessed}<span title="Right, guessed: {tally.right_guessed}" style="width: {share(tally.right_guessed)}%; background: var(--guessed-fill)"></span>{/if}
  {#if wrong}<span title="Wrong: {wrong}" style="width: {share(wrong)}%; background: var(--chart-wrong)"></span>{/if}
</div>

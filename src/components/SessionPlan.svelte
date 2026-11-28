<script lang="ts">
  // How a review session will go, before it starts: the groups in order (sized by how many
  // cards each holds) and where missed cards come back. Changing the order here is for this
  // session only; the default lives in card settings.
  import { GripVertical, RotateCcw } from "@lucide/svelte";
  import type { CardGroup, ReviewPlan, SessionOrder } from "../lib/api";

  let { plan, order = $bindable(), onSettings }: { plan: ReviewPlan; order: SessionOrder; onSettings: () => void } = $props();

  const ALL: CardGroup[] = ["learning", "review", "new"];
  const NAMES: Record<CardGroup, string> = { learning: "Learning", review: "Reviews", new: "New" };
  const TONE: Record<CardGroup, string> = { learning: "plan-learning", review: "plan-review", new: "plan-new" };
  const mixed = $derived(order === "mixed");
  // Every group exactly once, in the chosen order, so a stored order missing one still works.
  const groups = $derived.by(() => {
    const chosen = order === "mixed" ? ALL : order.groups;
    return [...chosen, ...ALL].filter((group, index, list) => list.indexOf(group) === index);
  });
  const count = (group: CardGroup) => plan[group];
  const total = $derived(plan.learning + plan.review + plan.new);

  function move(group: CardGroup, by: -1 | 1) {
    const list = [...groups];
    const from = list.indexOf(group);
    const to = from + by;
    if (to < 0 || to >= list.length) return;
    [list[from], list[to]] = [list[to], list[from]];
    order = { groups: list };
    status = `${NAMES[group]} moved to place ${to + 1} of ${list.length}`;
  }
  let status = $state("");

  // Drag a group onto another to swap places.
  let dragging = $state<CardGroup | null>(null);
  function drop(onto: CardGroup) {
    if (!dragging || dragging === onto) return;
    const list = [...groups];
    const from = list.indexOf(dragging);
    list.splice(from, 1);
    list.splice(list.indexOf(onto) + (from <= list.indexOf(onto) ? 1 : 0), 0, dragging);
    order = { groups: list };
    dragging = null;
  }

  const missedText = $derived(plan.missed_after === null ? "at the end of the session" : `${plan.missed_after} ${plan.missed_after === 1 ? "card" : "cards"} later`);
</script>

<div class="mt-5 border-t border-base-300 pt-4" aria-labelledby="plan-h">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h3 id="plan-h" class="text-[16px]">How this session will go</h3>
    <div class="join" role="radiogroup" aria-label="Session order">
      <button type="button" role="radio" aria-checked={!mixed} class="btn join-item btn-sm {mixed ? 'btn-ghost' : 'btn-active'}" onclick={() => (order = { groups })}>In groups</button>
      <button type="button" role="radio" aria-checked={mixed} class="btn join-item btn-sm {mixed ? 'btn-active' : 'btn-ghost'}" onclick={() => (order = "mixed")}>All mixed</button>
    </div>
  </div>
  {#if mixed}
    <div class="plan-row mt-3"><div class="plan-block plan-mixed" style="flex: 1"><b>All {total} {total === 1 ? "card" : "cards"}</b><span>{plan.learning} learning, {plan.review} reviews, {plan.new} new, shuffled together</span></div>
      <div class="plan-block plan-missed"><RotateCcw class="size-4" aria-hidden="true" /><span>Missed: {plan.missed_after === null ? "at the end" : `${plan.missed_after} later`}</span></div></div>
  {:else}
    <ol class="plan-row mt-3" aria-label="Groups in order. Focus one and press Alt with the left or right arrow to move it.">
      {#each groups as group, index (group)}
        <li class="plan-slot" style="flex: {Math.max(count(group), total / 8, 1)}"><button
          type="button"
          class="plan-block {TONE[group]}"
          class:opacity-50={!count(group)}
          class:plan-dragging={dragging === group}
          draggable="true"
          aria-label="{index + 1}. {NAMES[group]}, {count(group)} {count(group) === 1 ? 'card' : 'cards'}"
          ondragstart={(event) => { dragging = group; event.dataTransfer?.setData("text/plain", group); }}
          ondragend={() => (dragging = null)}
          ondragover={(event) => event.preventDefault()}
          ondrop={(event) => { event.preventDefault(); drop(group); }}
          onkeydown={(event) => { if (event.altKey && (event.key === "ArrowLeft" || event.key === "ArrowRight")) { event.preventDefault(); move(group, event.key === "ArrowLeft" ? -1 : 1); requestAnimationFrame(() => (event.currentTarget as HTMLElement | null)?.focus()); } }}
        ><GripVertical class="size-4 shrink-0 opacity-70" aria-hidden="true" /><span><b>{index + 1} · {NAMES[group]}</b><span>{count(group)} {count(group) === 1 ? "card" : "cards"}{group === "review" && count(group) > 1 ? ", shuffled" : ""}</span></span></button></li>
      {/each}
      <li class="plan-block plan-missed" aria-label="Missed cards come back {missedText}"><RotateCcw class="size-4 shrink-0" aria-hidden="true" /><span><b>Missed</b><span>{plan.missed_after === null ? "at the end" : `${plan.missed_after} later`}</span></span></li>
    </ol>
  {/if}
  <p class="mt-2 text-[13px] text-muted">{mixed ? "" : "Drag a group, or focus it and press Alt+← or Alt+→, to change the order for this session. "}Missed cards come back {missedText} (<button type="button" class="link" onclick={onSettings}>change in card settings</button>).</p>
  <p class="sr-only" role="status">{status}</p>
</div>

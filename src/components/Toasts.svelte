<script lang="ts">
  // toasts.feature (R155): calm notes about what just happened, bottom centre so they never
  // sit on the primary button. Errors are announced straight away and stay until closed.
  import { CalendarDays, Check, CircleAlert, Copy, Download, Move, Plus, RefreshCw, Trash2, Undo2, X } from "@lucide/svelte";
  import { calm, dismissToast, live, type ToastItem } from "../lib/celebrations.svelte";

  const ICONS = { plus: Plus, check: Check, trash: Trash2, move: Move, calendar: CalendarDays, download: Download, refresh: RefreshCw, copy: Copy, alert: CircleAlert, undo: Undo2 };
  const quiet = $derived(calm());
  const normal = $derived(live.toasts.filter((item) => !item.error));
  const errors = $derived(live.toasts.filter((item) => item.error));
</script>

{#snippet one(item: ToastItem)}
  {@const Icon = ICONS[item.icon ?? "check"]}
  <div class="toast" class:err={item.error}>
    <Icon class="size-5 shrink-0" strokeWidth={2.4} aria-hidden="true" />
    <span class="min-w-0 flex-1">{#if item.error}<span class="sr-only">{"Error: "}</span>{/if}{item.text}{#if item.sub}<span class="block text-sm font-bold text-muted">{item.sub}</span>{/if}</span>
    {#if item.action}<button type="button" class="btn btn-sm" onclick={item.action.run}>{item.action.label}</button>{/if}
    <button type="button" class="btn btn-quiet btn-sm btn-square" aria-label="Dismiss" onclick={() => dismissToast(item.id)}><X class="size-4" /></button>
    {#if item.drain}<i class="drain" style="animation-duration: {item.ms ?? 4000}ms" aria-hidden="true"></i>{/if}
  </div>
{/snippet}

<div class="toasts" class:quiet>
  <div role="status" aria-live="polite" class="contents">{#each normal as item (item.id)}{@render one(item)}{/each}</div>
  <div role="alert" class="contents">{#each errors as item (item.id)}{@render one(item)}{/each}</div>
</div>

<style>
  .toasts { position: fixed; z-index: 75; left: 50%; bottom: 16px; transform: translateX(-50%); display: grid; gap: 8px; width: min(460px, calc(100vw - 32px)); }
  .toast {
    position: relative; overflow: hidden; display: flex; align-items: center; gap: 10px; padding: 8px 8px 8px 12px; font-weight: 800;
    background: var(--surface); border: 2px solid var(--color-base-content); border-radius: 14px; box-shadow: 0 4px 0 var(--color-base-content);
    animation: in 0.24s cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .toast.err { border-color: var(--color-error); box-shadow: 0 4px 0 var(--color-error); }
  .toast.err > :global(svg:first-child) { color: var(--color-error); }
  .drain { position: absolute; left: 0; bottom: 0; height: 4px; width: 100%; background: var(--color-primary); transform-origin: 0 50%; animation: drain linear both; }
  .quiet .toast { animation: fade 0.15s both; }
  @keyframes in { from { opacity: 0; transform: translateY(16px); } to { opacity: 1; transform: none; } }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
  @keyframes drain { from { transform: scaleX(1); } to { transform: scaleX(0); } }
</style>

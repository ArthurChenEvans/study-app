<script lang="ts">
  import { stackTable } from "../lib/stack";
  // sync.feature (R119 to R128), on Your data. The section's goal: keep the computers in
  // step without thinking about it. Most important: whether it's working, which the status
  // card above says in words. Here: which computers there are, the folder, and anything to
  // check. When something needs fixing, the fix is the page's one primary button.
  import { Check, FolderSync, Pencil, Power, RefreshCw, Trash2 } from "@lucide/svelte";
  import { api, errorMessage, type SyncResult, type SyncStatus, type ToCheck } from "../lib/api";
  import { agoLabel, fieldLabel, groupChecks, whenLabel, wordDiff } from "../lib/sync";
  import SyncSetupDialog from "./SyncSetupDialog.svelte";

  let { status, checks, fix, onStatus, onChanged }: { status: SyncStatus; checks: ToCheck[]; fix: "paused" | "folder" | null; onStatus: (status: SyncStatus) => Promise<void>; onChanged: () => void } = $props();

  let setup = $state<"setup" | "folder" | null>(null);
  let renaming = $state<string | null>(null);
  let busy = $state(false);
  let error = $state("");

  const now = () => Math.floor(Date.now() / 1000);
  const groups = $derived(groupChecks(checks));
  const thingCount = (count: number) => `${count} ${count === 1 ? "thing" : "things"} to check`;

  async function run(action: () => Promise<SyncResult | SyncStatus>) {
    busy = true;
    error = "";
    try {
      const result = await action();
      const next = "status" in result ? result.status : result;
      if ("received" in result && result.received) onChanged();
      await onStatus(next);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  async function setUp(result: SyncResult) {
    setup = null;
    if (result.received) onChanged();
    await onStatus(result.status);
  }

  async function rename(event: SubmitEvent) {
    event.preventDefault();
    const name = renaming ?? "";
    await run(() => api.syncRename(name));
    if (!error) renaming = null;
  }

  async function settle(group: { fields: ToCheck[] }, useOther: boolean) {
    busy = true;
    error = "";
    try {
      for (const field of group.fields) await api.syncSettle(field.n, useOther);
      if (useOther) onChanged();
      await onStatus(await api.syncStatus());
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function waitingLabel(device: SyncStatus["devices"][number]): string {
    const waiting = device.this_one ? status.waiting : device.waiting;
    if (!waiting) return "None";
    if (device.this_one) return fix === "folder" ? `${waiting} waiting for the folder` : `${waiting} to send`;
    return `${waiting} on the way`;
  }
</script>

<section class="panel p-6" aria-labelledby="sync-h">
  <div class="flex items-start justify-between gap-4">
    <div>
      <h2 id="sync-h" class="text-lg font-semibold">Sync</h2>
      <p class="mt-1 text-sm text-muted">Keeps your computers (and the browser version) in step through a folder you already keep in sync, like Syncthing. No account, no server, nothing to pay for.</p>
    </div>
    {#if !status.on}
      <button class="btn btn-outline shrink-0" onclick={() => (setup = "setup")}><RefreshCw class="size-4" />Set up sync</button>
    {:else if fix === "paused"}
      <button class="btn btn-primary shrink-0" disabled={busy} onclick={() => run(() => api.syncResume())}><RefreshCw class="size-4" />Turn sync back on</button>
    {:else}
      <button class="btn btn-outline shrink-0" disabled={busy} onclick={() => run(() => api.syncNow(false))}><RefreshCw class="size-4" />{busy ? "Syncing…" : "Sync now"}</button>
    {/if}
  </div>

  {#if error}<p role="alert" class="mt-4 text-sm text-error">{error}</p>{/if}

  {#if status.on}
    <div class="mt-5 overflow-hidden rounded-lg border border-base-300">
      <table class="table" aria-label="Computers" use:stackTable>
        <thead><tr class="text-muted"><th>Computer</th><th>Last seen</th><th class="text-right">Changes waiting</th></tr></thead>
        <tbody>
          {#each status.devices as device (device.id)}
            <tr>
              <td><span class="font-semibold">{device.name}</span>{#if device.this_one} <span class="kind ml-1">This one</span>{/if}{#if device.newer_app} <span class="kind ml-1">Newer app</span>{/if}</td>
              <td>{device.this_one ? "Now" : device.last_seen === null ? "Not seen yet" : agoLabel(device.last_seen, now()).replace(/^./, (letter) => letter.toUpperCase())}</td>
              <td class="text-right tabular-nums">{waitingLabel(device)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <dl class="mt-5 grid grid-cols-[9rem_minmax(0,1fr)_auto] items-center gap-x-4 gap-y-3 text-sm">
      <dt class="text-muted">Sync folder</dt>
      <dd class="min-w-0"><span class="path break-all">{status.folder}</span></dd>
      <dd><button class="btn btn-sm {fix === 'folder' ? 'btn-primary' : 'btn-ghost'}" onclick={() => (setup = "folder")}><FolderSync class="size-4" />Change folder</button></dd>
      <dt class="text-muted">This computer</dt>
      <dd>{status.name}</dd>
      <dd><button class="btn btn-ghost btn-sm" aria-label="Rename this computer" onclick={() => (renaming = status.name)}><Pencil class="size-4" />Rename</button></dd>
      <dt class="text-muted">Checks the folder</dt>
      <dd>Every 30 seconds while the app is open, and when it starts</dd>
      <dd></dd>
      {#if status.held}
        <dt class="text-muted">Waiting</dt>
        <dd>{status.held} {status.held === 1 ? "thing refers" : "things refer"} to something still on its way from another computer</dd>
        <dd></dd>
      {/if}
    </dl>

    {#if groups.length}
      <div class="mt-6 rounded-lg border border-base-content p-5" role="region" aria-labelledby="checks-h">
        <h3 id="checks-h" class="font-semibold">{thingCount(groups.length)}</h3>
        <p class="mt-1 text-sm text-muted">Changed on two computers before they synced. The newer change is in use everywhere. Keep it, or switch to the other one. Nothing is lost either way.</p>
        {#each groups as group (group.record)}
          {@const first = group.first}
          <div class="mt-5 border-t border-base-300 pt-4 first-of-type:border-0 first-of-type:pt-0">
            <p class="text-sm"><span class="text-muted">{first.what} in</span> <b>{first.label || "(untitled)"}</b>{#if first.context}{` · ${first.context}`}{/if}</p>
            {#if first.deleted}
              <p class="mt-2 text-[15px]">Deleted on {first.other_by}, but {first.kept_by} changed or added to it before it knew, so it's been kept.</p>
              <div class="mt-3 flex gap-2">
                <button class="btn btn-outline btn-sm" disabled={busy} onclick={() => settle(group, false)}><Check class="size-4" />Keep it</button>
                <button class="btn btn-ghost btn-sm" disabled={busy} onclick={() => settle(group, true)}><Trash2 class="size-4" />Delete it after all</button>
              </div>
            {:else}
              {#each group.fields as field (field.n)}
                {@const diff = wordDiff(field.other ?? "", field.kept ?? "")}
                <div class="mt-3 grid grid-cols-2 gap-4">
                  <div class="note-box p-4" role="group" aria-label="{field.kept_by}'s version, in use">
                    <p class="flex items-center justify-between gap-2 text-xs text-muted"><span><b class="text-base-content">{field.kept_by}</b>, {whenLabel(field.kept_when, now())}</span><span class="kind">In use, newer</span></p>
                    <p class="mt-3 text-sm text-muted">{fieldLabel(field.field)}</p>
                    <p class="whitespace-pre-line text-[15px]">{#each diff.newer as piece, index (index)}{#if piece.kind === "added"}<ins class="chg">{piece.text}</ins>{:else}{piece.text}{/if}{:else}<span class="text-muted">(empty)</span>{/each}</p>
                  </div>
                  <div class="note-box p-4" role="group" aria-label="{field.other_by}'s version">
                    <p class="text-xs text-muted"><b class="text-base-content">{field.other_by}</b>, {whenLabel(field.other_when, now())}</p>
                    <p class="mt-3 text-sm text-muted">{fieldLabel(field.field)}</p>
                    <p class="whitespace-pre-line text-[15px]">{#each diff.older as piece, index (index)}{#if piece.kind === "removed"}<del class="chg">{piece.text}</del>{:else}{piece.text}{/if}{:else}<span class="text-muted">(empty)</span>{/each}</p>
                  </div>
                </div>
              {/each}
              <p class="mt-2 text-xs text-muted">Underlined: only in the newer one. Struck through: only in the older one.</p>
              <div class="mt-3 flex gap-2">
                <button class="btn btn-outline btn-sm" disabled={busy} onclick={() => settle(group, false)}><Check class="size-4" />Keep {first.kept_by}'s</button>
                <button class="btn btn-ghost btn-sm" disabled={busy} onclick={() => settle(group, true)}>Use {first.other_by}'s instead</button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <div class="mt-4 flex justify-end"><button class="btn btn-ghost btn-sm" disabled={busy} onclick={() => run(() => api.syncOff())}><Power class="size-4" />Turn sync off</button></div>
  {/if}
</section>

{#if setup}<SyncSetupDialog {status} moving={setup === "folder"} onClose={() => (setup = null)} onDone={setUp} />{/if}

{#if renaming !== null}
  <dialog open class="modal modal-open" aria-labelledby="rename-h">
    <form class="modal-box max-w-md" onsubmit={rename}>
      <h3 id="rename-h" class="text-lg font-semibold">Rename this computer</h3>
      <p class="mt-1 text-sm text-muted">So the other computers know which one this is.</p>
      <label class="mt-5 block"><span class="text-sm font-semibold">Name</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full" bind:value={renaming} maxlength="40" autofocus /></label>
      {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (renaming = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary" disabled={busy}>Rename</button>
      </div>
    </form>
  </dialog>
{/if}

<script lang="ts">
  // sync.feature: Setting up the second computer (R119, R120). The dialog's goal: connect
  // two computers once and forget about it. Most important: which folder, and that the other
  // computer just picks the same one. Each step's state is in its number (a tick when done),
  // and in words for screen readers, not only in colour.
  import { Check, Info, RefreshCw } from "@lucide/svelte";
  import { api, errorMessage, type SyncResult, type SyncStatus } from "../lib/api";

  let { status, moving = false, onClose, onDone }: { status: SyncStatus; moving?: boolean; onClose: () => void; onDone: (result: SyncResult) => void } = $props();

  // svelte-ignore state_referenced_locally
  let folder = $state(moving ? (status.folder ?? "") : "");
  // svelte-ignore state_referenced_locally
  let name = $state(status.name);
  let busy = $state(false);
  let error = $state("");

  const folderDone = $derived(folder.trim().startsWith("/"));
  const nameDone = $derived(name.trim().length > 0);

  async function start(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = "";
    try {
      onDone(await api.syncSetup({ folder: folder.trim(), name: name.trim() }));
    } catch (failure) {
      error = errorMessage(failure);
      busy = false;
    }
  }
</script>

<dialog open class="modal modal-open" aria-labelledby="sync-setup-h">
  <form class="modal-box max-w-lg" onsubmit={start}>
    <h3 id="sync-setup-h" class="text-lg font-semibold">{moving ? "Change the sync folder" : "Set up sync"}</h3>
    <ol class="mt-5 space-y-5">
      <li class="flex gap-3">
        <span class="step-dot {folderDone ? 'done' : 'now'}" aria-hidden="true">{#if folderDone}<Check class="size-4" />{:else}1{/if}</span>
        <div class="min-w-0 flex-1">
          <p class="font-semibold">{#if folderDone}<span class="sr-only">Done: </span>{/if}Pick a folder that's kept in sync</p>
          <p class="mt-1 text-sm text-muted">Any folder another app keeps the same on each computer. Syncthing is free and needs no account. Nextcloud, a network drive or a USB stick you carry also work.</p>
          <label class="mt-2 block"><span class="sr-only">Sync folder</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="input w-full font-mono text-sm" bind:value={folder} placeholder="/home/you/Sync/StudyApp" autofocus /></label>
          <p class="mt-1 text-sm text-muted">The full path, starting with /. The folder needs to exist already.</p>
        </div>
      </li>
      <li class="flex gap-3">
        <span class="step-dot {nameDone && folderDone ? 'done' : folderDone ? 'now' : ''}" aria-hidden="true">{#if nameDone && folderDone}<Check class="size-4" />{:else}2{/if}</span>
        <div class="min-w-0 flex-1">
          <p class="font-semibold">{#if nameDone && folderDone}<span class="sr-only">Done: </span>{/if}Name this computer</p>
          <label class="mt-2 block"><span class="sr-only">Name this computer</span><input class="input w-full" bind:value={name} maxlength="40" /></label>
        </div>
      </li>
      <li class="flex gap-3">
        <span class="step-dot" aria-hidden="true">3</span>
        <div>
          <p class="font-semibold">On your other computer, pick the same folder</p>
          <p class="mt-1 text-sm text-muted">It finds this one there and brings everything over. The first sync of a big profile takes a minute or two.</p>
        </div>
      </li>
    </ol>
    <p class="mt-6 flex gap-2 note-box p-3 text-sm"><Info class="mt-0.5 size-4 shrink-0" /><span>The app never puts its database in the folder, only small files of changes, one per computer. Syncing a database file itself corrupts it.</span></p>
    {#if !moving}<p class="mt-3 text-sm text-muted">A backup is made first.</p>{/if}
    {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
    <div class="modal-action">
      <button type="button" class="btn btn-ghost" onclick={onClose}>Cancel</button>
      <button type="submit" class="btn btn-primary" disabled={busy}><RefreshCw class="size-4" />{busy ? "Syncing…" : moving ? "Use this folder" : "Start syncing"}</button>
    </div>
  </form>
</dialog>

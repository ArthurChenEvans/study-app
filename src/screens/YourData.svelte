<script lang="ts">
  // backup.feature (R101 to R108). The view's goal: know your study is safe, and put it
  // back if something goes wrong. So it leads with when the last backup was, in words.
  import { Copy, FolderInput, HardDriveDownload, Plus, RotateCcw, ShieldAlert, ShieldCheck } from "@lucide/svelte";
  import { tick } from "svelte";
  import { api, errorMessage, type BackupInfo, type BackupSettings, type BackupStatus, type RestorePreview } from "../lib/api";
  import { keptLabel, madeDay, madeInline, madeLabel, reasonLabel, sizeLabel, statusLine } from "../lib/backup";
  import PageHeader from "../components/PageHeader.svelte";
  import ExportDialog from "../components/ExportDialog.svelte";
  import ImportDialog from "../components/ImportDialog.svelte";
  import { FileInput, FileOutput } from "@lucide/svelte";
  import type { ExportFile, ImportPreview, ImportResult } from "../lib/api";
  import { countList } from "../lib/transfer";
  import type { SyncStatus, ToCheck } from "../lib/api";
  import { syncLine } from "../lib/sync";
  import SyncSection from "../components/SyncSection.svelte";
  import { FolderX, GitCompare, RefreshCw, RefreshCwOff } from "@lucide/svelte";

  let { onRestored = () => {}, onSync = () => {} }: { onRestored?: () => void; onSync?: (status: SyncStatus | null) => void } = $props();

  let status = $state<BackupStatus | null>(null);
  let list = $state<BackupInfo[]>([]);
  let today = $state(0);
  let showAll = $state(false);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");

  // ---------- Export and import (R112 to R118) ----------
  let exporting = $state(false);
  let importing = $state<ImportPreview | null>(null);

  function exported(file: ExportFile) {
    exporting = false;
    error = "";
    if (file.data) {
      // The browser version hands the file to the browser to save.
      const bytes = Uint8Array.from(atob(file.data), (letter) => letter.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: "application/zip" }));
      const link = document.createElement("a");
      link.href = url;
      link.download = file.name;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 10_000);
      notice = `Downloaded ${file.name}.`;
    } else {
      notice = `Saved ${file.name} to ${file.path}.`;
    }
  }

  async function pickFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    error = "";
    notice = "";
    try {
      importing = await api.importPreview(file, file.name);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  function closeImport() {
    importing = null;
    Promise.resolve(api.cancelImport()).catch(() => {});
  }

  async function imported(result: ImportResult) {
    importing = null;
    const replaced = result.replaced ? `, and used the file's version of ${result.replaced} ${result.replaced === 1 ? "thing" : "things"}` : "";
    await load();
    const backup = result.backup;
    const undo = backup ? ` To undo it, restore the backup from ${madeDay(backup, today)} at ${madeLabel(backup, today).split(", ")[1]}.` : "";
    notice = `Imported ${countList(result.added)}${replaced}.${undo}`;
    onRestored();
  }

  let restoring = $state<RestorePreview | null>(null);
  let keeping = $state<{ keep_days: number; keep_weeks: number; keep_months: number } | null>(null);
  let copying = $state<string | null>(null);
  let dialogError = $state("");
  let copied = $state(false);

  async function load() {
    [status, list, today] = await Promise.all([api.backupStatus(), api.backups(), api.today()]);
    await loadSync();
  }

  // ---------- Sync (R119 to R128) ----------
  let sync = $state<SyncStatus | null>(null);
  let checks = $state<ToCheck[]>([]);
  async function loadSync(next?: SyncStatus) {
    try {
      sync = next ?? (await api.syncStatus());
      checks = sync?.to_check ? await api.syncToCheck() : [];
      onSync(sync);
    } catch {
      sync = null;
    }
  }
  const syncWords = $derived(sync ? syncLine(sync, Math.floor(Date.now() / 1000)) : null);
  // The page's one primary button is whatever fixes sync, when it needs fixing.
  const syncFix = $derived<"paused" | "folder" | null>(sync?.on ? (sync.paused ? "paused" : sync.problem?.startsWith("Can't find the sync folder") ? "folder" : null) : null);

  async function backUp() {
    busy = true;
    error = "";
    notice = "";
    try {
      await api.backUpNow();
      await load();
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  async function askRestore(backup: BackupInfo) {
    error = "";
    try {
      restoring = await api.restorePreview(backup.name);
      await tick();
      document.getElementById("restore-cancel")?.focus();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function restore() {
    if (!restoring) return;
    const chosen = restoring.backup;
    busy = true;
    try {
      const safety = await api.restoreBackup(chosen.name);
      restoring = null;
      notice = `Restored the backup from ${madeInline(chosen, today)}. Your data from just before is in the backup from ${madeDay(safety, today)} at ${madeLabel(safety, today).split(", ")[1]}, restore that to undo this.`;
      onRestored();
      await load();
    } catch (failure) {
      restoring = null;
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  async function save(settings: BackupSettings) {
    dialogError = "";
    try {
      status = await api.setBackupSettings(settings);
      list = await api.backups();
      keeping = null;
      copying = null;
    } catch (failure) {
      dialogError = errorMessage(failure);
    }
  }

  async function copyPath() {
    if (!status) return;
    try {
      await navigator.clipboard.writeText(status.folder);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      /* the path is selectable text too */
    }
  }

  function openKeeping() {
    if (!status) return;
    const { keep_days, keep_weeks, keep_months } = status.settings;
    keeping = { keep_days, keep_weeks, keep_months };
    dialogError = "";
  }

  function openCopying() {
    copying = status?.settings.copy_to ?? "";
    dialogError = "";
  }

  const line = $derived(status ? statusLine(status, today) : null);
  const shown = $derived(showAll ? list : list.slice(0, 5));
  const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

  load();
</script>

<div class="mx-auto max-w-4xl px-12 pb-24">
  <PageHeader title="Your data" subtitle="Backups, sync, and export and import. Everything stays on your computers." />

  {#if status && line}
    <div class="space-y-6">
      <section class="panel p-6" aria-labelledby="safe-h">
        <h2 id="safe-h" class="sr-only">Is your data safe?</h2>
        <div role="status" class="space-y-4">
          <div class="flex items-start gap-3">
            <span class="grid size-9 shrink-0 place-items-center rounded-full border-2 {line.safe && status.latest ? 'text-right-ink' : 'text-wrong-ink'}">
              {#if line.safe && status.latest}<ShieldCheck class="size-4" />{:else}<ShieldAlert class="size-4" />{/if}
            </span>
            <div>
              <p class="font-semibold">{status.latest ? line.title : status.has_data ? "No backups yet" : "Nothing to back up yet"}</p>
              <p class="text-sm text-muted">
                {#if status.latest}Each day you use the app, plus before restores and updates. {status.count} kept, {sizeLabel(status.bytes)}.
                {:else if status.has_data}One is made each day you use the app, or press Back up now.
                {:else}Backups start once you've added a topic or a card.{/if}
              </p>
            </div>
          </div>
          {#if sync && syncWords}
            <div class="flex items-start gap-3">
              <span class="grid size-9 shrink-0 place-items-center rounded-full border-2 {syncWords.ok ? 'text-right-ink' : 'text-wrong-ink'}">
                {#if !sync.on}<RefreshCwOff class="size-4" />{:else if syncFix === "folder"}<FolderX class="size-4" />{:else if sync.to_check}<GitCompare class="size-4" />{:else}<RefreshCw class="size-4" />{/if}
              </span>
              <div><p class="font-semibold">{syncWords.title}</p><p class="text-sm text-muted">{syncWords.detail}</p></div>
            </div>
          {/if}
          {#if status.copy_problem}
            <div class="flex items-start gap-3">
              <span class="grid size-9 shrink-0 place-items-center rounded-full border-2 text-wrong-ink"><ShieldAlert class="size-4" /></span>
              <div><p class="font-semibold">{status.copy_problem}</p><p class="text-sm text-muted">Plug the drive back in, or pick another folder below.</p></div>
            </div>
          {/if}
        </div>
      </section>

      {#if notice}<p role="alert" class="panel p-4 text-sm">{notice}</p>{/if}
      {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}

      <section class="panel p-6" aria-labelledby="backups-h">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 id="backups-h" class="text-lg font-semibold">Backups</h2>
            <p class="mt-1 text-sm text-muted">A full copy of everything, made while the app is open. Your study isn't interrupted.</p>
          </div>
          <button class="btn shrink-0 {syncFix ? 'btn-outline border-field-border' : 'btn-primary'}" disabled={busy} onclick={backUp}><HardDriveDownload class="size-4" />{busy ? "Backing up…" : "Back up now"}</button>
        </div>

        {#if list.length}
          <div class="mt-5 overflow-hidden rounded-lg border border-base-300">
            <table class="table" aria-label="Backups">
              <thead><tr class="text-muted"><th>Made</th><th>Why</th><th class="text-right">Size</th><th><span class="sr-only">Restore</span></th></tr></thead>
              <tbody>
                {#each shown as backup (backup.name)}
                  <tr>
                    <td class="tabular-nums">{madeLabel(backup, today)}</td>
                    <td><span class="badge-src">{reasonLabel[backup.reason]}</span></td>
                    <td class="text-right tabular-nums">{sizeLabel(backup.bytes)}</td>
                    <td class="text-right"><button class="btn btn-ghost btn-sm" onclick={() => askRestore(backup)} aria-label="Restore the backup from {madeInline(backup, today)}"><RotateCcw class="size-4" />Restore</button></td>
                  </tr>
                {/each}
                {#if !showAll && list.length > 5}
                  <tr><td colspan="4"><button class="btn btn-ghost btn-sm" onclick={() => (showAll = true)}>Show all {list.length}</button></td></tr>
                {/if}
              </tbody>
            </table>
          </div>
        {/if}

        <dl class="mt-5 grid grid-cols-[9rem_1fr_auto] items-center gap-x-4 gap-y-3 text-sm">
          <dt class="text-muted">Kept</dt>
          <dd>{keptLabel(status.settings)}</dd>
          <dd><button class="btn btn-ghost btn-sm" onclick={openKeeping} aria-label="Change how many are kept">Change</button></dd>
          <dt class="text-muted">Saved in</dt>
          <dd class="min-w-0"><span class="path break-all">{status.folder}</span></dd>
          <dd><button class="btn btn-ghost btn-sm" onclick={copyPath}><Copy class="size-4" />{copied ? "Copied" : "Copy path"}</button></dd>
          <dt class="text-muted">Also copy to</dt>
          <dd class="min-w-0">{#if status.settings.copy_to}<span class="path break-all">{status.settings.copy_to}</span>{:else}<span class="text-muted">Nowhere else. A USB stick or another drive survives this one failing.</span>{/if}</dd>
          <dd><button class="btn btn-ghost btn-sm" onclick={openCopying}>{#if status.settings.copy_to}<FolderInput class="size-4" />Change{:else}<Plus class="size-4" />Add a place{/if}</button></dd>
        </dl>
      </section>

      {#if sync}<SyncSection status={sync} {checks} fix={syncFix} onStatus={loadSync} onChanged={onRestored} />{/if}

      <section class="panel p-6" aria-labelledby="transfer-h">
        <h2 id="transfer-h" class="text-lg font-semibold">Export and import</h2>
        <p class="mt-1 text-sm text-muted">One <span class="path">.studyapp</span> file: readable JSON inside a zip, with images. Share topics with a friend, or move everything to another computer.</p>
        <div class="mt-5 flex gap-2">
          <button class="btn btn-outline border-field-border" onclick={() => { notice = ""; exporting = true; }}><FileOutput class="size-4" />Export…</button>
          <label class="btn btn-outline border-field-border file-button"><FileInput class="size-4" />Import…<input type="file" accept=".studyapp,application/zip" class="sr-only" aria-label="Import a .studyapp file" onchange={pickFile} /></label>
        </div>
      </section>
    </div>
  {/if}
</div>

{#if exporting}<ExportDialog onClose={() => (exporting = false)} onDone={exported} />{/if}
{#if importing}<ImportDialog preview={importing} {today} onClose={closeImport} onDone={imported} />{/if}

{#if restoring}
  {@const chosen = restoring.backup}
  <dialog open class="modal modal-open" aria-labelledby="restore-h">
    <div class="modal-box max-w-lg">
      <h3 id="restore-h" class="text-lg font-semibold">Restore the backup from {madeInline(chosen, today)}?</h3>
      {#if restoring.undone.length}
        <p class="mt-3 text-[15px]">Everything goes back to how it was then. That undoes:</p>
        <ul class="mt-2 list-disc space-y-1 pl-6 text-[15px]">
          {#each restoring.undone as item (item.many)}<li>{item.count} {item.count === 1 ? item.one : item.many}</li>{/each}
        </ul>
      {:else}
        <p class="mt-3 text-[15px]">Everything goes back to how it was then. Nothing you've done since then would be lost.</p>
      {/if}
      {#if restoring.older}<p class="mt-3 text-sm text-muted">It was made by an older version of the app, so it's upgraded as it comes back.</p>{/if}
      <p class="mt-4 flex gap-2 rounded-lg bg-base-200 p-3 text-sm"><ShieldCheck class="mt-0.5 size-4 shrink-0" />Your data as it is now is backed up first, so you can undo this by restoring that one.</p>
      <div class="modal-action">
        <button id="restore-cancel" class="btn btn-ghost" onclick={() => (restoring = null)}>Cancel</button>
        <button class="btn btn-primary" disabled={busy} onclick={restore}><RotateCcw class="size-4" />Restore {madeDay(chosen, today) === "today" ? "today's" : madeDay(chosen, today)}</button>
      </div>
    </div>
  </dialog>
{/if}

{#if keeping}
  <dialog open class="modal modal-open" aria-labelledby="keep-h">
    <form class="modal-box max-w-md" onsubmit={(event) => { event.preventDefault(); if (status && keeping) save({ ...status.settings, ...keeping }); }}>
      <h3 id="keep-h" class="text-lg font-semibold">How many backups to keep</h3>
      <p class="mt-1 text-sm text-muted">The newest backup of each. Anything made today is always kept.</p>
      <div class="mt-5 grid grid-cols-3 gap-4">
        <label class="block"><span class="text-sm font-semibold">Days</span><input type="number" min="1" max="365" class="input mt-1 w-full" bind:value={keeping.keep_days} /></label>
        <label class="block"><span class="text-sm font-semibold">Weeks</span><input type="number" min="0" max="520" class="input mt-1 w-full" bind:value={keeping.keep_weeks} /></label>
        <label class="block"><span class="text-sm font-semibold">Months</span><input type="number" min="0" max="240" class="input mt-1 w-full" bind:value={keeping.keep_months} /></label>
      </div>
      <p class="mt-3 text-sm text-muted">{keptLabel(keeping)}, about {plural(keeping.keep_days + keeping.keep_weeks + keeping.keep_months, "backup", "backups")} at most.</p>
      {#if dialogError}<p role="alert" class="mt-3 text-sm text-error">{dialogError}</p>{/if}
      <div class="modal-action">
        <button type="button" class="btn btn-ghost" onclick={() => (keeping = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Save</button>
      </div>
    </form>
  </dialog>
{/if}

{#if copying !== null}
  <dialog open class="modal modal-open" aria-labelledby="copy-h">
    <form class="modal-box max-w-lg" onsubmit={(event) => { event.preventDefault(); if (status) save({ ...status.settings, copy_to: copying || null }); }}>
      <h3 id="copy-h" class="text-lg font-semibold">Also copy backups to</h3>
      <p class="mt-1 text-sm text-muted">Every backup is copied here too, and thinned out the same way. Another drive or a USB stick survives this one failing.</p>
      <label class="mt-5 block"><span class="text-sm font-semibold">Folder</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input mt-1 w-full font-mono text-sm" bind:value={copying} placeholder="/run/media/you/USB/Study backups" autofocus /></label>
      <p class="mt-2 text-sm text-muted">The full path, starting with /. The folder needs to exist already.</p>
      {#if dialogError}<p role="alert" class="mt-3 text-sm text-error">{dialogError}</p>{/if}
      <div class="modal-action">
        {#if status?.settings.copy_to}<button type="button" class="btn btn-ghost mr-auto" onclick={() => status && save({ ...status.settings, copy_to: null })}>Stop copying</button>{/if}
        <button type="button" class="btn btn-ghost" onclick={() => (copying = null)}>Cancel</button>
        <button type="submit" class="btn btn-primary">Copy backups here</button>
      </div>
    </form>
  </dialog>
{/if}

<script lang="ts">
  // accounts.feature (R206 to R210): sign in with a code emailed to you, two-step sign-in with
  // an authenticator app, the computers signed in, your data, and the admin's image switch.
  import { KeyRound, LogOut, Mail, RefreshCw, ShieldCheck, Trash2 } from "@lucide/svelte";
  import { tick } from "svelte";
  import { api, errorMessage, type CloudStatus, type ExportFile, type TwoStepSetup } from "../lib/api";
  import { toast } from "../lib/celebrations.svelte";
  import ExportDialog from "./ExportDialog.svelte";

  let status = $state<CloudStatus | null>(null);
  let busy = $state(false);
  let error = $state("");

  // Signing in: email, then the emailed code, then (with two-step on) the authenticator code.
  let step = $state<"idle" | "email" | "code">("idle");
  let email = $state("");
  let code = $state("");
  let twoStepCode = $state("");
  let wait = $state(0);
  let timer: ReturnType<typeof setInterval> | null = null;

  let setup = $state<TwoStepSetup | null>(null);
  let setupCode = $state("");
  let signingOut = $state(false);
  let deleting = $state(false);
  let deleteWord = $state("");
  let exporting = $state(false);

  async function load() {
    try {
      status = await api.cloudStatus();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }
  load();

  async function run(action: () => Promise<void>) {
    busy = true;
    error = "";
    try {
      await action();
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      busy = false;
    }
  }

  function countdown() {
    wait = 60;
    if (timer) clearInterval(timer);
    timer = setInterval(() => {
      wait -= 1;
      if (wait <= 0 && timer) clearInterval(timer);
    }, 1000);
  }

  const sendCode = (event?: Event) => run(async () => {
    event?.preventDefault();
    await api.cloudRequestCode(email.trim());
    step = "code";
    code = "";
    countdown();
    await tick();
    document.getElementById("email-code")?.focus();
  });

  const verify = (event?: Event) => run(async () => {
    event?.preventDefault();
    status = await api.cloudVerifyCode(email.trim(), code);
    step = "idle";
    await tick();
    if (status.needs_two_step) document.getElementById("two-step-code")?.focus();
    else {
      toast({ text: "Signed in", sub: "Your study syncs to your account now" });
      void Promise.resolve(api.syncNow(false)).catch(() => {});
    }
  });

  const verifyTwoStep = (event?: Event) => run(async () => {
    event?.preventDefault();
    status = await api.cloudVerifyTwoStep(twoStepCode);
    twoStepCode = "";
    toast({ text: "Signed in", sub: "Your study syncs to your account now" });
    void Promise.resolve(api.syncNow(false)).catch(() => {});
  });

  const syncNow = () => run(async () => {
    await api.syncNow(false);
    status = await api.cloudStatus();
    toast({ text: "Synced" });
  });

  const startTwoStep = () => run(async () => {
    setup = await api.cloudTwoStepStart();
    setupCode = "";
  });

  const confirmTwoStep = (event: Event) => run(async () => {
    event.preventDefault();
    if (!setup) return;
    status = await api.cloudTwoStepConfirm(setup.factor, setupCode);
    setup = null;
    toast({ text: "Two-step sign-in is on", sub: "Signing in now also asks for your authenticator code" });
  });

  const twoStepOff = () => run(async () => {
    status = await api.cloudTwoStepOff();
    toast({ text: "Two-step sign-in is off" });
  });

  const signOutOthers = () => run(async () => {
    status = await api.cloudSignOutOthers();
    toast({ text: "Signed out everywhere else" });
  });

  const signOut = () => run(async () => {
    status = await api.cloudSignOut();
    signingOut = false;
    toast({ text: "Signed out", sub: "Everything stays on this computer" });
  });

  const deleteAccount = () => run(async () => {
    status = await api.cloudDeleteAccount();
    deleting = false;
    deleteWord = "";
    toast({ text: "Account deleted", sub: "Everything synced to it is gone. This computer's copy is still here" });
  });

  const setImages = (allowed: boolean) => run(async () => {
    status = await api.cloudSetImages(allowed);
  });

  function exported(file: ExportFile) {
    exporting = false;
    if (file.data) {
      const bytes = Uint8Array.from(atob(file.data), (letter) => letter.charCodeAt(0));
      const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
      const link = document.createElement("a");
      link.href = url;
      link.download = file.name;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 10_000);
    }
    toast({ text: `Downloaded ${file.name}`, icon: "export" });
  }

  const when = (unix: number | null) => (unix === null ? "Not yet" : new Date(unix * 1000).toLocaleString(undefined, { weekday: "short", hour: "2-digit", minute: "2-digit" }));
</script>

{#snippet otpField(id: string, label: string, value: string, onInput: (text: string) => void)}
  <label class="block" for={id}><span class="block text-sm font-bold">{label}</span></label>
  <div class="otp mt-1">
    <input {id} type="text" inputmode="numeric" autocomplete="one-time-code" pattern="[0-9]*" maxlength="6" required aria-label={label} {value} oninput={(event) => onInput(event.currentTarget.value.replace(/\D/g, "").slice(0, 6))} />
    <span></span><span></span><span></span><span></span><span></span><span></span>
  </div>
{/snippet}

<section class="panel p-5" aria-labelledby="account-h">
  <h2 id="account-h" class="eyebrow">Account</h2>
  {#if !status}
    <p class="mt-2 text-sm text-muted" aria-live="polite">Checking…</p>
  {:else if !status.signed_in}
    {#if status.problem}<p class="mt-2 text-sm font-bold">{status.problem}</p>{/if}
    {#if step === "idle"}
      <div class="mt-2 flex flex-wrap items-center gap-3">
        <p class="flex-1 text-sm text-muted">Sign in to sync your quizzes, cards and free recall across computers. Everything still works offline.</p>
        <button class="btn btn-primary" onclick={() => { step = "email"; error = ""; void tick().then(() => document.getElementById("account-email")?.focus()); }}><Mail class="size-4" />Sign in to sync</button>
      </div>
    {:else if step === "email"}
      <form class="mt-3 max-w-md space-y-3" onsubmit={sendCode}>
        <label class="block"><span class="block text-sm font-bold">Email</span>
          <input id="account-email" type="email" autocomplete="email" class="input mt-1 w-full" bind:value={email} required /></label>
        <p class="text-sm text-muted">No password. We email you a 6-digit code. A new email makes a new account.</p>
        <div class="flex gap-2"><button type="submit" class="btn btn-primary" disabled={busy}>Email me a code</button><button type="button" class="btn btn-ghost" onclick={() => (step = "idle")}>Cancel</button></div>
      </form>
    {:else}
      <form class="mt-3 max-w-md space-y-3" onsubmit={verify}>
        <p class="text-sm">We sent a 6-digit code to <span class="font-extrabold">{email}</span>. It works for 10 minutes. <button type="button" class="link" onclick={() => (step = "email")}>Wrong email?</button></p>
        {@render otpField("email-code", "Code from the email", code, (text) => (code = text))}
        <div class="flex flex-wrap items-center gap-2">
          <button type="submit" class="btn btn-primary" disabled={busy || code.length !== 6}>Sign in</button>
          {#if wait > 0}<span class="text-sm text-muted tabular-nums">No email? Check spam, or ask again in {wait}s</span>{:else}<button type="button" class="btn btn-ghost btn-sm" disabled={busy} onclick={() => sendCode()}>Send a new code</button>{/if}
        </div>
      </form>
    {/if}
  {:else if status.needs_two_step}
    <form class="mt-3 max-w-md space-y-3" onsubmit={verifyTwoStep}>
      <p class="font-extrabold">One more step</p>
      <p class="text-sm text-muted">Two-step sign-in is on for {status.email}. Type the code from your authenticator app.</p>
      {@render otpField("two-step-code", "Code from your authenticator app", twoStepCode, (text) => (twoStepCode = text))}
      <div class="flex gap-2"><button type="submit" class="btn btn-primary" disabled={busy || twoStepCode.length !== 6}>Continue</button><button type="button" class="btn btn-ghost" onclick={signOut}>Sign out</button></div>
      <p class="text-sm text-muted">Lost your phone? Ask Chen to turn two-step sign-in off for your account.</p>
    </form>
  {:else}
    <div class="mt-3 flex flex-wrap items-center gap-3">
      <span class="grid size-11 place-items-center rounded-full bg-primary text-lg font-black text-primary-content" aria-hidden="true">{status.email?.[0]?.toUpperCase()}</span>
      <span class="min-w-0 flex-1"><span class="block truncate font-extrabold">{status.email}</span><span class="block text-sm text-muted">Synced {when(status.last_synced)}</span></span>
      <button class="btn btn-outline btn-sm" disabled={busy} onclick={syncNow}><RefreshCw class="size-4" />Sync now</button>
      <button class="btn btn-ghost btn-sm" onclick={() => (signingOut = true)}><LogOut class="size-4" />Sign out…</button>
    </div>
    {#if status.problem}<p class="mt-2 text-sm font-bold">{status.problem}</p>{/if}

    <div class="mt-5 border-t-2 border-base-300 pt-4">
      <h3 class="font-extrabold">Two-step sign-in</h3>
      {#if status.two_step}
        <div class="mt-1 flex flex-wrap items-center gap-3"><p class="flex-1 text-sm"><ShieldCheck class="mr-1 inline size-4" />Two-step sign-in is on. Signing in asks for your authenticator code too.</p><button class="btn btn-ghost btn-sm" disabled={busy} onclick={twoStepOff}>Turn off</button></div>
      {:else}
        <div class="mt-1 flex flex-wrap items-center gap-3"><p class="flex-1 text-sm text-muted">Off. Turn it on and signing in also asks for a code from an authenticator app (Aegis, Google Authenticator, 1Password).</p><button class="btn btn-ghost btn-sm" aria-label="Turn on two-step sign-in" disabled={busy} onclick={startTwoStep}><KeyRound class="size-4" />Turn on</button></div>
      {/if}
    </div>

    <div class="mt-5 border-t-2 border-base-300 pt-4">
      <h3 class="font-extrabold" id="devices-h">Signed in on</h3>
      <ul class="mt-1" aria-labelledby="devices-h">
        {#each status.devices as device (device.id)}
          <li class="flex min-h-11 items-center gap-3 border-b border-base-300 last:border-0"><span class="flex-1 font-bold">{device.name}</span><span class="text-sm text-muted">{device.this_one ? "This computer" : `Last synced ${when(device.last_seen)}`}</span></li>
        {/each}
      </ul>
      {#if status.devices.some((device) => !device.this_one)}<button class="btn btn-ghost btn-sm mt-2" disabled={busy} onclick={signOutOthers}>Sign out everywhere else</button>{/if}
    </div>

    <div class="mt-5 border-t-2 border-base-300 pt-4">
      <h3 class="font-extrabold">Your data</h3>
      <div class="mt-1 flex flex-wrap items-center gap-3"><p class="flex-1 text-sm text-muted">Download everything: quizzes, cards, reviews, checklists and logs, in one file.</p><button class="btn btn-ghost btn-sm" onclick={() => (exporting = true)}>Download</button></div>
      <div class="mt-3 flex flex-wrap items-center gap-3"><p class="flex-1 text-sm text-muted">Delete your account and everything synced to it, for good. This computer's copy stays.</p><button class="btn btn-ghost btn-sm text-error" onclick={() => (deleting = true)}><Trash2 class="size-4" />Delete account…</button></div>
    </div>

    {#if status.admin}
      <div class="mt-5 rounded-xl border-2 border-warning p-4">
        <h3 class="font-extrabold">Admin</h3>
        <p class="text-sm text-muted">Only you see this. It changes the app for everyone.</p>
        <label class="mt-3 flex items-start gap-3"><input type="checkbox" class="checkbox mt-0.5" checked={status.images_allowed} disabled={busy} onchange={(event) => setImages(event.currentTarget.checked)} />
          <span><span class="block font-bold">Let people add images</span><span class="block text-sm text-muted">Off: nobody signed in can paste, pick or import images. Images already added keep showing on the computers that have them.</span></span></label>
      </div>
    {/if}
  {/if}
  {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
</section>

{#if setup}
  <dialog open class="modal modal-open" aria-labelledby="two-step-h">
    <form class="modal-box max-w-md" onsubmit={confirmTwoStep}>
      <h3 id="two-step-h" class="text-lg font-semibold">Turn on two-step sign-in</h3>
      <p class="mt-2 text-sm text-muted">Scan this with your authenticator app, or type the key in by hand. Then type the code it shows.</p>
      <img class="mx-auto mt-4 size-48 rounded-lg bg-white p-2" src={setup.qr} alt="QR code for your authenticator app" />
      <p class="mt-3 text-center font-mono text-sm break-all">{setup.secret}</p>
      <div class="mt-4">{@render otpField("setup-code", "Code from your authenticator app", setupCode, (text) => (setupCode = text))}</div>
      {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
      <div class="modal-action"><button type="button" class="btn btn-ghost" onclick={() => (setup = null)}>Cancel</button><button type="submit" class="btn btn-primary" disabled={busy || setupCode.length !== 6}>Turn on</button></div>
    </form>
  </dialog>
{/if}

{#if signingOut && status?.signed_in}
  <dialog open class="modal modal-open" aria-labelledby="sign-out-h">
    <div class="modal-box max-w-md">
      <h3 id="sign-out-h" class="text-lg font-semibold">Sign out of {status.email}?</h3>
      <p class="mt-2 text-sm text-muted">Everything stays on this computer. Changes made while signed out sync once you sign back in.</p>
      <div class="modal-action"><button class="btn btn-ghost" onclick={() => (signingOut = false)}>Cancel</button><button class="btn btn-primary" disabled={busy} onclick={signOut}>Sign out</button></div>
    </div>
  </dialog>
{/if}

{#if deleting && status?.signed_in}
  <dialog open class="modal modal-open" aria-labelledby="delete-account-h">
    <div class="modal-box max-w-md">
      <h3 id="delete-account-h" class="text-lg font-semibold">Delete your account?</h3>
      <p class="mt-2 text-sm text-muted">{status.email} and everything synced to it are deleted for good, on every computer's account copy. What's on this computer stays here.</p>
      <label class="mt-4 block"><span class="block text-sm font-bold">Type DELETE to confirm</span><input class="input mt-1 w-full" bind:value={deleteWord} autocomplete="off" /></label>
      {#if error}<p role="alert" class="mt-3 text-sm text-error">{error}</p>{/if}
      <div class="modal-action"><button class="btn btn-ghost" onclick={() => { deleting = false; deleteWord = ""; }}>Cancel</button><button class="btn btn-error" disabled={busy || deleteWord !== "DELETE"} onclick={deleteAccount}>Delete account</button></div>
    </div>
  </dialog>
{/if}

{#if exporting}<ExportDialog onClose={() => (exporting = false)} onDone={exported} />{/if}

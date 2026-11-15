// The phone version (R211): the study core runs here, off the page, so syncing with Supabase
// never holds the screen up. The database and the sign-in are kept in IndexedDB between visits.
/// <reference lib="webworker" />

interface PhoneCore {
  call(command: string, args: string): unknown;
  sign_in(): string;
  database(): Uint8Array;
}

const KEEP = "shudhana-phone";
let phone: PhoneCore | null = null;
let saving: ReturnType<typeof setTimeout> | null = null;

function store(): Promise<IDBObjectStore> {
  return new Promise((resolve, reject) => {
    const opened = indexedDB.open(KEEP, 1);
    opened.onupgradeneeded = () => opened.result.createObjectStore("kept");
    opened.onsuccess = () => resolve(opened.result.transaction("kept", "readwrite").objectStore("kept"));
    opened.onerror = () => reject(opened.error);
  });
}

async function read(key: string): Promise<unknown> {
  try {
    const kept = await store();
    return await new Promise((resolve) => {
      const request = kept.get(key);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => resolve(undefined);
    });
  } catch {
    return undefined;
  }
}

async function keep() {
  if (!phone) return;
  try {
    const database = phone.database();
    const signIn = phone.sign_in();
    const kept = await store();
    kept.put(database, "database");
    kept.put(signIn, "sign-in");
  } catch {
    // Keeping is a convenience: the account has everything that's synced.
  }
}

/** Saved a second after the last command, so a burst of changes is saved once. */
function keepSoon() {
  if (saving) clearTimeout(saving);
  saving = setTimeout(() => void keep(), 1000);
}

async function start(base: string) {
  const core = await import(/* @vite-ignore */ `${base}phone-pkg/study_phone.js`);
  await core.default();
  const database = await read("database");
  const signIn = await read("sign-in");
  phone = new core.Phone(database instanceof Uint8Array ? database : undefined, typeof signIn === "string" ? signIn : undefined) as PhoneCore;
}

let ready: Promise<void> | null = null;

self.onmessage = async (event: MessageEvent<{ id: number; command: string; args?: unknown; base?: string }>) => {
  const { id, command, args, base } = event.data;
  try {
    if (command === "__start") {
      ready ??= start(base ?? "/");
      await ready;
      self.postMessage({ id, ok: true, value: null });
      return;
    }
    await ready;
    if (command === "__keep") {
      await keep();
      self.postMessage({ id, ok: true, value: null });
      return;
    }
    const value = phone!.call(command, JSON.stringify(args ?? {}));
    self.postMessage({ id, ok: true, value });
    keepSoon();
  } catch (failure) {
    const message = failure && typeof failure === "object" && "message" in failure ? String((failure as { message: unknown }).message) : String(failure);
    self.postMessage({ id, ok: false, error: { message } });
  }
};

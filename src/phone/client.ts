// The phone version's side of the worker (R211): each command is a message, answered by id.
let worker: Worker | null = null;
let next = 1;
const waiting = new Map<number, { resolve: (value: unknown) => void; reject: (error: unknown) => void }>();

function connect(): Worker {
  if (worker) return worker;
  worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  worker.onmessage = (event: MessageEvent<{ id: number; ok: boolean; value?: unknown; error?: unknown }>) => {
    const entry = waiting.get(event.data.id);
    if (!entry) return;
    waiting.delete(event.data.id);
    if (event.data.ok) entry.resolve(event.data.value);
    else entry.reject(event.data.error);
  };
  void send("__start", { base: import.meta.env.BASE_URL });
  return worker;
}

function send(command: string, extra: Record<string, unknown> = {}): Promise<unknown> {
  const target = connect();
  const id = next++;
  return new Promise((resolve, reject) => {
    waiting.set(id, { resolve, reject });
    target.postMessage({ id, command, ...extra });
  });
}

export function phoneInvoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  return send(command, { args }) as Promise<T>;
}

/** Saves the database now, for when the page is being put away. */
export function phoneKeep(): Promise<unknown> {
  return send("__keep");
}

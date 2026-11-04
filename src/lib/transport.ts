// How api.ts reaches the backend. In the desktop app that's Tauri's invoke. In a browser
// (the study-web server, see crates/study-web) it's POST /api/<command> with the same JSON
// arguments, and errors come back as { message } just like a rejected invoke.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export const inDesktopApp = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inDesktopApp()) return tauriInvoke<T>(command, args);
  let response: Response;
  try {
    response = await fetch(`/api/${command}`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(args) });
  } catch {
    throw { message: "couldn't reach the Study App server, check it's running" };
  }
  const kind = response.headers.get("content-type") ?? "";
  if (!response.ok) {
    throw kind.includes("json") ? await response.json() : { message: `the server said ${response.status}` };
  }
  if (kind.includes("json")) return (await response.json()) as T;
  return (await response.arrayBuffer()) as T;
}

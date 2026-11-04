import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { invoke } from "./transport";

const reply = (body: BodyInit, status: number, kind: string) => new Response(body, { status, headers: { "content-type": kind } });

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  vi.unstubAllGlobals();
  vi.mocked(tauriInvoke).mockReset();
});

describe("transport", () => {
  it("uses Tauri inside the desktop app", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    vi.mocked(tauriInvoke).mockResolvedValueOnce([1]);
    const fetching = vi.fn();
    vi.stubGlobal("fetch", fetching);
    expect(await invoke("topics")).toEqual([1]);
    expect(tauriInvoke).toHaveBeenCalledWith("topics", {});
    expect(fetching).not.toHaveBeenCalled();
  });

  it("posts the same arguments to the web server in a browser", async () => {
    const fetching = vi.fn().mockResolvedValue(reply("7", 200, "application/json"));
    vi.stubGlobal("fetch", fetching);
    expect(await invoke("create_topic", { name: "CPU", module: null })).toBe(7);
    expect(fetching).toHaveBeenCalledWith("/api/create_topic", { method: "POST", headers: { "content-type": "application/json" }, body: '{"name":"CPU","module":null}' });
  });

  it("gives bytes back as a buffer, and errors as { message }", async () => {
    vi.stubGlobal("fetch", vi.fn()
      .mockResolvedValueOnce(reply(new Uint8Array([37, 80]), 200, "application/pdf"))
      .mockResolvedValueOnce(reply('{"message":"a topic needs a name"}', 400, "application/json"))
      .mockResolvedValueOnce(reply("", 502, "text/plain"))
      .mockRejectedValueOnce(new TypeError("Failed to fetch")));
    expect(new Uint8Array(await invoke<ArrayBuffer>("image", { id: 1 }))).toEqual(new Uint8Array([37, 80]));
    await expect(invoke("create_topic")).rejects.toEqual({ message: "a topic needs a name" });
    await expect(invoke("topics")).rejects.toEqual({ message: "the server said 502" });
    await expect(invoke("topics")).rejects.toEqual({ message: "couldn't reach the Study App server, check it's running" });
  });
});

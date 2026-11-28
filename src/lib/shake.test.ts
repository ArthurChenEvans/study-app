import { afterEach, describe, expect, it, vi } from "vitest";
import { disableShake, enableShake, listenForShake, shake } from "./shake.svelte";

function jolt(strength: number) {
  const event = new Event("devicemotion") as DeviceMotionEvent;
  Object.defineProperty(event, "acceleration", { value: { x: strength, y: 0, z: 0 } });
  window.dispatchEvent(event);
}

describe("shake to undo", () => {
  afterEach(() => { disableShake(); vi.useRealTimers(); });

  it("fires on a firm shake, at most once a second, and not on gentle movement", () => {
    vi.useFakeTimers();
    const onShake = vi.fn();
    const stop = listenForShake(onShake);
    jolt(5);
    expect(onShake).not.toHaveBeenCalled();
    jolt(25);
    jolt(25);
    expect(onShake).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1100);
    jolt(25);
    expect(onShake).toHaveBeenCalledTimes(2);
    stop();
    vi.advanceTimersByTime(1100);
    jolt(25);
    expect(onShake).toHaveBeenCalledTimes(2);
  });

  it("asks for motion access where the browser wants that, and remembers the answer", async () => {
    const requestPermission = vi.fn().mockResolvedValueOnce("denied").mockResolvedValueOnce("granted");
    vi.stubGlobal("DeviceMotionEvent", Object.assign(function DeviceMotionEvent() {}, { requestPermission }));
    expect(await enableShake()).toBe(false);
    expect(shake.on).toBe(false);
    expect(await enableShake()).toBe(true);
    expect(shake.on).toBe(true);
    expect(localStorage.getItem("shake-to-undo")).toBe("on");
    vi.unstubAllGlobals();
  });
});

// Shake to undo, like Anki on phones. Browsers report the phone's movement through
// DeviceMotionEvent. iPhones (Safari) only do so after asking, which has to happen from a tap,
// so it's switched on from Settings or the review's "…" menu. Off until then.
const KEY = "shake-to-undo";

function stored(): boolean {
  try { return localStorage.getItem(KEY) === "on"; } catch { return false; }
}

type MotionWithPermission = typeof DeviceMotionEvent & { requestPermission?: () => Promise<"granted" | "denied"> };

export const shake = $state({
  on: stored(),
  /** The device can report motion at all (phones and tablets, not desktops). */
  supported: typeof window !== "undefined" && "DeviceMotionEvent" in window && (navigator.maxTouchPoints ?? 0) > 0,
});

/** Asks for motion access where the browser wants that (call from a tap), then turns it on. */
export async function enableShake(): Promise<boolean> {
  const Motion = (typeof DeviceMotionEvent !== "undefined" ? DeviceMotionEvent : undefined) as MotionWithPermission | undefined;
  if (!Motion) return false;
  if (typeof Motion.requestPermission === "function") {
    try {
      if ((await Motion.requestPermission()) !== "granted") return false;
    } catch {
      return false;
    }
  }
  shake.on = true;
  try { localStorage.setItem(KEY, "on"); } catch { /* kept for this visit only */ }
  return true;
}

export function disableShake() {
  shake.on = false;
  try { localStorage.removeItem(KEY); } catch { /* nothing kept */ }
}

/** A firm shake (well past walking or tilting), at most once a second. */
const THRESHOLD = 18;
const QUIET_MS = 1000;

/** Calls `onShake` on each shake while shake to undo is on. Returns a stop function. */
export function listenForShake(onShake: () => void): () => void {
  let last = 0;
  function motion(event: DeviceMotionEvent) {
    const a = event.acceleration;
    const g = event.accelerationIncludingGravity;
    // Without gravity removed, take it off the total roughly.
    const strength = a && a.x !== null ? Math.hypot(a.x ?? 0, a.y ?? 0, a.z ?? 0) : Math.max(0, Math.hypot(g?.x ?? 0, g?.y ?? 0, g?.z ?? 0) - 9.81);
    const now = Date.now();
    if (strength > THRESHOLD && now - last > QUIET_MS) {
      last = now;
      onShake();
    }
  }
  window.addEventListener("devicemotion", motion);
  return () => window.removeEventListener("devicemotion", motion);
}

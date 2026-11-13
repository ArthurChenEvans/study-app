// celebrations.feature (R150 to R153): when to celebrate, worked out from your settings and
// the size of the session. Nothing here touches the screen, so it's all unit tested.

export type Mode = "full" | "pop" | "off";
export type Big = "reviews" | "quiz" | "recall";
export type Frequency = "fewer" | "normal" | "more";

export interface CelebrationPrefs {
  /** The master switch. Off leaves only the plain end-of-session summary. */
  on: boolean;
  moments: Record<Big, Mode>;
  counts: boolean;
  frequency: Frequency;
  markers: boolean;
  streaks: boolean;
  streakLength: 5 | 10;
  nudge: boolean;
  nudgeMinutes: 3 | 5 | 10;
  toasts: boolean;
}

export const DEFAULTS: CelebrationPrefs = {
  on: true,
  moments: { reviews: "full", quiz: "full", recall: "full" },
  counts: true,
  frequency: "normal",
  markers: true,
  streaks: true,
  streakLength: 5,
  nudge: true,
  nudgeMinutes: 5,
  toasts: true,
};

const RANGES: Record<Frequency, [number, number]> = { fewer: [25, 60], normal: [15, 40], more: [10, 25] };
const FRIENDLY = [10, 15, 20, 25, 30, 40, 50, 60];
/** Below this, the finish screen is enough. */
export const SMALL_QUEUE = 15;
const MIN_SPACING = 8;

/** Cards between small pops: a quarter of the queue, kept in range, rounded to a friendly number. */
export function popGap(queue: number, frequency: Frequency): number {
  const [low, high] = RANGES[frequency];
  const raw = Math.min(high, Math.max(low, queue / 4));
  return FRIENDLY.filter((n) => n >= low && n <= high).reduce((best, n) => (Math.abs(n - raw) < Math.abs(best - raw) ? n : best));
}

export interface Pop {
  at: number;
  kind: "count" | "halfway" | "stretch";
}

/** Where the small pops fall in a session of `queue` cards. */
export function popsFor(queue: number, frequency: Frequency, markers: boolean): Pop[] {
  if (queue < SMALL_QUEUE) return [];
  const gap = popGap(queue, frequency);
  let pops: Pop[] = [];
  for (let at = gap; at < queue; at += gap) if (queue - at >= gap / 2) pops.push({ at, kind: "count" });
  if (markers) {
    for (const [kind, share] of [["halfway", 0.5], ["stretch", 0.75]] as const) {
      const at = Math.round(queue * share);
      const counts = pops.filter((pop) => pop.kind === "count");
      const nearest = counts.reduce<Pop | null>((best, pop) => (!best || Math.abs(pop.at - at) < Math.abs(best.at - at) ? pop : best), null);
      if (nearest && Math.abs(nearest.at - at) <= gap / 2) {
        nearest.at = at;
        nearest.kind = kind;
      } else if (pops.every((pop) => Math.abs(pop.at - at) >= gap * 0.75) && queue - at >= gap / 2) {
        pops.push({ at, kind });
      }
    }
    pops.sort((a, b) => a.at - b.at);
  }
  // Keep them apart: a marker beats a plain count when two crowd together.
  const kept: Pop[] = [];
  for (const pop of pops) {
    const last = kept.at(-1);
    if (last && pop.at - last.at < MIN_SPACING) {
      if (last.kind === "count" && pop.kind !== "count") kept[kept.length - 1] = pop;
      continue;
    }
    kept.push(pop);
  }
  pops = kept;
  return pops;
}

const pick = <T>(value: unknown, allowed: readonly T[], fallback: T): T => (allowed.includes(value as T) ? (value as T) : fallback);
const flag = (value: unknown, fallback: boolean) => (typeof value === "boolean" ? value : fallback);

/** Settings from storage, merged over the defaults. Anything unknown or broken is ignored. */
export function parsePrefs(json: string | null): CelebrationPrefs {
  let raw: Record<string, unknown> = {};
  try {
    const parsed = json ? JSON.parse(json) : {};
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) raw = parsed;
  } catch {
    // Broken settings fall back to the defaults.
  }
  const moments = (raw.moments && typeof raw.moments === "object" ? raw.moments : {}) as Record<string, unknown>;
  const modes = ["full", "pop", "off"] as const;
  return {
    on: flag(raw.on, DEFAULTS.on),
    moments: {
      reviews: pick(moments.reviews, modes, DEFAULTS.moments.reviews),
      quiz: pick(moments.quiz, modes, DEFAULTS.moments.quiz),
      recall: pick(moments.recall, modes, DEFAULTS.moments.recall),
    },
    counts: flag(raw.counts, DEFAULTS.counts),
    frequency: pick(raw.frequency, ["fewer", "normal", "more"] as const, DEFAULTS.frequency),
    markers: flag(raw.markers, DEFAULTS.markers),
    streaks: flag(raw.streaks, DEFAULTS.streaks),
    streakLength: pick(raw.streakLength, [5, 10] as const, DEFAULTS.streakLength),
    nudge: flag(raw.nudge, DEFAULTS.nudge),
    nudgeMinutes: pick(raw.nudgeMinutes, [3, 5, 10] as const, DEFAULTS.nudgeMinutes),
    toasts: flag(raw.toasts, DEFAULTS.toasts),
  };
}

export function bigMode(kind: Big, prefs: CelebrationPrefs): Mode {
  return prefs.on ? prefs.moments[kind] : "off";
}

export type SmallEvent =
  | { kind: "count"; done: number }
  | { kind: "halfway"; done: number }
  | { kind: "stretch"; done: number }
  | { kind: "streak"; streak: number }
  | { kind: "nudge" };

/** Tracks one session and says which small moment, if any, an answer earns. The queue is
 *  fixed when the session starts, so cards coming back don't push the markers around. */
export class Pacer {
  private done = 0;
  private streak = 0;
  private lastPopAt: number;
  private readonly pops: Map<number, Pop>;

  constructor(queue: number, private readonly prefs: CelebrationPrefs, now: number) {
    this.lastPopAt = now;
    const pops = prefs.counts || prefs.markers ? popsFor(queue, prefs.frequency, prefs.markers) : [];
    this.pops = new Map(pops.filter((pop) => prefs.counts || pop.kind !== "count").map((pop) => [pop.at, pop]));
  }

  answered(right: boolean, now: number): SmallEvent | null {
    this.done += 1;
    this.streak = right ? this.streak + 1 : 0;
    if (!this.prefs.on) return null;
    let event: SmallEvent | null = null;
    if (this.prefs.streaks && right && this.streak % this.prefs.streakLength === 0) event = { kind: "streak", streak: this.streak };
    else {
      const pop = this.pops.get(this.done);
      if (pop) event = pop.kind === "count" ? { kind: "count", done: this.done } : { kind: pop.kind, done: this.done };
      else if (this.prefs.nudge && right && now - this.lastPopAt >= this.prefs.nudgeMinutes * 60_000) event = { kind: "nudge" };
    }
    if (event) this.lastPopAt = now;
    return event;
  }
}

// celebrations.feature and toasts.feature (R150 to R157): the one place screens go to
// celebrate, pop a small moment, or say what just happened. The overlay, pops and toasts
// components read this state, so screens never draw any of it themselves.

import { api, errorMessage } from "./api";
import { DEFAULTS, Pacer, bigMode, parsePrefs, popGap, type Big, type CelebrationPrefs, type Mode, type SmallEvent } from "./celebrate";
import { playBig, playSmall, setJingleVolume, type SmallKind } from "./jingles";

export interface Stat { label: string; value: number; suffix?: string; tone: "primary" | "accent" | "success" | "info" }
export interface BigDetails { title: string; line: string; stats: Stat[]; score?: number }
export interface BigMoment extends BigDetails { id: number; kind: Big }
export type PopIcon = "check" | "trophy" | "list" | "layers" | "flame" | "flag" | "sparkles";
export interface PopItem { id: number; text: string; sub?: string; icon: PopIcon; step: number; burst: number }
export type ToastIcon = "plus" | "check" | "trash" | "move" | "calendar" | "download" | "refresh" | "copy" | "alert" | "undo" | "info" | "export";
export interface ToastAction { label: string; run: () => void }
export interface ToastOptions { text: string; sub?: string; icon?: ToastIcon; error?: boolean; action?: ToastAction; ms?: number; drain?: boolean }
export interface ToastItem extends ToastOptions { id: number }

export type Motion = "system" | "full" | "calm";
export interface DevicePrefs { motion: Motion; volume: number; bigSoundOnly: boolean; confetti: ConfettiColours }

import type { ConfettiColours } from "./confettiColours";
const DEVICE_KEY = "studyapp-celebrations-device";
const UNDO_MS = 6000;
const TOAST_MS = 4000;
const POP_MS = 2600;
let nextId = 1;
/** A plain copy: structuredClone can't copy reactive state. */
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value));

export const live = $state<{ prefs: CelebrationPrefs; big: BigMoment[]; pops: PopItem[]; toasts: ToastItem[] }>({
  prefs: copy(DEFAULTS),
  big: [],
  pops: [],
  toasts: [],
});

function loadDevice(): DevicePrefs {
  const fallback: DevicePrefs = { motion: "system", volume: 0.8, bigSoundOnly: false, confetti: "theme" };
  try {
    const raw = JSON.parse(localStorage.getItem(DEVICE_KEY) ?? "{}");
    return {
      motion: ["system", "full", "calm"].includes(raw.motion) ? raw.motion : fallback.motion,
      volume: typeof raw.volume === "number" ? Math.min(1, Math.max(0, raw.volume)) : fallback.volume,
      bigSoundOnly: typeof raw.bigSoundOnly === "boolean" ? raw.bigSoundOnly : fallback.bigSoundOnly,
      confetti: raw.confetti === "rainbow" ? "rainbow" : "theme",
    };
  } catch {
    return fallback;
  }
}

/** Sound, volume and motion stay on this computer. Which moments you want syncs. */
export const device = $state<DevicePrefs>(loadDevice());
setJingleVolume(device.volume);

export function saveDevice(next: DevicePrefs): void {
  Object.assign(device, next);
  setJingleVolume(next.volume);
  try {
    localStorage.setItem(DEVICE_KEY, JSON.stringify(next));
  } catch {
    // Storage blocked: the choice lasts for this session.
  }
}

/** Fades only, no confetti: chosen here, or asked for by the system. */
export function calm(): boolean {
  if (device.motion === "calm") return true;
  if (device.motion === "full") return false;
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export async function loadPrefs(): Promise<void> {
  try {
    live.prefs = parsePrefs(await api.celebrations());
  } catch {
    live.prefs = copy(DEFAULTS);
  }
}

export async function savePrefs(next: CelebrationPrefs): Promise<void> {
  live.prefs = copy(next);
  await api.setCelebrations(JSON.stringify(next));
}

export async function resetPrefs(): Promise<void> {
  live.prefs = copy(DEFAULTS);
  await api.resetCelebrations();
}

const ICONS: Record<Big, PopIcon> = { reviews: "check", quiz: "trophy", recall: "list" };

/** A big moment: full screen, a small pop, or nothing, as you've chosen for it. */
export function celebrate(kind: Big, details: BigDetails): Mode {
  const mode = bigMode(kind, live.prefs);
  if (mode === "off") return mode;
  if (mode === "pop") {
    pushPop({ text: details.title, sub: details.line || undefined, icon: ICONS[kind], step: 3, burst: 1 });
    if (!device.bigSoundOnly) playSmall("pop", 3);
    return mode;
  }
  live.big.push({ ...details, id: nextId++, kind });
  // Start the jingle once the moment has painted, so building the sound doesn't hold up its
  // first frame.
  requestAnimationFrame(() => playBig(kind, details.score));
  return mode;
}

const PRAISE = {
  reviews: ["Reviews done!", "Deck cleared!", "All caught up!", "Clean sweep!"],
  quizTop: ["Quiz smashed!", "Nailed it!", "Top marks!"],
  quizMid: ["Quiz complete!", "Solid work!"],
  quizLow: ["Quiz done!", "That's in the bag!"],
  recall: ["Recall done!", "Brain dumped!", "Memory lane!"],
};

/** A title that changes each time, so it doesn't go stale after a week. */
export function praise(kind: Big, score?: number): string {
  const list = kind === "quiz" ? (score === 100 ? PRAISE.quizTop : (score ?? 0) >= 80 ? PRAISE.quizMid : PRAISE.quizLow) : PRAISE[kind];
  return list[Math.floor(Math.random() * list.length)];
}

/** One session's small moments. The queue is fixed when it starts. */
export function startSession(queue: number): { answered: (right: boolean) => void } {
  const pacer = new Pacer(queue, copy(live.prefs), Date.now());
  return {
    answered(right: boolean) {
      const event = pacer.answered(right, Date.now());
      if (event) small(event, queue);
    },
  };
}

export function closeBig(): void {
  live.big.shift();
}

function pushPop(pop: Omit<PopItem, "id">): void {
  const item = { ...pop, id: nextId++ };
  live.pops.push(item);
  if (live.pops.length > 3) live.pops.shift();
  setTimeout(() => dismissPop(item.id), POP_MS);
}

export function dismissPop(id: number): void {
  live.pops = live.pops.filter((pop) => pop.id !== id);
}

const COUNT_LINES = ["Nice pace.", "You're on a roll.", "Brain's warmed up.", "Unstoppable."];

/** A small moment from the session pacer, in words, with a chime that climbs as you go. */
export function small(event: SmallEvent, queue: number): void {
  let pop: Omit<PopItem, "id">;
  let sound: SmallKind;
  let step = 1;
  if (event.kind === "count") {
    step = Math.max(1, Math.round(event.done / popGap(queue, live.prefs.frequency)));
    pop = { text: `${event.done} cards done!`, sub: `${queue - event.done} left. ${COUNT_LINES[Math.min(step, 4) - 1]}`, icon: "layers", step, burst: step >= 3 ? step - 2 : 0 };
    sound = "count";
  } else if (event.kind === "halfway") {
    pop = { text: "Halfway there!", sub: `${event.done} of ${queue}`, icon: "flag", step: 2, burst: 0 };
    sound = "halfway";
  } else if (event.kind === "stretch") {
    pop = { text: "Last stretch!", sub: `${queue - event.done} to go`, icon: "flag", step: 3, burst: 0 };
    sound = "stretch";
  } else if (event.kind === "streak") {
    step = Math.round(event.streak / live.prefs.streakLength);
    pop = { text: `${event.streak} in a row!`, sub: step > 1 ? "Still going." : "Keep it up.", icon: "flame", step, burst: step >= 4 ? 1 : 0 };
    sound = "streak";
  } else {
    pop = { text: "Keep going!", sub: "You're doing the work.", icon: "sparkles", step: 1, burst: 0 };
    sound = "nudge";
  }
  pushPop(pop);
  if (!device.bigSoundOnly) playSmall(sound, step);
}

/** A calm note about what just happened. Errors always show and stay until closed. */
export function toast(options: ToastOptions, force = false): number | null {
  if (!live.prefs.toasts && !options.error && !force) return null;
  const item: ToastItem = { icon: options.error ? "alert" : "check", ...options, id: nextId++ };
  if (live.toasts.length >= 2) live.toasts.shift();
  live.toasts.push(item);
  if (!item.error) setTimeout(() => dismissToast(item.id), item.ms ?? TOAST_MS);
  return item.id;
}

export function dismissToast(id: number): void {
  live.toasts = live.toasts.filter((item) => item.id !== id);
}

interface PendingDelete { timer: ReturnType<typeof setTimeout>; run: () => Promise<void> }
const pending = new Map<number, PendingDelete>();

export interface UndoableDelete {
  text: string;
  sub?: string;
  /** Take it off screen now. */
  hide: () => void;
  /** Put it back, after Undo or a failed delete. */
  restore: () => void;
  /** Delete it for real, once the undo window has closed. */
  commit: () => Promise<unknown> | unknown;
}

/** Single deletes skip the "are you sure" box: it goes at once, with six seconds to undo. */
export function deleteWithUndo({ text, sub, hide, restore, commit }: UndoableDelete): void {
  hide();
  const key = nextId++;
  let toastId: number | null = null;
  const run = async () => {
    const entry = pending.get(key);
    if (!entry) return;
    clearTimeout(entry.timer);
    pending.delete(key);
    if (toastId !== null) dismissToast(toastId);
    try {
      await commit();
    } catch (failure) {
      restore();
      toast({ text: "Couldn't delete it", sub: errorMessage(failure), error: true });
    }
  };
  pending.set(key, { timer: setTimeout(run, UNDO_MS), run });
  toastId = toast({
    text, sub, icon: "trash", ms: UNDO_MS, drain: true,
    action: {
      label: "Undo",
      run: () => {
        const entry = pending.get(key);
        if (!entry) return;
        clearTimeout(entry.timer);
        pending.delete(key);
        if (toastId !== null) dismissToast(toastId);
        restore();
        toast({ text: "Restored", icon: "undo", ms: 2500 }, true);
      },
    },
  }, true);
}

/** Finish any deletes still waiting on their undo window, when the app is closing. */
export async function flushPendingDeletes(): Promise<void> {
  await Promise.all([...pending.values()].map((entry) => entry.run()));
}

const SAMPLES: Record<Big, BigDetails> = {
  reviews: { title: "Reviews done!", line: "Every card due today is cleared.", stats: [{ label: "Cards", value: 64, tone: "primary" }, { label: "Minutes", value: 18, tone: "accent" }, { label: "Remembered", value: 89, suffix: "%", tone: "success" }] },
  quiz: { title: "Quiz smashed!", line: "Networks · OSI model", score: 100, stats: [{ label: "Score", value: 100, suffix: "%", tone: "primary" }, { label: "Right", value: 12, suffix: "/12", tone: "success" }, { label: "Guessed", value: 0, tone: "accent" }] },
  recall: { title: "Recall done!", line: "Up to Monthly.", stats: [{ label: "Score", value: 78, suffix: "%", tone: "primary" }, { label: "Recalled", value: 14, suffix: "/18", tone: "info" }] },
};

/** Settings previews play even when that moment is turned off. */
export function previewBig(kind: Big): void {
  live.big.push({ ...SAMPLES[kind], id: nextId++, kind });
  playBig(kind, SAMPLES[kind].score);
}

export function previewSmall(event: SmallEvent): void {
  small(event, 64);
}

export function previewToast(): void {
  toast({ text: "Topic deleted", sub: "TCP/IP and its 4 questions", icon: "trash", ms: 6000, drain: true, action: { label: "Undo", run: () => undefined } }, true);
}

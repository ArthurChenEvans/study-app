// Short feedback cues, synthesised with Web Audio so there are no files to ship.
// Right rises, wrong falls, finishing a quiz is a longer rising phrase. Every cue
// is paired with a word and an icon on screen, so turning sound off loses nothing.

export type Cue = "right" | "wrong" | "finished";

const KEY = "studyapp-sound";
const ANSWER_GAP_MS = 300;

// Soft sine notes, roughly a warm mallet. Frequencies in Hz.
const PHRASES: Record<Cue, { notes: number[]; step: number; length: number; level: number }> = {
  right: { notes: [523.25, 783.99], step: 0.09, length: 0.16, level: 0.12 },
  wrong: { notes: [392.0, 311.13], step: 0.1, length: 0.18, level: 0.08 },
  finished: { notes: [523.25, 659.25, 783.99], step: 0.12, length: 0.3, level: 0.12 },
};

let context: AudioContext | null = null;
let lastAnswerAt = -Infinity;

export function isSoundOn(): boolean {
  try {
    return localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

export function setSoundOn(on: boolean): void {
  try {
    localStorage.setItem(KEY, on ? "on" : "off");
  } catch {
    // Storage blocked: the choice lasts for this session only.
  }
}

function audio(): AudioContext | null {
  if (context) return context;
  const Ctor = (globalThis as { AudioContext?: typeof AudioContext }).AudioContext;
  if (!Ctor) return null;
  context = new Ctor();
  return context;
}

/** Play a cue. `now` is injectable for tests; it defaults to the page clock. */
export function play(cue: Cue, now: number = performance.now()): void {
  if (!isSoundOn()) return;
  if (cue !== "finished") {
    if (now - lastAnswerAt < ANSWER_GAP_MS) return;
    lastAnswerAt = now;
  }
  const ctx = audio();
  if (!ctx) return;
  if (ctx.state === "suspended") void ctx.resume();
  const phrase = PHRASES[cue];
  phrase.notes.forEach((hz, index) => {
    const start = ctx.currentTime + index * phrase.step;
    const oscillator = ctx.createOscillator();
    const gain = ctx.createGain();
    oscillator.type = "sine";
    oscillator.frequency.setValueAtTime(hz, start);
    gain.gain.setValueAtTime(0, start);
    gain.gain.linearRampToValueAtTime(phrase.level, start + 0.01);
    gain.gain.exponentialRampToValueAtTime(0.0001, start + phrase.length);
    oscillator.connect(gain);
    gain.connect(ctx.destination);
    oscillator.start(start);
    oscillator.stop(start + phrase.length + 0.02);
  });
}

export function resetSoundForTests(): void {
  context = null;
  lastAnswerAt = -Infinity;
}

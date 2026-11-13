// celebrations.feature (R156): the celebration jingles, synthesised with Web Audio like the
// answer cues in sound.ts, so there are no files to ship. Two slightly detuned voices, a soft
// reverb and a gentle compressor. They follow the Sound switch and the volume.

import { isSoundOn } from "./sound";
import type { Big } from "./celebrate";

export type SmallKind = "count" | "halfway" | "stretch" | "streak" | "nudge" | "pop";

let context: AudioContext | null = null;
let bus: GainNode | null = null;
let verb: ConvolverNode | null = null;
let volume = 0.8;

export function setJingleVolume(level: number): void {
  volume = Math.min(1, Math.max(0, level));
  if (bus) bus.gain.value = volume;
}

function audio(): AudioContext | null {
  if (!context) {
    const Ctor = (globalThis as { AudioContext?: typeof AudioContext }).AudioContext;
    if (!Ctor) return null;
    context = new Ctor();
    const comp = context.createDynamicsCompressor();
    comp.threshold.value = -18;
    comp.ratio.value = 4;
    bus = context.createGain();
    bus.gain.value = volume;
    bus.connect(comp).connect(context.destination);
    verb = context.createConvolver();
    const length = Math.floor(context.sampleRate * 1.4);
    const impulse = context.createBuffer(2, length, context.sampleRate);
    for (let channel = 0; channel < 2; channel++) {
      const data = impulse.getChannelData(channel);
      for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / length, 3);
    }
    verb.buffer = impulse;
    const wet = context.createGain();
    wet.gain.value = 0.22;
    verb.connect(wet).connect(comp);
  }
  if (context.state === "suspended") void context.resume();
  return context;
}

const hz = (midi: number) => 440 * Math.pow(2, (midi - 69) / 12);

interface NoteOptions { type?: OscillatorType; vol?: number; bend?: number; detune?: number }

function note(midi: number, at: number, length: number, { type = "triangle", vol = 0.16, bend = 0, detune = 7 }: NoteOptions = {}): void {
  const ctx = audio();
  if (!ctx || !bus || !verb) return;
  const start = ctx.currentTime + at;
  const gain = ctx.createGain();
  gain.gain.setValueAtTime(0, start);
  gain.gain.linearRampToValueAtTime(vol, start + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
  gain.connect(bus);
  gain.connect(verb);
  for (const offset of [-detune, detune]) {
    const oscillator = ctx.createOscillator();
    oscillator.type = type;
    oscillator.detune.value = offset;
    oscillator.frequency.setValueAtTime(hz(midi), start);
    if (bend) oscillator.frequency.exponentialRampToValueAtTime(hz(midi + bend), start + length * 0.8);
    oscillator.connect(gain);
    oscillator.start(start);
    oscillator.stop(start + length + 0.05);
  }
}

const chord = (notes: number[], at: number, length: number, options?: NoteOptions) => notes.forEach((midi) => note(midi, at, length, options));
const sparkle = (at: number, count = 6, base = 84) => {
  for (let i = 0; i < count; i++) note(base + [0, 4, 7, 12, 16, 19, 24][i % 7], at + i * 0.04, 0.22, { type: "sine", vol: 0.05, detune: 0 });
};

/** The pickup notes lead into a chord at 0.66s, which is when the badge lands. */
export function playBig(kind: Big, score?: number): void {
  if (!isSoundOn()) return;
  if (kind === "reviews") {
    note(67, 0, 0.12); note(72, 0.12, 0.12); note(76, 0.24, 0.12); note(79, 0.38, 0.1);
    chord([72, 76, 79, 84], 0.66, 0.9, { vol: 0.09 });
    note(48, 0.66, 1, { type: "sine", vol: 0.2, detune: 0 });
    sparkle(0.95, 7);
  } else if (kind === "quiz") {
    const value = score ?? 100;
    const top = value === 100 ? 88 : value >= 80 ? 84 : 79;
    note(64, 0, 0.12); note(67, 0.12, 0.12); note(71, 0.24, 0.12); note(74, 0.38, 0.1);
    chord(value >= 80 ? [top - 12, top - 8, top - 5, top] : [top - 12, top - 9, top - 5, top], 0.66, 0.9, { vol: 0.08 });
    note(top - 24, 0.66, 1, { type: "sine", vol: 0.2, detune: 0 });
    if (value >= 80) sparkle(0.95, value === 100 ? 10 : 6, value === 100 ? 88 : 84);
  } else {
    note(65, 0, 0.2, { type: "sine" }); note(69, 0.18, 0.2, { type: "sine" }); note(72, 0.36, 0.2, { type: "sine" });
    chord([65, 69, 72, 77], 0.66, 1.4, { type: "sine", vol: 0.08 });
    note(41, 0.66, 1.5, { type: "sine", vol: 0.18, detune: 0 });
    sparkle(1.1, 8, 89);
  }
}

/** `step` climbs with each repeat (20, 40, 60 cards or 5, 10, 15 in a row). */
export function playSmall(kind: SmallKind, step = 1): void {
  if (!isSoundOn()) return;
  if (kind === "streak") {
    const base = 72 + step * 2;
    [0, 4, 7, 12].forEach((interval, index) => note(base + interval, index * 0.065, index === 3 ? 0.3 : 0.08, { type: "square", vol: 0.05, detune: 3 }));
  } else if (kind === "halfway" || kind === "stretch") {
    note(kind === "halfway" ? 67 : 72, 0.1, 0.6, { type: "sine", vol: 0.14, bend: 12, detune: 0 });
    note(kind === "halfway" ? 79 : 84, 0.75, 0.3, { vol: 0.12 });
  } else if (kind === "nudge") {
    note(76, 0, 0.12, { type: "sine", vol: 0.1, detune: 0 }); note(81, 0.1, 0.25, { type: "sine", vol: 0.1, detune: 0 });
  } else {
    const base = 76 + Math.min(step, 4) * 2;
    note(base, 0, 0.1); note(base + 4, 0.08, 0.1); note(base + 7, 0.16, step >= 3 ? 0.4 : 0.25);
    if (step >= 3) sparkle(0.25, step, 88);
  }
}

export function playTick(index: number): void {
  if (!isSoundOn()) return;
  note(84 + index * 3, 0, 0.09, { type: "square", vol: 0.035, detune: 0 });
}

import { beforeEach, describe, expect, it, vi } from "vitest";
import { isSoundOn, play, resetSoundForTests, setSoundOn } from "./sound";

// A stand-in AudioContext that records the notes it was asked to play
function fakeAudio() {
  const notes: number[] = [];
  class FakeContext {
    currentTime = 0;
    destination = {};
    state = "running";
    resume() { return Promise.resolve(); }
    createOscillator() {
      return {
        type: "",
        frequency: { setValueAtTime: (hz: number) => notes.push(hz) },
        connect: () => undefined,
        start: () => undefined,
        stop: () => undefined,
      };
    }
    createGain() {
      return { gain: { setValueAtTime: () => undefined, linearRampToValueAtTime: () => undefined, exponentialRampToValueAtTime: () => undefined }, connect: () => undefined };
    }
  }
  vi.stubGlobal("AudioContext", FakeContext);
  return notes;
}

describe("sound", () => {
  beforeEach(() => {
    localStorage.clear();
    resetSoundForTests();
    vi.unstubAllGlobals();
  });

  it("is on by default and the choice is remembered", () => {
    expect(isSoundOn()).toBe(true);
    setSoundOn(false);
    expect(isSoundOn()).toBe(false);
    expect(localStorage.getItem("studyapp-sound")).toBe("off");
  });

  it("plays a rising pair for right and a falling pair for wrong", () => {
    const notes = fakeAudio();
    play("right", 0);
    expect(notes).toHaveLength(2);
    expect(notes[1]).toBeGreaterThan(notes[0]);
    notes.length = 0;
    play("wrong", 1000);
    expect(notes[1]).toBeLessThan(notes[0]);
  });

  it("plays a longer rising phrase when a quiz is finished", () => {
    const notes = fakeAudio();
    play("finished", 0);
    expect(notes).toHaveLength(3);
    expect(notes[2]).toBeGreaterThan(notes[0]);
  });

  it("stays quiet when sound is off", () => {
    const notes = fakeAudio();
    setSoundOn(false);
    play("right", 0);
    expect(notes).toHaveLength(0);
  });

  it("skips an answer cue that comes within 300ms of the last one", () => {
    const notes = fakeAudio();
    play("right", 1000);
    play("wrong", 1200);
    expect(notes).toHaveLength(2);
    play("wrong", 1400);
    expect(notes).toHaveLength(4);
  });

  it("does nothing where there is no audio", () => {
    expect(() => play("right", 0)).not.toThrow();
  });
});

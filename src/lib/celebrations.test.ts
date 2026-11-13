import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (original) => {
  const actual = await original<typeof import("./api")>();
  return { ...actual, api: { celebrations: vi.fn(), setCelebrations: vi.fn(), resetCelebrations: vi.fn() } };
});
vi.mock("./jingles", () => ({ playBig: vi.fn(), playSmall: vi.fn(), playTick: vi.fn(), setJingleVolume: vi.fn() }));

import { api } from "./api";
import { DEFAULTS } from "./celebrate";
import { celebrate, deleteWithUndo, device, flushPendingDeletes, live, loadPrefs, savePrefs, small, startSession, toast } from "./celebrations.svelte";
import { playBig, playSmall } from "./jingles";

const mocked = vi.mocked(api);

beforeEach(() => {
  vi.useFakeTimers();
  vi.resetAllMocks();
  live.big = [];
  live.pops = [];
  live.toasts = [];
  live.prefs = JSON.parse(JSON.stringify(DEFAULTS));
});
afterEach(() => vi.useRealTimers());

// celebrations.feature (R150): settings load from the store and save back to it.
describe("prefs", () => {
  it("loads saved choices over the defaults and saves the whole set", async () => {
    mocked.celebrations.mockResolvedValue(JSON.stringify({ streaks: false }));
    await loadPrefs();
    expect(live.prefs.streaks).toBe(false);
    expect(live.prefs.counts).toBe(true);
    await savePrefs({ ...live.prefs, counts: false });
    expect(mocked.setCelebrations).toHaveBeenCalledWith(expect.stringContaining('"counts":false'));
    expect(live.prefs.counts).toBe(false);
  });
  it("keeps the defaults when the store can't be read", async () => {
    mocked.celebrations.mockRejectedValue(new Error("offline"));
    await loadPrefs();
    expect(live.prefs).toEqual(DEFAULTS);
  });
});

// celebrations.feature (R154): each big moment follows its own choice.
describe("celebrate", () => {
  const details = { title: "Reviews done!", line: "", stats: [] };
  it("queues a full screen, one at a time, with its jingle", () => {
    celebrate("reviews", details);
    celebrate("quiz", { ...details, title: "Quiz done!", score: 58 });
    expect(live.big.map((moment) => moment.title)).toEqual(["Reviews done!", "Quiz done!"]);
    expect(playBig).toHaveBeenCalledWith("reviews", undefined);
  });
  it("shrinks to a small pop when that's the choice, and does nothing when it's off", () => {
    live.prefs.moments.quiz = "pop";
    live.prefs.moments.recall = "off";
    celebrate("quiz", { ...details, title: "Quiz done!" });
    celebrate("recall", details);
    expect(live.big).toEqual([]);
    expect(live.pops.map((pop) => pop.text)).toEqual(["Quiz done!"]);
  });
});

describe("small", () => {
  it("turns pacer events into pops with words", () => {
    small({ kind: "count", done: 40 }, 64);
    expect(live.pops.map((pop) => pop.text)).toEqual(["40 cards done!"]);
    small({ kind: "halfway", done: 32 }, 64);
    small({ kind: "streak", streak: 10 }, 64);
    small({ kind: "nudge" }, 64);
    expect(live.pops.map((pop) => pop.text)).toEqual(["Halfway there!", "10 in a row!", "Keep going!"]);
    expect(playSmall).toHaveBeenCalledTimes(4);
    vi.advanceTimersByTime(3000);
    expect(live.pops).toEqual([]);
  });
  it("keeps the small pops silent when sound is for big moments only", () => {
    device.bigSoundOnly = true;
    small({ kind: "count", done: 20 }, 64);
    expect(playSmall).not.toHaveBeenCalled();
    device.bigSoundOnly = false;
  });
});

// toasts.feature (R155): calm, two at most, errors stay until closed.
describe("toast", () => {
  it("shows two at most, pushing the oldest out, and clears itself", () => {
    toast({ text: "Topic added" });
    toast({ text: "Saved" });
    toast({ text: "Copied", ms: 1000 });
    expect(live.toasts.map((item) => item.text)).toEqual(["Saved", "Copied"]);
    vi.advanceTimersByTime(1100);
    expect(live.toasts.map((item) => item.text)).toEqual(["Saved"]);
  });
  it("keeps errors until they're closed, even with toasts turned off", () => {
    live.prefs.toasts = false;
    toast({ text: "Saved" });
    toast({ text: "Couldn't save the topic", error: true });
    vi.advanceTimersByTime(60_000);
    expect(live.toasts.map((item) => item.text)).toEqual(["Couldn't save the topic"]);
  });
});

describe("deleteWithUndo", () => {
  it("hides straight away and deletes for real once the undo window closes", async () => {
    const hide = vi.fn(), restore = vi.fn(), commit = vi.fn().mockResolvedValue(undefined);
    deleteWithUndo({ text: "Topic deleted", hide, restore, commit });
    expect(hide).toHaveBeenCalled();
    expect(commit).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(6100);
    expect(commit).toHaveBeenCalledTimes(1);
    expect(restore).not.toHaveBeenCalled();
  });
  it("puts it back on Undo and never deletes", async () => {
    const hide = vi.fn(), restore = vi.fn(), commit = vi.fn();
    deleteWithUndo({ text: "Topic deleted", hide, restore, commit });
    live.toasts[0].action!.run();
    expect(restore).toHaveBeenCalled();
    expect(live.toasts.map((item) => item.text)).toEqual(["Restored"]);
    await vi.advanceTimersByTimeAsync(7000);
    expect(commit).not.toHaveBeenCalled();
  });
  it("finishes waiting deletes when the app closes", async () => {
    const commit = vi.fn().mockResolvedValue(undefined);
    deleteWithUndo({ text: "Topic deleted", hide: vi.fn(), restore: vi.fn(), commit });
    await flushPendingDeletes();
    expect(commit).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(7000);
    expect(commit).toHaveBeenCalledTimes(1);
  });
  it("brings it back and says so when the real delete fails", async () => {
    const restore = vi.fn(), commit = vi.fn().mockRejectedValue(new Error("locked"));
    deleteWithUndo({ text: "Topic deleted", hide: vi.fn(), restore, commit });
    await vi.advanceTimersByTimeAsync(6100);
    expect(restore).toHaveBeenCalled();
    expect(live.toasts.at(-1)).toMatchObject({ error: true });
  });
});

describe("startSession", () => {
  it("passes each answer to a pacer and pops what it earns", () => {
    const session = startSession(64);
    for (let card = 0; card < 15; card++) session.answered(card % 2 === 0);
    expect(live.pops.map((pop) => pop.text)).toEqual(["15 cards done!"]);
  });
});

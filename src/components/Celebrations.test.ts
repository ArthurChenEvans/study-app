import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("canvas-confetti", () => ({ default: Object.assign(vi.fn(), { reset: vi.fn() }) }));
vi.mock("../lib/jingles", () => ({ playBig: vi.fn(), playSmall: vi.fn(), playTick: vi.fn(), setJingleVolume: vi.fn() }));

import confetti from "canvas-confetti";
import { device, live } from "../lib/celebrations.svelte";
import Celebration from "./Celebration.svelte";
import Pops from "./Pops.svelte";
import Toasts from "./Toasts.svelte";

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.mocked(confetti).mockClear();
  live.big = [];
  live.pops = [];
  live.toasts = [];
  device.motion = "full";
});
afterEach(() => vi.useRealTimers());

const quiz = { id: 1, kind: "quiz" as const, title: "Quiz smashed!", line: "Networks · OSI model", score: 100, stats: [{ label: "Score", value: 100, suffix: "%", tone: "primary" as const }, { label: "Right", value: 12, suffix: "/12", tone: "success" as const }] };

// celebrations.feature (R154): the full screen moment.
describe("Celebration", () => {
  it("shows the title, line and stats, then fires confetti as the badge lands", async () => {
    live.big = [quiz];
    render(Celebration);
    const dialog = screen.getByRole("dialog", { name: "Quiz smashed!" });
    expect(dialog).toHaveTextContent("Networks · OSI model");
    expect(within(dialog).getByText("Score")).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(3000);
    expect(dialog).toHaveTextContent("100%");
    expect(dialog).toHaveTextContent("12/12");
    expect(confetti).toHaveBeenCalled();
  });

  it("ignores a stray press in the first half second, then Continue closes it", async () => {
    live.big = [quiz];
    render(Celebration);
    const u = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await u.click(screen.getByRole("button", { name: "Continue" }));
    expect(live.big).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(700);
    await u.click(screen.getByRole("button", { name: "Continue" }));
    expect(live.big).toHaveLength(0);
  });

  it("closes on Escape and shows the next one waiting", async () => {
    live.big = [quiz, { ...quiz, id: 2, kind: "reviews", title: "Reviews done!" }];
    render(Celebration);
    await vi.advanceTimersByTimeAsync(700);
    await userEvent.setup({ advanceTimers: vi.advanceTimersByTime }).keyboard("{Escape}");
    flushSync();
    expect(screen.getByRole("dialog", { name: "Reviews done!" })).toBeInTheDocument();
  });

  it("drops the confetti in calm mode and shows the numbers straight away", async () => {
    device.motion = "calm";
    live.big = [quiz];
    render(Celebration);
    await vi.advanceTimersByTimeAsync(3000);
    expect(confetti).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toHaveTextContent("100%");
  });
});

describe("Pops", () => {
  it("says each small moment in words for everyone, screen readers included", () => {
    live.pops = [{ id: 1, text: "20 cards done!", sub: "44 left. Nice pace.", icon: "layers", step: 1, burst: 0 }];
    render(Pops);
    const region = screen.getByRole("status");
    expect(region).toHaveTextContent("20 cards done!");
    expect(region).toHaveTextContent("44 left. Nice pace.");
  });
});

// toasts.feature (R155): calm notes with an action, a close button, and errors called out.
describe("Toasts", () => {
  it("runs the action and closes with the cross", async () => {
    const run = vi.fn();
    live.toasts = [{ id: 1, text: "Topic deleted", icon: "trash", action: { label: "Undo", run }, drain: true, ms: 6000 }, { id: 2, text: "Saved", icon: "check" }];
    render(Toasts);
    const u = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await u.click(screen.getByRole("button", { name: "Undo" }));
    expect(run).toHaveBeenCalled();
    await u.click(screen.getAllByRole("button", { name: "Dismiss" })[1]);
    expect(live.toasts.map((item) => item.text)).toEqual(["Topic deleted"]);
  });

  it("announces errors straight away and doesn't rely on colour", () => {
    live.toasts = [{ id: 3, text: "Couldn't save the topic", sub: "The file is locked.", error: true, icon: "alert" }];
    render(Toasts);
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Error: Couldn't save the topic");
    expect(alert).toHaveTextContent("The file is locked.");
  });
});

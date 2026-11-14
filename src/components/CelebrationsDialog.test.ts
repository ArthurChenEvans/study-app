import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return { ...actual, api: { celebrations: vi.fn(), setCelebrations: vi.fn(), resetCelebrations: vi.fn() } };
});
vi.mock("../lib/jingles", () => ({ playBig: vi.fn(), playSmall: vi.fn(), playTick: vi.fn(), setJingleVolume: vi.fn() }));

import { api } from "../lib/api";
import { DEFAULTS } from "../lib/celebrate";
import { device, live } from "../lib/celebrations.svelte";
import CelebrationsDialog from "./CelebrationsDialog.svelte";

const mocked = vi.mocked(api);
const saved = () => JSON.parse(mocked.setCelebrations.mock.calls.at(-1)![0]);

beforeEach(() => {
  vi.resetAllMocks();
  mocked.setCelebrations.mockResolvedValue();
  mocked.resetCelebrations.mockResolvedValue();
  live.prefs = JSON.parse(JSON.stringify(DEFAULTS));
  live.big = [];
  live.pops = [];
  localStorage.clear();
});

// celebrations.feature (R152): turn everything off, or tune each moment.
describe("CelebrationsDialog", () => {
  it("turns everything off with one switch, and the rest steps back", async () => {
    const u = userEvent.setup();
    render(CelebrationsDialog, { onClose: vi.fn() });
    const master = screen.getByRole("switch", { name: "Celebrations" });
    expect(master).toBeChecked();
    await u.click(master);
    expect(saved().on).toBe(false);
    expect(screen.getByText(/The end-of-session summary still shows/)).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Answer streaks" })).toBeDisabled();
  });

  it("sets each big moment to full screen, a small pop or off", async () => {
    const u = userEvent.setup();
    render(CelebrationsDialog, { onClose: vi.fn() });
    const quiz = screen.getByRole("radiogroup", { name: "Quiz complete" });
    expect(within(quiz).getByRole("radio", { name: "Full screen" })).toHaveAttribute("aria-checked", "true");
    await u.click(within(quiz).getByRole("radio", { name: "Small pop" }));
    expect(saved().moments).toEqual({ reviews: "full", quiz: "pop", recall: "full" });
  });

  it("tunes the small moments", async () => {
    const u = userEvent.setup();
    render(CelebrationsDialog, { onClose: vi.fn() });
    await u.click(within(screen.getByRole("radiogroup", { name: "How often" })).getByRole("radio", { name: "Fewer" }));
    expect(saved().frequency).toBe("fewer");
    await u.click(within(screen.getByRole("radiogroup", { name: "Streak length" })).getByRole("radio", { name: "10" }));
    expect(saved().streakLength).toBe(10);
    await u.click(screen.getByRole("switch", { name: "Halfway and Last stretch" }));
    expect(saved().markers).toBe(false);
    await u.click(within(screen.getByRole("radiogroup", { name: "Nudge after" })).getByRole("radio", { name: "10 min" }));
    expect(saved().nudgeMinutes).toBe(10);
  });

  it("keeps sound and motion on this computer", async () => {
    const u = userEvent.setup();
    render(CelebrationsDialog, { onClose: vi.fn() });
    await u.click(within(screen.getByRole("radiogroup", { name: "Motion" })).getByRole("radio", { name: "Calm" }));
    await u.click(screen.getByRole("switch", { name: "Sound for big moments only" }));
    expect(device.motion).toBe("calm");
    expect(JSON.parse(localStorage.getItem("studyapp-celebrations-device")!)).toMatchObject({ motion: "calm", bigSoundOnly: true });
    expect(mocked.setCelebrations).not.toHaveBeenCalled();
  });

  // R193: confetti in the theme's colours, or a rainbow, kept on this computer
  it("picks the confetti colours", async () => {
    const u = userEvent.setup();
    render(CelebrationsDialog, { onClose: vi.fn() });
    const colours = screen.getByRole("radiogroup", { name: "Confetti colours" });
    expect(within(colours).getByRole("radio", { name: "My theme's" })).toHaveAttribute("aria-checked", "true");
    await u.click(within(colours).getByRole("radio", { name: "Rainbow" }));
    expect(device.confetti).toBe("rainbow");
    expect(JSON.parse(localStorage.getItem("studyapp-celebrations-device") ?? "{}").confetti).toBe("rainbow");
  });

  it("previews a moment even when it's turned off", async () => {
    const u = userEvent.setup();
    live.prefs.moments.reviews = "off";
    render(CelebrationsDialog, { onClose: vi.fn() });
    await u.click(screen.getByRole("button", { name: "Play Reviews done" }));
    expect(live.big).toHaveLength(1);
    await u.click(screen.getByRole("button", { name: "Play Answer streaks" }));
    expect(live.pops.at(-1)?.text).toBe("5 in a row!");
  });

  it("resets to the defaults", async () => {
    const u = userEvent.setup();
    live.prefs.streaks = false;
    render(CelebrationsDialog, { onClose: vi.fn() });
    await u.click(screen.getByRole("button", { name: "Reset to defaults" }));
    expect(mocked.resetCelebrations).toHaveBeenCalled();
    expect(screen.getByRole("switch", { name: "Answer streaks" })).toBeChecked();
  });
});

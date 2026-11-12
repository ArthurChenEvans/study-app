import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

// A quiz was running when the app last closed: the backend hands it back from current_question.
const running = { position: 3, total: 4, exam: false, topic: 1, text: "Where were we?", options: [{ index: 0, text: "Here" }, { index: 1, text: "There" }, { index: 2, text: "Nowhere" }], multiple: false, short: false, answers_to_pick: null, flagged: false, total_seconds: null, per_question_seconds: null, track: ["RightSure", "Wrong", "Current", "Upcoming"] };

vi.mock("./lib/api", async (original) => {
  const actual = await original<typeof import("./lib/api")>();
  const empty = vi.fn().mockResolvedValue([]);
  return {
    ...actual,
    api: new Proxy({}, { get: (_, name) => (name === "currentQuestion" ? vi.fn().mockResolvedValue(running) : name === "overview" ? vi.fn().mockResolvedValue("FirstRun") : name === "today" ? vi.fn().mockResolvedValue(1) : empty) }),
  };
});

import App from "./App.svelte";

describe("A quiz left running", () => {
  it("is picked up where it stopped when the app opens", async () => {
    render(App);
    expect(await screen.findByText("Where were we?")).toBeInTheDocument();
    expect(screen.getByText("There")).toBeInTheDocument();
  });
});

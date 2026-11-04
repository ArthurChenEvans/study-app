import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ScoreChart from "./ScoreChart.svelte";

const points = [
  { label: "29 Oct", percent: 27, retry: false, detail: "OSI model · 4 / 15" },
  { label: "Today", percent: 80, retry: false, detail: "CPU, GPU · 16 / 20" },
  { label: "Today", percent: 86, retry: true, detail: "CPU, GPU · 6 / 7" },
];

// overview.feature: Score per quiz / Retries on the chart
describe("ScoreChart", () => {
  it("draws a point per quiz, hollow for retries", () => {
    render(ScoreChart, { points, width: 600 });
    const dots = screen.getAllByTestId("score-point");
    expect(dots).toHaveLength(3);
    expect(dots.map((dot) => dot.dataset.retry)).toEqual(["false", "false", "true"]);
  });

  it("labels the latest score and describes the trend", () => {
    render(ScoreChart, { points, width: 600 });
    expect(screen.getByRole("img", { name: "Score per quiz, from 27% to 86%" })).toBeInTheDocument();
    expect(within(screen.getByRole("img")).getByText("86%")).toBeInTheDocument();
  });

  it("reads out points with the arrow keys", async () => {
    render(ScoreChart, { points, width: 600 });
    const chart = screen.getByRole("img");
    await fireEvent.keyDown(chart, { key: "ArrowLeft" });
    expect(screen.getByRole("status")).toHaveTextContent("Today: 80% · CPU, GPU · 16 / 20");
    await fireEvent.keyDown(chart, { key: "ArrowRight" });
    expect(screen.getByRole("status")).toHaveTextContent("Today: 86% (retry) · CPU, GPU · 6 / 7");
  });

  it("has a table view", () => {
    render(ScoreChart, { points, width: 600 });
    const table = screen.getByRole("table");
    expect(within(table).getAllByRole("row")).toHaveLength(4);
    expect(table).toHaveTextContent("29 Oct27%");
  });
});

// search-query.feature, "Pills" and "How pills look" (R220).
// Test IDs: specs/tests/nested-modules-and-search.md, section 11 (component rows).
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/api", async (original) => {
  const actual = await original<typeof import("../../lib/api")>();
  return { ...actual, api: { querySearch: vi.fn() } };
});

import { api } from "../../lib/api";
import QueryBox from "./QueryBox.svelte";
import { broken, group, op, pill, scope, searched } from "./fixtures";

const mocked = vi.mocked(api);
const box = () => screen.getByRole("textbox", { name: "Search" });
const calls = () => mocked.querySearch.mock.calls.map(([input]) => input);

describe("QueryBox", () => {
  beforeEach(() => {
    mocked.querySearch.mockReset();
    mocked.querySearch.mockImplementation(async (input) => searched(input.trim() ? input.trim().split(/\s+/).map((word) => pill(word.replace(/"/g, ""))) : []));
  });
  afterEach(() => vi.useRealTimers());

  it("T-PILL-01: space turns a term into a pill and clears the input", async () => {
    render(QueryBox);
    await userEvent.type(box(), "CPU ");
    expect(await screen.findByRole("listitem", { name: /CPU/ })).toBeInTheDocument();
    expect(box()).toHaveValue("");
    expect(calls()).toContain("CPU");
  });

  it("T-PILL-02: Enter makes a pill and searches straight away", async () => {
    render(QueryBox);
    await userEvent.type(box(), "CPU{Enter}");
    expect(calls().at(-1)).toBe("CPU");
    expect(await screen.findByRole("listitem", { name: /CPU/ })).toBeInTheDocument();
  });

  it("T-PILL-03: a space inside an open quote doesn't make a pill", async () => {
    render(QueryBox);
    await userEvent.type(box(), '"CPU Lo ');
    expect(box()).toHaveValue('"CPU Lo ');
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("T-PILL-04: Enter closes an unclosed quote and makes one phrase pill", async () => {
    mocked.querySearch.mockResolvedValue(searched([{ Pill: { text: "CPU Logic", kind: "Phrase", negated: false } }]));
    render(QueryBox);
    await userEvent.type(box(), '"CPU Logic{Enter}');
    expect(calls().at(-1)).toBe('"CPU Logic"');
    expect(await screen.findAllByRole("listitem")).toHaveLength(1);
  });

  it("T-PILL-05: Backspace on an empty input edits the last pill", async () => {
    render(QueryBox, { value: "CPU ALU" });
    await screen.findByRole("listitem", { name: /ALU/ });
    box().focus();
    await userEvent.keyboard("{Backspace}");
    expect(box()).toHaveValue("ALU");
    expect(calls().at(-1)).toBe("CPU");
  });

  it("T-PILL-06: a pill is removed by its button or by Delete", async () => {
    render(QueryBox, { value: "CPU ALU RAM" });
    await userEvent.click(await screen.findByRole("button", { name: "Remove ALU" }));
    expect(calls().at(-1)).toBe("CPU RAM");
    const ram = await screen.findByRole("listitem", { name: /RAM/ });
    ram.focus();
    await userEvent.keyboard("{Delete}");
    expect(calls().at(-1)).toBe("CPU");
  });

  it("T-PILL-07: pasting a whole query searches it in one go", async () => {
    render(QueryBox);
    box().focus();
    await userEvent.paste("module:N2T CPU $ ALU !draft");
    await vi.waitFor(() => expect(calls()).toEqual(["module:N2T CPU $ ALU !draft"]));
  });

  it("T-PILL-08: typing searches once per pause, not per key", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(QueryBox);
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await user.type(box(), "CPUs");
    expect(calls()).toEqual([]);
    await vi.advanceTimersByTimeAsync(400);
    expect(calls()).toEqual(["CPUs"]);
  });

  it("T-PILL-09/11/13: OR shows as a word inside one outlined group", async () => {
    mocked.querySearch.mockResolvedValue(searched([group("Solid", [pill("CPU"), op("OR"), pill("ALU")])], { spoken: "CPU or ALU" }));
    render(QueryBox, { value: "CPU $ ALU" });
    const joined = await screen.findByRole("group", { name: "CPU or ALU" });
    expect(joined).toHaveTextContent(/CPU\s*OR\s*ALU/);
    expect(joined).toHaveAttribute("data-outline", "solid");
  });

  it("T-PILL-13: XOR groups are dashed", async () => {
    mocked.querySearch.mockResolvedValue(searched([group("Dashed", [pill("CPU"), op("XOR"), pill("ALU")])], { spoken: "CPU xor ALU" }));
    render(QueryBox, { value: "CPU ^ ALU" });
    expect(await screen.findByRole("group", { name: "CPU xor ALU" })).toHaveAttribute("data-outline", "dashed");
  });

  it("T-PILL-14: NOT pills are greyed and struck through", async () => {
    mocked.querySearch.mockResolvedValue(searched([op("NOT"), pill("CPU", true)]));
    render(QueryBox, { value: "!CPU" });
    const item = await screen.findByRole("listitem", { name: /CPU/ });
    expect(item).toHaveAttribute("data-negated", "true");
    expect(screen.getByText("NOT")).toBeInTheDocument();
  });

  it("T-PILL-15: faint brackets are marked as worked out, typed ones aren't", async () => {
    mocked.querySearch.mockResolvedValue(searched([pill("N2T"), op("AND"), group("Solid", [pill("CPU"), op("OR"), pill("ALU")], "Faint")]));
    render(QueryBox, { value: "N2T CPU $ ALU" });
    const brackets = await screen.findAllByText(/^[()]$/);
    expect(brackets).toHaveLength(2);
    brackets.forEach((bracket) => expect(bracket).toHaveAttribute("data-faint", "true"));
  });

  it("T-PILL-16: nested groups nest in the page", async () => {
    mocked.querySearch.mockResolvedValue(searched([pill("CPU"), op("AND"), group("Solid", [pill("ALU"), op("OR"), group("Dashed", [pill("RAM"), op("XOR"), pill("ROM")], "Typed")], "Typed")]));
    render(QueryBox, { value: "CPU & (ALU $ (RAM ^ ROM))" });
    const groups = await screen.findAllByRole("group");
    const or = groups.find((element) => element.dataset.outline === "solid")!;
    expect(within(or).getAllByRole("group").some((element) => element.dataset.outline === "dashed")).toBe(true);
  });

  it("T-PILL-18: scope pills show their kind and full path", async () => {
    mocked.querySearch.mockResolvedValue(searched([scope("N2T")], { scope_paths: [["N2T", ["Computer Architecture › N2T"]]] }));
    render(QueryBox, { value: "module:N2T" });
    const item = await screen.findByRole("listitem", { name: /module N2T/i });
    expect(item).toHaveAttribute("title", "Computer Architecture › N2T");
    expect(item.querySelector("svg")).not.toBeNull();
  });

  it("T-PILL-20: a broken query says what's wrong and marks the box", async () => {
    mocked.querySearch.mockResolvedValue(broken("$ needs something on its right", 4, 5));
    render(QueryBox);
    await userEvent.type(box(), "CPU ${Enter}");
    expect(await screen.findByRole("alert")).toHaveTextContent("$ needs something on its right");
    expect(box()).toHaveAttribute("aria-invalid", "true");
  });

  it("T-PILL-22: Edit as text gives back what was typed", async () => {
    render(QueryBox, { value: "CPU & (ALU $ RAM)" });
    await userEvent.click(await screen.findByRole("button", { name: "Edit as text" }));
    expect(box()).toHaveValue("CPU & (ALU $ RAM)");
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("T-PILL-25: every group carries its word, not just a colour", async () => {
    mocked.querySearch.mockResolvedValue(searched([group("Solid", [pill("CPU"), op("OR"), pill("ALU")]), op("AND"), group("Dashed", [pill("RAM"), op("XOR"), pill("ROM")])]));
    render(QueryBox, { value: "(CPU $ ALU) & (RAM ^ ROM)" });
    await screen.findAllByRole("group");
    for (const word of ["OR", "AND", "XOR"]) expect(screen.getByText(word)).toBeVisible();
  });

  it("T-PILL-27: ? opens a cheat sheet of scopes and operators", async () => {
    render(QueryBox);
    await userEvent.click(screen.getByRole("button", { name: "Search help" }));
    const sheet = screen.getByRole("dialog", { name: "Search help" });
    for (const token of ["module:", "topic:", "&", "$", "^", "!", "AND", "OR", "XOR", "NOT"]) expect(sheet).toHaveTextContent(token);
  });

  it("removing every pill clears the search", async () => {
    render(QueryBox, { value: "CPU" });
    await fireEvent.click(await screen.findByRole("button", { name: "Remove CPU" }));
    expect(calls().at(-1)).toBe("");
  });
});

import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { closeContextMenu } from "../lib/contextMenu.svelte";
import ContextRows from "./__fixtures__/ContextRows.svelte";

// R199: right-click, long-press, Shift+F10 or the Menu key open a row's menu
describe("Row context menus", () => {
  afterEach(() => closeContextMenu(false));

  it("opens the row's menu on right-click and runs an action", async () => {
    const onPick = vi.fn();
    render(ContextRows, { onPick });
    await fireEvent.contextMenu(screen.getByText("DNS"), { clientX: 40, clientY: 50 });
    const menu = screen.getByRole("menu", { name: "DNS" });
    expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Start quiz", "Delete quiz…"]);
    expect(within(menu).getByRole("menuitem", { name: "Start quiz" })).toHaveFocus();
    await userEvent.click(within(menu).getByRole("menuitem", { name: "Start quiz" }));
    expect(onPick).toHaveBeenCalledWith("start DNS");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens from the keyboard with Shift+F10 or the Menu key, and closes with Escape", async () => {
    render(ContextRows, { onPick: vi.fn() });
    const row = screen.getByText("TLS");
    row.focus();
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    expect(screen.getByRole("menu", { name: "TLS" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(row).toHaveFocus();
    await fireEvent.keyDown(row, { key: "ContextMenu" });
    expect(screen.getByRole("menu", { name: "TLS" })).toBeInTheDocument();
  });

  it("acts on every ticked row when the row is one of them", async () => {
    const onPick = vi.fn();
    render(ContextRows, { onPick });
    await userEvent.click(screen.getByRole("checkbox", { name: "Tick CPU" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Tick TLS" }));
    await fireEvent.contextMenu(screen.getByText("TLS"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Delete 2 quizzes…" }));
    expect(onPick).toHaveBeenCalledWith("delete CPU,TLS");
  });

  it("opens on a long press", async () => {
    vi.useFakeTimers();
    render(ContextRows, { onPick: vi.fn() });
    const row = screen.getByText("CPU");
    await fireEvent.pointerDown(row, { pointerType: "touch", clientX: 10, clientY: 10 });
    vi.advanceTimersByTime(550);
    await fireEvent.pointerUp(row, { pointerType: "touch" });
    vi.useRealTimers();
    expect(await screen.findByRole("menu", { name: "CPU" })).toBeInTheDocument();
  });
});

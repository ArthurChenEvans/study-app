import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import DateField from "./DateField.svelte";

describe("DateField (the app's own date picker)", () => {
  it("shows the date and picks another by keyboard", async () => {
    const u = userEvent.setup();
    const { component } = render(DateField, { value: "2026-12-15", "aria-label": "Date" });
    const field = screen.getByRole("button", { name: /^Date, Tuesday 15 December 2026/ });
    expect(field).toHaveTextContent("15 Dec 2026");
    await u.click(field);
    expect(screen.getByRole("dialog", { name: "December 2026" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Tuesday 15 December 2026" })).toHaveFocus();
    await u.keyboard("{ArrowRight}{ArrowDown}");
    expect(screen.getByRole("button", { name: "Wednesday 23 December 2026" })).toHaveFocus();
    await u.keyboard("{PageDown}");
    expect(screen.getByRole("dialog", { name: "January 2027" })).toBeInTheDocument();
    await u.keyboard("{Enter}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(field).toHaveTextContent("23 Jan 2027");
    expect(field).toHaveFocus();
    void component;
  });

  it("won't pick a day before the earliest, and Esc closes", async () => {
    const u = userEvent.setup();
    render(DateField, { value: "2026-12-15", min: "2026-12-10", "aria-label": "Pick a day" });
    await u.click(screen.getByRole("button", { name: /^Pick a day/ }));
    const early = screen.getByRole("button", { name: "Wednesday 9 December 2026" });
    expect(early).toHaveAttribute("aria-disabled", "true");
    await u.click(early);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await u.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^Pick a day/ })).toHaveTextContent("15 Dec 2026");
  });
});

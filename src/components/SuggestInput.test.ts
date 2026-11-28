import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import SuggestInput from "./SuggestInput.svelte";

describe("SuggestInput (suggestions under a text field)", () => {
  it("filters as you type, and picks with the arrows and Enter", async () => {
    const u = userEvent.setup();
    const onpick = vi.fn();
    render(SuggestInput, { suggestions: ["Exam", "Mock exam", "Deadline"], "aria-label": "Kind", onpick });
    const field = screen.getByRole("combobox", { name: "Kind" });
    await u.type(field, "ex");
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual(["Exam", "Mock exam"]);
    await u.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    expect(field).toHaveValue("Mock exam");
    expect(onpick).toHaveBeenCalledWith("Mock exam");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("keeps whatever you type, and Esc closes the list", async () => {
    const u = userEvent.setup();
    render(SuggestInput, { suggestions: ["Exam"], "aria-label": "Kind" });
    const field = screen.getByRole("combobox", { name: "Kind" });
    await u.click(field);
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    await u.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    await u.type(field, "Viva");
    expect(field).toHaveValue("Viva");
  });
});

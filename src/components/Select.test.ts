import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import Select from "./Select.svelte";

const options = [
  { value: "a", label: "Apple" },
  { value: "b", label: "Banana", disabled: true },
  { value: "c", label: "Cherry", group: "Stone" },
  { value: "d", label: "Damson", group: "Stone" },
];

describe("Select (the app's own dropdown)", () => {
  it("is a labelled combobox showing the picked option", () => {
    render(Select, { value: "c", options, "aria-label": "Fruit" });
    const box = screen.getByRole("combobox", { name: "Fruit" });
    expect(box).toHaveTextContent("Cherry");
    expect(box).toHaveAttribute("aria-expanded", "false");
  });

  it("opens on click, marks the picked option, and picks by click", async () => {
    const u = userEvent.setup();
    const onchange = vi.fn();
    render(Select, { value: "a", options, onchange, "aria-label": "Fruit" });
    await u.click(screen.getByRole("combobox"));
    expect(screen.getByRole("option", { name: "Apple" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("group", { name: "Stone" })).toBeInTheDocument();
    await u.click(screen.getByRole("option", { name: "Damson" }));
    expect(onchange).toHaveBeenCalledWith("d");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox")).toHaveFocus();
  });

  it("works by keyboard, skipping disabled options", async () => {
    const u = userEvent.setup();
    const onchange = vi.fn();
    render(Select, { value: "a", options, onchange, "aria-label": "Fruit" });
    const box = screen.getByRole("combobox");
    box.focus();
    await u.keyboard("{Enter}");
    expect(box).toHaveAttribute("aria-activedescendant", screen.getByRole("option", { name: "Apple" }).id);
    await u.keyboard("{ArrowDown}");
    expect(box).toHaveAttribute("aria-activedescendant", screen.getByRole("option", { name: "Cherry" }).id);
    await u.keyboard("{End}{Enter}");
    expect(onchange).toHaveBeenLastCalledWith("d");
    await u.keyboard("{ArrowDown}{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(onchange).toHaveBeenCalledTimes(1);
  });

  it("jumps by typing the start of an option", async () => {
    const u = userEvent.setup();
    const onchange = vi.fn();
    render(Select, { value: "a", options, onchange, "aria-label": "Fruit" });
    screen.getByRole("combobox").focus();
    await u.keyboard("da");
    expect(onchange).toHaveBeenLastCalledWith("d");
  });

  it("shows a placeholder when nothing is picked", () => {
    render(Select, { value: null, options, placeholder: "Choose a fruit", "aria-label": "Fruit" });
    expect(screen.getByRole("combobox")).toHaveTextContent("Choose a fruit");
  });
});

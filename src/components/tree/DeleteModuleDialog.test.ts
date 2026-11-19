// nested-modules.feature, "Deleting" (R217). Test IDs: T-DEL-01 (screen half), T-DEL-04, T-DEL-07.
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DeletePreview } from "../../lib/api";
import DeleteModuleDialog from "./DeleteModuleDialog.svelte";

const preview: DeletePreview = { modules: [], topics: ["CPU", "Motherboard", "ALU"], questions: 51, cards: 38, checklists: 1, clashes: [] };

function open(overrides: Partial<DeletePreview> = {}) {
  const onConfirm = vi.fn();
  render(DeleteModuleDialog, { name: "N2T", preview: { ...preview, ...overrides }, onConfirm, onCancel: vi.fn() });
  return onConfirm;
}

describe("DeleteModuleDialog", () => {
  it("T-DEL-01: names the topics and counts what goes", () => {
    open();
    const dialog = screen.getByRole("dialog", { name: "Delete N2T?" });
    for (const text of ["CPU", "Motherboard", "ALU", "51 questions", "38 cards", "1 recall checklist"]) expect(dialog).toHaveTextContent(text);
  });

  it("T-DEL-07: Delete stays off until the module's name is typed", async () => {
    const onConfirm = open();
    const remove = screen.getByRole("button", { name: "Delete N2T" });
    expect(remove).toBeDisabled();
    await userEvent.type(screen.getByRole("textbox", { name: "Type N2T to confirm" }), "N2");
    expect(remove).toBeDisabled();
    await userEvent.type(screen.getByRole("textbox", { name: "Type N2T to confirm" }), "T");
    expect(remove).toBeEnabled();
    await userEvent.click(remove);
    expect(onConfirm).toHaveBeenCalledWith("MoveContentsUp");
  });

  it("offers Move contents up first, and Delete everything", async () => {
    const onConfirm = open();
    expect(screen.getByRole("radio", { name: /Move contents up/ })).toBeChecked();
    await userEvent.click(screen.getByRole("radio", { name: /Delete everything/ }));
    await userEvent.type(screen.getByRole("textbox", { name: "Type N2T to confirm" }), "N2T");
    await userEvent.click(screen.getByRole("button", { name: "Delete N2T" }));
    expect(onConfirm).toHaveBeenCalledWith("Everything");
  });

  it("T-DEL-04: clashes are listed and block moving contents up", async () => {
    open({ clashes: ["Motherboard"] });
    expect(screen.getByRole("alert")).toHaveTextContent("Motherboard");
    await userEvent.type(screen.getByRole("textbox", { name: "Type N2T to confirm" }), "N2T");
    expect(screen.getByRole("button", { name: "Delete N2T" })).toBeDisabled();
    await userEvent.click(screen.getByRole("radio", { name: /Delete everything/ }));
    expect(screen.getByRole("button", { name: "Delete N2T" })).toBeEnabled();
  });
});

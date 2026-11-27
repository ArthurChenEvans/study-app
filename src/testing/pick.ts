import { screen } from "@testing-library/svelte";
import type { UserEvent } from "@testing-library/user-event";

/** Picks an option from the app's own dropdown (components/Select.svelte), by its visible label. */
export async function pick(user: UserEvent, combobox: HTMLElement, label: string | RegExp) {
  if (combobox.getAttribute("aria-expanded") !== "true") await user.click(combobox);
  await user.click(screen.getByRole("option", { name: label }));
}

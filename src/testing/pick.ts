import { screen } from "@testing-library/svelte";
import type { UserEvent } from "@testing-library/user-event";

/** Picks an option from the app's own dropdown (components/Select.svelte), by its visible label. */
export async function pick(user: UserEvent, combobox: HTMLElement, label: string | RegExp) {
  if (combobox.getAttribute("aria-expanded") !== "true") await user.click(combobox);
  await user.click(screen.getByRole("option", { name: label }));
}

/** Picks a day ("2026-11-05") in the app's own date picker (components/DateField.svelte). */
export async function pickDate(user: UserEvent, field: HTMLElement, iso: string) {
  if (field.getAttribute("aria-expanded") !== "true") await user.click(field);
  const dialog = screen.getByRole("dialog", { name: /\d{4}$/ });
  const [year, month] = iso.split("-").map(Number);
  const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  for (let step = 0; step < 240; step++) {
    const shown = dialog.querySelector(`[id$="-month"]`)?.textContent ?? "";
    const [name, shownYear] = shown.split(" ");
    const diff = (year - Number(shownYear)) * 12 + (month - 1 - MONTHS.indexOf(name));
    if (diff === 0) break;
    await user.click(screen.getByRole("button", { name: diff > 0 ? "Next month" : "Previous month" }));
  }
  const day = dialog.querySelector<HTMLElement>(`[data-day="${iso}"]`);
  if (!day) throw new Error(`no day ${iso} in the date picker`);
  await user.click(day);
}

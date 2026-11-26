// R228: the colour theme, dark or light and the colour-blind palette sync to every device.
// Each device keeps its own copy too (theme.ts, colours.ts) so the app opens in the right
// colours before the database answers.
import { api } from "./api";
import { COLOURS, initialColours, type ColoursId } from "./colours";
import { DARK, LIGHT, applyPalette, applyTheme } from "./theme";
import { inDesktopApp } from "./transport";

const COLOURS_KEY = "studyapp-colours";

/** Sends this device's look to the synced settings. */
export async function saveAppearance(): Promise<void> {
  const root = document.documentElement;
  try {
    await api.setAppearance({ colours: initialColours(), dark: root.dataset.theme !== LIGHT, colour_blind: root.dataset.palette === "cb" });
  } catch {
    // Offline or storage blocked: it's still saved on this device and goes up next change.
  }
}

/** Applies the synced look. With nothing synced yet, the desktop app sends its own up
 *  (a fresh browser doesn't, so it can't replace the desktop's theme with the default). */
export async function pullAppearance(): Promise<void> {
  let synced;
  try {
    synced = await api.appearance();
  } catch {
    return;
  }
  if (!synced) {
    if (inDesktopApp()) await saveAppearance();
    return;
  }
  if (!COLOURS.some((theme) => theme.id === synced.colours)) return;
  try {
    localStorage.setItem(COLOURS_KEY, synced.colours as ColoursId);
  } catch {
    // Storage blocked: applyTheme below still paints it for this session.
  }
  applyPalette(synced.colour_blind ? "colour-blind" : "standard");
  applyTheme(synced.dark ? DARK : LIGHT);
}

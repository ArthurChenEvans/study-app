export type Screen = "overview" | "calendar" | "topics" | "cards" | "recall" | "projects" | "bibliography" | "history" | "search" | "setup";

const screens: Screen[] = ["overview", "calendar", "topics", "cards", "recall", "projects", "bibliography", "history", "search", "setup"];

/** Screen from a location hash like "#topics", falling back to the overview. */
export function screenFromHash(hash: string): Screen {
  const name = hash.replace(/^#/, "") as Screen;
  return screens.includes(name) ? name : "overview";
}

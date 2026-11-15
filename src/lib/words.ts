// R214: the app's own terms said the way you'd say them out loud.

const OFTEN: Record<string, string> = { day: "day", daily: "day", week: "week", weekly: "week", fortnight: "2 weeks", fortnightly: "2 weeks", month: "month", monthly: "month", year: "year", yearly: "year" };

/** A ladder step ("Week", "Monthly 1 of 6") as how often it repeats ("Every week",
 *  "Every month, 1 of 6"). A step with a name of your own is shown as it is. */
export function repeatsLabel(step: string): string {
  const match = step.trim().match(/^(\p{L}+)(?:\s+(\d+ of \d+))?$/u);
  const often = match && OFTEN[match[1].toLowerCase()];
  if (!often) return step;
  return `Every ${often}${match[2] ? `, ${match[2]}` : ""}`;
}

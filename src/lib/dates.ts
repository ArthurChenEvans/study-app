// Days are numbered from 1970-01-01 in local time by the backend (see src-tauri today()).
// Read back as a UTC calendar date so no timezone shifts the day.
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

export function dayLabel(day: number, today: number): string {
  if (day === today) return "Today";
  if (day === today - 1) return "Yesterday";
  const date = new Date(day * 86_400_000);
  const year = date.getUTCFullYear();
  const label = `${date.getUTCDate()} ${MONTHS[date.getUTCMonth()]}`;
  return year === new Date(today * 86_400_000).getUTCFullYear() ? label : `${label} ${year}`;
}

/** "YYYY-MM-DD HH:MM:SS" in UTC (SQLite's datetime('now')) to a local day number. */
export function dayFromUtc(timestamp: string): number {
  const date = new Date(`${timestamp.replace(" ", "T")}Z`);
  return Math.floor((date.getTime() - date.getTimezoneOffset() * 60_000) / 86_400_000);
}

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Upcoming dates: "Today", "Tomorrow", "Thu 5 Nov" within the year, "3 Feb 2027" after. */
export function dueLabel(day: number, today: number): string {
  if (day <= today) return "Today";
  if (day === today + 1) return "Tomorrow";
  const date = new Date(day * 86_400_000);
  const label = dayLabel(day, today);
  return date.getUTCFullYear() === new Date(today * 86_400_000).getUTCFullYear() ? `${WEEKDAYS[date.getUTCDay()]} ${label}` : label;
}

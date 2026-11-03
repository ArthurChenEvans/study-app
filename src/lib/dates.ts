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

/** For use mid-sentence: "today", "tomorrow", "on Thu 5 Nov". */
export function onDay(day: number, today: number): string {
  const label = dueLabel(day, today);
  return label === "Today" || label === "Tomorrow" ? label.toLowerCase() : `on ${label}`;
}

/** Upcoming dates: "Today", "Tomorrow", "Thu 5 Nov" within the year, "3 Feb 2027" after. */
export function dueLabel(day: number, today: number): string {
  if (day <= today) return "Today";
  if (day === today + 1) return "Tomorrow";
  const date = new Date(day * 86_400_000);
  const label = dayLabel(day, today);
  return date.getUTCFullYear() === new Date(today * 86_400_000).getUTCFullYear() ? `${WEEKDAYS[date.getUTCDay()]} ${label}` : label;
}

const MONTH_NAMES = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/** The day number of a calendar date (month from 0). */
export function dayOf(year: number, month: number, date: number): number {
  return Math.round(Date.UTC(year, month, date) / 86_400_000);
}

/** A day number as its calendar date. Weekday is 0 for Monday. */
export function dateOf(day: number): { year: number; month: number; date: number; weekday: number } {
  const d = new Date(day * 86_400_000);
  return { year: d.getUTCFullYear(), month: d.getUTCMonth(), date: d.getUTCDate(), weekday: (d.getUTCDay() + 6) % 7 };
}

/** "November 2026" */
export function monthLabel(year: number, month: number): string {
  return `${MONTH_NAMES[month]} ${year}`;
}

/** The Monday-to-Sunday weeks covering a month, as day numbers. */
export function monthGrid(year: number, month: number): number[] {
  const first = dayOf(year, month, 1);
  const start = first - dateOf(first).weekday;
  const last = dayOf(year, month + 1, 0);
  const end = last + (6 - dateOf(last).weekday);
  return Array.from({ length: end - start + 1 }, (_, index) => start + index);
}

/** "2026-11-05" for a date input, and back. */
export function isoOf(day: number): string {
  return new Date(day * 86_400_000).toISOString().slice(0, 10);
}
export function dayFromIso(iso: string): number {
  const [year, month, date] = iso.split("-").map(Number);
  return dayOf(year, month - 1, date);
}

/** "Tue 3 Nov", with the year when it isn't this year's. */
export function fullLabel(day: number, today: number): string {
  const date = new Date(day * 86_400_000);
  const label = `${WEEKDAYS[date.getUTCDay()]} ${date.getUTCDate()} ${MONTHS[date.getUTCMonth()]}`;
  return date.getUTCFullYear() === new Date(today * 86_400_000).getUTCFullYear() ? label : `${label} ${date.getUTCFullYear()}`;
}

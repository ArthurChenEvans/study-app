// Words for backups (R104, R108).
import type { BackupInfo, BackupReason, BackupStatus } from "./api";
import { dayLabel, fullLabel } from "./dates";

const clock = (minute: number) => `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;

/** "Today, 09:12", "Yesterday, 08:47", "Mon 2 Nov, 19:30". */
export function madeLabel(backup: Pick<BackupInfo, "day" | "minute">, today: number): string {
  const day = backup.day >= today - 1 ? dayLabel(backup.day, today) : fullLabel(backup.day, today);
  return `${day}, ${clock(backup.minute)}`;
}

/** Mid-sentence: "today, 09:12", "yesterday, 08:47", "Mon 2 Nov, 19:30". */
export function madeInline(backup: Pick<BackupInfo, "day" | "minute">, today: number): string {
  const label = madeLabel(backup, today);
  return backup.day >= today - 1 ? label.charAt(0).toLowerCase() + label.slice(1) : label;
}

/** The day alone, for buttons: "today", "Mon 2 Nov". */
export function madeDay(backup: Pick<BackupInfo, "day">, today: number): string {
  return backup.day === today ? "today" : backup.day === today - 1 ? "yesterday" : fullLabel(backup.day, today);
}

export const reasonLabel: Record<BackupReason, string> = {
  Daily: "Daily",
  ByHand: "By hand",
  BeforeRestore: "Before restore",
  BeforeUpdate: "Before update",
  BeforeImport: "Before import",
  BeforeSync: "Before sync",
};

/** "6.1 MB", "820 KB". */
export function sizeLabel(bytes: number): string {
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1000))} KB`;
}

/** The one-line status, for the sidebar and the top of Your data (R104, R108). */
export function statusLine(status: BackupStatus, today: number): { safe: boolean; short: string; title: string } {
  const latest = status.latest;
  if (!latest) {
    return status.has_data
      ? { safe: false, short: "No backup yet", title: "No backups yet" }
      : { safe: true, short: "Nothing to back up yet", title: "Nothing to back up yet" };
  }
  const ago = today - latest.day;
  const when = ago === 0 ? `today at ${clock(latest.minute)}` : ago === 1 ? `yesterday at ${clock(latest.minute)}` : `${ago} days ago`;
  const short = ago === 0 ? "Backed up today" : ago === 1 ? "Backed up yesterday" : `Backed up ${ago} days ago`;
  return { safe: ago <= 1 && !status.copy_problem, short: status.copy_problem ? `${short}, copy failed` : short, title: `Backed up ${when}` };
}

/** "The last 7 days, 4 weeks and 12 months". */
export function keptLabel(settings: { keep_days: number; keep_weeks: number; keep_months: number }): string {
  const part = (count: number, one: string) => `${count} ${count === 1 ? one : `${one}s`}`;
  const parts = [part(settings.keep_days, "day")];
  if (settings.keep_weeks) parts.push(part(settings.keep_weeks, "week"));
  if (settings.keep_months) parts.push(part(settings.keep_months, "month"));
  const last = parts.pop();
  return `The last ${parts.length ? `${parts.join(", ")} and ${last}` : last}`;
}

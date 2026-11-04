// Past a few dozen quizzes, a dot per quiz is noise. Overview switches to one point per
// week: the average of that week's quizzes (retries left out, as they don't move the
// schedule). The current week is drawn hollow because it isn't over yet.

export type Score = { day: number; percent: number; retry: boolean };
export type Week = { start: number; percent: number; quizzes: number; partial: boolean };

/** Quizzes beyond this many switch the chart to weekly points. */
export const WEEKLY_AFTER = 40;

/** Monday on or before `day` (day 0, 1 Jan 1970, was a Thursday). */
export const weekStart = (day: number) => day - ((day + 3) % 7);

export function weekly(scores: Score[], today: number): Week[] {
  const weeks = new Map<number, number[]>();
  for (const score of scores) {
    if (score.retry) continue;
    const start = weekStart(score.day);
    weeks.set(start, [...(weeks.get(start) ?? []), score.percent]);
  }
  return [...weeks.entries()]
    .sort(([a], [b]) => a - b)
    .map(([start, percents]) => ({
      start,
      percent: Math.round(percents.reduce((sum, p) => sum + p, 0) / percents.length),
      quizzes: percents.length,
      partial: start === weekStart(today),
    }));
}

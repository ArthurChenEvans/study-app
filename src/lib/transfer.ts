// Words for export and import (R112 to R118).
import type { Count } from "./api";

const ONE: Record<string, string> = {
  modules: "module", topics: "topic", sources: "source", images: "image", questions: "question", cards: "card",
  "card reviews": "card review", checklists: "checklist", "recall checks": "recall check", quizzes: "quiz",
  projects: "project", sessions: "session", issues: "issue", drills: "drill", "drill runs": "drill run",
  struggles: "struggle", settings: "setting",
};

/** "18 questions", "1 quiz". */
export function counted(count: Count): string {
  const many = count.what.toLowerCase();
  return `${count.count} ${count.count === 1 ? (ONE[many] ?? many) : many}`;
}

/** "18 questions, 12 cards and 1 source". Notes and images aren't worth a mention. */
export function countList(counts: Count[]): string {
  const parts = counts.filter((count) => count.count > 0 && !["notes", "images"].includes(count.what.toLowerCase())).map(counted);
  if (parts.length <= 1) return parts[0] ?? "nothing";
  return `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}`;
}

// Typed wrappers around the commands in src-tauri/src/lib.rs (desktop) and
// crates/study-web (browser). Each one forwards to study-api, where the behaviour is tested.
import { invoke } from "./transport";

export type Id = number;

export interface ModuleRow { id: Id; name: string }
export interface TopicRow { id: Id; name: string; module: Id | null; tags: string[]; question_count: number; short_count: number }

export interface OptionInput { text: string; correct: boolean }
export interface CitationInput { source: Id; location: string }
export interface QuestionInput {
  topic: Id;
  text: string;
  multiple: boolean;
  options: OptionInput[];
  explanation: string;
  tags: string[];
  citations: CitationInput[];
  /** Set for a short answer question (R78). Options are then ignored. */
  short: ShortAnswer | null;
}
// ---------- Short answer (R78 to R86) ----------
export interface Keyword { word: string; also: string[] }
export type ShortCheck = { Accepted: { answers: string[]; typos: boolean } } | { Keywords: { keywords: Keyword[]; needed: number | null } };
export interface ShortAnswer { check: ShortCheck; model_answer: string }
export type MarkingDetail = { Matched: { answer: string; typo: string | null } } | "NoMatch" | { Keywords: { hit: string[]; missed: string[]; needed: number } };
export interface Marking { right: boolean; detail: MarkingDetail }
export interface ShortFeedback { typed: string; marking: Marking; shown_answer: string; also_accepted: string[]; overruled: boolean }
export type QuestionTypes = "Both" | "Mcq" | "Short";

export interface SourceFields {
  authors: string;
  year: string | null;
  title: string;
  edition: string | null;
  description: string | null;
  publisher: string | null;
  url: string | null;
}
export interface SourceRow { id: Id; fields: SourceFields; apa: string; used_by: Id[]; cited_by_questions: number }

export interface QuizSetupInput {
  topics: Id[];
  count: number | null;
  even: boolean;
  shuffle_questions: boolean;
  shuffle_options: boolean;
  exam: boolean;
  total_minutes: number | null;
  per_question_seconds: number | null;
  easy: boolean;
  types: QuestionTypes;
}

export type TrackCell = "RightSure" | "RightGuessed" | "Wrong" | "Answered" | "Current" | "Upcoming";
export interface QuizView {
  position: number;
  total: number;
  exam: boolean;
  topic: Id;
  text: string;
  options: { index: number; text: string }[];
  multiple: boolean;
  short: boolean;
  answers_to_pick: number | null;
  flagged: boolean;
  total_seconds: number | null;
  per_question_seconds: number | null;
  track: TrackCell[];
}

export type AutoReason = "Incomplete" | "TimedOut";
export interface Feedback { index: number; right: boolean; auto_reason: AutoReason | null; correct: number[]; explanation: string; citations: { source: Id; location: string }[]; short: ShortFeedback | null }
export type Next = { Question: QuizView } | { Finished: Id };
export interface AnswerResult { feedback: Feedback | null; next: Next }

export interface Tally { right_sure: number; right_guessed: number; wrong: number; wrong_flagged: number; unanswered: number }
export interface Summary { overall: Tally; per_topic: Record<string, Tally> }
export type Status = "InProgress" | "Finished" | "Incomplete";
export type GivenReason =
  | "Blank" | "Misremembered" | "ConfusedWithSimilar" | "MisreadQuestion" | "CarelessSlip" | "SecondGuessed" | "NeverLearned"
  | { Other: string };
export interface ReviewItem {
  index: number;
  topic: Id;
  text: string;
  options: string[];
  picked: number[];
  correct: number[];
  explanation: string;
  status: "Right" | "RightGuessed" | "Wrong" | "Unanswered";
  auto_reason: AutoReason | null;
  reason: GivenReason | null;
  short: ShortFeedback | null;
}
export interface MoveView { topic: Id; name: string; score: number; passed: boolean; from: string | null; to: string; next_due: number; tightened_due: number | null; tightened_for: string | null }
export interface ResultsView { attempt: Id; status: Status; exam: boolean; summary: Summary; review: ReviewItem[]; to_prompt: number[]; retry_count: number; moves: MoveView[] }
export interface HistoryRow { id: Id; day: number; status: Status; retry_of: Id | null; topics: Id[]; summary: Summary }

export interface ScorePoint { day: number; percent: number; retry: boolean }
export interface Progress {
  accuracy: number | null;
  change: number | null;
  answered_last_7_days: number;
  quizzes_last_7_days: number;
  guessed_share: number | null;
  scores: ScorePoint[];
  weakest_topics: [Id, Tally][];
  never_quizzed: Id[];
  reasons: [string, number][];
}
export type Overview = "FirstRun" | { Progress: Progress };

/** Where a question matched, when it wasn't its own text. */
export type Matched = { Option: string } | { Explanation: string } | { Tag: string };
export type SearchHit =
  | { Question: { id: Id; text: string; topic: Id; topic_name: string; tags: string[]; edited: string; matched?: Matched | null } }
  | { Module: { id: Id; name: string } }
  | { Topic: { id: Id; name: string } }
  | { Tag: { name: string } }
  | { Source: { id: Id; apa: string } };
export interface SearchQuery { text: string; tab: "All" | "Questions" | "Modules" | "Topics" | "Tags" | "Sources"; module: Id | null; tag: string | null; sort: "Question" | "Topic" | "Edited"; descending: boolean }


// ---------- Free recall (R52 to R61) ----------
export type ChecklistId = number;
export type Subject = { Module: Id } | { Topic: Id };
export interface RecallPoint { text: string; question: Id | null; note: Id | null }
export interface RecallRow {
  id: ChecklistId;
  subject_name: string;
  module: boolean;
  subject_exists: boolean;
  points: number;
  step: string | null;
  next_step: string;
  position: [number, number] | null;
  due: number;
  due_today: boolean;
  days_late: number;
  last_score: number | null;
}
export interface ChecklistView { row: RecallRow; points: RecallPoint[]; topics: string[] }
/** Something a checklist point can come from (R54). */
export type RecallItem = { Question: Id } | { Note: Id };
export interface Candidate { item: RecallItem; kind: string; topic_name: string; text: string; answer: string; in_checklist: boolean }
export interface Preview { score: number; passed: boolean; to: string; next_due: number }
export interface Missed { text: string; missed: number; checks: number }
export interface CheckResult {
  score: number; recalled: number; total: number; passed: boolean; previous_score: number | null;
  from: string | null; to: string; position: [number, number]; next_step: string; next_due: number; missed: Missed[];
}
export interface CheckView { day: number; score: number; passed: boolean; from: string | null; to: string; dump: string | null }
export interface RecallHistory { checks: CheckView[]; most_missed: Missed[]; pass_mark: number }
export type Unit = "Days" | "Weeks" | "Months" | "Years";
export interface ScheduleStep { name: string; every: { count: number; unit: Unit }; repeats: number }
export interface Schedule { steps: ScheduleStep[]; pass_mark: number; under_pass: "BackOneStep" | "Stay" }


// ---------- Flashcards (R62 to R77) ----------
export type NoteKind = "Basic" | "Reversed" | "Cloze";
export interface Note { topic: Id; kind: NoteKind; front: string; back: string; cues: string[]; text: string; extra: string }
export interface NoteInput { note: Note; suggestion: string | null }
export interface DueCounts { new: number; learning: number; review: number }
export interface TopicCounts { topic: Id; name: string; module: string | null; due: DueCounts; total: number; retention: number | null }
export interface CardsHome { due: DueCounts; total_cards: number; minutes: number; forecast: number[]; retention: number | null; target: number; per_topic: TopicCounts[]; suggestions: number }
export interface CardListRow { id: Id; note: Id; front: string; kind: string; source: string | null; due: number; is_new: boolean; lapses: number; leech: boolean; archived: boolean }
export interface TopicCards { topic: Id; name: string; module: string | null; due: number; cards: CardListRow[] }
export interface CardDetail { id: Id; key: string; reps: number; lapses: number; leech: boolean; due: number; is_new: boolean; archived: boolean; remember_now: number | null }
export interface NoteView { id: Id; note: Note; source: string | null; cards: CardDetail[] }
export interface ReviewCard {
  card: Id; note: Id; front: string; back: string; topic_name: string; kind: string; source: string | null;
  intervals: [number, number, number, number]; lapses: number; leech: boolean; left: DueCounts; queue: "new" | "learning" | "review";
  done: number; total: number; can_undo: boolean;
}
export interface Rewrite { note: Id; topic: Id; front: string; agains: number }
export interface ReviewDone { reviewed: number; new: number; again: number; remembered: number | null; target: number; seconds: number; worth_rewriting: Rewrite[]; due_tomorrow: number; busiest: [number, number] | null }
export type ReviewStep = { Card: ReviewCard } | { Done: ReviewDone };
export type CardRating = "Again" | "Hard" | "Good" | "Easy";
export interface Suggestion { key: string; kind: "Recall" | "Quiz" | "Struggle"; from: string; day: number; note: string; text: string; answer: string | null; topic: Id | null }
export interface CardSettings { desired_retention: number; new_per_day: number; max_reviews: number | null; leech_after: number; parameters: number[] }

// ---------- Projects, drills, quiz schedules and the calendar (R87 to R100) ----------
export type StruggleKind = "Recall" | "Apply" | "Unsure";
export type DrillRating = "Struggled" | "Shaky" | "Solid";
export interface SessionRecord { id: Id; project: Id; day: number; started: number; ended: number | null }
export interface ProjectRow { id: Id; name: string; kind: string; created: number; last_day: number | null; struggles: number; to_review: number; drills_due: number; session: SessionRecord | null }
export type OutcomeView = "Card" | "Nothing" | { Checklist: { id: Id; name: string } } | { Drill: { id: Id; due: number | null } };
export interface StruggleView { id: Id; project: Id; project_name: string; day: number; minute: number; what: string; why: string; kind: StruggleKind; topic: Id | null; topic_name: string | null; issue: string | null; outcome: OutcomeView | null }
export interface ProjectView { row: ProjectRow; struggles: StruggleView[] }
export interface DrillRun { drill: Id; day: number; rating: DrillRating; seconds: number; note: string }
export interface DrillView { id: Id; task: string; hints: string; issue: string | null; topic: Id | null; topic_name: string | null; created: number; due: number | null; last: DrillRun | null; runs: number; from: { day: number; project_name: string; what: string }[] }
export interface IssueView { id: Id; name: string; seen: number; projects: string[]; last_seen: number | null; drill: DrillView | null }
export interface StruggleInput { what: string; why: string; kind: StruggleKind; topic: Id | null }
export type ReviewChoice =
  | "Card" | "Nothing"
  | { Checklist: { checklist: Id } }
  | { NewDrill: { issue: string; task: string; hints: string; topic: Id | null; day: number } }
  | { DrillAgain: { issue: string; drill: Id; day: number } };
export interface TopicQuizView { topic: Id; step: string; position: number; steps: number; due: number; due_today: boolean; normal_due: number; tightened_for: string | null; waiting: boolean }
export interface QuizQueue { daily: number; today: Id[]; waiting: Id[]; clears_by: number | null; next_four_weeks: number }
// Dates on the calendar (R130 to R137)
export type Covers = "Nothing" | { Module: Id } | { Topics: Id[] };
export interface DateView { id: Id; name: string; day: number; kind: string; covers: Covers; covers_label: string; topics: Id[]; tighten: boolean; days_left: number }
export interface DateSave { name: string; day: number; kind: string; module: Id | null; topics: Id[]; tighten: boolean }
export interface DateMove { topic: Id; name: string; step: string; from: number; to: number }
export interface UnquizzedTopic { topic: Id; name: string; questions: number }
export interface DatePreview { quizzes: DateMove[]; unquizzed: UnquizzedTopic[]; cards: number; drills: number }
export interface DatePanel { date: DateView; quizzes: DateMove[]; unquizzed: UnquizzedTopic[]; cards_before: number; cards_pulled: number }
export type When = "Past" | "Today" | "Future";
export interface CalendarDay { day: number; when: When; cards: number; checklists: number; quizzes: number; drills: number; struggles: number }
export interface DueToday { cards: number; checklists: number; quizzes: number; drills: number; total: number }
export interface DayView {
  day: number; when: When;
  cards: { due: DueCounts; minutes: number; reviewed: number; again: number; seconds: number };
  checklists: { id: Id; name: string; step: string | null; points: number }[];
  checks: { id: Id; name: string; score: number; passed: boolean; to: string }[];
  quizzes: { topic: Id; name: string; step: string; questions: number; tightened_for: string | null }[];
  attempts: { attempt: Id; topics: string[]; right: number; total: number; moves: string[] }[];
  drills: DrillView[];
  drill_runs: { task: string; rating: DrillRating }[];
  struggles: StruggleView[];
  dates: DatePanel[];
  soon: DateView[];
  waiting: DayView["quizzes"];
  clears_by: number | null;
  daily: number;
}

/** Errors from the backend arrive as { message }. */
export function errorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}

export const api = {
  modules: () => invoke<ModuleRow[]>("modules"),
  createModule: (name: string) => invoke<Id>("create_module", { name }),
  deleteModule: (module: Id) => invoke<void>("delete_module", { module }),
  topics: () => invoke<TopicRow[]>("topics"),
  createTopic: (name: string, module: Id | null) => invoke<Id>("create_topic", { name, module }),
  moveTopic: (topic: Id, module: Id | null) => invoke<void>("move_topic", { topic, module }),
  setTopicTags: (topic: Id, tags: string[]) => invoke<void>("set_topic_tags", { topic, tags }),
  deleteTopic: (topic: Id) => invoke<void>("delete_topic", { topic }),

  saveQuestion: (input: QuestionInput) => invoke<Id>("save_question", { input }),
  updateQuestion: (id: Id, input: QuestionInput) => invoke<void>("update_question", { id, input }),
  question: (id: Id) => invoke<QuestionInput>("question", { id }),
  questionsIn: (topic: Id) => invoke<[Id, string][]>("questions_in", { topic }),
  deleteQuestion: (id: Id) => invoke<void>("delete_question", { id }),

  addSource: (fields: SourceFields) => invoke<Id>("add_source", { fields }),
  sources: () => invoke<SourceRow[]>("sources"),
  updateSource: (id: Id, fields: SourceFields) => invoke<void>("update_source", { id, fields }),
  /** Returns how many questions cited the source. */
  deleteSource: (id: Id) => invoke<number>("delete_source", { id }),

  startQuiz: (setup: QuizSetupInput) => invoke<QuizView>("start_quiz", { setup }),
  currentQuestion: () => invoke<QuizView>("current_question"),
  toggleFlag: () => invoke<QuizView>("toggle_flag"),
  answer: (picked: number[]) => invoke<AnswerResult>("answer", { picked }),
  questionTimedOut: () => invoke<AnswerResult>("question_timed_out"),
  answerText: (typed: string) => invoke<AnswerResult>("answer_text", { typed }),
  /** R84. `attempt` is set once the quiz has finished, for its last question. */
  overruleAnswer: (attempt: Id | null, index: number) => invoke<Feedback>("overrule_answer", { attempt, index }),
  overruleResult: (attempt: Id, index: number) => invoke<ResultsView>("overrule_result", { attempt, index }),
  tryShortAnswer: (check: ShortCheck, typed: string) => invoke<Marking>("try_short_answer", { check, typed }),
  totalTimedOut: () => invoke<Id>("total_timed_out"),
  quitQuiz: () => invoke<Id>("quit_quiz"),
  retry: (attempt: Id) => invoke<QuizView>("retry", { attempt }),

  results: (attempt: Id) => invoke<ResultsView>("results", { attempt }),
  giveReason: (attempt: Id, index: number, reason: GivenReason) => invoke<ResultsView>("give_reason", { attempt, index, reason }),
  history: () => invoke<HistoryRow[]>("history"),
  overview: () => invoke<Overview>("overview"),
  today: () => invoke<number>("today_day"),
  search: (query: SearchQuery) => invoke<SearchHit[]>("search", { query }),
  /** R7: stores a pasted or picked image and returns the id the Markdown points at. */
  addImage: async (file: Blob) => invoke<Id>("add_image", { data: await toBase64(file) }),
  image: async (id: Id) => new Uint8Array(await invoke<ArrayBuffer | number[]>("image", { id })),
  /** Sends slides as base64 JSON (works on every IPC path) and gets PDF bytes back. Nothing is stored. */
  recallList: () => invoke<RecallRow[]>("recall_list"),
  recallDueCount: () => invoke<number>("recall_due_count"),
  createChecklist: (subject: Subject) => invoke<ChecklistId>("create_checklist", { subject }),
  deleteChecklist: (id: ChecklistId) => invoke<void>("delete_checklist", { id }),
  checklist: (id: ChecklistId) => invoke<ChecklistView>("checklist", { id }),
  savePoints: (id: ChecklistId, points: RecallPoint[]) => invoke<RecallPoint[]>("save_points", { id, points }),
  recallCandidates: (id: ChecklistId) => invoke<Candidate[]>("recall_candidates", { id }),
  addFromItems: (id: ChecklistId, add: RecallItem[], remove: RecallItem[]) => invoke<RecallPoint[]>("add_from_items", { id, add, remove }),
  previewCheck: (id: ChecklistId, recalled: number) => invoke<Preview>("preview_check", { id, recalled }),
  finishCheck: (id: ChecklistId, dump: string | null, recalled: number[]) => invoke<CheckResult>("finish_check", { id, dump, recalled }),
  recallHistory: (id: ChecklistId) => invoke<RecallHistory>("recall_history", { id }),
  recallSchedule: () => invoke<Schedule>("recall_schedule"),
  setRecallSchedule: (schedule: Schedule) => invoke<void>("set_recall_schedule", { schedule }),
  resetRecallSchedule: () => invoke<Schedule>("reset_recall_schedule"),
  cardsHome: () => invoke<CardsHome>("cards_home"),
  cardsDueCount: () => invoke<number>("cards_due_count"),
  topicCards: (topic: Id) => invoke<TopicCards>("topic_cards", { topic }),
  noteView: (id: Id) => invoke<NoteView>("note_view", { id }),
  saveNote: (id: Id | null, input: NoteInput) => invoke<Id>("save_note", { id, input }),
  deleteNote: (id: Id) => invoke<void>("delete_note", { id }),
  archiveCard: (id: Id, archived: boolean) => invoke<void>("archive_card", { id, archived }),
  startReview: (scope: Subject | null) => invoke<ReviewStep>("start_review", { scope }),
  currentCard: () => invoke<ReviewStep>("current_card"),
  gradeCard: (rating: CardRating, seconds: number) => invoke<ReviewStep>("grade_card", { rating, seconds }),
  undoCard: () => invoke<ReviewStep>("undo_card"),
  leaveReview: () => invoke<void>("leave_review"),
  cardSuggestions: () => invoke<Suggestion[]>("card_suggestions"),
  dismissSuggestion: (key: string) => invoke<void>("dismiss_suggestion", { key }),
  cardSettings: () => invoke<CardSettings>("card_settings"),
  setCardSettings: (settings: CardSettings) => invoke<void>("set_card_settings", { settings }),
  resetCardSettings: () => invoke<CardSettings>("reset_card_settings"),
  fitScheduler: () => invoke<number>("fit_scheduler"),
  projects: () => invoke<ProjectRow[]>("projects"),
  createProject: (name: string, kind: string) => invoke<Id>("create_project", { name, kind }),
  deleteProject: (id: Id) => invoke<void>("delete_project", { id }),
  project: (id: Id) => invoke<ProjectView>("project", { id }),
  startProjectSession: (project: Id) => invoke<ProjectView>("start_project_session", { project }),
  endProjectSession: (project: Id) => invoke<Id>("end_project_session", { project }),
  logStruggle: (project: Id, input: StruggleInput) => invoke<StruggleView>("log_struggle", { project, input }),
  deleteStruggle: (id: Id) => invoke<void>("delete_struggle", { id }),
  strugglesToReview: (project: Id) => invoke<StruggleView[]>("struggles_to_review", { project }),
  reviewStruggle: (id: Id, choice: ReviewChoice) => invoke<StruggleView>("review_struggle", { id, choice }),
  issues: () => invoke<IssueView[]>("issues"),
  knownDrill: (issue: string) => invoke<DrillView | null>("known_drill", { issue }),
  drill: (id: Id) => invoke<DrillView>("drill", { id }),
  drillsDue: () => invoke<DrillView[]>("drills_due"),
  rateDrill: (id: Id, rating: DrillRating, seconds: number, note: string) => invoke<DrillView>("rate_drill", { id, rating, seconds, note }),
  moveDrill: (id: Id, day: number) => invoke<DrillView>("move_drill", { id, day }),
  quizSchedule: () => invoke<Schedule>("quiz_schedule"),
  setQuizSchedule: (schedule: Schedule) => invoke<void>("set_quiz_schedule", { schedule }),
  resetQuizSchedule: () => invoke<Schedule>("reset_quiz_schedule"),
  topicQuizzes: () => invoke<TopicQuizView[]>("topic_quizzes"),
  quizQueue: () => invoke<QuizQueue>("quiz_queue"),
  quizzesADay: () => invoke<number>("quizzes_a_day"),
  appVersion: () => invoke<string>("app_version"),
  updateSetAuto: (on: boolean) => invoke<void>("update_set_auto", { on }),
  updateCheck: () => invoke<UpdateInfo | null>("update_check"),
  updateDownload: () => invoke<UpdateInfo>("update_download"),
  updateInstall: (restart: boolean) => invoke<void>("update_install", { restart }),
  celebrations: () => invoke<string | null>("celebrations"),
  setCelebrations: (json: string) => invoke<void>("set_celebrations", { json }),
  resetCelebrations: () => invoke<void>("reset_celebrations"),
  setQuizzesADay: (count: number) => invoke<void>("set_quizzes_a_day", { count }),
  calendarMonth: (from: number, to: number) => invoke<CalendarDay[]>("calendar_month", { from, to }),
  calendarDay: (day: number) => invoke<DayView>("calendar_day", { day }),
  dates: () => invoke<DateView[]>("dates"),
  addDate: (date: DateSave) => invoke<Id>("add_date", { date }),
  updateDate: (id: Id, date: DateSave) => invoke<void>("update_date", { id, date }),
  deleteDate: (id: Id) => invoke<void>("delete_date", { id }),
  previewDate: (id: Id | null, date: DateSave) => invoke<DatePreview>("preview_date", { id, date }),
  datePanel: (id: Id) => invoke<DatePanel>("date_panel", { id }),
  dueToday: () => invoke<DueToday>("due_today"),
  // Backups (R101 to R108)
  backupStatus: () => invoke<BackupStatus>("backup_status"),
  backups: () => invoke<BackupInfo[]>("backups"),
  backUpNow: () => invoke<BackupInfo>("back_up_now"),
  dailyBackup: () => invoke<BackupInfo | null>("daily_backup"),
  restorePreview: (name: string) => invoke<RestorePreview>("restore_preview", { name }),
  restoreBackup: (name: string) => invoke<BackupInfo>("restore_backup", { name }),
  setBackupSettings: (settings: BackupSettings) => invoke<BackupStatus>("set_backup_settings", { settings }),
  // Export and import (R112 to R118)
  exportSummary: (request: ExportRequest) => invoke<ExportSummary>("export_summary", { request }),
  exportFile: (request: ExportRequest) => invoke<ExportFile>("export_file", { request }),
  importPreview: async (file: Blob, name: string) => invoke<ImportPreview>("import_preview", { name, data: await toBase64(file) }),
  importApply: (choice: ImportChoice) => invoke<ImportResult>("import_apply", { choice }),
  cancelImport: () => invoke<void>("cancel_import"),
  // Sync (R119 to R128)
  syncStatus: () => invoke<SyncStatus>("sync_status"),
  syncNow: (auto: boolean) => invoke<SyncResult>("sync_now", { auto }),
  syncSetup: (setup: SyncSetup) => invoke<SyncResult>("sync_setup", { setup }),
  syncRename: (name: string) => invoke<SyncStatus>("sync_rename", { name }),
  syncOff: () => invoke<SyncStatus>("sync_off"),
  syncResume: () => invoke<SyncResult>("sync_resume"),
  syncToCheck: () => invoke<ToCheck[]>("sync_to_check"),
  syncSettle: (n: number, useOther: boolean) => invoke<ToCheck[]>("sync_settle", { n, other: useOther }),
  slidesToPdf: async (file: File) => {
    const pdf = await invoke<ArrayBuffer | number[]>("slides_to_pdf", { name: file.name, data: await toBase64(file) });
    return new Uint8Array(pdf);
  },
};

/** Base64 of a file's bytes, without the data: URL prefix. */
function toBase64(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",", 2)[1] ?? "");
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

// ---------- Backups (R101 to R108) ----------
export type BackupReason = "Daily" | "ByHand" | "BeforeRestore" | "BeforeUpdate" | "BeforeImport" | "BeforeSync";
export interface BackupInfo { name: string; day: number; minute: number; reason: BackupReason; bytes: number }
export interface BackupSettings { keep_days: number; keep_weeks: number; keep_months: number; copy_to: string | null }
export interface BackupStatus { latest: BackupInfo | null; count: number; bytes: number; folder: string; settings: BackupSettings; copy_problem: string | null; has_data: boolean }
export interface Undone { count: number; one: string; many: string }
export interface RestorePreview { backup: BackupInfo; undone: Undone[]; older: boolean }

// ---------- Export and import (R112 to R118) ----------
export interface ExportRequest { topics: Id[] | null; history: boolean }
export interface Count { what: string; count: number }
export interface ExportSummary { name: string; counts: Count[] }
export interface ExportFile { name: string; path: string | null; data: string | null }
export interface PreviewRow { what: string; new: number; same: number; changed: number }
export interface Clash { uid: string; what: string; label: string; mine_day: number; theirs_day: number; theirs_newer: boolean }
export type ClashRule = "KeepMine" | "TakeTheirs" | "Newer" | "EachOne";
export interface ImportPreview { file_name: string; everything: boolean; history: boolean; exported: number; from_here: boolean; rows: PreviewRow[]; clashes: Clash[]; suggested: ClashRule }
export interface ImportChoice { rule: ClashRule; theirs: string[] }
export interface ImportResult { added: Count[]; replaced: number; backup: BackupInfo | null }

// ---------- Sync (R119 to R128) ----------
export interface SyncSetup { folder: string; name: string }
export interface SyncDevice { id: string; name: string; this_one: boolean; last_seen: number | null; waiting: number; newer_app: boolean }
export interface SyncStatus { on: boolean; paused: string | null; folder: string | null; device: string; name: string; devices: SyncDevice[]; last_synced: number | null; problem: string | null; waiting: number; held: number; to_check: number }
export interface SyncResult { sent: number; received: number; deferred: boolean; status: SyncStatus }
export interface ToCheck { n: number; record: string; what: string; label: string; context: string | null; field: string; deleted: boolean; kept: string | null; other: string | null; kept_by: string; kept_when: number; other_by: string; other_when: number }

// ---------- Updates (R165 to R168) ----------
export interface UpdateInfo { version: string; current: string; notes: string | null; date: string | null; ready: boolean }

// Typed wrappers around the Tauri commands in src-tauri/src/lib.rs.
// Each one forwards to study-api, where the behaviour is tested.
import { invoke } from "@tauri-apps/api/core";

export type Id = number;

export interface ModuleRow { id: Id; name: string }
export interface TopicRow { id: Id; name: string; module: Id | null; tags: string[]; question_count: number }

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
}

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
  answers_to_pick: number | null;
  flagged: boolean;
  total_seconds: number | null;
  per_question_seconds: number | null;
  track: TrackCell[];
}

export type AutoReason = "Incomplete" | "TimedOut";
export interface Feedback { right: boolean; auto_reason: AutoReason | null; correct: number[]; explanation: string; citations: { source: Id; location: string }[] }
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
}
export interface ResultsView { attempt: Id; status: Status; exam: boolean; summary: Summary; review: ReviewItem[]; to_prompt: number[]; retry_count: number }
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

export type SearchHit =
  | { Question: { id: Id; text: string; topic: Id; topic_name: string; tags: string[]; edited: string } }
  | { Module: { id: Id; name: string } }
  | { Topic: { id: Id; name: string } }
  | { Tag: { name: string } }
  | { Source: { id: Id; apa: string } };
export interface SearchQuery { text: string; tab: "All" | "Questions" | "Modules" | "Topics" | "Tags" | "Sources"; module: Id | null; tag: string | null; sort: "Question" | "Topic" | "Edited"; descending: boolean }

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
  totalTimedOut: () => invoke<Id>("total_timed_out"),
  quitQuiz: () => invoke<Id>("quit_quiz"),
  retry: (attempt: Id) => invoke<QuizView>("retry", { attempt }),

  results: (attempt: Id) => invoke<ResultsView>("results", { attempt }),
  giveReason: (attempt: Id, index: number, reason: GivenReason) => invoke<ResultsView>("give_reason", { attempt, index, reason }),
  history: () => invoke<HistoryRow[]>("history"),
  overview: () => invoke<Overview>("overview"),
  today: () => invoke<number>("today_day"),
  search: (query: SearchQuery) => invoke<SearchHit[]>("search", { query }),
  /** Sends slides as raw bytes and gets PDF bytes back. Nothing is stored. */
  slidesToPdf: async (file: File) =>
    new Uint8Array(await invoke<ArrayBuffer>("slides_to_pdf", new Uint8Array(await file.arrayBuffer()), { headers: { "x-file-name": encodeURIComponent(file.name) } })),
};

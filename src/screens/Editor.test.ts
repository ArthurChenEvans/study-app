import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { topics: vi.fn(), questionsIn: vi.fn(), question: vi.fn(), saveQuestion: vi.fn(), updateQuestion: vi.fn(), deleteQuestion: vi.fn(), sources: vi.fn() },
  };
});

import { api } from "../lib/api";
import Editor from "./Editor.svelte";

const mocked = vi.mocked(api);
const user = () => userEvent.setup();

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topics.mockResolvedValue([{ id: 1, name: "CPU", module: null, tags: [], question_count: 1 }]);
  mocked.questionsIn.mockResolvedValue([[10, "What does the ALU do?"]]);
  mocked.sources.mockResolvedValue([
    { id: 5, fields: { authors: "Kirk", year: "2022", title: "Parallel", edition: null, description: null, publisher: null, url: null }, apa: "Kirk. (2022). *Parallel*.", used_by: [], cited_by_questions: 0 },
  ]);
});

async function fillOption(u: ReturnType<typeof user>, index: number, text: string) {
  await u.type(screen.getByLabelText(`Option ${index}`), text);
}

describe("Question editor", () => {
  it("lists the topic's questions", async () => {
    render(Editor, { topic: 1 });
    expect(await screen.findByText("What does the ALU do?")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "New question" })).toBeInTheDocument();
  });

  // questions.feature: Write a question with an explanation / Tag a question
  it("saves a new question and clears the form", async () => {
    const u = user();
    mocked.saveQuestion.mockResolvedValue(11);
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.type(screen.getByLabelText("Question"), "Which stage reads registers?");
    await fillOption(u, 1, "Fetch");
    await fillOption(u, 2, "Decode");
    await fillOption(u, 3, "Execute");
    await u.click(screen.getByLabelText("Option 2 is correct"));
    await u.type(screen.getByLabelText("Explanation"), "Operands are read in decode.");
    await u.type(screen.getByLabelText("Tags"), "pipeline, registers");
    await u.click(screen.getByRole("button", { name: "Save question" }));
    expect(mocked.saveQuestion).toHaveBeenCalledWith({
      topic: 1,
      text: "Which stage reads registers?",
      multiple: false,
      options: [
        { text: "Fetch", correct: false },
        { text: "Decode", correct: true },
        { text: "Execute", correct: false },
        { text: "", correct: false },
      ].filter((option) => option.text !== ""),
      explanation: "Operands are read in decode.",
      tags: ["pipeline", "registers"],
      citations: [],
    });
    expect(screen.getByLabelText("Question")).toHaveValue("");
  });

  it("shows the reason a save was refused", async () => {
    const u = user();
    mocked.saveQuestion.mockRejectedValue({ message: "mark at least one correct option before saving" });
    render(Editor, { topic: 1 });
    await u.click(await screen.findByRole("button", { name: "Save question" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("mark at least one correct option before saving");
  });

  // questions.feature: Option count limits
  it("allows 3 to 5 options", async () => {
    const u = user();
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    expect(screen.getAllByLabelText(/^Option \d$/)).toHaveLength(4);
    await u.click(screen.getByRole("button", { name: "Add option" }));
    expect(screen.getAllByLabelText(/^Option \d$/)).toHaveLength(5);
    expect(screen.getByRole("button", { name: "Add option" })).toBeDisabled();
    await u.click(screen.getByRole("button", { name: "Remove option 5" }));
    await u.click(screen.getByRole("button", { name: "Remove option 4" }));
    expect(screen.getAllByLabelText(/^Option \d$/)).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Remove option 1" })).toBeDisabled();
  });

  // questions.feature: Single-answer / Multiple-answer question
  it("single answer keeps one tick, multiple answer allows several", async () => {
    const u = user();
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.click(screen.getByLabelText("Option 1 is correct"));
    await u.click(screen.getByLabelText("Option 2 is correct"));
    expect(screen.getByLabelText("Option 1 is correct")).not.toBeChecked();
    await u.click(screen.getByRole("radio", { name: "Multiple" }));
    await u.click(screen.getByLabelText("Option 3 is correct"));
    expect(screen.getByLabelText("Option 2 is correct")).toBeChecked();
    expect(screen.getByLabelText("Option 3 is correct")).toBeChecked();
  });

  it("edits an existing question", async () => {
    const u = user();
    mocked.question.mockResolvedValue({
      topic: 1, text: "What does the ALU do?", multiple: false,
      options: [{ text: "Arithmetic", correct: true }, { text: "Storage", correct: false }, { text: "Fetching", correct: false }],
      explanation: "It computes.", tags: ["alu"], citations: [],
    });
    render(Editor, { topic: 1 });
    await u.click(await screen.findByRole("button", { name: "What does the ALU do?" }));
    expect(screen.getByRole("heading", { name: "Edit question" })).toBeInTheDocument();
    expect(screen.getByLabelText("Question")).toHaveValue("What does the ALU do?");
    await u.clear(screen.getByLabelText("Question"));
    await u.type(screen.getByLabelText("Question"), "What is the ALU's job?");
    await u.click(screen.getByRole("button", { name: "Save question" }));
    expect(mocked.updateQuestion).toHaveBeenCalledWith(10, expect.objectContaining({ text: "What is the ALU's job?", tags: ["alu"] }));
  });

  // sources.feature: Cite sources on a question
  it("cites a source with a page", async () => {
    const u = user();
    mocked.saveQuestion.mockResolvedValue(11);
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.click(screen.getByRole("button", { name: "Cite a source" }));
    const citation = screen.getByRole("group", { name: "Citation 1" });
    await u.selectOptions(within(citation).getByLabelText("Source"), "Kirk. (2022). *Parallel*.");
    await u.type(within(citation).getByLabelText("Page, slide or line"), "p. 88");
    await u.click(screen.getByRole("button", { name: "Save question" }));
    expect(mocked.saveQuestion).toHaveBeenCalledWith(expect.objectContaining({ citations: [{ source: 5, location: "p. 88" }] }));
  });
});

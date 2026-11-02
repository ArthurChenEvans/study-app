import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { topics: vi.fn(), questionsIn: vi.fn(), question: vi.fn(), saveQuestion: vi.fn(), updateQuestion: vi.fn(), deleteQuestion: vi.fn(), sources: vi.fn(), addImage: vi.fn(), image: vi.fn() },
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

  // R49
  it("switching to single keeps the first tick and says so", async () => {
    const u = user();
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.click(screen.getByRole("radio", { name: "Multiple" }));
    await u.click(screen.getByLabelText("Option 2 is correct"));
    await u.click(screen.getByLabelText("Option 4 is correct"));
    await u.click(screen.getByRole("radio", { name: "Single" }));
    expect(screen.getByLabelText("Option 2 is correct")).toBeChecked();
    expect(screen.getByLabelText("Option 4 is correct")).not.toBeChecked();
    expect(screen.getByRole("status")).toHaveTextContent("Kept option 2 as the correct answer");
  });

  // R48
  it("Ctrl+S saves the question", async () => {
    const u = user();
    mocked.saveQuestion.mockResolvedValue(11);
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.type(screen.getByLabelText("Question"), "Q?");
    await u.keyboard("{Control>}s{/Control}");
    expect(mocked.saveQuestion).toHaveBeenCalledWith(expect.objectContaining({ text: "Q?" }));
  });

  // questions.feature: Add an image by pasting or picking
  describe("images", () => {
    const png = () => new File([new Uint8Array([137, 80, 78, 71])], "pipeline.png", { type: "image/png" });

    it("pastes an image into the question at the cursor and previews it", async () => {
      const u = user();
      mocked.addImage.mockResolvedValue(12);
      mocked.image.mockResolvedValue(new Uint8Array([137]));
      render(Editor, { topic: 1 });
      await screen.findByText("What does the ALU do?");
      const field = screen.getByLabelText("Question") as HTMLTextAreaElement;
      await u.type(field, "Which stage is this?");
      await u.paste({ files: [png()] } as unknown as DataTransfer);
      expect(mocked.addImage).toHaveBeenCalledWith(expect.any(File));
      await vi.waitFor(() => expect(field.value).toBe("Which stage is this? ![pipeline](image:12)"));
      expect(await screen.findByRole("status")).toHaveTextContent("Image added to the question");
      expect(within(screen.getByLabelText("Question preview")).getByAltText("pipeline")).toBeInTheDocument();
    });

    it("pastes into an option and the image is kept when saving", async () => {
      const u = user();
      mocked.addImage.mockResolvedValue(7);
      mocked.image.mockResolvedValue(new Uint8Array([137]));
      mocked.saveQuestion.mockResolvedValue(11);
      render(Editor, { topic: 1 });
      await screen.findByText("What does the ALU do?");
      await u.type(screen.getByLabelText("Question"), "Which diagram shows a hazard?");
      await u.click(screen.getByLabelText("Option 1"));
      await u.paste({ files: [png()] } as unknown as DataTransfer);
      await vi.waitFor(() => expect((screen.getByLabelText("Option 1") as HTMLInputElement).value).toBe("![pipeline](image:7)"));
      expect(screen.getByRole("status")).toHaveTextContent("Image added to option 1");
      await fillOption(u, 2, "Neither");
      await fillOption(u, 3, "Both");
      await u.click(screen.getByLabelText("Option 1 is correct"));
      await u.click(screen.getByRole("button", { name: "Save question" }));
      expect(mocked.saveQuestion.mock.calls[0][0].options[0]).toEqual({ text: "![pipeline](image:7)", correct: true });
    });

    it("leaves plain text pastes alone", async () => {
      const u = user();
      render(Editor, { topic: 1 });
      await screen.findByText("What does the ALU do?");
      await u.click(screen.getByLabelText("Question"));
      await u.paste("plain words");
      expect((screen.getByLabelText("Question") as HTMLTextAreaElement).value).toBe("plain words");
      expect(mocked.addImage).not.toHaveBeenCalled();
    });

    it("picks an image from a file into the field used last", async () => {
      const u = user();
      mocked.addImage.mockResolvedValue(3);
      render(Editor, { topic: 1 });
      await screen.findByText("What does the ALU do?");
      await u.type(screen.getByLabelText("Explanation"), "See:");
      await u.upload(screen.getByLabelText("Image file"), png());
      await vi.waitFor(() => expect((screen.getByLabelText("Explanation") as HTMLTextAreaElement).value).toBe("See: ![pipeline](image:3)"));
      expect(screen.getByRole("status")).toHaveTextContent("Image added to the explanation");
    });

    it("picking with no field used goes into the question", async () => {
      const u = user();
      mocked.addImage.mockResolvedValue(4);
      render(Editor, { topic: 1 });
      await screen.findByText("What does the ALU do?");
      await u.upload(screen.getByLabelText("Image file"), png());
      await vi.waitFor(() => expect((screen.getByLabelText("Question") as HTMLTextAreaElement).value).toBe("![pipeline](image:4)"));
    });

    it("shows the backend's reason when an image is refused", async () => {
      const u = user();
      mocked.addImage.mockRejectedValue({ message: "the image is over 10 MB, use a smaller one" });
      render(Editor, { topic: 1 });
      await screen.findByText("What does the ALU do?");
      await u.click(screen.getByLabelText("Question"));
      await u.paste({ files: [png()] } as unknown as DataTransfer);
      expect(await screen.findByRole("alert")).toHaveTextContent("the image is over 10 MB, use a smaller one");
      expect((screen.getByLabelText("Question") as HTMLTextAreaElement).value).toBe("");
    });
  });
});

import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: { topics: vi.fn(), questionsIn: vi.fn(), question: vi.fn(), saveQuestion: vi.fn(), updateQuestion: vi.fn(), deleteQuestion: vi.fn(), sources: vi.fn(), addImage: vi.fn(), image: vi.fn(), tryShortAnswer: vi.fn() },
  };
});

import { api } from "../lib/api";
import Editor from "./Editor.svelte";

const mocked = vi.mocked(api);
const user = () => userEvent.setup();

beforeEach(() => {
  vi.resetAllMocks();
  mocked.topics.mockResolvedValue([{ id: 1, name: "CPU", module: null, tags: [], question_count: 1, short_count: 0 }]);
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
      short: null,
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
  // UI polish: the number of boxes says how many options there are, the placeholder says which
  it("labels each empty option with a placeholder", async () => {
    render(Editor, { topic: 1 });
    expect(await screen.findByLabelText("Option 1")).toHaveAttribute("placeholder", "Option 1");
    expect(screen.getByLabelText("Option 4")).toHaveAttribute("placeholder", "Option 4");
  });

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
      explanation: "It computes.", tags: ["alu"], citations: [], short: null,
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

  // R185: a question can be deleted, after asking, and the list updates
  it("deletes the question being edited after asking", async () => {
    const u = user();
    mocked.question.mockResolvedValue({ topic: 1, text: "What does the ALU do?", multiple: false, options: [{ text: "A", correct: true }, { text: "B", correct: false }, { text: "C", correct: false }], explanation: "", tags: [], citations: [], short: null });
    render(Editor, { topic: 1 });
    await u.click(await screen.findByRole("button", { name: "What does the ALU do?" }));
    await u.click(screen.getByRole("button", { name: "Delete question" }));
    const dialog = screen.getByRole("dialog", { name: "Delete this question?" });
    expect(dialog).toHaveTextContent("Past quiz results keep their answers.");
    mocked.questionsIn.mockResolvedValue([]);
    await u.click(within(dialog).getByRole("button", { name: "Delete question" }));
    expect(mocked.deleteQuestion).toHaveBeenCalledWith(10);
    expect(screen.getByRole("heading", { name: "New question" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "What does the ALU do?" })).toBeNull();
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

  // sources.feature: Showing where a question cites (R181)
  it("shows the cited page in the preview", async () => {
    const u = user();
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.click(screen.getByRole("button", { name: "Cite a source" }));
    const citation = screen.getByRole("group", { name: "Citation 1" });
    expect(within(citation).queryByRole("button", { name: /Show/ })).not.toBeInTheDocument();
    await u.selectOptions(within(citation).getByLabelText("Source"), "Kirk. (2022). *Parallel*.");
    await u.type(within(citation).getByLabelText("Page, slide or line"), "p. 88");
    await u.click(within(citation).getByRole("button", { name: "Show page 88 in the preview" }));
    const preview = screen.getByRole("region", { name: "Source preview" });
    expect(within(preview).getByRole("status")).toHaveTextContent("Open the file for Kirk (2022) to see page 88.");
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
  // R7: fenced code needs its line breaks kept
  it("keeps line breaks in the question and explanation", async () => {
    const u = user();
    mocked.saveQuestion.mockResolvedValue(11);
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.type(screen.getByLabelText("Question"), "Read this:{Enter}{Enter}```c{Enter}int index = 0;{Enter}```");
    await fillOption(u, 1, "A");
    await fillOption(u, 2, "B");
    await fillOption(u, 3, "C");
    await u.click(screen.getByLabelText("Option 1 is correct"));
    await u.type(screen.getByLabelText("Explanation"), "Line one{Enter}Line two");
    await u.click(screen.getByRole("button", { name: "Save question" }));
    expect(mocked.saveQuestion).toHaveBeenCalledWith(expect.objectContaining({
      text: "Read this:\n\n```c\nint index = 0;\n```",
      explanation: "Line one\nLine two",
    }));
  });

  // short-answer.feature: Switching type keeps what's shared, A short answer question needs something to check against
  it("writes a short answer question checked by accepted answers", async () => {
    const u = user();
    mocked.saveQuestion.mockRejectedValueOnce({ message: "add at least one accepted answer, so the app can check it" }).mockResolvedValueOnce(12);
    render(Editor, { topic: 1 });
    await screen.findByText("What does the ALU do?");
    await u.type(screen.getByLabelText("Question"), "Which layer adds port numbers?");
    await fillOption(u, 1, "Network");
    await u.type(screen.getByLabelText("Explanation"), "Ports let apps share an address.");
    await u.type(screen.getByLabelText("Tags"), "layers");
    await u.click(screen.getByRole("radio", { name: "Short answer" }));
    expect(screen.queryByLabelText("Option 1")).toBeNull();
    expect(screen.getByLabelText("Question")).toHaveValue("Which layer adds port numbers?");
    await u.click(screen.getByRole("button", { name: /Save question/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("add at least one accepted answer");

    await u.type(screen.getByLabelText("Accepted answer 1"), "Transport layer");
    await u.click(screen.getByRole("button", { name: /Add another accepted answer/ }));
    await u.type(screen.getByLabelText("Accepted answer 2"), "Layer 4");
    await u.click(screen.getByLabelText("Allow small typos"));
    await u.type(screen.getByLabelText(/Model answer/), "The **transport** layer");
    await u.click(screen.getByRole("button", { name: /Save question/ }));
    expect(mocked.saveQuestion).toHaveBeenLastCalledWith({
      topic: 1, text: "Which layer adds port numbers?", multiple: false, options: [],
      explanation: "Ports let apps share an address.", tags: ["layers"], citations: [],
      short: { check: { Accepted: { answers: ["Transport layer", "Layer 4"], typos: false } }, model_answer: "The **transport** layer" },
    });
    expect(screen.getByRole("radio", { name: "Short answer" })).toHaveAttribute("aria-checked", "true"); // stays on short answer for the next one
  });

  it("brings the options back when switching back before saving", async () => {
    const u = user();
    render(Editor, { topic: 1 });
    await fillOption(u, 1, "Network");
    await u.click(screen.getByRole("radio", { name: "Short answer" }));
    await u.click(screen.getByRole("radio", { name: "Multiple choice" }));
    expect(screen.getByLabelText("Option 1")).toHaveValue("Network");
  });

  // R81
  it("writes keywords with other spellings and how many are needed", async () => {
    const u = user();
    mocked.saveQuestion.mockResolvedValue(13);
    render(Editor, { topic: 1 });
    await u.type(screen.getByLabelText("Question"), "What opens a TCP connection?");
    await u.click(screen.getByRole("radio", { name: "Short answer" }));
    await u.click(screen.getByRole("radio", { name: "Keywords" }));
    await u.type(screen.getByLabelText("Keyword 1"), "SYN");
    await u.click(screen.getByRole("button", { name: /Add keyword/ }));
    await u.type(screen.getByLabelText("Keyword 2"), "SYN-ACK");
    await u.type(screen.getByLabelText("Other ways to write keyword 2"), "synack, syn ack");
    await u.click(screen.getByRole("button", { name: /Add keyword/ }));
    await u.type(screen.getByLabelText("Keyword 3"), "ACK");
    await u.selectOptions(screen.getByLabelText("How many keywords are needed"), "At least 2");
    await u.click(screen.getByRole("button", { name: /Save question/ }));
    expect(mocked.saveQuestion.mock.calls[0][0].short).toEqual({
      check: { Keywords: { keywords: [{ word: "SYN", also: [] }, { word: "SYN-ACK", also: ["synack", "syn ack"] }, { word: "ACK", also: [] }], needed: 2 } },
      model_answer: "",
    });
  });

  // short-answer.feature: Try an answer before saving
  it("tries an answer against what's written so far", async () => {
    const u = user();
    mocked.tryShortAnswer.mockResolvedValue({ right: true, detail: { Matched: { answer: "Transport layer", typo: "transprot" } } });
    render(Editor, { topic: 1 });
    await u.click(screen.getByRole("radio", { name: "Short answer" }));
    await u.type(screen.getByLabelText("Accepted answer 1"), "Transport layer");
    await u.type(screen.getByLabelText("A test answer"), "the transprot layer");
    const tryIt = screen.getByRole("region", { name: "Try an answer" });
    expect(await within(tryIt).findByText("Right, with a typo")).toBeInTheDocument();
    expect(tryIt).toHaveTextContent('matches "Transport layer"');
    expect(mocked.tryShortAnswer).toHaveBeenLastCalledWith({ Accepted: { answers: ["Transport layer"], typos: true } }, "the transprot layer");
    expect(mocked.saveQuestion).not.toHaveBeenCalled();
  });

  it("opens a short answer question for editing", async () => {
    const u = user();
    mocked.question.mockResolvedValue({
      topic: 1, text: "What opens a TCP connection?", multiple: false, options: [], explanation: "", tags: [], citations: [],
      short: { check: { Keywords: { keywords: [{ word: "SYN-ACK", also: ["synack"] }], needed: null } }, model_answer: "SYN, SYN-ACK, ACK" },
    });
    render(Editor, { topic: 1 });
    await u.click(await screen.findByRole("button", { name: "What does the ALU do?" }));
    expect(screen.getByRole("radio", { name: "Short answer" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "Keywords" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByLabelText("Keyword 1")).toHaveValue("SYN-ACK");
    expect(screen.getByLabelText("Other ways to write keyword 1")).toHaveValue("synack");
    expect(screen.getByLabelText(/Model answer/)).toHaveValue("SYN, SYN-ACK, ACK");
  });
});

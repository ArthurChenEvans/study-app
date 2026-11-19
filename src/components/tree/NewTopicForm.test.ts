// nested-modules.feature, "Default module" (R217). Test IDs: T-DEF-09 to T-DEF-11.
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/api", async (original) => {
  const actual = await original<typeof import("../../lib/api")>();
  return { ...actual, api: { createTopic: vi.fn() } };
});

import { api } from "../../lib/api";
import NewTopicForm from "./NewTopicForm.svelte";
import { background } from "./fixtures";

const mocked = vi.mocked(api);

async function createWithoutModule() {
  render(NewTopicForm, { tree: background().tree });
  await userEvent.type(screen.getByRole("textbox", { name: "Topic name" }), "Misc");
  await userEvent.click(screen.getByRole("button", { name: "Create topic" }));
}

describe("NewTopicForm", () => {
  beforeEach(() => {
    mocked.createTopic.mockReset();
    mocked.createTopic.mockResolvedValue(42);
  });

  it("T-DEF-09: no module shows the Default warning, word for word", async () => {
    await createWithoutModule();
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent("Creating this topic without selecting a module will put this under Default. Are you sure you want this?");
    expect(dialog).toHaveTextContent("Default is not the fundamental module. It is a root module like all your other first layer modules. It is just where topics with unnamed modules go. Your root modules are not children of default");
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "No" })).toBeInTheDocument();
    expect(mocked.createTopic).not.toHaveBeenCalled();
  });

  it("T-DEF-10: No keeps the typing and focuses the module picker", async () => {
    await createWithoutModule();
    await userEvent.click(screen.getByRole("button", { name: "No" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("textbox", { name: "Topic name" })).toHaveValue("Misc");
    expect(screen.getByRole("combobox", { name: "Module" })).toHaveFocus();
    expect(mocked.createTopic).not.toHaveBeenCalled();
  });

  it("T-DEF-11: Yes creates the topic with no module, which puts it in Default", async () => {
    const onCreated = vi.fn();
    render(NewTopicForm, { tree: background().tree, onCreated });
    await userEvent.type(screen.getByRole("textbox", { name: "Topic name" }), "Misc");
    await userEvent.click(screen.getByRole("button", { name: "Create topic" }));
    await userEvent.click(screen.getByRole("button", { name: "Yes" }));
    expect(mocked.createTopic).toHaveBeenCalledWith("Misc", null);
    await vi.waitFor(() => expect(onCreated).toHaveBeenCalledWith(42));
  });

  it("with a module picked, no warning", async () => {
    const { n2t, tree } = background();
    render(NewTopicForm, { tree });
    await userEvent.type(screen.getByRole("textbox", { name: "Topic name" }), "ALU");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Module" }), String(n2t.node.id));
    await userEvent.click(screen.getByRole("button", { name: "Create topic" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mocked.createTopic).toHaveBeenCalledWith("ALU", n2t.node.id);
  });

  it("the module picker shows full paths so namesakes are told apart", () => {
    render(NewTopicForm, { tree: background().tree });
    expect(screen.getByRole("option", { name: "Computer Architecture › N2T" })).toBeInTheDocument();
  });
});

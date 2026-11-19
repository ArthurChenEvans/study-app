// nested-modules.feature, "Graph view" (R219). Test IDs: T-GRAPH-01 to T-GRAPH-10.
// Nodes are buttons named by their full path, so namesakes are told apart (T-GRAPH-04).
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/api", async (original) => {
  const actual = await original<typeof import("../../lib/api")>();
  return { ...actual, api: { querySearch: vi.fn() } };
});

import { api, type SourceRow } from "../../lib/api";
import GraphView from "./GraphView.svelte";
import { background, large, pill, searched } from "./fixtures";

const mocked = vi.mocked(api);
const source = (id: number, title: string): SourceRow => ({ id, fields: { authors: "A", year: null, title, edition: null, description: null, publisher: null, url: null }, apa: title, used_by: [], cited_by_questions: 1 });
const sources = [source(1, "The Elements of Computing Systems"), source(2, "Computer Systems: A Programmer's Perspective")];
const nodes = () => document.querySelectorAll("[data-graph-node]");
const edges = (kind: string) => document.querySelectorAll(`[data-graph-edge="${kind}"]`);

describe("GraphView", () => {
  beforeEach(() => mocked.querySearch.mockReset());

  it("T-GRAPH-01: one node per module and topic, joined to its parent", () => {
    render(GraphView, { tree: background().tree, onOpen: vi.fn() });
    expect(nodes()).toHaveLength(6);
    expect(edges("parent")).toHaveLength(5);
  });

  it("T-GRAPH-02: Related draws dashed links", async () => {
    render(GraphView, { tree: background().tree, onOpen: vi.fn() });
    const related = screen.getByRole("checkbox", { name: "Related" });
    if ((related as HTMLInputElement).checked) await userEvent.click(related);
    expect(edges("related")).toHaveLength(0);
    await userEvent.click(related);
    expect(edges("related")).toHaveLength(1);
    expect(edges("related")[0].getAttribute("stroke-dasharray")).toBeTruthy();
  });

  it("T-GRAPH-03: Sources adds a node per source, linked to the topics citing it", async () => {
    render(GraphView, { tree: background().tree, sources, onOpen: vi.fn() });
    const toggle = screen.getByRole("checkbox", { name: "Sources" });
    if (!(toggle as HTMLInputElement).checked) await userEvent.click(toggle);
    expect(document.querySelectorAll("[data-graph-source]")).toHaveLength(2);
    expect(edges("source")).toHaveLength(2);
  });

  it("T-GRAPH-04: namesakes show their parent on hover and full path when selected", async () => {
    const { tree } = background();
    render(GraphView, { tree, onOpen: vi.fn() });
    const cpu = screen.getByRole("button", { name: "Computer Architecture › N2T › CPU" });
    expect(screen.getByRole("button", { name: "Computer Architecture › CS:APP › CPU" })).toBeInTheDocument();
    await userEvent.hover(cpu);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("N2T");
    await userEvent.click(cpu);
    expect(screen.getByRole("region", { name: "Selected" })).toHaveTextContent("Computer Architecture › N2T");
  });

  it("T-GRAPH-05: graph search highlights matches and dims the rest", async () => {
    const { tree, n2tCpu, csappCpu } = background();
    mocked.querySearch.mockResolvedValue(searched([pill("CPU")], { paths: [[n2tCpu.node, n2tCpu.breadcrumb], [csappCpu.node, csappCpu.breadcrumb]] }));
    render(GraphView, { tree, onOpen: vi.fn() });
    await userEvent.type(screen.getByRole("textbox", { name: "Search the graph" }), "CPU{Enter}");
    await vi.waitFor(() => expect(document.querySelectorAll("[data-graph-node][data-match='true']")).toHaveLength(2));
    expect(document.querySelectorAll("[data-graph-node][data-dimmed='true']")).toHaveLength(4);
    expect(screen.getByText("2 matches")).toBeInTheDocument();
  });

  it("T-GRAPH-06: double-clicking a topic opens it", async () => {
    const { tree, n2tCpu } = background();
    const onOpen = vi.fn();
    render(GraphView, { tree, onOpen });
    await userEvent.dblClick(screen.getByRole("button", { name: "Computer Architecture › N2T › CPU" }));
    expect(onOpen).toHaveBeenCalledWith(n2tCpu.node);
  });

  it("T-GRAPH-07: arrow keys move along links and Enter opens", async () => {
    const { tree, ca, n2t } = background();
    const onOpen = vi.fn();
    render(GraphView, { tree, onOpen });
    screen.getByRole("button", { name: "Computer Architecture" }).focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("button", { name: "Computer Architecture › N2T" })).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    expect(screen.getByRole("button", { name: "Computer Architecture" })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}{Enter}");
    expect(onOpen).toHaveBeenCalledWith(n2t.node);
    void ca;
  });

  it("T-GRAPH-08: Show as list gives the same nodes as an indented tree", async () => {
    render(GraphView, { tree: background().tree, onOpen: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Show as list" }));
    const tree = screen.getByRole("tree");
    expect(tree.querySelectorAll("[role='treeitem']")).toHaveLength(6);
  });

  it("T-GRAPH-09: a big library starts with modules collapsed and renders quickly", async () => {
    const started = performance.now();
    render(GraphView, { tree: large(10, 500), onOpen: vi.fn() });
    expect(performance.now() - started).toBeLessThan(1000);
    expect(nodes()).toHaveLength(10);
    await fireEvent.click(screen.getByRole("button", { name: "Expand Module 1" }));
    expect(nodes()).toHaveLength(10 + 50);
  });

  it("T-GRAPH-10: an empty graph says what to do next", async () => {
    const onCreateModule = vi.fn();
    render(GraphView, { tree: { nodes: [] }, onOpen: vi.fn(), onCreateModule });
    expect(screen.getByText("No modules yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Create a module" }));
    expect(onCreateModule).toHaveBeenCalled();
  });
});

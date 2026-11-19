// Shared test data for the nested modules and query search components (R217 to R220).
import type { Part, QuerySearch, TreeNode, TreeView } from "../../lib/api";

export const pill = (text: string, negated = false): Part => ({ Pill: { text, kind: "Term", negated } });
export const scope = (text: string): Part => ({ Pill: { text, kind: { Scope: "Module" }, negated: false } });
export const op = (word: "AND" | "OR" | "XOR" | "NOT"): Part => ({ Op: word });
export const group = (outline: "None" | "Solid" | "Dashed", parts: Part[], brackets: "None" | "Typed" | "Faint" = "None", negated = false): Part => ({ Group: { outline, brackets, negated, parts } });

export function searched(parts: Part[], extra: Partial<QuerySearch> = {}): QuerySearch {
  return { parts, error: null, results: { groups: {}, children: {} }, paths: [], unknown: [], scope_paths: [], spoken: "", items: [], ...extra };
}

export function broken(message: string, start = 0, end = 1): QuerySearch {
  return searched([], { error: { message, start, end } });
}

let next = 100;
function node(kind: "Module" | "Topic", name: string, parent: number | null, path: string[], extra: Partial<TreeNode> = {}): TreeNode {
  const id = next++;
  return { node: { kind, id }, name, parent, path, breadcrumb: path.join(" › "), is_default: false, due: 0, question_count: 0, card_count: 0, sources: [], related: [], ...extra };
}

/** The Background: Computer Architecture › N2T, CS:APP, UCM CySec CA Module, with a CPU in N2T and CS:APP. */
export function background(): { tree: TreeView; ca: TreeNode; n2t: TreeNode; csapp: TreeNode; ucm: TreeNode; n2tCpu: TreeNode; csappCpu: TreeNode } {
  const ca = node("Module", "Computer Architecture", null, ["Computer Architecture"]);
  const n2t = node("Module", "N2T", ca.node.id, ["Computer Architecture", "N2T"]);
  const csapp = node("Module", "CS:APP", ca.node.id, ["Computer Architecture", "CS:APP"]);
  const ucm = node("Module", "UCM CySec CA Module", ca.node.id, ["Computer Architecture", "UCM CySec CA Module"]);
  const n2tCpu = node("Topic", "CPU", n2t.node.id, ["Computer Architecture", "N2T", "CPU"], { sources: [1] });
  const csappCpu = node("Topic", "CPU", csapp.node.id, ["Computer Architecture", "CS:APP", "CPU"], { sources: [2] });
  n2tCpu.related = [csappCpu.node.id];
  csappCpu.related = [n2tCpu.node.id];
  return { tree: { nodes: [ca, n2t, csapp, ucm, n2tCpu, csappCpu] }, ca, n2t, csapp, ucm, n2tCpu, csappCpu };
}

/** A big library: `roots` root modules, each with topics, `topics` topics in all. */
export function large(roots: number, topics: number): TreeView {
  const nodes: TreeNode[] = [];
  const modules = Array.from({ length: roots }, (_, index) => node("Module", `Module ${index + 1}`, null, [`Module ${index + 1}`]));
  nodes.push(...modules);
  for (let index = 0; index < topics; index++) {
    const parent = modules[index % roots];
    nodes.push(node("Topic", `Topic ${index + 1}`, parent.node.id, [...parent.path, `Topic ${index + 1}`]));
  }
  return { nodes };
}

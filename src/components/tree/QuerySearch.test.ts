// search-query.feature, "Results are grouped by kind", "A broken query keeps the last good
// results", "No results" (R220). Test IDs: T-EVAL-15 (screen half), T-PILL-21, T-PILL-26.
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/api", async (original) => {
  const actual = await original<typeof import("../../lib/api")>();
  return { ...actual, api: { querySearch: vi.fn() } };
});

import { api } from "../../lib/api";
import QuerySearch from "./QuerySearch.svelte";
import { broken, pill, searched } from "./fixtures";

const mocked = vi.mocked(api);
const box = () => screen.getByRole("searchbox", { name: "Search" });
const found = searched([pill("CPU")], {
  results: { groups: { Topic: [10, 11], Question: [20] }, children: {} },
  paths: [[{ kind: "Topic", id: 10 }, "Computer Architecture › N2T › CPU"], [{ kind: "Topic", id: 11 }, "Computer Architecture › CS:APP › CPU"]],
});

describe("QuerySearch", () => {
  beforeEach(() => mocked.querySearch.mockReset());

  it("T-EVAL-15: results are grouped by kind with counts, namesakes shown by path", async () => {
    mocked.querySearch.mockResolvedValue(found);
    render(QuerySearch, { value: "CPU" });
    expect(await screen.findByRole("tab", { name: /Topics\s*2/ })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Questions\s*1/ })).toBeInTheDocument();
    expect(screen.getByText("Computer Architecture › N2T › CPU")).toBeInTheDocument();
    expect(screen.getByText("Computer Architecture › CS:APP › CPU")).toBeInTheDocument();
  });

  it("T-PILL-21: a broken query keeps the last good results", async () => {
    mocked.querySearch.mockResolvedValueOnce(found).mockResolvedValue(broken("$ needs something on its right"));
    render(QuerySearch, { value: "CPU" });
    await screen.findByText("Computer Architecture › N2T › CPU");
    await userEvent.type(box(), "${Enter}");
    await screen.findByRole("alert");
    expect(screen.getByText("Computer Architecture › N2T › CPU")).toBeInTheDocument();
  });

  it("T-PILL-26: no results says so and offers to clear", async () => {
    mocked.querySearch.mockResolvedValue(searched([pill("zzz")]));
    render(QuerySearch, { value: "zzz" });
    expect(await screen.findByText("Nothing matches zzz")).toBeInTheDocument();
    mocked.querySearch.mockResolvedValue(searched([]));
    await userEvent.click(screen.getByRole("button", { name: "Clear search" }));
    expect(mocked.querySearch).toHaveBeenLastCalledWith("");
  });

  it("T-EVAL-13: an unknown scope offers the closest name", async () => {
    mocked.querySearch.mockResolvedValue(searched([], { unknown: [{ kind: "Module", value: "n2", suggestion: "N2T" }] }));
    render(QuerySearch, { value: "module:n2" });
    expect(await screen.findByText(/No module called n2/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "N2T" })).toBeInTheDocument();
  });
});

import { describe, expect, it } from "vitest";
import { stackTable } from "./stack";

// navigation.feature (R162): on phones every list table is a stack of cards, each value with
// its column's name in front. The names come from the table's own headers.
function table(rows: string[][]) {
  const element = document.createElement("table");
  element.innerHTML = `<thead><tr><th>Checklist</th><th>Points</th><th class="text-right">Last check</th><th><span class="sr-only">Open</span></th></tr></thead>
    <tbody>${rows.map((row) => `<tr>${row.map((cell) => `<td>${cell}</td>`).join("")}</tr>`).join("")}</tbody>`;
  document.body.append(element);
  return element;
}
const labels = (element: HTMLTableElement) => [...element.querySelectorAll("tbody tr")].map((row) => [...row.children].map((cell) => cell.getAttribute("data-label")));
const tick = () => new Promise((resolve) => setTimeout(resolve));

describe("stackTable", () => {
  it("names each value by its column, leaving the first column and unnamed ones bare", () => {
    const element = table([["Caches", "11", "91%", "<button>Open</button>"]]);
    stackTable(element);
    expect(element).toHaveClass("stack-table");
    expect(labels(element)).toEqual([[null, "Points", "Last check", null]]);
  });

  it("keeps a label a cell already has", () => {
    const element = table([["Caches", "11", "91%", ""]]);
    element.querySelector("tbody td:nth-child(2)")!.setAttribute("data-label", "Points to check");
    stackTable(element);
    expect(labels(element)[0][1]).toBe("Points to check");
  });

  it("labels rows that turn up later, and stops when the table goes", async () => {
    const element = table([]);
    const action = stackTable(element);
    element.querySelector("tbody")!.insertAdjacentHTML("beforeend", "<tr><td>Paging</td><td>9</td><td>70%</td><td></td></tr>");
    await tick();
    expect(labels(element)).toEqual([[null, "Points", "Last check", null]]);
    action.destroy();
    element.querySelector("tbody")!.insertAdjacentHTML("beforeend", "<tr><td>Heaps</td><td>3</td><td>50%</td><td></td></tr>");
    await tick();
    expect(labels(element)[1]).toEqual([null, null, null, null]);
  });
});

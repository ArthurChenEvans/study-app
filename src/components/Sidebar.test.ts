import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Sidebar from "./Sidebar.svelte";

describe("Sidebar", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.dataset.theme = "studyapp-dark";
  });

  it("marks the current screen", () => {
    render(Sidebar, { current: "topics", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: "Topics" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "Overview" })).not.toHaveAttribute("aria-current");
  });

  it("navigates", async () => {
    const onNavigate = vi.fn();
    render(Sidebar, { current: "overview", onNavigate });
    await userEvent.click(screen.getByRole("link", { name: "History" }));
    expect(onNavigate).toHaveBeenCalledWith("history");
  });

  // appearance.feature: Switch to light
  it("switches theme", async () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("aria-checked", "true");
    await userEvent.click(screen.getByRole("radio", { name: "Light" }));
    expect(document.documentElement.dataset.theme).toBe("studyapp");
    expect(screen.getByRole("radio", { name: "Light" })).toHaveAttribute("aria-checked", "true");
  });

  // free-recall.feature: Due checklists come first (the sidebar shows how many are due)
  it("shows how many checklists are due", () => {
    render(Sidebar, { current: "recall", onNavigate: vi.fn(), due: { recall: 2 } });
    const link = screen.getByRole("link", { name: /Recall/ });
    expect(link).toHaveAttribute("aria-current", "page");
    expect(screen.getByLabelText("2 due today")).toBeInTheDocument();
  });

  it("hides the count when nothing is due", () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn(), due: { recall: 0 } });
    expect(screen.queryByLabelText(/due today/)).toBeNull();
  });

  // flashcards.feature: See what's due today (the sidebar shows how many cards are due)
  it("shows how many cards are due next to Cards", () => {
    render(Sidebar, { current: "cards", onNavigate: vi.fn(), due: { recall: 1, cards: 27 } });
    const link = screen.getByRole("link", { name: /Cards/ });
    expect(link).toHaveAttribute("aria-current", "page");
    expect(link).toHaveTextContent("27");
    expect(screen.getByRole("link", { name: /Recall/ })).toHaveTextContent("1");
  });

  // calendar.feature (R100): today's total by Calendar, quizzes by Topics, drills by Projects
  it("shows today's counts by Calendar, Topics and Projects", () => {
    render(Sidebar, { current: "calendar", onNavigate: vi.fn(), due: { calendar: 17, topics: 2, projects: 1 } });
    expect(screen.getByRole("link", { name: /Calendar/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: /Calendar/ })).toHaveTextContent("17");
    expect(screen.getByRole("link", { name: /Topics/ })).toHaveTextContent("2");
    expect(screen.getByRole("link", { name: /Projects/ })).toHaveTextContent("1");
    const names = screen.getAllByRole("link").map((link) => link.textContent?.replace(/\d+/g, "").trim());
    expect(names.slice(0, 6)).toEqual(["Overview", "Calendar", "Topics", "Cards", "Recall", "Projects"]);
  });

  // UI polish: a count says what it counts, out loud and on hover
  it("says the Projects count is drills", () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn(), due: { projects: 1, topics: 2 } });
    expect(screen.getByLabelText("1 drill due today")).toHaveAttribute("title", "1 drill due today");
    expect(screen.getByLabelText("2 due today")).toHaveAttribute("title", "2 due today");
  });

  it("shows the search shortcut", () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: /Search/ })).toHaveTextContent("/");
  });

  it("marks New quiz as the current page on the setup screen", () => {
    render(Sidebar, { current: "setup", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: "New quiz" })).toHaveAttribute("aria-current", "page");
  });

  // UI polish: sounds can be turned off, and the choice is remembered on this device
  it("switches sound off and remembers it", async () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    const toggle = screen.getByRole("switch", { name: "Sound" });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(toggle).not.toBeChecked();
    expect(localStorage.getItem("studyapp-sound")).toBe("off");
  });
});

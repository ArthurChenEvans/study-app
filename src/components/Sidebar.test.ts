import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Sidebar from "./Sidebar.svelte";

describe("Sidebar", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.dataset.theme = "studyapp-dark";
  });

  it("marks the current screen", () => {
    render(Sidebar, { current: "quizzes", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: "Quizzes" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "Overview" })).not.toHaveAttribute("aria-current");
  });

  it("navigates", async () => {
    const onNavigate = vi.fn();
    render(Sidebar, { current: "overview", onNavigate });
    await userEvent.click(screen.getByRole("link", { name: "Calendar" }));
    expect(onNavigate).toHaveBeenCalledWith("calendar");
  });

  // free-recall.feature: Due checklists come first (the sidebar shows how many are due)
  it("shows how many checklists are due", () => {
    render(Sidebar, { current: "recall", onNavigate: vi.fn(), due: { recall: 2 } });
    const link = screen.getByRole("link", { name: /Free Recall/ });
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
    expect(screen.getByRole("link", { name: /Free Recall/ })).toHaveTextContent("1");
  });

  // calendar.feature (R100): today's total by Calendar, quizzes by Quizzes, drills by Practice Log
  it("shows today's counts by Calendar, Quizzes and Practice Log", () => {
    render(Sidebar, { current: "calendar", onNavigate: vi.fn(), due: { calendar: 17, quizzes: 2, projects: 1 } });
    expect(screen.getByRole("link", { name: /Calendar/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: /Calendar/ })).toHaveTextContent("17");
    expect(screen.getByRole("link", { name: /Quizzes/ })).toHaveTextContent("2");
    expect(screen.getByRole("link", { name: /Practice Log/ })).toHaveTextContent("1");
    const names = screen.getAllByRole("link").map((link) => link.textContent?.replace(/\d+/g, "").trim()).filter((name) => !name?.startsWith("Search"));
    expect(names.slice(0, 6)).toEqual(["Overview", "Calendar", "Quizzes", "Cards", "Free Recall", "Practice Log"]);
  });

  // UI polish: a count says what it counts, out loud and on hover
  it("says the Projects count is drills", () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn(), due: { projects: 1, quizzes: 2 } });
    expect(screen.getByLabelText("1 drill due today")).toHaveAttribute("title", "1 drill due today");
    expect(screen.getByLabelText("2 due today")).toHaveAttribute("title", "2 due today");
  });

  it("shows the search shortcut", () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: /Search/ })).toHaveTextContent("/");
  });

  // R189: starting a quiz is "Start a quiz", so it doesn't clash with making a new one on Quizzes
  it("marks Start a quiz as the current page on the setup screen", () => {
    render(Sidebar, { current: "setup", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: "Start a quiz" })).toHaveAttribute("aria-current", "page");
  });

  // backup.feature (R108)
  it("shows the backup status in one line and links it to Your data", async () => {
    const onNavigate = vi.fn();
    const { rerender } = render(Sidebar, { current: "overview", onNavigate, backup: { safe: true, short: "Backed up today" } });
    const line = screen.getByRole("link", { name: "Backed up today" });
    await userEvent.click(line);
    expect(onNavigate).toHaveBeenCalledWith("data");
    await rerender({ current: "overview", onNavigate, backup: { safe: false, short: "No backup yet" } });
    expect(screen.getByRole("link", { name: "No backup yet" })).toHaveClass("font-extrabold");
  });

  // sync.feature (R128): with sync on, the line says how sync is going, unless the backup needs attention
  it("shows the sync status in the same line once sync is on", async () => {
    const onNavigate = vi.fn();
    const { rerender } = render(Sidebar, { current: "overview", onNavigate, backup: { safe: true, short: "Backed up today" }, sync: { ok: true, short: "Synced 2 minutes ago" } });
    expect(screen.getByRole("link", { name: "Synced 2 minutes ago" })).toHaveClass("text-muted");
    expect(screen.queryByRole("link", { name: "Backed up today" })).not.toBeInTheDocument();
    await rerender({ current: "overview", onNavigate, backup: { safe: true, short: "Backed up today" }, sync: { ok: false, short: "Synced, 1 to check" } });
    expect(screen.getByRole("link", { name: "Synced, 1 to check" })).toHaveClass("font-extrabold");
    await rerender({ current: "overview", onNavigate, backup: { safe: false, short: "No backup yet" }, sync: { ok: true, short: "Synced just now" } });
    expect(screen.getByRole("link", { name: "No backup yet" })).toBeInTheDocument();
  });

  it("leaves the backup line out until the status has loaded", () => {
    render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    expect(screen.queryByRole("link", { name: /Backed up|No backup/ })).not.toBeInTheDocument();
  });

  // navigation.feature (R158, R189): links come in three groups, and preferences live in Settings
  it("groups the links and keeps preferences out of the sidebar", () => {
    render(Sidebar, { current: "transfer", onNavigate: vi.fn() });
    const groups = screen.getAllByRole("group").map((group) => [group.getAttribute("aria-label"), within(group).queryAllByRole("link").map((link) => link.textContent?.trim())]);
    expect(groups).toEqual([["Today", ["Overview", "Calendar"]], ["Study", ["Quizzes", "Cards", "Free Recall", "Practice Log"]], ["Manage", ["Topics & Modules", "Import & Export", "Bibliography", "History"]]]);
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
    expect(screen.queryByRole("radiogroup", { name: "Theme" })).not.toBeInTheDocument();
  });

  // navigation.feature (R189): each group folds from its heading, remembered. Manage starts
  // folded unless you're on one of its pages.
  it("folds a group from its heading and remembers it", async () => {
    const u = userEvent.setup();
    const { unmount } = render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    const manage = screen.getByRole("button", { name: /Manage/ });
    expect(manage).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("link", { name: "Bibliography" })).not.toBeInTheDocument();
    await u.click(manage);
    expect(manage).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("link", { name: "Bibliography" })).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: /Study/ }));
    expect(screen.queryByRole("link", { name: "Quizzes" })).not.toBeInTheDocument();
    unmount();
    render(Sidebar, { current: "overview", onNavigate: vi.fn() });
    expect(screen.getByRole("button", { name: /Study/ })).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByRole("button", { name: /Manage/ })).toHaveAttribute("aria-expanded", "true");
  });

  it("opens a folded group that holds the page you're on, and still shows its counts folded", () => {
    localStorage.setItem("studyapp-nav-folded", JSON.stringify(["Study"]));
    const { unmount } = render(Sidebar, { current: "cards", onNavigate: vi.fn() });
    expect(screen.getByRole("link", { name: "Cards" })).toHaveAttribute("aria-current", "page");
    unmount();
    render(Sidebar, { current: "overview", onNavigate: vi.fn(), due: { cards: 4, recall: 2 } });
    expect(screen.getByRole("button", { name: /Study/ })).toHaveAccessibleName("Study, 6 due today");
  });

  it("opens Settings and Your data from the bottom", async () => {
    const onNavigate = vi.fn();
    render(Sidebar, { current: "settings", onNavigate });
    const settings = screen.getByRole("link", { name: "Settings" });
    expect(settings).toHaveAttribute("aria-current", "page");
    await userEvent.click(settings);
    expect(onNavigate).toHaveBeenCalledWith("settings");
  });

  // navigation.feature (R159): a tablet gets an icon rail, with the library under More
  it("shrinks to a rail with the rest under More", async () => {
    const onNavigate = vi.fn();
    render(Sidebar, { current: "quizzes", onNavigate, mode: "rail", due: { cards: 13 } });
    const nav = screen.getByRole("navigation", { name: "Main" });
    expect(within(nav).getAllByRole("link").map((link) => link.getAttribute("aria-label") ?? link.textContent?.trim())).toEqual(["Search", "Overview", "Calendar", "Quizzes", "Cards, 13 due today", "Free Recall", "Practice Log", "Settings"]);
    await userEvent.click(within(nav).getByRole("button", { name: "More" }));
    const sheet = screen.getByRole("dialog", { name: "More" });
    await userEvent.click(within(sheet).getByRole("link", { name: "Bibliography" }));
    expect(onNavigate).toHaveBeenCalledWith("bibliography");
    expect(screen.queryByRole("dialog", { name: "More" })).not.toBeInTheDocument();
  });

  // navigation.feature (R160): a phone gets five tabs with More in the middle
  it("becomes a dock of five on a phone, More in the middle", async () => {
    const onNavigate = vi.fn();
    render(Sidebar, { current: "cards", onNavigate, mode: "dock", due: { calendar: 16, cards: 13, recall: 1 } });
    const dock = screen.getByRole("navigation", { name: "Main" });
    const items = [...dock.querySelectorAll("a, button")].map((item) => item.textContent?.replace(/\d+/g, "").trim());
    expect(items).toEqual(["Today", "Quizzes", "More", "Cards", "Free Recall"]);
    expect(within(dock).getByRole("link", { name: /Cards/ })).toHaveAttribute("aria-current", "page");
    await userEvent.click(within(dock).getByRole("button", { name: "More" }));
    const sheet = screen.getByRole("dialog", { name: "More" });
    expect(within(sheet).getAllByRole("link").map((link) => link.textContent?.replace(/\d+/g, "").trim())).toEqual(["Calendar", "Practice Log", "Topics & Modules", "Import & Export", "Bibliography", "History", "Search", "Start a quiz", "Your data", "Settings"]);
    expect(within(sheet).getByLabelText("16 due today")).toBeInTheDocument();
    await userEvent.click(within(sheet).getByRole("link", { name: /Import & Export/ }));
    expect(onNavigate).toHaveBeenCalledWith("transfer");
  });
});

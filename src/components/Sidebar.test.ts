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
});

// Hazard 16: a link in question text or a book used to navigate the app's own window
// away, with no way back and the running quiz gone.
import { afterEach, describe, expect, it, vi } from "vitest";
import { safeLinks } from "./links";

function page(html: string) {
  const root = document.createElement("div");
  root.innerHTML = html;
  document.body.append(root);
  const action = safeLinks(root);
  return { root, done: () => { action.destroy(); root.remove(); } };
}
const click = (element: Element) => {
  const event = new MouseEvent("click", { bubbles: true, cancelable: true });
  element.dispatchEvent(event);
  return event;
};

describe("links in rendered text", () => {
  afterEach(() => { vi.restoreAllMocks(); delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__; });

  it("never navigate the app's window", () => {
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    const { root, done } = page(`<p><a href="https://example.com/a">out</a> <a href="chapter2.xhtml#x">inside</a> <a href="mailto:a@b.c">mail</a> <a href="asset://localhost/etc/passwd">file</a> <a>bare</a> <a href="https://example.com/b"><strong>nested</strong></a></p>`);
    for (const link of root.querySelectorAll("a")) expect(click(link).defaultPrevented, link.outerHTML).toBe(true);
    expect(click(root.querySelector("strong")!).defaultPrevented).toBe(true);
    expect(open.mock.calls.map((call) => call[0])).toEqual(["https://example.com/a", "https://example.com/b", "https://example.com/b"]);
    for (const call of open.mock.calls) expect(call.slice(1)).toEqual(["_blank", "noopener,noreferrer"]);
    done();
  });

  it("open nothing inside the desktop app, where a new window would be the app's own", () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    const { root, done } = page(`<a href="https://example.com/a">out</a>`);
    expect(click(root.querySelector("a")!).defaultPrevented).toBe(true);
    expect(open).not.toHaveBeenCalled();
    done();
  });

  it("leave other clicks alone", () => {
    const { root, done } = page(`<p>text <code>code</code></p>`);
    expect(click(root.querySelector("code")!).defaultPrevented).toBe(false);
    done();
  });
});

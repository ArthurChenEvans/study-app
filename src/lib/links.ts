// Links inside rendered text (questions, cards, books). Followed normally, a link would
// navigate the app's own window to the site: the app is gone, there's no back button in
// the desktop app, and a quiz in progress is interrupted. So a click never navigates.
// In the browser version an http(s) link opens in a new tab. In the desktop app nothing
// opens (a new window there would still be the app's own), and the address shows on
// hover so it can be read.
import { inDesktopApp } from "./transport";

export function safeLinks(node: HTMLElement) {
  const onClick = (event: MouseEvent) => {
    const target = event.target as Element | null;
    const link = target?.closest?.("a");
    if (!link || !node.contains(link)) return;
    event.preventDefault();
    const href = link.getAttribute("href") ?? "";
    if (/^https?:\/\//i.test(href) && !inDesktopApp()) window.open(href, "_blank", "noopener,noreferrer");
  };
  node.addEventListener("click", onClick);
  node.addEventListener("auxclick", onClick);
  return {
    destroy() {
      node.removeEventListener("click", onClick);
      node.removeEventListener("auxclick", onClick);
    },
  };
}

// R199: right-click, long-press, Shift+F10 or the Menu key open a row's own menu, so the
// actions behind each row's "…" are a click closer. One host draws the menu for every list.
import type { MenuItem } from "../components/RowMenu.svelte";

export type RowMenuSpec = { label: string; items: MenuItem[] };

export const contextMenu = $state<{ open: boolean; x: number; y: number; label: string; items: MenuItem[]; from: HTMLElement | null }>({ open: false, x: 0, y: 0, label: "", items: [], from: null });

export function openContextMenu(spec: RowMenuSpec, x: number, y: number, from: HTMLElement | null) {
  if (!spec.items.length) return;
  Object.assign(contextMenu, { open: true, x, y, label: spec.label, items: spec.items, from });
}

export function closeContextMenu(refocus = true) {
  const from = contextMenu.from;
  contextMenu.open = false;
  contextMenu.from = null;
  if (refocus) from?.focus();
}

const LONG_PRESS = 500;

/** Svelte action: `use:rowMenu={() => ({ label, items })}`. */
export function rowMenu(node: HTMLElement, spec: () => RowMenuSpec) {
  let current = spec;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pressedOpen = false;
  const typing = (target: EventTarget | null) => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) && (target as HTMLInputElement).type !== "checkbox");

  function onContext(event: MouseEvent) {
    if (typing(event.target)) return;
    event.preventDefault();
    if (pressedOpen) return;
    // A keyboard-made contextmenu event comes at 0,0: put the menu by the row instead.
    if (event.clientX === 0 && event.clientY === 0) return atRow();
    openContextMenu(current(), event.clientX, event.clientY, node);
  }
  function atRow() {
    const box = node.getBoundingClientRect();
    openContextMenu(current(), box.left + 24, box.bottom, node);
  }
  function onKey(event: KeyboardEvent) {
    if (typing(event.target)) return;
    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
      event.preventDefault();
      atRow();
    }
  }
  function cancel() {
    if (timer) clearTimeout(timer);
    timer = null;
  }
  function onDown(event: PointerEvent) {
    pressedOpen = false;
    if (event.pointerType !== "touch") return;
    const { clientX, clientY } = event;
    cancel();
    timer = setTimeout(() => {
      pressedOpen = true;
      openContextMenu(current(), clientX, clientY, node);
    }, LONG_PRESS);
  }
  function onUp(event: PointerEvent) {
    cancel();
    // The tap that ends a long press shouldn't also press whatever is under the finger.
    if (pressedOpen) event.preventDefault();
  }
  function onClick(event: MouseEvent) {
    if (pressedOpen) { event.preventDefault(); event.stopPropagation(); pressedOpen = false; }
  }

  node.addEventListener("contextmenu", onContext);
  node.addEventListener("keydown", onKey);
  node.addEventListener("pointerdown", onDown);
  node.addEventListener("pointerup", onUp);
  node.addEventListener("pointercancel", cancel);
  node.addEventListener("pointermove", cancel);
  node.addEventListener("click", onClick, true);
  return {
    update(next: () => RowMenuSpec) { current = next; },
    destroy() {
      cancel();
      node.removeEventListener("contextmenu", onContext);
      node.removeEventListener("keydown", onKey);
      node.removeEventListener("pointerdown", onDown);
      node.removeEventListener("pointerup", onUp);
      node.removeEventListener("pointercancel", cancel);
      node.removeEventListener("pointermove", cancel);
      node.removeEventListener("click", onClick, true);
    },
  };
}

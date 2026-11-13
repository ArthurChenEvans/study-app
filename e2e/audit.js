
  const canvas = document.createElement("canvas"); canvas.width = canvas.height = 1;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  const rgba = (c) => { ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = "#000"; ctx.fillStyle = c; ctx.fillRect(0, 0, 1, 1); const d = ctx.getImageData(0, 0, 1, 1).data; return [d[0], d[1], d[2], d[3] / 255]; };
  const lum = ([r, g, b]) => { const f = v => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
  const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  const bgOf = (el) => {
    const layers = [];
    for (let n = el; n; n = n.parentElement) {
      const cs = getComputedStyle(n);
      if (cs.backgroundImage.includes("gradient")) return null; // hatch fills checked by hand
      const c = rgba(cs.backgroundColor); if (c[3] > 0) layers.push(c); if (c[3] >= 1) break;
    }
    let out = [255, 255, 255];
    for (const c of layers.reverse()) out = out.map((v, i) => v * (1 - c[3]) + c[i] * c[3]);
    return out;
  };
  const visible = (el) => { const r = el.getBoundingClientRect(); const cs = getComputedStyle(el); return r.width > 0 && r.height > 0 && cs.visibility !== "hidden" && cs.display !== "none" && !el.closest("[aria-label='Prototype state'], .katex, [hidden], dialog:not([open])"); };
  const fails = [];
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const seen = new Set();
  while (walker.nextNode()) {
    const t = walker.currentNode; if (!t.textContent.trim()) continue;
    const el = t.parentElement; if (seen.has(el) || !visible(el)) continue; seen.add(el);
    if (el.closest("dialog[open]") === null && document.querySelector("dialog[open]")) continue; // behind the dialog scrim
    const cs = getComputedStyle(el);
    let fg = rgba(cs.color); const bg = bgOf(el); if (!bg) continue;
    const op = [...(function* () { for (let n = el; n; n = n.parentElement) yield Number(getComputedStyle(n).opacity); })()].reduce((a, b) => a * b, 1);
    fg = fg.slice(0, 3).map((v, i) => v * fg[3] * op + bg[i] * (1 - fg[3] * op));
    const size = parseFloat(cs.fontSize), bold = Number(cs.fontWeight) >= 600;
    const need = size >= 24 || (size >= 18.66 && bold) ? 3 : 4.5;
    const disabled = el.closest("button:disabled, [disabled], .opacity-60");
    const r = ratio(fg, bg);
    if (r < need && !disabled) fails.push(`${r.toFixed(2)} < ${need}: "${t.textContent.trim().slice(0, 40)}"`);
  }
  const small = [...document.querySelectorAll("button, a, [role=checkbox], input, select")].filter(visible)
    // A control inside a label is clicked through the label, so measure that instead.
    .filter(b => { const target = b.matches("input") && b.closest("label") ? b.closest("label") : b; const r = target.getBoundingClientRect(); return r.height < 24 || r.width < 24; })
    .map(b => `${Math.round(b.getBoundingClientRect().width)}x${Math.round(b.getBoundingClientRect().height)} "${(b.textContent || b.getAttribute("aria-label") || b.tagName).trim().slice(0, 30)}"`);
  const scope = document.querySelector("dialog[open]") || document.body;
  const primaries = [...scope.querySelectorAll(".btn-primary")].filter(visible).length;
  return { fails, small, primaries };
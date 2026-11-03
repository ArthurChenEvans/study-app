<script lang="ts">
  // Score per quiz (R33, design rules section 9): one 2px line, area wash, filled dots with a
  // surface ring, hollow dots for retries, only the latest point labelled. Hover, arrow keys
  // and a table view make every value reachable without the graphic.
  type Point = { label: string; percent: number; retry: boolean; detail: string };
  let { points, width: fixedWidth, height = 220, detailLabel = "Quiz" }: { points: Point[]; width?: number; height?: number; detailLabel?: string } = $props();

  let measured = $state(0);
  const width = $derived(fixedWidth ?? (measured || 640));
  let selected = $state<number | null>(null);

  const margin = { left: 40, right: 48, top: 16, bottom: 28 };
  const x = (index: number) => margin.left + (points.length === 1 ? (width - margin.left - margin.right) / 2 : (index * (width - margin.left - margin.right)) / (points.length - 1));
  const y = (percent: number) => margin.top + (1 - percent / 100) * (height - margin.top - margin.bottom);
  const line = $derived(points.map((point, index) => `${index ? "L" : "M"}${x(index).toFixed(1)},${y(point.percent).toFixed(1)}`).join(""));
  const area = $derived(points.length ? `${line}L${x(points.length - 1)},${height - margin.bottom}L${x(0)},${height - margin.bottom}Z` : "");
  const labelEvery = $derived(Math.max(1, Math.ceil(points.length / Math.max(1, Math.floor(width / 90)))));
  const describe = (point: Point) => `${point.label}: ${point.percent}%${point.retry ? " (retry)" : ""} · ${point.detail}`;
  const summary = $derived(points.length ? `Score per quiz, from ${points[0].percent}% to ${points.at(-1)!.percent}%` : "No scores yet");

  function onKey(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const start = selected ?? points.length - 1;
    selected = Math.max(0, Math.min(points.length - 1, start + (event.key === "ArrowRight" ? 1 : -1)));
  }

  function onMove(event: MouseEvent) {
    const left = (event.currentTarget as SVGElement).getBoundingClientRect().left;
    const offset = event.clientX - left;
    let best = 0;
    points.forEach((_, index) => {
      if (Math.abs(x(index) - offset) < Math.abs(x(best) - offset)) best = index;
    });
    selected = best;
  }
</script>

<div bind:clientWidth={measured} class="relative">
  {#if width > margin.left + margin.right}
    <!-- The chart is an image to screen readers, but focusable so the arrow keys can step
         through points (read out by the status line below). Design rules section 9. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <svg
      {width}
      {height}
      role="img"
      aria-label={summary}
      tabindex="0"
      class="focus-visible:outline-2"
      onkeydown={onKey}
      onmousemove={onMove}
      onmouseleave={() => (selected = null)}
      onblur={() => (selected = null)}
    >
      {#each [0, 50, 100] as tick (tick)}
        <line x1={margin.left} x2={width - margin.right} y1={y(tick)} y2={y(tick)} stroke="var(--chart-grid)" stroke-width="1" />
        <text x={margin.left - 8} y={y(tick) + 4} text-anchor="end" font-size="11" fill="var(--muted)" class="tabular-nums">{tick}%</text>
      {/each}
      <path d={area} fill="var(--chart-ink)" opacity="0.08" />
      <path d={line} fill="none" stroke="var(--chart-ink)" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
      {#each points as point, index (index)}
        {#if index % labelEvery === 0 || index === points.length - 1}
          <text x={x(index)} y={height - 8} text-anchor="middle" font-size="11" fill="var(--muted)">{point.label}</text>
        {/if}
      {/each}
      {#if selected !== null}<line x1={x(selected)} x2={x(selected)} y1={margin.top} y2={height - margin.bottom} stroke="var(--muted)" opacity="0.5" />{/if}
      {#each points as point, index (index)}
        <circle
          data-testid="score-point"
          data-retry={String(point.retry)}
          cx={x(index)}
          cy={y(point.percent)}
          r={selected === index ? 6 : 4.5}
          fill={point.retry ? "var(--surface)" : "var(--chart-ink)"}
          stroke={point.retry ? "var(--chart-ink)" : "var(--surface)"}
          stroke-width="2"
        />
      {/each}
      {#if points.length}
        <text x={x(points.length - 1) - 8} y={y(points.at(-1)!.percent) - 12} text-anchor="end" font-size="12" font-weight="600" fill="var(--color-base-content)">{points.at(-1)!.percent}%</text>
      {/if}
    </svg>
    {#if selected !== null}
      <div class="chart-tip show" style="position: absolute; left: {Math.min(x(selected) + 12, width - 240)}px; top: {Math.max(0, y(points[selected].percent) - 56)}px">{describe(points[selected])}</div>
    {/if}
  {/if}
  <p role="status" class="sr-only">{selected !== null ? describe(points[selected]) : ""}</p>
  <details class="table-view mt-4">
    <summary>Show as table</summary>
    <table class="table table-xs mt-2">
      <thead><tr class="text-muted"><th>Date</th><th>Score</th><th>{detailLabel}</th></tr></thead>
      <tbody>
        {#each points as point, index (index)}<tr><td>{point.label}</td><td class="tabular-nums">{point.percent}%</td><td>{point.detail}{point.retry ? " (retry)" : ""}</td></tr>{/each}
      </tbody>
    </table>
  </details>
</div>

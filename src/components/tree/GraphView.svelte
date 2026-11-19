<script lang="ts">
  // R219: modules, topics, sources and related links as a graph, searchable with the query
  // language, usable by keyboard, with a list fallback. Spec: nested-modules.feature, "Graph view".
  // Tests: GraphView.test.ts (T-GRAPH-01 to T-GRAPH-10).
  //
  // Layout is a plain top-down tree: each module above its children, leaves side by side.
  // Nodes are named by their full path, so two "CPU" topics are told apart (T-GRAPH-04).
  import { tick } from "svelte";
  import { List, Network, Search } from "@lucide/svelte";
  import { api, type NodeRef, type SourceRow, type TreeNode, type TreeView } from "../../lib/api";

  let { tree, sources = [], onOpen, onCreateModule }: {
    tree: TreeView;
    sources?: SourceRow[];
    onOpen: (node: NodeRef) => void;
    onCreateModule?: () => void;
  } = $props();

  const BIG = 200;
  const keyOf = (node: NodeRef) => `${node.kind}:${node.id}`;
  const fullPath = (node: TreeNode) => node.path.join(" › ");

  let showRelated = $state(true);
  let showSources = $state(false);
  let asList = $state(false);
  // svelte-ignore state_referenced_locally
  let collapsed = $state(new Set<string>(tree.nodes.length > BIG ? tree.nodes.filter((node) => node.node.kind === "Module" && node.parent === null).map((node) => keyOf(node.node)) : []));
  let hovered = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let query = $state("");
  let matches = $state<Set<string> | null>(null);
  let container = $state<HTMLElement>();

  const byKey = $derived(new Map(tree.nodes.map((node) => [keyOf(node.node), node])));
  const childrenOf = $derived.by(() => {
    const map = new Map<string, TreeNode[]>();
    for (const node of tree.nodes) {
      const parent = node.parent === null ? "" : `Module:${node.parent}`;
      if (!map.has(parent)) map.set(parent, []);
      map.get(parent)!.push(node);
    }
    return map;
  });
  const parentKey = (node: TreeNode) => (node.parent === null ? null : `Module:${node.parent}`);

  type Placed = { node: TreeNode; key: string; x: number; y: number; depth: number };
  const GAP_X = 150;
  const GAP_Y = 110;
  const layout = $derived.by(() => {
    const placed: Placed[] = [];
    let next = 0;
    const visit = (node: TreeNode, depth: number): number => {
      const key = keyOf(node.node);
      const kids = collapsed.has(key) ? [] : (childrenOf.get(key) ?? []);
      let x: number;
      if (kids.length === 0) {
        x = next++ * GAP_X;
      } else {
        const xs = kids.map((kid) => visit(kid, depth + 1));
        x = (xs[0] + xs[xs.length - 1]) / 2;
      }
      placed.push({ node, key, x: x + 80, y: depth * GAP_Y + 50, depth });
      return x;
    };
    for (const root of childrenOf.get("") ?? []) visit(root, 0);
    return placed;
  });
  const at = $derived(new Map(layout.map((placed) => [placed.key, placed])));
  const deepest = $derived(layout.reduce((most, placed) => Math.max(most, placed.depth), 0));

  const parentEdges = $derived(layout.filter((placed) => placed.node.parent !== null && at.has(parentKey(placed.node)!)).map((placed) => ({ from: at.get(parentKey(placed.node)!)!, to: placed })));
  const relatedEdges = $derived.by(() => {
    if (!showRelated) return [];
    const edges: { from: Placed; to: Placed }[] = [];
    for (const placed of layout) {
      if (placed.node.node.kind !== "Topic") continue;
      for (const other of placed.node.related) {
        const target = at.get(`Topic:${other}`);
        if (target && placed.node.node.id < other) edges.push({ from: placed, to: target });
      }
    }
    return edges;
  });
  const sourceNodes = $derived.by(() => {
    if (!showSources) return [];
    const used = new Set(layout.flatMap((placed) => placed.node.sources));
    const shown = sources.filter((source) => used.has(source.id));
    const y = (deepest + 1) * GAP_Y + 60;
    return shown.map((source, index) => ({ source, x: 80 + index * 220, y }));
  });
  const sourceEdges = $derived(sourceNodes.flatMap((spot) => layout.filter((placed) => placed.node.sources.includes(spot.source.id)).map((placed) => ({ from: placed, to: spot }))));

  const width = $derived(Math.max(400, ...layout.map((placed) => placed.x + 120), ...sourceNodes.map((spot) => spot.x + 220)));
  const height = $derived(Math.max(240, (deepest + 1) * GAP_Y + (sourceNodes.length ? 140 : 40)));

  function toggle(key: string) {
    const next = new Set(collapsed);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    collapsed = next;
  }

  async function focusKey(key: string | null | undefined) {
    if (!key) return;
    await tick();
    container?.querySelector<SVGGElement>(`[data-key="${key}"]`)?.focus();
  }

  function onKey(event: KeyboardEvent, placed: Placed) {
    const siblings = (childrenOf.get(parentKey(placed.node) ?? "") ?? []).filter((node) => at.has(keyOf(node.node)));
    const index = siblings.findIndex((node) => keyOf(node.node) === placed.key);
    const kids = (childrenOf.get(placed.key) ?? []).filter((node) => at.has(keyOf(node.node)));
    const moves: Record<string, string | null | undefined> = {
      ArrowDown: kids[0] ? keyOf(kids[0].node) : undefined,
      ArrowUp: parentKey(placed.node),
      ArrowRight: siblings[index + 1] ? keyOf(siblings[index + 1].node) : undefined,
      ArrowLeft: siblings[index - 1] ? keyOf(siblings[index - 1].node) : undefined,
    };
    if (event.key in moves) {
      event.preventDefault();
      void focusKey(moves[event.key]);
    } else if (event.key === "Enter") {
      event.preventDefault();
      onOpen(placed.node.node);
    } else if (event.key === " ") {
      event.preventDefault();
      selected = placed.key;
    }
  }

  async function runSearch(event: KeyboardEvent) {
    if (event.key !== "Enter") return;
    event.preventDefault();
    if (!query.trim()) {
      matches = null;
      return;
    }
    const found = await api.querySearch(query.trim());
    if (found.error) return;
    const keys = new Set<string>(found.paths.map(([node]) => keyOf(node)));
    for (const item of found.items ?? []) if (item.topic != null) keys.add(`Topic:${item.topic}`);
    matches = keys;
  }

  const listed = $derived.by(() => {
    const rows: { node: TreeNode; depth: number }[] = [];
    const visit = (node: TreeNode, depth: number) => {
      rows.push({ node, depth });
      const key = keyOf(node.node);
      if (!collapsed.has(key)) for (const kid of childrenOf.get(key) ?? []) visit(kid, depth + 1);
    };
    for (const root of childrenOf.get("") ?? []) visit(root, 0);
    return rows;
  });

  const hoveredNode = $derived(hovered ? byKey.get(hovered) : undefined);
  const selectedNode = $derived(selected ? byKey.get(selected) : undefined);
  const parentName = (node: TreeNode) => (node.parent === null ? "the top level" : (byKey.get(`Module:${node.parent}`)?.name ?? "a module"));
</script>

<section aria-label="Graph" class="flex flex-col gap-4">
  {#if tree.nodes.length === 0}
    <div class="panel flex flex-col items-start gap-3 p-6">
      <p class="font-bold">No modules yet</p>
      <button type="button" class="btn btn-primary" onclick={() => onCreateModule?.()}>Create a module</button>
    </div>
  {:else}
    <div class="flex flex-wrap items-center gap-4">
      <label class="q-field min-w-64 flex-1">
        <Search class="size-4 opacity-70" strokeWidth={2.4} aria-hidden="true" />
        <input type="text" class="q-input" aria-label="Search the graph" placeholder="e.g. module:N2T CPU" bind:value={query} onkeydown={runSearch} />
        {#if matches}<span class="text-sm text-muted tabular-nums">{matches.size} {matches.size === 1 ? "match" : "matches"}</span>{/if}
      </label>
      <label class="flex items-center gap-2 text-[13px] font-extrabold uppercase tracking-wide"><input type="checkbox" class="checkbox" bind:checked={showRelated} />Related</label>
      <label class="flex items-center gap-2 text-[13px] font-extrabold uppercase tracking-wide"><input type="checkbox" class="checkbox" bind:checked={showSources} />Sources</label>
      <button type="button" class="btn btn-ghost btn-sm" onclick={() => (asList = !asList)}>
        {#if asList}<Network class="size-4" strokeWidth={2.4} />Show as graph{:else}<List class="size-4" strokeWidth={2.4} />Show as list{/if}
      </button>
    </div>

    {#if asList}
      <ul role="tree" aria-label="Modules and topics" class="panel p-3">
        {#each listed as row (keyOf(row.node.node))}
          <li role="treeitem" aria-selected="false" aria-level={row.depth + 1} class="py-1" style="padding-left: {row.depth * 24 + 8}px">
            <button type="button" class="btn btn-link btn-sm p-0" onclick={() => onOpen(row.node.node)}>{row.node.name}</button>
            {#if row.node.node.kind === "Module"}<span class="text-xs font-extrabold uppercase text-muted">module</span>{/if}
          </li>
        {/each}
      </ul>
    {:else}
      <div class="panel relative overflow-auto" bind:this={container}>
        <svg width={width} height={height} viewBox="0 0 {width} {height}" class="block" aria-label="Graph of modules and topics" role="group">
          {#each parentEdges as edge (edge.to.key)}
            <line data-graph-edge="parent" x1={edge.from.x} y1={edge.from.y} x2={edge.to.x} y2={edge.to.y} stroke="var(--color-base-300)" stroke-width="2" />
          {/each}
          {#each sourceEdges as edge (`${edge.from.key}-${edge.to.source.id}`)}
            <line data-graph-edge="source" x1={edge.from.x} y1={edge.from.y} x2={edge.to.x} y2={edge.to.y} stroke="var(--color-base-300)" stroke-width="1.5" />
          {/each}
          {#each relatedEdges as edge (`${edge.from.key}-${edge.to.key}`)}
            <path data-graph-edge="related" d="M{edge.from.x} {edge.from.y} Q{(edge.from.x + edge.to.x) / 2} {edge.from.y - 70} {edge.to.x} {edge.to.y}" fill="none" stroke="var(--color-info)" stroke-width="2.5" stroke-dasharray="7 6" />
          {/each}
          {#each sourceNodes as spot (spot.source.id)}
            <g data-graph-source transform="translate({spot.x} {spot.y})">
              <rect x="-100" y="-16" width="200" height="32" rx="8" fill="var(--surface)" stroke="var(--color-base-300)" stroke-width="2" />
              <text text-anchor="middle" y="5" font-size="12" font-weight="700" fill="currentColor">{spot.source.fields.title.length > 30 ? `${spot.source.fields.title.slice(0, 29)}…` : spot.source.fields.title}</text>
              <title>{spot.source.apa}</title>
            </g>
          {/each}
          {#each layout as placed (placed.key)}
            {@const module = placed.node.node.kind === "Module"}
            {@const radius = module ? 24 : 16}
            {@const matched = matches ? matches.has(placed.key) : false}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <g role="button" tabindex="0" data-graph-node data-key={placed.key} aria-label={fullPath(placed.node)}
              data-match={matched ? "true" : "false"} data-dimmed={matches && !matched ? "true" : "false"}
              transform="translate({placed.x} {placed.y})" opacity={matches && !matched ? 0.35 : 1} class="graph-node"
              onmouseenter={() => (hovered = placed.key)} onmouseleave={() => (hovered = null)} onfocus={() => (hovered = placed.key)} onblur={() => (hovered = null)}
              onclick={() => (selected = placed.key)} ondblclick={() => onOpen(placed.node.node)} onkeydown={(event) => onKey(event, placed)}>
              {#if selected === placed.key}<circle r={radius + 7} fill="none" stroke="var(--color-primary)" stroke-width="3" />{/if}
              {#if matched}<circle r={radius + 3} fill="none" stroke="currentColor" stroke-width="2" />{/if}
              <circle r={radius} fill={module ? (placed.node.is_default ? "var(--color-base-300)" : "var(--color-primary)") : "var(--color-info)"} />
              <text text-anchor="middle" y={radius + 18} font-size={module ? 14 : 13} font-weight={module ? 900 : 700} fill="currentColor">{placed.node.name}</text>
            </g>
            {#if module && (childrenOf.get(placed.key)?.length ?? 0) > 0}
              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <g role="button" tabindex="0" aria-label="{collapsed.has(placed.key) ? 'Expand' : 'Collapse'} {placed.node.name}" transform="translate({placed.x + radius + 6} {placed.y - radius})"
                onclick={() => toggle(placed.key)} onkeydown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); toggle(placed.key); } }}>
                <circle r="9" fill="var(--surface)" stroke="var(--color-base-300)" stroke-width="2" />
                <text text-anchor="middle" y="4" font-size="13" font-weight="900" fill="currentColor">{collapsed.has(placed.key) ? "+" : "−"}</text>
              </g>
            {/if}
          {/each}
        </svg>
        {#if hoveredNode}
          <div role="tooltip" class="graph-tip">In {parentName(hoveredNode)}</div>
        {/if}
      </div>
      {#if selectedNode}
        <div role="region" aria-label="Selected" class="panel flex flex-wrap items-center gap-4 p-4">
          <div class="flex-1">
            <div class="text-[13px] font-bold text-muted">{selectedNode.path.slice(0, -1).join(" › ") || "Top level"}</div>
            <div class="text-lg font-black">{selectedNode.name}</div>
            <div class="text-[13px] text-muted">{fullPath(selectedNode)}</div>
          </div>
          {#if selectedNode.due}<span class="chip chip-due">{selectedNode.due} due</span>{/if}
          <button type="button" class="btn btn-primary btn-sm" onclick={() => onOpen(selectedNode.node)}>Open {selectedNode.node.kind === "Topic" ? "topic" : "module"}</button>
        </div>
      {/if}
      <p class="text-sm text-muted">Arrow keys move between linked nodes, Enter opens one.</p>
    {/if}
  {/if}
</section>

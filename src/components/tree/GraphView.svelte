<script lang="ts">
  // R219: modules, topics, sources and related links as a graph, searchable with the query
  // language, usable by keyboard, with a list fallback. Spec: nested-modules.feature, "Graph view".
  // Tests: GraphView.test.ts (T-GRAPH-01 to T-GRAPH-10).
  import type { NodeRef, SourceRow, TreeView } from "../../lib/api";

  let { tree, sources = [], onOpen, onCreateModule }: {
    tree: TreeView;
    sources?: SourceRow[];
    onOpen: (node: NodeRef) => void;
    onCreateModule?: () => void;
  } = $props();
</script>

<section aria-label="Graph">
  <!-- TODO(R219) -->
</section>

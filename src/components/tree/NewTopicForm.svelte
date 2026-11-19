<script lang="ts">
  // R217: making a topic. With no module picked, the Default warning asks first.
  // Spec: nested-modules.feature, "Default module". Tests: NewTopicForm.test.ts (T-DEF-09 to T-DEF-11).
  import type { TreeView } from "../../lib/api";

  let { tree, onCreated }: { tree: TreeView; onCreated?: (topic: number) => void } = $props();
</script>

<form aria-label="New topic">
  <!-- TODO(R217): name, module picker, Default warning -->
</form>

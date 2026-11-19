<script lang="ts">
  // R217: deleting a module names everything it removes, offers "Move contents up", lists
  // clashes and needs the module's name typed. Spec: nested-modules.feature, "Deleting".
  // Tests: DeleteModuleDialog.test.ts (T-DEL-01, T-DEL-04, T-DEL-07).
  import type { DeleteChoice, DeletePreview } from "../../lib/api";

  let { name, preview, onConfirm, onCancel }: {
    name: string;
    preview: DeletePreview;
    onConfirm: (choice: DeleteChoice) => void;
    onCancel: () => void;
  } = $props();
</script>

<div role="dialog" aria-modal="true" aria-label="Delete module">
  <!-- TODO(R217) -->
</div>

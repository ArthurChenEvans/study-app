<script lang="ts">
  // R220: the search box. Space or Enter turns what's typed into pills, symbols show as words
  // and grouping stays visible. The pills come from the backend's display model
  // (study_core::query::display), so this only draws them.
  // Spec: search-query.feature, "Pills" and "How pills look". Tests: QueryBox.test.ts (T-PILL-*).
  import type { QuerySearch } from "../../lib/api";

  let { value = "", label = "Search", onResults }: {
    /** The query to start with, as text. */
    value?: string;
    label?: string;
    /** Called with each search the box runs. */
    onResults?: (search: QuerySearch, input: string) => void;
  } = $props();
</script>

<div class="query-box" role="search" aria-label={label}>
  <!-- TODO(R220): pills, input, Edit as text, cheat sheet -->
</div>

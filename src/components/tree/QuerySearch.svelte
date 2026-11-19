<script lang="ts">
  // R220: the query box with grouped results under it, for the Search screen and list filters.
  // Spec: search-query.feature, "Results are grouped by kind", "A broken query keeps the last
  // good results", "No results". Tests: QuerySearch.test.ts (T-PILL-21, T-PILL-26).
  let { value = "" }: { value?: string } = $props();
</script>

<section aria-label="Search results">
  <!-- TODO(R220) -->
</section>

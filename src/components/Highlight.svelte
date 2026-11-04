<script lang="ts">
  // Marks every place the search words appear. <mark> is announced by screen readers,
  // and is underlined as well as tinted so it doesn't rely on colour.
  let { text, term }: { text: string; term: string } = $props();

  const parts = $derived.by(() => {
    const needle = term.trim().toLowerCase();
    if (!needle) return [{ text, hit: false }];
    const out: { text: string; hit: boolean }[] = [];
    const lower = text.toLowerCase();
    let at = 0;
    for (let found = lower.indexOf(needle); found !== -1; found = lower.indexOf(needle, at)) {
      if (found > at) out.push({ text: text.slice(at, found), hit: false });
      out.push({ text: text.slice(found, found + needle.length), hit: true });
      at = found + needle.length;
    }
    if (at < text.length) out.push({ text: text.slice(at), hit: false });
    return out;
  });
</script>

{#each parts as part, index (index)}{#if part.hit}<mark class="hit">{part.text}</mark>{:else}{part.text}{/if}{/each}

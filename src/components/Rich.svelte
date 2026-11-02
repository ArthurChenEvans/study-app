<script lang="ts">
  // Markdown + LaTeX, sanitised in renderMarkdown before it reaches {@html}.
  import { renderInlineMarkdown, renderMarkdown } from "../lib/markdown";
  let { text, inline = false, class: className = "" }: { text: string; inline?: boolean; class?: string } = $props();
  const html = $derived(inline ? renderInlineMarkdown(text) : renderMarkdown(text));
</script>

{#if inline}<span class={className}>{@html html}</span>{:else}<div class="rich {className}">{@html html}</div>{/if}

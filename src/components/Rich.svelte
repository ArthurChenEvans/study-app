<script lang="ts">
  // Markdown + LaTeX + highlighted code, sanitised in renderMarkdown before it reaches {@html}.
  // Links in it never navigate the app (lib/links.ts).
  // Stored images arrive as placeholders and are filled in after each render. Code grammars
  // load on demand, so the view renders again when one arrives.
  import { onGrammarLoaded, renderInlineMarkdown, renderMarkdown } from "../lib/markdown";
  import { showImages } from "../lib/images";
  import { safeLinks } from "../lib/links";
  let { text, inline = false, class: className = "" }: { text: string; inline?: boolean; class?: string } = $props();
  let grammars = $state(0);
  $effect(() => onGrammarLoaded(() => grammars++));
  const html = $derived.by(() => {
    grammars;
    return inline ? renderInlineMarkdown(text) : renderMarkdown(text);
  });
  let root = $state<HTMLElement>();
  $effect(() => {
    html;
    if (root) showImages(root);
  });
</script>

{#if inline}<span bind:this={root} use:safeLinks class="rich-inline {className}">{@html html}</span>{:else}<div bind:this={root} use:safeLinks class="rich {className}">{@html html}</div>{/if}

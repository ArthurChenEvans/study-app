<script lang="ts">
  // R220: the search box. Space or Enter turns what's typed into pills, symbols show as words
  // and grouping stays visible. The pills come from the backend's display model
  // (study_core::query::display), so this only draws them.
  // Spec: search-query.feature, "Pills" and "How pills look". Tests: QueryBox.test.ts (T-PILL-*).
  import { onDestroy, onMount } from "svelte";
  import { Folder, FileText, HelpCircle, X } from "@lucide/svelte";
  import { api, type Part, type Pill, type QuerySearch } from "../../lib/api";

  let { value = "", label = "Search", onResults }: {
    /** The query to start with, as text. */
    value?: string;
    label?: string;
    /** Called with each search the box runs. */
    onResults?: (search: QuerySearch, input: string) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let committed = $state(value.trim());
  let draft = $state("");
  let search = $state<QuerySearch | null>(null);
  let shown = $state<Part[]>([]);
  let helpOpen = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let run_id = 0;

  const PAUSE = 300;
  const tokens = (text: string) => text.match(/"[^"]*"?|\S+/g) ?? [];
  const join = (...parts: string[]) => parts.map((part) => part.trim()).filter(Boolean).join(" ");
  const quoteOpen = (text: string) => (text.match(/"/g) ?? []).length % 2 === 1;

  async function run(text: string) {
    clearTimeout(timer);
    const mine = ++run_id;
    const result = await api.querySearch(text);
    if (mine !== run_id) return;
    search = result;
    if (!result.error) shown = result.parts;
    onResults?.(result, text);
  }

  function commit() {
    let typed = draft;
    if (quoteOpen(typed)) typed += '"';
    committed = join(committed, typed);
    draft = "";
    void run(committed);
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === " " && !quoteOpen(draft)) {
      if (draft.trim()) {
        event.preventDefault();
        commit();
      }
    } else if (event.key === "Enter") {
      event.preventDefault();
      commit();
    } else if (event.key === "Backspace" && draft === "" && committed) {
      event.preventDefault();
      const all = tokens(committed);
      draft = all.pop() ?? "";
      committed = all.join(" ");
      void run(committed);
    }
  }

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(() => void run(join(committed, draft)), PAUSE);
  }

  function onPaste(event: ClipboardEvent) {
    const text = event.clipboardData?.getData("text") ?? "";
    if (!text) return;
    event.preventDefault();
    committed = join(committed, draft + text);
    draft = "";
    void run(committed);
  }

  /** The word a token stands for, without NOT, brackets, quotes or a scope's prefix. */
  const bare = (token: string) => token.replace(/^[!(]+|[)]+$/g, "").replace(/^(module|topic):/i, "").replace(/^"|"$/g, "").toLowerCase();
  const OPERATORS = new Set(["&", "$", "^", "AND", "OR", "XOR"]);

  function remove(pill: Pill) {
    const all = tokens(committed);
    let at = -1;
    for (let index = all.length - 1; index >= 0; index--) {
      if (bare(all[index]) === pill.text.toLowerCase()) { at = index; break; }
    }
    if (at < 0) return;
    all.splice(at, 1);
    // An operator left with nothing on one side goes too.
    if (OPERATORS.has(all[at] ?? "") && (at === 0 || OPERATORS.has(all[at - 1] ?? ""))) all.splice(at, 1);
    else if (OPERATORS.has(all[at - 1] ?? "") && (at >= all.length || OPERATORS.has(all[at] ?? ""))) all.splice(at - 1, 1);
    committed = all.join(" ");
    void run(committed);
  }

  function editAsText() {
    draft = join(committed, draft);
    committed = "";
    shown = [];
  }

  const spokenOf = (parts: Part[]): string => parts.map((part) => ("Pill" in part ? pillName(part.Pill) : "Op" in part ? part.Op.toLowerCase() : spokenOf(part.Group.parts))).join(" ");
  const pillName = (pill: Pill) => (typeof pill.kind === "object" ? `${pill.kind.Scope.toLowerCase()} ${pill.text}` : pill.text);
  const scopeTitle = (pill: Pill) => search?.scope_paths.find(([name]) => name.toLowerCase() === pill.text.toLowerCase())?.[1].join(", ") || undefined;

  onMount(() => {
    if (committed) void run(committed);
  });
  onDestroy(() => clearTimeout(timer));

  const invalid = $derived(!!search?.error);
</script>

{#snippet render(parts: Part[], top: boolean)}
  {#each parts as part, index (index)}
    {#if "Pill" in part}
      {@const pill = part.Pill}
      <!-- A pill is focusable so Delete removes it (search-query.feature, "Pills can be removed by click or keyboard"). -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <span role="listitem" class="q-pill" data-negated={pill.negated ? "true" : "false"} data-kind={typeof pill.kind === "object" ? "scope" : pill.kind.toLowerCase()}
        aria-label={pill.negated ? `not ${pillName(pill)}` : pillName(pill)} title={typeof pill.kind === "object" ? scopeTitle(pill) : undefined} tabindex="0"
        onkeydown={(event) => { if (event.key === "Delete" || event.key === "Backspace") { event.preventDefault(); remove(pill); } }}>
        {#if typeof pill.kind === "object"}
          {#if pill.kind.Scope === "Module"}<Folder class="size-4" strokeWidth={2.4} aria-hidden="true" />{:else}<FileText class="size-4" strokeWidth={2.4} aria-hidden="true" />{/if}
          <span class="q-scope">{pill.kind.Scope}</span>
        {/if}
        <span class="q-text" class:q-phrase={pill.kind === "Phrase"}>{pill.text}</span>
        <button type="button" class="q-x" aria-label="Remove {pill.text}" tabindex="-1" onclick={() => remove(pill)}><X class="size-3" strokeWidth={2.4} /></button>
      </span>
    {:else if "Op" in part}
      <span class="q-op">{part.Op}</span>
    {:else}
      {@const group = part.Group}
      {#if group.brackets !== "None"}<span class="q-bracket" data-faint={group.brackets === "Faint" ? "true" : "false"}>(</span>{/if}
      <span role="group" class="q-group" data-outline={group.outline.toLowerCase()} data-negated={group.negated ? "true" : "false"}
        aria-label={top && search?.spoken && parts.length === 1 ? search.spoken : spokenOf(group.parts)}>
        {@render render(group.parts, false)}
      </span>
      {#if group.brackets !== "None"}<span class="q-bracket" data-faint={group.brackets === "Faint" ? "true" : "false"}>)</span>{/if}
    {/if}
  {/each}
{/snippet}

<div class="query-box" role="search" aria-label="{label} box">
  <div class="q-field" class:q-invalid={invalid}>
    {#if shown.length}<div class="q-pills" role="list" aria-label="Search terms">{@render render(shown, true)}</div>{/if}
    <input type="search" class="q-input" aria-label={label} bind:value={draft} aria-invalid={invalid ? "true" : undefined}
      placeholder={shown.length ? "" : "Search, e.g. module:N2T CPU $ ALU"} onkeydown={onKey} oninput={onInput} onpaste={onPaste} />
    {#if committed}<button type="button" class="btn btn-link btn-sm" onclick={editAsText}>Edit as text</button>{/if}
    <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Search help" aria-expanded={helpOpen} onclick={() => (helpOpen = !helpOpen)}><HelpCircle class="size-4" strokeWidth={2.4} /></button>
  </div>
  {#if search?.error}<p role="alert" class="q-error">{search.error.message}</p>{/if}
  {#if helpOpen}
    <div role="dialog" aria-label="Search help" class="panel q-help">
      <dl>
        <dt><code>module:N2T</code></dt><dd>anything inside N2T and its submodules</dd>
        <dt><code>topic:CPU</code></dt><dd>only things under topics named CPU</dd>
        <dt><code>"cpu logic"</code></dt><dd>that exact phrase</dd>
        <dt><code>&amp;</code> or <code>AND</code></dt><dd>both</dd>
        <dt><code>$</code> or <code>OR</code></dt><dd>either</dd>
        <dt><code>^</code> or <code>XOR</code></dt><dd>one, not both</dd>
        <dt><code>!</code> or <code>NOT</code></dt><dd>leave out</dd>
        <dt><code>( )</code></dt><dd>group</dd>
      </dl>
      <p class="q-example">Example: <code>CPU &amp; (ALU $ RAM)</code></p>
    </div>
  {/if}
</div>

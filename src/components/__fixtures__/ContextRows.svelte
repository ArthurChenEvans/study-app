<script lang="ts">
  import ContextMenuHost from "../ContextMenuHost.svelte";
  import { rowMenu } from "../../lib/contextMenu.svelte";
  let { onPick }: { onPick: (what: string) => void } = $props();
  let ticked = $state<string[]>([]);
  const rows = ["CPU", "DNS", "TLS"];
  const menuFor = (name: string) => () => ticked.includes(name) && ticked.length > 1
    ? { label: `${ticked.length} quizzes`, items: [{ label: `Delete ${ticked.length} quizzes…`, danger: true, onSelect: () => onPick(`delete ${ticked.join(",")}`) }] }
    : { label: name, items: [{ label: "Start quiz", onSelect: () => onPick(`start ${name}`) }, { label: "Delete quiz…", danger: true, onSelect: () => onPick(`delete ${name}`) }] };
</script>

<ul>
  {#each rows as name (name)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <li tabindex="0" use:rowMenu={menuFor(name)}>
      <input type="checkbox" aria-label="Tick {name}" onchange={(event) => (ticked = event.currentTarget.checked ? [...ticked, name] : ticked.filter((t) => t !== name))} />{name}
    </li>
  {/each}
</ul>
<ContextMenuHost />

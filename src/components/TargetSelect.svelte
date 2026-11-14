<script lang="ts">
  // Where a deck or a group of imported items goes: a new topic, a topic already here, or nowhere.
  import type { ModuleRow, Target, TopicRow } from "../lib/api";

  let { label, target = $bindable(), fresh, topics, modules, skip = true }: { label: string; target: Target; fresh: Target; topics: TopicRow[]; modules: ModuleRow[]; skip?: boolean } = $props();

  const value = $derived(target.kind === "Topic" ? `topic:${target.topic}` : target.kind === "Skip" ? "skip" : "new");
  const moduleName = (id: number | null) => modules.find((row) => row.id === id)?.name;
  const freshName = $derived(fresh.kind === "NewTopic" ? (fresh.module ? `${fresh.module}: ${fresh.name}` : fresh.name) : "");

  function choose(event: Event) {
    const picked = (event.target as HTMLSelectElement).value;
    target = picked === "skip" ? { kind: "Skip" } : picked.startsWith("topic:") ? { kind: "Topic", topic: Number(picked.slice(6)) } : fresh;
  }
</script>

<select class="select select-sm w-full min-w-0" aria-label={label} {value} onchange={choose}>
  <option value="new">New topic: {freshName}</option>
  {#each topics as topic (topic.id)}
    <option value="topic:{topic.id}">{moduleName(topic.module) ? `${moduleName(topic.module)}: ` : ""}{topic.name}</option>
  {/each}
  {#if skip}<option value="skip">Don't import</option>{/if}
</select>

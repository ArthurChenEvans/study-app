<script lang="ts">
  // Where a deck or a group of imported items goes: a new topic, a topic already here, or nowhere.
  import Select from "./Select.svelte";
  import type { ModuleRow, Target, TopicRow } from "../lib/api";

  let { label, target = $bindable(), fresh, topics, modules, skip = true }: { label: string; target: Target; fresh: Target; topics: TopicRow[]; modules: ModuleRow[]; skip?: boolean } = $props();

  const value = $derived(target.kind === "Topic" ? `topic:${target.topic}` : target.kind === "Skip" ? "skip" : "new");
  const moduleName = (id: number | null) => modules.find((row) => row.id === id)?.name;
  const freshName = $derived(fresh.kind === "NewTopic" ? (fresh.module ? `${fresh.module}: ${fresh.name}` : fresh.name) : "");

  function choose(picked: string) {
    target = picked === "skip" ? { kind: "Skip" } : picked.startsWith("topic:") ? { kind: "Topic", topic: Number(picked.slice(6)) } : fresh;
  }
</script>

<Select small class="w-full min-w-0" aria-label={label} {value} options={[{ value: "new", label: `New topic: ${freshName}` }, ...topics.map((topic) => ({ value: `topic:${topic.id}`, label: `${moduleName(topic.module) ? `${moduleName(topic.module)}: ` : ""}${topic.name}` })), ...(skip ? [{ value: "skip", label: "Don't import" }] : [])]} onchange={(picked) => choose(String(picked))} />

<script lang="ts" generics="T">
  // Segmented control (design rules section 6). Each option is a radio.
  let { label, options, value = $bindable() }: { label: string; options: { value: T; label: string; name?: string }[]; value: T } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as option (option.label)}
    <button type="button" role="radio" aria-checked={value === option.value} aria-label={option.name ?? option.label} onclick={() => (value = option.value)}>{option.label}</button>
  {/each}
</div>

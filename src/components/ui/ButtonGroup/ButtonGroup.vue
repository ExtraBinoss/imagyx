<script setup lang="ts">
withDefaults(defineProps<{ full?: boolean }>(), { full: false })
</script>

<template>
  <div class="ui-button-group" :class="{ 'ui-button-group--full': full }" role="group">
    <slot />
  </div>
</template>

<style scoped>
.ui-button-group {
  display: flex;
  width: fit-content;
  max-width: 100%;
  padding: 4px;
  gap: 3px;
  border: 1px solid var(--border);
  border-radius: calc(var(--radius-md) + 3px);
  background: color-mix(in srgb, var(--surface-hover) 88%, var(--surface));
  box-shadow: inset 0 1px 3px rgb(15 23 42 / 0.07), inset 0 1px 0 rgb(255 255 255 / 0.24);
}
.ui-button-group--full { width: 100%; }
.ui-button-group :slotted(.ui-button) {
  flex: 1 1 0;
  min-width: 0;
  border-color: transparent;
  box-shadow: none;
}
.ui-button-group :slotted(.ui-button:hover:not(:disabled)) { transform: none; }
.ui-button-group :slotted(.ui-button--pressed) {
  border-color: var(--border);
  background: var(--surface-elevated);
  color: var(--text);
  transform: none;
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.26),
    0 1px 2px rgb(15 23 42 / 0.08),
    0 5px 12px -10px rgb(15 23 42 / 0.35);
}
:global(:root[data-theme='dark']) .ui-button-group {
  background: color-mix(in srgb, var(--background) 76%, black);
  box-shadow: inset 0 1px 4px rgb(0 0 0 / 0.32), inset 0 1px 0 rgb(255 255 255 / 0.04);
}
</style>

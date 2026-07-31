<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue: boolean
  disabled?: boolean
  ariaLabel: string
}>(), {
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

function handleChange(event: Event) {
  emit('update:modelValue', (event.target as HTMLInputElement).checked)
}
</script>

<template>
  <label class="ui-switch" :class="{ 'ui-switch--disabled': disabled }">
    <input
      class="ui-switch__input"
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      :aria-label="ariaLabel"
      @change="handleChange"
    >
    <span class="ui-switch__track" aria-hidden="true"><span class="ui-switch__thumb" /></span>
  </label>
</template>

<style scoped>
.ui-switch { display: inline-flex; flex: 0 0 auto; cursor: pointer; }
.ui-switch--disabled { cursor: not-allowed; opacity: 0.55; }
.ui-switch__input { position: absolute; width: 1px; height: 1px; margin: -1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
.ui-switch__track {
  display: flex;
  align-items: center;
  width: 38px;
  height: 22px;
  padding: 2px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-full);
  background: var(--surface-hover);
  box-shadow: inset 0 1px 2px rgb(15 23 42 / 0.12);
  transition: background-color 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}
.ui-switch__thumb { width: 16px; height: 16px; border-radius: 50%; background: var(--surface-elevated); box-shadow: 0 1px 3px rgb(2 6 23 / 0.28); transition: transform 180ms cubic-bezier(0.16, 1, 0.3, 1); }
.ui-switch__input:checked + .ui-switch__track { border-color: color-mix(in srgb, var(--primary) 62%, var(--border)); background: var(--primary); }
.ui-switch__input:checked + .ui-switch__track .ui-switch__thumb { transform: translateX(16px); }
.ui-switch__input:focus-visible + .ui-switch__track { box-shadow: 0 0 0 4px color-mix(in srgb, var(--focus-ring) 32%, transparent); }
.ui-switch__input:active:not(:disabled) + .ui-switch__track .ui-switch__thumb { transform: translateX(2px); }
.ui-switch__input:checked:active:not(:disabled) + .ui-switch__track .ui-switch__thumb { transform: translateX(14px); }
@media (prefers-reduced-motion: reduce) { .ui-switch__track, .ui-switch__thumb { transition-duration: 0.01ms; } }
</style>

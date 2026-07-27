<script setup lang="ts">
import { ChevronDown } from '@lucide/vue'

withDefaults(
  defineProps<{
    open?: boolean
    disabled?: boolean
    label: string
  }>(),
  {
    open: false,
    disabled: false,
  },
)

const emit = defineEmits<{
  toggle: []
}>()
</script>

<template>
  <button
    type="button"
    class="ui-disclosure-button"
    :aria-expanded="open"
    :disabled="disabled"
    @click="emit('toggle')"
  >
    <span class="ui-disclosure-button__label">{{ label }}</span>
    <ChevronDown
      :size="15"
      class="ui-disclosure-button__chevron"
      :class="{ 'ui-disclosure-button__chevron--open': open }"
    />
  </button>
</template>

<style scoped>
.ui-disclosure-button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  width: 100%;
  min-height: 32px;
  padding: 0 var(--space-2);
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: var(--text-xs);
  font-weight: 650;
  cursor: pointer;
}

.ui-disclosure-button:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}

.ui-disclosure-button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
}

.ui-disclosure-button__label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ui-disclosure-button__chevron {
  flex: 0 0 auto;
  transition: transform var(--transition-fast);
}

.ui-disclosure-button__chevron--open {
  transform: rotate(180deg);
}
</style>

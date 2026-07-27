<script setup lang="ts">
import { LoaderCircle } from '@lucide/vue'

withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    size?: 'sm' | 'md' | 'lg' | 'icon'
    type?: 'button' | 'submit' | 'reset'
    block?: boolean
    disabled?: boolean
    pressed?: boolean
    depth?: boolean
    loading?: boolean
  }>(),
  {
    variant: 'secondary',
    size: 'md',
    type: 'button',
    block: false,
    disabled: false,
    pressed: false,
    depth: true,
    loading: false,
  },
)
</script>

<template>
  <button
    class="ui-button"
    :class="[
      `ui-button--${variant}`,
      `ui-button--${size}`,
      {
        'ui-button--block': block,
        'ui-button--pressed': pressed,
        'ui-button--flat': !depth,
        'ui-button--loading': loading,
      },
    ]"
    :type="type"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
    :aria-pressed="pressed || undefined"
  >
    <span v-if="loading" class="ui-button__icon ui-button__spinner" aria-hidden="true">
      <LoaderCircle :size="15" />
    </span>
    <span v-else-if="$slots.leading" class="ui-button__icon"><slot name="leading" /></span>
    <span v-if="$slots.default" class="ui-button__content"><slot /></span>
    <span v-if="!loading && $slots.trailing" class="ui-button__icon"><slot name="trailing" /></span>
  </button>
</template>

<style scoped>
.ui-button {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font: inherit;
  font-weight: 580;
  line-height: 1;
  cursor: pointer;
  isolation: isolate;
  overflow: hidden;
  transition:
    transform 140ms ease,
    background-color var(--transition-fast),
    border-color var(--transition-fast),
    color var(--transition-fast),
    box-shadow var(--transition-fast),
    opacity var(--transition-fast);
}

.ui-button:not(.ui-button--flat):not(.ui-button--ghost) {
  box-shadow: var(--btn-depth-shadow);
}
.ui-button:not(.ui-button--flat):not(.ui-button--ghost):hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: var(--btn-depth-hover);
}
.ui-button:not(.ui-button--flat):not(.ui-button--ghost):active:not(:disabled),
.ui-button--pressed:not(.ui-button--flat):not(:disabled) {
  transform: translateY(1px);
  box-shadow: var(--btn-depth-active);
}

.ui-button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
}

.ui-button:disabled {
  cursor: not-allowed;
  opacity: 0.48;
  transform: none;
}
.ui-button--loading:disabled {
  opacity: 0.82;
}

.ui-button--sm {
  min-height: 30px;
  padding: 0 var(--space-3);
  font-size: var(--text-xs);
}

.ui-button--md {
  min-height: 36px;
  padding: 0 var(--space-4);
  font-size: var(--text-sm);
}

.ui-button--lg {
  min-height: 42px;
  padding: 0 var(--space-5);
  font-size: var(--text-sm);
}

.ui-button--icon {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: var(--radius-full);
}

.ui-button--block {
  width: 100%;
}

.ui-button--primary {
  border-color: var(--primary);
  background: var(--primary);
  color: var(--primary-foreground);
}
.ui-button--primary:hover:not(:disabled) {
  border-color: var(--primary-hover);
  background: var(--primary-hover);
}

.ui-button--secondary {
  border-color: var(--border);
  background: var(--surface);
  color: var(--text);
}
.ui-button--secondary:hover:not(:disabled) {
  border-color: var(--border-strong);
  background: var(--surface-hover);
}

.ui-button--ghost {
  border-color: transparent;
  background: transparent;
  color: var(--text-secondary);
}
.ui-button--ghost:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}

.ui-button--danger {
  border-color: var(--danger-border);
  background: var(--danger-surface);
  color: var(--danger-text);
}
.ui-button--danger:hover:not(:disabled) {
  background: var(--danger-surface-hover);
}

.ui-button__icon,
.ui-button__content {
  position: relative;
  z-index: 1;
}
.ui-button__icon {
  display: inline-flex;
  flex: 0 0 auto;
}
.ui-button__content {
  display: inline-flex;
  align-items: center;
  gap: inherit;
  min-width: 0;
  max-width: 100%;
}
.ui-button__spinner {
  animation: ui-button-spin 720ms linear infinite;
}

@keyframes ui-button-spin {
  to { transform: rotate(1turn); }
}

@media (prefers-reduced-motion: reduce) {
  .ui-button__spinner { animation-duration: 1.4s; }
}
</style>

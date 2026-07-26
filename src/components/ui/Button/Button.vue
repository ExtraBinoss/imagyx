<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    size?: 'sm' | 'md' | 'lg' | 'icon'
    type?: 'button' | 'submit' | 'reset'
    block?: boolean
    disabled?: boolean
    pressed?: boolean
    depth?: boolean
  }>(),
  {
    variant: 'secondary',
    size: 'md',
    type: 'button',
    block: false,
    disabled: false,
    pressed: false,
    depth: true,
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
      },
    ]"
    :type="type"
    :disabled="disabled"
    :aria-pressed="pressed || undefined"
  >
    <span v-if="$slots.leading" class="ui-button__icon"><slot name="leading" /></span>
    <span v-if="$slots.default" class="ui-button__content"><slot /></span>
    <span v-if="$slots.trailing" class="ui-button__icon"><slot name="trailing" /></span>
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
  font-weight: 620;
  line-height: 1;
  cursor: pointer;
  isolation: isolate;
  overflow: hidden;
  transform: translateY(0);
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.34),
    inset 0 -1px 0 rgb(15 23 42 / 0.05),
    0 1px 2px rgb(15 23 42 / 0.08),
    0 4px 12px -8px rgb(15 23 42 / 0.28);
  transition:
    transform 150ms cubic-bezier(0.16, 1, 0.3, 1),
    box-shadow 170ms ease,
    background-color var(--transition-fast),
    border-color var(--transition-fast),
    color var(--transition-fast),
    opacity var(--transition-fast);
}

.ui-button::before {
  content: '';
  position: absolute;
  z-index: -1;
  inset: 0;
  background: linear-gradient(180deg, rgb(255 255 255 / 0.1), transparent 44%);
  pointer-events: none;
}

.ui-button:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.42),
    inset 0 -1px 0 rgb(15 23 42 / 0.05),
    0 2px 4px rgb(15 23 42 / 0.09),
    0 8px 18px -12px rgb(15 23 42 / 0.38);
}

.ui-button:active:not(:disabled),
.ui-button--pressed:not(:disabled) {
  transform: translateY(1px);
  box-shadow:
    inset 0 2px 5px rgb(15 23 42 / 0.14),
    inset 0 1px 0 rgb(255 255 255 / 0.08),
    0 1px 1px rgb(15 23 42 / 0.05);
}

.ui-button--flat {
  box-shadow: none;
}
.ui-button--flat::before { display: none; }
.ui-button--flat:hover:not(:disabled) { box-shadow: none; }

.ui-button:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
}

.ui-button:disabled {
  cursor: not-allowed;
  opacity: 0.48;
  transform: none;
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
  width: 32px;
  height: 32px;
  padding: 0;
}

.ui-button--block { width: 100%; }

.ui-button--primary {
  border-color: color-mix(in srgb, var(--primary) 88%, black);
  background: linear-gradient(180deg, color-mix(in srgb, var(--primary) 92%, white), var(--primary));
  color: var(--primary-foreground);
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.28),
    inset 0 -1px 0 rgb(15 23 42 / 0.16),
    0 2px 4px color-mix(in srgb, var(--primary) 22%, transparent),
    0 7px 16px -11px color-mix(in srgb, var(--primary) 72%, transparent);
}
.ui-button--primary:hover:not(:disabled) {
  border-color: var(--primary-hover);
  background: linear-gradient(180deg, color-mix(in srgb, var(--primary-hover) 88%, white), var(--primary-hover));
}

.ui-button--secondary {
  border-color: var(--border);
  background: linear-gradient(180deg, color-mix(in srgb, var(--surface) 92%, white), var(--surface));
  color: var(--text);
}
.ui-button--secondary:hover:not(:disabled) {
  border-color: var(--border-strong);
  background: linear-gradient(180deg, color-mix(in srgb, var(--surface-hover) 88%, white), var(--surface-hover));
}

.ui-button--ghost {
  border-color: transparent;
  background: transparent;
  color: var(--text-secondary);
  box-shadow: none;
}
.ui-button--ghost::before { opacity: 0; }
.ui-button--ghost:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--border) 78%, transparent);
  background: var(--surface-hover);
  color: var(--text);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.16), 0 1px 2px rgb(15 23 42 / 0.05);
}

.ui-button--danger {
  border-color: var(--danger-border);
  background: linear-gradient(180deg, color-mix(in srgb, var(--danger-surface) 90%, white), var(--danger-surface));
  color: var(--danger-text);
}
.ui-button--danger:hover:not(:disabled) { background: var(--danger-surface-hover); }

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
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:root[data-theme='dark'] .ui-button {
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.08),
    inset 0 -1px 0 rgb(0 0 0 / 0.24),
    0 1px 2px rgb(0 0 0 / 0.28),
    0 7px 18px -13px rgb(0 0 0 / 0.72);
}
:root[data-theme='dark'] .ui-button--ghost { box-shadow: none; }
:root[data-theme='dark'] .ui-button--secondary {
  background: linear-gradient(180deg, color-mix(in srgb, var(--surface) 94%, white 6%), var(--surface));
}

@media (prefers-reduced-motion: reduce) {
  .ui-button { transition-duration: 0.01ms; }
}
</style>

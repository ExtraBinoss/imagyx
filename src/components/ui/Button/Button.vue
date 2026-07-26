<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md" | "lg" | "icon";
    type?: "button" | "submit" | "reset";
    block?: boolean;
    disabled?: boolean;
    pressed?: boolean;
    depth?: boolean;
  }>(),
  {
    variant: "secondary",
    size: "md",
    type: "button",
    block: false,
    disabled: false,
    pressed: false,
    depth: true,
  },
);
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
    <span v-if="$slots.leading" class="ui-button__icon"
      ><slot name="leading"
    /></span>
    <span v-if="$slots.default" class="ui-button__content"><slot /></span>
    <span v-if="$slots.trailing" class="ui-button__icon"
      ><slot name="trailing"
    /></span>
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

/* Depth mode enabled via CSS variables defined in style.css */
.ui-button:not(.ui-button--flat) {
  box-shadow: var(--btn-depth-shadow);
}
.ui-button:not(.ui-button--flat):hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: var(--btn-depth-hover);
}
.ui-button:not(.ui-button--flat):active:not(:disabled),
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
</style>

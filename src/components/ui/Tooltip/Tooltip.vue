<script setup lang="ts">
withDefaults(
  defineProps<{
    text: string
    side?: 'top' | 'right' | 'bottom' | 'left'
  }>(),
  {
    side: 'top',
  },
)
</script>

<template>
  <span class="ui-tooltip" :data-side="side">
    <slot />
    <span class="ui-tooltip__content" role="tooltip">{{ text }}</span>
  </span>
</template>

<style scoped>
.ui-tooltip {
  position: relative;
  display: inline-flex;
}

.ui-tooltip__content {
  position: absolute;
  z-index: var(--z-tooltip);
  max-width: 240px;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--overlay);
  color: var(--overlay-foreground);
  font-size: var(--text-xs);
  font-weight: 500;
  line-height: 1.35;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  transform: translateY(2px);
  transition:
    opacity var(--transition-fast),
    transform var(--transition-fast);
}

.ui-tooltip:hover .ui-tooltip__content,
.ui-tooltip:focus-within .ui-tooltip__content {
  opacity: 1;
  transform: translate(0, 0);
}

.ui-tooltip[data-side='top'] .ui-tooltip__content {
  left: 50%;
  bottom: calc(100% + var(--space-2));
  transform: translate(-50%, 2px);
}

.ui-tooltip[data-side='top']:hover .ui-tooltip__content,
.ui-tooltip[data-side='top']:focus-within .ui-tooltip__content {
  transform: translate(-50%, 0);
}

.ui-tooltip[data-side='bottom'] .ui-tooltip__content {
  top: calc(100% + var(--space-2));
  left: 50%;
  transform: translate(-50%, -2px);
}

.ui-tooltip[data-side='bottom']:hover .ui-tooltip__content,
.ui-tooltip[data-side='bottom']:focus-within .ui-tooltip__content {
  transform: translate(-50%, 0);
}

.ui-tooltip[data-side='left'] .ui-tooltip__content {
  top: 50%;
  right: calc(100% + var(--space-2));
  transform: translate(2px, -50%);
}

.ui-tooltip[data-side='left']:hover .ui-tooltip__content,
.ui-tooltip[data-side='left']:focus-within .ui-tooltip__content {
  transform: translate(0, -50%);
}

.ui-tooltip[data-side='right'] .ui-tooltip__content {
  top: 50%;
  left: calc(100% + var(--space-2));
  transform: translate(-2px, -50%);
}

.ui-tooltip[data-side='right']:hover .ui-tooltip__content,
.ui-tooltip[data-side='right']:focus-within .ui-tooltip__content {
  transform: translate(0, -50%);
}
</style>

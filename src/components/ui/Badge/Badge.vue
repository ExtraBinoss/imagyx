<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'neutral' | 'primary' | 'success' | 'warning'
  }>(),
  {
    variant: 'neutral',
  },
)
</script>

<template>
  <span class="ui-badge" :class="`ui-badge--${variant}`"><slot /></span>
</template>

<style scoped>
.ui-badge {
  display: inline-grid;
  place-items: center;
  grid-auto-flow: column;
  gap: var(--space-1);
  height: 22px;
  padding: 0 var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  background: var(--surface);
  color: var(--text-secondary);
  font-size: var(--text-xs);
  font-weight: 600;
  line-height: 1;
  text-align: center;
  white-space: nowrap;
}

.ui-badge--primary {
  border-color: var(--primary-border);
  background: var(--primary-soft);
  color: var(--primary-text);
}

.ui-badge--success {
  border-color: var(--success-border);
  background: var(--success-surface);
  color: var(--success-text);
}

.ui-badge--warning {
  border-color: var(--warning-border);
  background: var(--warning-surface);
  color: var(--warning-text);
}
</style>

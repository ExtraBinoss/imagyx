<script setup lang="ts">
withDefaults(
  defineProps<{
    radius?: 'sm' | 'md' | 'lg'
  }>(),
  {
    radius: 'md',
  },
)
</script>

<template>
  <span class="ui-skeleton" :class="`ui-skeleton--${radius}`" aria-hidden="true" />
</template>

<style scoped>
.ui-skeleton {
  display: block;
  background: var(--skeleton);
  animation: skeleton-pulse 1.35s ease-in-out infinite alternate;
}

.ui-skeleton--sm {
  border-radius: var(--radius-sm);
}

.ui-skeleton--md {
  border-radius: var(--radius-md);
}

.ui-skeleton--lg {
  border-radius: var(--radius-lg);
}

@keyframes skeleton-pulse {
  from {
    opacity: 0.58;
  }
  to {
    opacity: 1;
  }
}

@media (prefers-reduced-motion: reduce) {
  .ui-skeleton {
    animation: none;
  }
}
</style>

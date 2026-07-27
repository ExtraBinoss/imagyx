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
  position: relative;
  display: block;
  overflow: hidden;
  background: var(--skeleton);
}

.ui-skeleton::after {
  position: absolute;
  inset: 0;
  content: '';
  background: radial-gradient(
    circle at 50% 50%,
    color-mix(in srgb, var(--surface) 80%, transparent) 0%,
    color-mix(in srgb, var(--surface) 30%, transparent) 45%,
    transparent 75%
  );
  animation: skeleton-shimmer 1.75s ease-in-out infinite;
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

@keyframes skeleton-shimmer {
  0% {
    transform: translateX(-120%);
    opacity: 0;
  }
  50% {
    opacity: 0.75;
  }
  100% {
    transform: translateX(120%);
    opacity: 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .ui-skeleton::after {
    animation: none;
  }
}
</style>

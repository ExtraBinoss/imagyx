<script setup lang="ts">
import { computed, type CSSProperties } from 'vue'

const props = withDefaults(
  defineProps<{
    borderRadius?: string
    duration?: number
    active?: boolean
    size?: 'sm' | 'md' | 'lg'
  }>(),
  {
    borderRadius: '20px',
    duration: 4200,
    active: true,
    size: 'sm',
  },
)

const styleVariables = computed(() => ({
  '--moving-border-radius': props.borderRadius,
  '--moving-border-duration': `${props.duration}ms`,
} as CSSProperties))
</script>

<template>
  <div
    class="moving-border"
    :class="[
      `moving-border--${size}`,
      { 'moving-border--active': active }
    ]"
    :style="styleVariables"
  >
    <svg
      class="moving-border__outline"
      width="100%"
      height="100%"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <rect class="moving-border__rect moving-border__track" pathLength="100" />
      <rect class="moving-border__rect moving-border__beam" pathLength="100" />
    </svg>
    <div class="moving-border__surface">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.moving-border {
  position: relative;
  isolation: isolate;
  padding: 1px;
  overflow: visible;
  border-radius: var(--moving-border-radius);
}

.moving-border__surface {
  position: relative;
  z-index: 2;
  overflow: hidden;
  border-radius: calc(var(--moving-border-radius) - 1px);
}

.moving-border__outline {
  position: absolute;
  z-index: 1;
  inset: 0;
  overflow: visible;
  pointer-events: none;
}

.moving-border__rect {
  x: 1px;
  y: 1px;
  width: calc(100% - 2px);
  height: calc(100% - 2px);
  rx: calc(var(--moving-border-radius) - 1px);
  ry: calc(var(--moving-border-radius) - 1px);
  fill: none;
  vector-effect: non-scaling-stroke;
  stroke-linecap: round;
}

.moving-border__track {
  stroke: color-mix(in srgb, var(--primary) 28%, var(--border));
  stroke-width: 1.25px;
  opacity: 0.8;
}

.moving-border__beam {
  stroke-dashoffset: 0;
  animation: moving-border-travel var(--moving-border-duration) linear infinite;
  will-change: stroke-dashoffset;
  transition: opacity 220ms cubic-bezier(0.16, 1, 0.3, 1);
}

.moving-border--sm .moving-border__beam {
  stroke: var(--primary);
  stroke-width: 2px;
  stroke-dasharray: 14 86;
  opacity: 0.9;
  filter: drop-shadow(0 0 2px var(--primary));
}

.moving-border--md .moving-border__beam {
  stroke: var(--primary);
  stroke-width: 2.5px;
  stroke-dasharray: 18 82;
  opacity: 0.92;
  filter: drop-shadow(0 0 3px var(--primary));
}

.moving-border--lg .moving-border__beam {
  stroke: var(--primary);
  stroke-width: 3.5px;
  stroke-dasharray: 24 76;
  opacity: 0.95;
  filter: drop-shadow(0 0 4px var(--primary));
}

.moving-border:not(.moving-border--active) .moving-border__beam {
  opacity: 0;
  pointer-events: none;
}

@keyframes moving-border-travel {
  to { stroke-dashoffset: -100; }
}

@media (prefers-reduced-motion: reduce) {
  .moving-border__beam {
    animation: none;
  }
}
</style>

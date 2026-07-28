<script setup lang="ts">
import { computed, type CSSProperties } from 'vue'

const props = withDefaults(
  defineProps<{
    borderRadius?: string
    duration?: number
    active?: boolean
    paused?: boolean
  }>(),
  {
    borderRadius: '20px',
    duration: 4200,
    active: true,
    paused: false,
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
    :class="{
      'moving-border--active': active,
      'moving-border--paused': paused,
    }"
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
      <rect class="moving-border__rect moving-border__aura" pathLength="100" />
      <rect class="moving-border__rect moving-border__beam" pathLength="100" />
      <rect class="moving-border__rect moving-border__spark" pathLength="100" />
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
  stroke: color-mix(in srgb, var(--primary) 25%, var(--border));
  stroke-width: 1.5;
  opacity: 0.8;
}

.moving-border__aura,
.moving-border__beam,
.moving-border__spark {
  stroke-dashoffset: 0;
  animation: moving-border-travel var(--moving-border-duration) linear infinite;
  will-change: stroke-dashoffset, opacity;
  transition: opacity 220ms cubic-bezier(0.16, 1, 0.3, 1);
}

.moving-border__aura {
  stroke: var(--primary);
  stroke-width: 8;
  stroke-dasharray: 20 80;
  filter: blur(5px);
  opacity: 0.35;
}

.moving-border__beam {
  stroke: var(--primary);
  stroke-width: 3.5;
  stroke-dasharray: 20 80;
  opacity: 0.95;
}

.moving-border__spark {
  stroke: var(--primary-text);
  stroke-width: 2.8;
  stroke-dasharray: 10 90;
  animation-delay: calc(var(--moving-border-duration) * -0.02);
  opacity: 0.95;
}

.moving-border:not(.moving-border--active) .moving-border__aura,
.moving-border:not(.moving-border--active) .moving-border__beam,
.moving-border:not(.moving-border--active) .moving-border__spark {
  opacity: 0;
  pointer-events: none;
}

.moving-border--paused .moving-border__aura,
.moving-border--paused .moving-border__beam,
.moving-border--paused .moving-border__spark {
  animation-play-state: paused;
  will-change: auto;
}

.moving-border--paused .moving-border__aura { opacity: 0; }

@keyframes moving-border-travel {
  to { stroke-dashoffset: -100; }
}

@media (prefers-reduced-motion: reduce) {
  .moving-border__aura,
  .moving-border__beam,
  .moving-border__spark {
    animation: none;
  }
}
</style>

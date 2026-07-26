<script setup lang="ts">
import { computed, type CSSProperties } from 'vue'

const props = withDefaults(
  defineProps<{
    borderRadius?: string
    duration?: number
    active?: boolean
  }>(),
  {
    borderRadius: '20px',
    duration: 3200,
    active: true,
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
    :class="{ 'moving-border--active': active }"
    :style="styleVariables"
  >
    <div class="moving-border__aura" aria-hidden="true" />
    <div class="moving-border__clip" aria-hidden="true">
      <div class="moving-border__beam" />
    </div>
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

.moving-border__clip {
  position: absolute;
  z-index: 0;
  inset: 0;
  overflow: hidden;
  border-radius: var(--moving-border-radius);
  pointer-events: none;
}

.moving-border__surface {
  position: relative;
  z-index: 2;
  overflow: hidden;
  border-radius: calc(var(--moving-border-radius) - 1px);
}

.moving-border__beam {
  position: absolute;
  inset: -155%;
  border-radius: 50%;
  background:
    conic-gradient(
      from 0deg,
      transparent 0deg 242deg,
      color-mix(in srgb, var(--primary) 10%, transparent) 264deg,
      color-mix(in srgb, var(--primary) 72%, #93c5fd) 292deg,
      #dbeafe 312deg,
      color-mix(in srgb, var(--primary) 90%, #60a5fa) 331deg,
      transparent 356deg 360deg
    );
  opacity: 0.82;
  animation: moving-border-spin var(--moving-border-duration) linear infinite;
  transform-origin: center;
  will-change: transform;
}

.moving-border__aura {
  position: absolute;
  z-index: -1;
  inset: -13px;
  border-radius: calc(var(--moving-border-radius) + 13px);
  background:
    radial-gradient(circle at 24% 52%, color-mix(in srgb, var(--primary) 22%, transparent), transparent 47%),
    radial-gradient(circle at 78% 46%, rgb(147 197 253 / 0.2), transparent 49%);
  filter: blur(14px);
  opacity: 0.42;
  transform: scale(0.985);
  transition:
    opacity 180ms ease,
    transform 260ms cubic-bezier(0.16, 1, 0.3, 1);
  pointer-events: none;
}

.moving-border--active .moving-border__aura,
.moving-border:focus-within .moving-border__aura {
  opacity: 0.82;
  transform: scale(1.01);
}

.moving-border:not(.moving-border--active) .moving-border__beam {
  animation-play-state: paused;
  opacity: 0.3;
}

@keyframes moving-border-spin {
  to { transform: rotate(1turn); }
}

@media (prefers-reduced-motion: reduce) {
  .moving-border__beam { animation: none; }
}
</style>

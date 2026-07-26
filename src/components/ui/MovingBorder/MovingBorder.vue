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
    <div class="moving-border__beam" aria-hidden="true" />
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

.moving-border__beam {
  position: absolute;
  z-index: 0;
  inset: -130%;
  border-radius: 50%;
  background:
    conic-gradient(
      from 0deg,
      transparent 0deg 245deg,
      color-mix(in srgb, var(--primary) 16%, transparent) 270deg,
      color-mix(in srgb, var(--primary) 92%, white 8%) 300deg,
      color-mix(in srgb, #7c3aed 76%, var(--primary)) 326deg,
      transparent 355deg 360deg
    );
  opacity: 0.72;
  animation: moving-border-spin var(--moving-border-duration) linear infinite;
  will-change: transform;
}

.moving-border__aura {
  position: absolute;
  z-index: -1;
  inset: -15px;
  border-radius: calc(var(--moving-border-radius) + 15px);
  background:
    radial-gradient(circle at 20% 50%, color-mix(in srgb, var(--primary) 20%, transparent), transparent 46%),
    radial-gradient(circle at 80% 50%, rgb(124 58 237 / 0.13), transparent 48%);
  filter: blur(13px);
  opacity: 0.45;
  transform: scale(0.98);
  transition: opacity 180ms ease, transform 260ms cubic-bezier(0.16, 1, 0.3, 1);
  pointer-events: none;
}

.moving-border--active .moving-border__aura,
.moving-border:focus-within .moving-border__aura {
  opacity: 0.9;
  transform: scale(1.015);
}

.moving-border:not(.moving-border--active) .moving-border__beam {
  animation-play-state: paused;
  opacity: 0.28;
}

@keyframes moving-border-spin {
  to { transform: rotate(1turn); }
}

@media (prefers-reduced-motion: reduce) {
  .moving-border__beam { animation: none; }
}
</style>

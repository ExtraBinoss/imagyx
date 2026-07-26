<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    value?: number
    indeterminate?: boolean
    size?: 'sm' | 'md'
    label?: string
  }>(),
  {
    value: 0,
    indeterminate: false,
    size: 'md',
    label: 'Progression',
  },
)

const normalizedValue = computed(() => Math.min(100, Math.max(0, props.value)))
</script>

<template>
  <div
    class="ui-progress"
    :class="[`ui-progress--${size}`, { 'ui-progress--indeterminate': indeterminate }]"
    role="progressbar"
    :aria-label="label"
    :aria-valuemin="indeterminate ? undefined : 0"
    :aria-valuemax="indeterminate ? undefined : 100"
    :aria-valuenow="indeterminate ? undefined : Math.round(normalizedValue)"
  >
    <span class="ui-progress__track">
      <span
        class="ui-progress__value"
        :style="indeterminate ? undefined : { width: `${normalizedValue}%` }"
      />
    </span>
  </div>
</template>

<style scoped>
.ui-progress {
  width: 100%;
}

.ui-progress__track {
  position: relative;
  display: block;
  width: 100%;
  overflow: hidden;
  border-radius: var(--radius-full);
  background: var(--surface-active);
}

.ui-progress--sm .ui-progress__track {
  height: 3px;
}

.ui-progress--md .ui-progress__track {
  height: 6px;
}

.ui-progress__value {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--primary);
  transition: width 160ms ease-out;
}

.ui-progress--indeterminate .ui-progress__value {
  width: 38%;
  animation: progress-indeterminate 1.1s ease-in-out infinite;
}

@keyframes progress-indeterminate {
  0% {
    transform: translateX(-120%);
  }
  100% {
    transform: translateX(330%);
  }
}

@media (prefers-reduced-motion: reduce) {
  .ui-progress__value {
    transition: none;
  }

  .ui-progress--indeterminate .ui-progress__value {
    animation-duration: 2s;
  }
}
</style>

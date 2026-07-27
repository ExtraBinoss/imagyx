<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { computeFloatingPosition, type FloatingSide } from '../floating'

const props = withDefaults(
  defineProps<{
    text: string
    side?: FloatingSide
  }>(),
  {
    side: 'top',
  },
)

const trigger = ref<HTMLElement | null>(null)
const content = ref<HTMLElement | null>(null)
const visible = ref(false)
const positioned = ref(false)
const floatingStyle = ref<Record<string, string>>({})
let resizeObserver: ResizeObserver | null = null

async function show() {
  visible.value = true
  positioned.value = false
  await nextTick()
  updatePosition()
}

function hide() {
  visible.value = false
  positioned.value = false
}

function updatePosition() {
  if (!visible.value || !trigger.value || !content.value) return

  const position = computeFloatingPosition(
    trigger.value.getBoundingClientRect(),
    content.value.getBoundingClientRect(),
    {
      side: props.side,
      align: 'center',
      gap: 8,
      viewportPadding: 8,
    },
  )

  floatingStyle.value = {
    left: `${position.left}px`,
    top: `${position.top}px`,
    maxWidth: `${Math.min(240, position.maxWidth)}px`,
    maxHeight: `${position.maxHeight}px`,
  }
  positioned.value = true
}

function handleViewportChange() {
  if (visible.value) updatePosition()
}

onMounted(() => {
  window.addEventListener('resize', handleViewportChange)
  window.addEventListener('scroll', handleViewportChange, true)
  resizeObserver = new ResizeObserver(handleViewportChange)
  if (trigger.value) resizeObserver.observe(trigger.value)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
  resizeObserver?.disconnect()
})
</script>

<template>
  <span
    ref="trigger"
    class="ui-tooltip"
    @mouseenter="show"
    @mouseleave="hide"
    @focusin="show"
    @focusout="hide"
  >
    <slot />
  </span>

  <Teleport to="body">
    <Transition name="tooltip">
      <span
        v-if="visible"
        ref="content"
        class="ui-tooltip__content"
        :class="{ 'ui-tooltip__content--positioned': positioned }"
        :style="floatingStyle"
        role="tooltip"
      >
        {{ text }}
      </span>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ui-tooltip {
  display: inline-flex;
}

.ui-tooltip__content {
  position: fixed;
  z-index: var(--z-tooltip);
  overflow: hidden;
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
  visibility: hidden;
}

.ui-tooltip__content--positioned {
  visibility: visible;
}

.tooltip-enter-active,
.tooltip-leave-active {
  transition:
    opacity var(--transition-fast),
    transform var(--transition-fast);
}

.tooltip-enter-from,
.tooltip-leave-to {
  opacity: 0;
  transform: translateY(2px);
}
</style>

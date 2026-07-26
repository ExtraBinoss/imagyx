<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { computeFloatingPosition, type FloatingAlign } from '../floating'

const props = withDefaults(
  defineProps<{
    align?: FloatingAlign
    width?: string
  }>(),
  {
    align: 'center',
    width: 'auto',
  },
)

const open = ref(false)
const trigger = ref<HTMLElement | null>(null)
const content = ref<HTMLElement | null>(null)
const positioned = ref(false)
const floatingStyle = ref<Record<string, string>>({})
let resizeObserver: ResizeObserver | null = null

function close() {
  open.value = false
  positioned.value = false
}

async function toggle() {
  open.value = !open.value
  positioned.value = false
  if (open.value) {
    await nextTick()
    updatePosition()
  }
}

function updatePosition() {
  if (!open.value || !trigger.value || !content.value) return

  const anchorRect = trigger.value.getBoundingClientRect()
  const contentRect = content.value.getBoundingClientRect()
  const position = computeFloatingPosition(anchorRect, contentRect, {
    side: 'bottom',
    align: props.align,
    gap: 8,
    viewportPadding: 10,
  })

  floatingStyle.value = {
    left: `${position.left}px`,
    top: `${position.top}px`,
    width: props.width,
    maxWidth: `${position.maxWidth}px`,
    maxHeight: `${position.maxHeight}px`,
  }
  positioned.value = true
}

function handlePointerDown(event: PointerEvent) {
  if (!open.value) return
  const target = event.target as Node
  if (trigger.value?.contains(target) || content.value?.contains(target)) return
  close()
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape') close()
}

function handleViewportChange() {
  if (open.value) updatePosition()
}

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDown)
  document.addEventListener('keydown', handleKeyDown)
  window.addEventListener('resize', handleViewportChange)
  window.addEventListener('scroll', handleViewportChange, true)

  resizeObserver = new ResizeObserver(handleViewportChange)
  if (trigger.value) resizeObserver.observe(trigger.value)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', handlePointerDown)
  document.removeEventListener('keydown', handleKeyDown)
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
  resizeObserver?.disconnect()
})
</script>

<template>
  <span class="ui-popover">
    <span ref="trigger" class="ui-popover__trigger" @click.stop="toggle">
      <slot name="trigger" :open="open" />
    </span>

    <Teleport to="body">
      <Transition name="popover">
        <span
          v-if="open"
          ref="content"
          class="ui-popover__content"
          :class="{ 'ui-popover__content--positioned': positioned }"
          :style="floatingStyle"
          @click.stop
        >
          <slot name="content" :close="close" />
        </span>
      </Transition>
    </Teleport>
  </span>
</template>

<style scoped>
.ui-popover {
  display: inline-flex;
}

.ui-popover__trigger {
  display: inline-flex;
}

.ui-popover__content {
  position: fixed;
  z-index: var(--z-popover);
  min-width: 180px;
  overflow: auto;
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface-elevated);
  color: var(--text);
  box-shadow: var(--shadow-popover);
  visibility: hidden;
}

.ui-popover__content--positioned {
  visibility: visible;
}

.popover-enter-active,
.popover-leave-active {
  transition:
    opacity var(--transition-fast),
    transform var(--transition-fast);
}

.popover-enter-from,
.popover-leave-to {
  opacity: 0;
  transform: translateY(-3px);
}
</style>

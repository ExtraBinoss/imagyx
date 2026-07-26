<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

withDefaults(
  defineProps<{
    align?: 'start' | 'center' | 'end'
    width?: string
  }>(),
  {
    align: 'center',
    width: 'auto',
  },
)

const open = ref(false)
const root = ref<HTMLElement | null>(null)

function close() {
  open.value = false
}

function toggle() {
  open.value = !open.value
}

function handlePointerDown(event: PointerEvent) {
  if (!open.value || !root.value) return
  if (!root.value.contains(event.target as Node)) close()
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape') close()
}

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDown)
  document.addEventListener('keydown', handleKeyDown)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', handlePointerDown)
  document.removeEventListener('keydown', handleKeyDown)
})
</script>

<template>
  <span ref="root" class="ui-popover">
    <span class="ui-popover__trigger" @click.stop="toggle">
      <slot name="trigger" :open="open" />
    </span>
    <Transition name="popover">
      <span
        v-if="open"
        class="ui-popover__content"
        :data-align="align"
        :style="{ width }"
        @click.stop
      >
        <slot name="content" :close="close" />
      </span>
    </Transition>
  </span>
</template>

<style scoped>
.ui-popover {
  position: relative;
  display: inline-flex;
}

.ui-popover__trigger {
  display: inline-flex;
}

.ui-popover__content {
  position: absolute;
  top: calc(100% + var(--space-2));
  z-index: var(--z-popover);
  min-width: 180px;
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface-elevated);
  color: var(--text);
  box-shadow: var(--shadow-popover);
}

.ui-popover__content[data-align='start'] {
  left: 0;
}

.ui-popover__content[data-align='center'] {
  left: 50%;
  transform: translateX(-50%);
}

.ui-popover__content[data-align='end'] {
  right: 0;
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

.ui-popover__content[data-align='center'].popover-enter-from,
.ui-popover__content[data-align='center'].popover-leave-to {
  transform: translate(-50%, -3px);
}
</style>

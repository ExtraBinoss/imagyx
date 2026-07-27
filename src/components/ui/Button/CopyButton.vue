<script setup lang="ts">
import { ref } from 'vue'
import { Check, Copy } from '@lucide/vue'
import Button from './Button.vue'

const props = withDefaults(
  defineProps<{
    copiedText?: string
    idleText?: string
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    size?: 'sm' | 'md' | 'lg' | 'icon'
    block?: boolean
    iconOnly?: boolean
  }>(),
  {
    copiedText: 'Copied',
    idleText: 'Copy',
    variant: 'secondary',
    size: 'sm',
    block: false,
    iconOnly: false,
  },
)

const emit = defineEmits<{
  copy: []
}>()

const copied = ref(false)
let timer: number | undefined

function triggerCopied() {
  copied.value = true
  if (timer) window.clearTimeout(timer)
  timer = window.setTimeout(() => {
    copied.value = false
  }, 1800)
}

function handleClick(event: MouseEvent) {
  event.stopPropagation()
  triggerCopied()
  emit('copy')
}

defineExpose({ triggerCopied, copied })
</script>

<template>
  <Button
    :variant="copied ? 'primary' : variant"
    :size="size"
    :block="block"
    :class="{ 'copy-button--copied': copied }"
    class="copy-button"
    @click="handleClick"
  >
    <template v-if="!iconOnly && $slots.leading" #leading>
      <slot name="leading" />
    </template>
    <template v-else-if="!iconOnly" #leading>
      <Transition name="copy-icon" mode="out-in">
        <Check v-if="copied" key="check" :size="14" class="copy-icon-check" />
        <Copy v-else key="copy" :size="14" />
      </Transition>
    </template>

    <template v-if="iconOnly">
      <Transition name="copy-icon" mode="out-in">
        <Check v-if="copied" key="check" :size="14" class="copy-icon-check" />
        <Copy v-else key="copy" :size="14" />
      </Transition>
    </template>
    <span v-else>
      {{ copied ? copiedText : idleText }}
    </span>

    <template v-if="$slots.trailing" #trailing>
      <slot name="trailing" />
    </template>
  </Button>
</template>

<style scoped>
.copy-button {
  transition: all 180ms cubic-bezier(0.16, 1, 0.3, 1);
}
.copy-button :deep(.ui-button__content) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}
.copy-button--copied {
  animation: copySuccessPop 280ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
.copy-icon-check {
  color: var(--primary-foreground);
}
@keyframes copySuccessPop {
  0% { transform: scale(0.94); }
  50% { transform: scale(1.05); }
  100% { transform: scale(1); }
}
.copy-icon-enter-active,
.copy-icon-leave-active {
  transition: transform 120ms ease, opacity 120ms ease;
}
.copy-icon-enter-from { opacity: 0; transform: scale(0.7); }
.copy-icon-leave-to { opacity: 0; transform: scale(0.7); }
</style>

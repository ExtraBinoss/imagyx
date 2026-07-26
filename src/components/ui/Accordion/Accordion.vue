<script setup lang="ts">
import { ref } from 'vue'
import { ChevronDown } from '@lucide/vue'

const props = withDefaults(
  defineProps<{
    title?: string
    defaultOpen?: boolean
  }>(),
  {
    title: '',
    defaultOpen: false,
  },
)

const isOpen = ref(props.defaultOpen)

function toggle() {
  isOpen.value = !isOpen.value
}
</script>

<template>
  <div class="ui-accordion" :class="{ 'ui-accordion--open': isOpen }">
    <button
      type="button"
      class="ui-accordion__trigger"
      :aria-expanded="isOpen"
      @click="toggle"
    >
      <span class="ui-accordion__title">
        <slot name="title">{{ title }}</slot>
      </span>
      <ChevronDown class="ui-accordion__chevron" :size="16" />
    </button>
    <div v-show="isOpen" class="ui-accordion__content">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.ui-accordion {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface);
  overflow: hidden;
  transition: border-color var(--transition-fast);
}

.ui-accordion__trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 14px;
  border: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: var(--text-xs);
  font-weight: 600;
  cursor: pointer;
  text-align: left;
  transition: background-color var(--transition-fast);
}

.ui-accordion__trigger:hover {
  background: var(--surface-hover);
}

.ui-accordion__title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.ui-accordion__chevron {
  color: var(--text-muted);
  transition: transform 200ms ease;
}

.ui-accordion--open .ui-accordion__chevron {
  transform: rotate(180deg);
}

.ui-accordion__content {
  padding: 12px 14px;
  border-top: 1px solid var(--border);
  background: color-mix(in srgb, var(--surface) 96%, transparent);
}
</style>

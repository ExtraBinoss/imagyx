<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    shortcut: string // e.g. "Ctrl+C", "Cmd+E", "Ctrl+Shift+P", "Enter"
    size?: 'sm' | 'md'
  }>(),
  {
    size: 'sm',
  },
)

const isMac = computed(() => {
  if (typeof navigator === 'undefined') return false
  return /mac/i.test(navigator.userAgent || navigator.platform || '')
})

interface TokenInfo {
  label: string
  isSymbol?: boolean
}

const parsedTokens = computed<TokenInfo[]>(() => {
  if (!props.shortcut) return []
  return props.shortcut
    .split('+')
    .map((raw) => raw.trim())
    .filter(Boolean)
    .map((token) => {
      const lower = token.toLowerCase()

      // Modifiers
      if (lower === 'ctrl' || lower === 'control') {
        return isMac.value
          ? { label: '⌃', isSymbol: true }
          : { label: 'Ctrl' }
      }
      if (lower === 'cmd' || lower === 'command' || lower === 'super' || lower === 'meta') {
        return isMac.value
          ? { label: '⌘', isSymbol: true }
          : { label: 'Win' }
      }
      if (lower === 'alt' || lower === 'option') {
        return isMac.value
          ? { label: '⌥', isSymbol: true }
          : { label: 'Alt' }
      }
      if (lower === 'shift') {
        return isMac.value
          ? { label: '⇧', isSymbol: true }
          : { label: 'Shift' }
      }

      // Keys
      if (lower === 'enter' || lower === 'return') {
        return isMac.value
          ? { label: '↵', isSymbol: true }
          : { label: '↵' }
      }
      if (lower === 'escape' || lower === 'esc') {
        return { label: 'Esc' }
      }
      if (lower === 'space') {
        return { label: 'Space' }
      }
      if (lower.startsWith('arrow')) {
        const dir = lower.replace('arrow', '')
        if (dir === 'up') return { label: '↑', isSymbol: true }
        if (dir === 'down') return { label: '↓', isSymbol: true }
        if (dir === 'left') return { label: '←', isSymbol: true }
        if (dir === 'right') return { label: '→', isSymbol: true }
      }

      // Default single char key or word
      return { label: token.toUpperCase() }
    })
})
</script>

<template>
  <span class="kbd-chip" :class="[`kbd-chip--${size}`]">
    <template v-for="(item, index) in parsedTokens" :key="index">
      <kbd class="kbd-chip__key" :class="{ 'kbd-chip__key--symbol': item.isSymbol }">
        {{ item.label }}
      </kbd>
      <span v-if="index < parsedTokens.length - 1" class="kbd-chip__plus">+</span>
    </template>
  </span>
</template>

<style scoped>
.kbd-chip {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  user-select: none;
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  line-height: 1;
}

.kbd-chip__key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  font-family: inherit;
  font-weight: 650;
  line-height: 1;
  background: color-mix(in srgb, var(--surface) 80%, black 20%);
  color: var(--text-muted);
  border: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
  box-shadow: 0 1px 1px rgb(0 0 0 / 0.15);
}

.kbd-chip__key--symbol {
  font-size: 1.15em;
  font-weight: 500;
}

.kbd-chip__plus {
  font-size: 9px;
  font-weight: 600;
  color: var(--text-subtle);
  opacity: 0.7;
  padding-inline: 1px;
}

/* Sizes */
.kbd-chip--sm .kbd-chip__key {
  min-width: 15px;
  height: 15px;
  padding: 0 4px;
  font-size: 9px;
}

.kbd-chip--md .kbd-chip__key {
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  font-size: 11px;
}
</style>

<script setup lang="ts">
import { ref } from 'vue'
import { ArrowLeft, LoaderCircle, Search, Settings2 } from '@lucide/vue'
import Button from '../ui/Button/Button.vue'
import type { SpotlightView } from './types'

const props = defineProps<{
  modelValue: string
  view: SpotlightView
  placeholder: string
  searching: boolean
  resultLabel: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  settings: []
  back: []
}>()

const input = ref<HTMLInputElement | null>(null)

function focus() { input.value?.focus() }
function select() { input.value?.select() }

defineExpose({ focus, select })
</script>

<template>
  <div class="spotlight-input">
    <Button
      v-if="view === 'settings'"
      class="spotlight-input__nav"
      variant="ghost"
      size="icon"
      aria-label="Retour à la recherche"
      @click="emit('back')"
    >
      <ArrowLeft :size="18" />
    </Button>
    <Search v-else class="spotlight-input__search-icon" :size="22" :stroke-width="1.9" />

    <input
      ref="input"
      :value="modelValue"
      type="search"
      autocomplete="off"
      spellcheck="false"
      :placeholder="placeholder"
      :aria-label="view === 'settings' ? 'Rechercher dans les réglages' : 'Recherche rapide'"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />

    <span v-if="view === 'search' && modelValue.trim() && !searching" class="spotlight-input__count">
      {{ resultLabel }}
    </span>
    <LoaderCircle v-if="view === 'search' && searching" class="spin" :size="18" />
    <Button
      v-if="view === 'search'"
      class="spotlight-input__settings"
      variant="ghost"
      size="icon"
      aria-label="Ouvrir les réglages"
      @click="emit('settings')"
    >
      <Settings2 :size="17" />
    </Button>
  </div>
</template>

<style scoped>
.spotlight-input {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 70px;
  padding: 0 15px 0 18px;
  color: var(--text-muted);
}
.spotlight-input__search-icon { flex: 0 0 auto; }
.spotlight-input__nav { margin-left: -5px; }
.spotlight-input input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 20px;
  font-weight: 570;
  letter-spacing: -0.38px;
  caret-color: var(--primary);
}
.spotlight-input input::-webkit-search-cancel-button { display: none; }
.spotlight-input input::placeholder { color: var(--text-subtle); opacity: 1; }
.spotlight-input__count {
  flex: 0 0 auto;
  padding: 5px 9px;
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  color: var(--text-muted);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}
.spotlight-input__settings { flex: 0 0 auto; }
.spin { animation: spin 0.85s linear infinite; }
@keyframes spin { to { transform: rotate(1turn); } }
</style>

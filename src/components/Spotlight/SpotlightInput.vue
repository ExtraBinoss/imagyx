<script setup lang="ts">
import { ref } from 'vue'
import { ArrowLeft, LoaderCircle, Search, Settings2, X } from '@lucide/vue'
import Button from '../ui/Button/Button.vue'
import type { SpotlightView } from './types'
import { useTranslate } from '../../i18n'

defineProps<{
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

const { t } = useTranslate()
const input = ref<HTMLInputElement | null>(null)

function focus() { input.value?.focus() }
function select() { input.value?.select() }
function handleInput(event: Event) {
  emit('update:modelValue', (event.target as HTMLInputElement).value)
}

defineExpose({ focus, select })
</script>

<template>
  <div class="spotlight-input">
    <Button
      v-if="view === 'settings'"
      class="spotlight-input__nav"
      variant="ghost"
      size="icon"
      :aria-label="t('spotlight.back_to_search')"
      @click="emit('back')"
    >
      <ArrowLeft :size="18" />
    </Button>
    <div v-else class="spotlight-input__icon-wrapper">
      <Transition name="icon-swap" mode="out-in">
        <LoaderCircle v-if="searching" key="loader" class="spotlight-input__search-icon spin" :size="22" :stroke-width="2.2" />
        <Search v-else key="search" class="spotlight-input__search-icon" :size="22" :stroke-width="1.9" />
      </Transition>
    </div>

    <input
      ref="input"
      :value="modelValue"
      type="search"
      autocomplete="off"
      spellcheck="false"
      :placeholder="placeholder"
      :aria-label="view === 'settings' ? t('spotlight.search_in_settings') : t('spotlight.quick_search')"
      @input="handleInput"
    />

    <span v-if="view === 'search' && modelValue.trim()" class="spotlight-input__count">
      {{ resultLabel }}
    </span>
    <Button
      v-if="modelValue"
      class="spotlight-input__clear"
      variant="ghost"
      size="icon"
      :aria-label="t('spotlight.clear_search')"
      @click="emit('update:modelValue', '')"
    >
      <X :size="16" />
    </Button>
    <Button
      v-if="view === 'search'"
      class="spotlight-input__settings"
      variant="ghost"
      size="icon"
      :aria-label="t('spotlight.open_settings')"
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
  min-width: 78px;
  text-align: center;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  color: var(--text-muted);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  transition: width 150ms ease, opacity 120ms ease;
}
.spotlight-input__icon-wrapper { flex: 0 0 22px; display: flex; align-items: center; justify-content: center; }
.spotlight-input__settings { flex: 0 0 auto; }
.icon-swap-enter-active,
.icon-swap-leave-active { transition: opacity 140ms ease, transform 140ms ease; }
.icon-swap-enter-from { opacity: 0; transform: scale(0.85); }
.icon-swap-leave-to { opacity: 0; transform: scale(0.85); }
@keyframes spin { to { transform: rotate(1turn); } }
</style>

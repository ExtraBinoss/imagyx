<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { Check, ChevronDown, Search, X } from '@lucide/vue'
import Popover from '../Popover/Popover.vue'
import { useTranslate } from '../../../i18n'

export interface SearchOption {
  label: string
  value: string
  searchText?: string
}

const props = withDefaults(
  defineProps<{
    modelValue?: string
    options: SearchOption[]
    placeholder?: string
    searchPlaceholder?: string
    ariaLabel?: string
    popoverWidth?: string
  }>(),
  {
    modelValue: '',
    placeholder: '',
    searchPlaceholder: '',
    ariaLabel: undefined,
    popoverWidth: '240px',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const { t } = useTranslate()

const search = ref('')
const highlightedIndex = ref(-1)
const optionsContainer = ref<HTMLElement | null>(null)

const selected = computed(() => props.options.find((o) => o.value === props.modelValue))

const normalizedSearch = computed(() => search.value.toLocaleLowerCase())

const filteredOptions = computed(() => {
  const q = normalizedSearch.value
  if (!q) return props.options
  return props.options.filter((o) => {
    const text = o.searchText ?? o.label
    return text.toLocaleLowerCase().includes(q)
  })
})

function onToggle(open: boolean) {
  if (open) {
    search.value = ''
    highlightedIndex.value = -1
    void nextTick(() => {
      const input = document.querySelector<HTMLInputElement>('.search-select__input')
      input?.focus()
    })
  } else {
    highlightedIndex.value = -1
  }
}

function onSelect(value: string, close: () => void) {
  emit('update:modelValue', value)
  close()
}

function onSearchInput(event: Event) {
  search.value = (event.target as HTMLInputElement).value
  highlightedIndex.value = filteredOptions.value.length > 0 ? 0 : -1
}

function clearSearch() {
  search.value = ''
  highlightedIndex.value = -1
}

function handleKeyDown(event: KeyboardEvent, close: () => void) {
  const options = filteredOptions.value
  if (options.length === 0) return

  if (event.key === 'ArrowDown') {
    event.preventDefault()
    highlightedIndex.value = (highlightedIndex.value + 1) % options.length
    scrollHighlighted()
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    highlightedIndex.value = highlightedIndex.value <= 0 ? options.length - 1 : highlightedIndex.value - 1
    scrollHighlighted()
  } else if (event.key === 'Enter') {
    event.preventDefault()
    const idx = highlightedIndex.value >= 0 ? highlightedIndex.value : 0
    if (options[idx]) {
      onSelect(options[idx].value, close)
    }
  } else if (event.key === 'Escape' && search.value) {
    event.preventDefault()
    event.stopPropagation()
    clearSearch()
  }
}

function scrollHighlighted() {
  void nextTick(() => {
    if (!optionsContainer.value) return
    const item = optionsContainer.value.children[highlightedIndex.value] as HTMLElement | null
    item?.scrollIntoView({ block: 'nearest' })
  })
}

watch(
  () => props.modelValue,
  () => {
    highlightedIndex.value = -1
  },
)
</script>

<template>
  <Popover align="start" :width="popoverWidth" @update:open="onToggle">
    <template #trigger="{ open }">
      <button
        class="search-select__trigger"
        :class="{ 'search-select__trigger--open': open }"
        :aria-label="ariaLabel || placeholder"
      >
        <span class="search-select__value" :class="{ 'search-select__value--placeholder': !selected }">
          {{ selected?.label ?? placeholder }}
        </span>
        <ChevronDown :size="14" class="search-select__chevron" :class="{ 'search-select__chevron--open': open }" />
      </button>
    </template>
    <template #content="{ close }">
      <div class="search-select__dropdown" @keydown="handleKeyDown($event, close)">
        <div class="search-select__search">
          <Search :size="14" class="search-select__search-icon" />
          <input
            :value="search"
            class="search-select__input"
            :placeholder="searchPlaceholder || t('search.placeholder')"
            @input="onSearchInput"
          />
          <button
            v-if="search"
            class="search-select__clear"
            @click="clearSearch"
          >
            <X :size="13" />
          </button>
        </div>
        <div ref="optionsContainer" class="search-select__options" role="listbox">
          <button
            v-for="(option, idx) in filteredOptions"
            :key="option.value"
            class="search-select__option"
            :class="{
              'search-select__option--highlighted': idx === highlightedIndex,
              'search-select__option--selected': option.value === modelValue,
            }"
            role="option"
            :aria-selected="option.value === modelValue"
            @click="onSelect(option.value, close)"
            @mouseenter="highlightedIndex = idx"
          >
            <span class="search-select__option-label">{{ option.label }}</span>
            <Check v-if="option.value === modelValue" :size="14" class="search-select__check" />
          </button>
          <div v-if="filteredOptions.length === 0" class="search-select__empty">
            {{ t('search.no_images_desc') }}
          </div>
        </div>
      </div>
    </template>
  </Popover>
</template>

<style scoped>
.search-select__trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  width: 100%;
  min-height: 34px;
  padding: 0 var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
  transition: border-color var(--transition-fast), background-color var(--transition-fast);
}
.search-select__trigger:hover {
  border-color: var(--border-strong);
  background: var(--surface-hover);
}
.search-select__trigger--open {
  border-color: var(--primary);
  outline: 2px solid var(--focus-ring-soft);
  outline-offset: 1px;
}
.search-select__value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.search-select__value--placeholder {
  color: var(--text-subtle);
}
.search-select__chevron {
  flex-shrink: 0;
  color: var(--text-muted);
  transition: transform var(--transition-fast);
}
.search-select__chevron--open {
  transform: rotate(180deg);
}
.search-select__dropdown {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.search-select__search {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2);
  border-bottom: 1px solid var(--border);
}
.search-select__search-icon {
  flex-shrink: 0;
  color: var(--text-muted);
}
.search-select__input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
}
.search-select__input::placeholder {
  color: var(--text-subtle);
}
.search-select__clear {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  flex-shrink: 0;
}
.search-select__clear:hover {
  background: var(--surface-hover);
  color: var(--text);
}
.search-select__options {
  display: grid;
  gap: var(--space-1);
  max-height: 240px;
  overflow-y: auto;
  padding: var(--space-1);
  align-content: start;
}
.search-select__option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
}
.search-select__option:hover,
.search-select__option--highlighted {
  background: var(--surface-hover);
  color: var(--text);
}
.search-select__option--selected {
  color: var(--text);
}
.search-select__option-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.search-select__check {
  flex-shrink: 0;
  color: var(--primary-text);
}
.search-select__empty {
  padding: var(--space-6) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-align: center;
}
</style>

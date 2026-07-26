<script setup lang="ts">
import { ref } from 'vue'
import { Search, Sparkles, X } from '@lucide/vue'
import Badge from './ui/Badge/Badge.vue'
import Button from './ui/Button/Button.vue'
import Input from './ui/Input/Input.vue'
import Select from './ui/Select/Select.vue'
import { useThemeStore, type ThemeMode } from '../stores/theme'

const props = defineProps<{
  modelReady: boolean
  modelBackend: string
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const searchInput = ref<{ focus: () => void; select: () => void } | null>(null)
const theme = useThemeStore()
const themeOptions = [
  { label: 'Système', value: 'system', description: 'Suit macOS ou Windows' },
  { label: 'Clair', value: 'light', description: 'Interface blanche' },
  { label: 'Sombre', value: 'dark', description: 'Interface sombre' },
]

function setTheme(value: string) {
  theme.setMode(value as ThemeMode)
}

function focusSearch() {
  searchInput.value?.focus()
}

function selectSearch() {
  searchInput.value?.select()
}

defineExpose({ focusSearch, selectSearch })
</script>

<template>
  <header class="search-header" data-tauri-drag-region>
    <div class="search-field">
      <Input
        ref="searchInput"
        :model-value="props.modelValue"
        type="search"
        placeholder="Nom de fichier ou description naturelle…"
        aria-label="Rechercher des images"
        @update:model-value="emit('update:modelValue', $event)"
      >
        <template #leading><Search :size="18" /></template>
        <template #trailing>
          <Button
            v-if="props.modelValue"
            variant="ghost"
            size="icon"
            aria-label="Effacer la recherche"
            @click="emit('update:modelValue', '')"
          >
            <X :size="15" />
          </Button>
        </template>
      </Input>
    </div>
  </header>
</template>

<script setup lang="ts">
import { Search, Sparkles, X } from '@lucide/vue'
import Badge from './ui/Badge/Badge.vue'
import Button from './ui/Button/Button.vue'
import Input from './ui/Input/Input.vue'
import Select, { type SelectOption } from './ui/Select/Select.vue'
import Tooltip from './ui/Tooltip/Tooltip.vue'
import { useThemeStore, type ThemeMode } from '../stores/theme'

const props = defineProps<{
  modelReady: boolean
  modelBackend: string
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const theme = useThemeStore()
const themeOptions: SelectOption[] = [
  { label: 'Système', value: 'system', description: 'Suit macOS ou Windows' },
  { label: 'Clair', value: 'light', description: 'Interface blanche' },
  { label: 'Sombre', value: 'dark', description: 'Interface sombre' },
]

function setTheme(value: string) {
  theme.setMode(value as ThemeMode)
}
</script>

<template>
  <header class="search-header">
    <div class="search-field">
      <Input
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

    <div class="header-actions">
      <Tooltip :text="props.modelBackend" side="bottom">
        <Badge :variant="props.modelReady ? 'success' : 'warning'">
          <Sparkles :size="14" />
          {{ props.modelReady ? props.modelBackend : 'IA en préparation' }}
        </Badge>
      </Tooltip>
      <Select
        :model-value="theme.mode"
        :options="themeOptions"
        aria-label="Thème de l’interface"
        @update:model-value="setTheme"
      />
    </div>
  </header>
</template>

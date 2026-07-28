<script setup lang="ts">
import { ref } from 'vue'
import MovingBorder from './ui/MovingBorder/MovingBorder.vue'
import TitleBar from './TitleBar.vue'
import UnifiedSearchInput from './UnifiedSearchInput.vue'
import { usePlatformStore } from '../stores/platform'
import { useTranslate } from '../i18n'
import type { FollowedFolder } from '../types'

const props = defineProps<{
  modelReady: boolean
  modelBackend: string
  modelValue: string
  searching?: boolean
  resultCount?: number
  placeholder?: string
  folderSuggestions?: FollowedFolder[]
  folderSuggestionIndex?: number
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  folderSelect: [folder: FollowedFolder]
  folderNavigate: [delta: number]
}>()

const platform = usePlatformStore()
const { t } = useTranslate()
const searchInput = ref<InstanceType<typeof UnifiedSearchInput> | null>(null)
const isFocused = ref(false)

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
    <div v-if="platform.controlsPosition === 'right'" class="search-header-controls">
      <TitleBar />
    </div>

    <div class="search-field">
      <MovingBorder
        border-radius="14px"
        :duration="searching ? 2600 : 4200"
        :active="isFocused"
      >
        <UnifiedSearchInput
          ref="searchInput"
          variant="app"
          view="search"
          :model-value="props.modelValue"
          :placeholder="placeholder ?? t('search.placeholder')"
          :searching="props.searching"
          :result-label="props.modelValue.trim() && props.resultCount != null
            ? (props.searching ? t('search.searching') : t('search.result_count', { count: props.resultCount }))
            : ''"
          :folder-suggestions="folderSuggestions ?? []"
          :folder-suggestion-index="folderSuggestionIndex ?? 0"
          @focusin="isFocused = true"
          @focusout="isFocused = false"
          @update:model-value="emit('update:modelValue', $event)"
          @folder-select="emit('folderSelect', $event)"
          @folder-navigate="emit('folderNavigate', $event)"
        />
      </MovingBorder>
    </div>
  </header>
</template>

<style scoped>
.search-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  min-height: 76px;
  padding: 14px 24px;
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--background) 92%, transparent) 0%,
    color-mix(in srgb, var(--background) 60%, transparent) 60%,
    transparent 100%
  );
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  -webkit-app-region: drag;
}
.search-header-controls {
  position: absolute;
  top: 0;
  right: 12px;
  z-index: 10;
  display: flex;
  align-items: center;
  height: 38px;
  pointer-events: auto;
  -webkit-app-region: no-drag;
}
.search-field {
  position: relative;
  width: 100%;
  max-width: 640px;
  pointer-events: auto;
  -webkit-app-region: no-drag;
}
</style>

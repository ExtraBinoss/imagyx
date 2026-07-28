<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { FollowedFolder } from '../../types'
import type { SpotlightView } from './types'
import { useTranslate } from '../../i18n'
import { imagyxApi } from '../../api/tauri'
import { spotlightSearchPagination } from '../../services/spotlight-search-pagination'
import UnifiedSearchInput from '../UnifiedSearchInput.vue'

const props = withDefaults(defineProps<{
  modelValue: string
  view: SpotlightView
  placeholder: string
  searching: boolean
  resultLabel: string
  folderSuggestions: FollowedFolder[]
  folderSuggestionIndex: number
}>(), {
  folderSuggestions: () => [],
  folderSuggestionIndex: 0,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  settings: []
  back: []
  folderSelect: [folder: FollowedFolder]
  folderNavigate: [delta: number]
}>()

const { t } = useTranslate()
const input = ref<InstanceType<typeof UnifiedSearchInput> | null>(null)
const displayedResultLabel = computed(() => {
  if (
    props.view !== 'search'
    || !props.modelValue.trim()
    || !spotlightSearchPagination.hasKnownTotal.value
  ) return props.resultLabel
  return t('spotlight.result_count', { count: spotlightSearchPagination.totalResults.value })
})

watch(
  () => [props.view, props.modelValue] as const,
  ([view, value]) => {
    spotlightSearchPagination.setRawQuery(view === 'search' ? value : '')
  },
  { immediate: true },
)

function focus() { input.value?.focus() }
function select() { input.value?.select() }

function handleDocumentScroll(event: Event): void {
  if (props.view !== 'search' || !props.modelValue.trim()) return
  const target = event.target
  if (!(target instanceof HTMLElement) || !target.classList.contains('spotlight-results')) return
  const remainingPx = target.scrollHeight - target.scrollTop - target.clientHeight
  if (remainingPx > 72 * 8) return
  void spotlightSearchPagination.loadMore(imagyxApi.searchPage)
}

onMounted(() => {
  document.addEventListener('scroll', handleDocumentScroll, true)
})

onBeforeUnmount(() => {
  document.removeEventListener('scroll', handleDocumentScroll, true)
})

defineExpose({ focus, select })
</script>

<template>
  <div class="spotlight-input">
    <UnifiedSearchInput
      ref="input"
      variant="spotlight"
      :model-value="modelValue"
      :view="view"
      :placeholder="placeholder"
      :searching="searching"
      :result-label="displayedResultLabel"
      :show-settings="true"
      :folder-suggestions="folderSuggestions"
      :folder-suggestion-index="folderSuggestionIndex"
      @update:model-value="emit('update:modelValue', $event)"
      @settings="emit('settings')"
      @back="emit('back')"
      @folder-select="emit('folderSelect', $event)"
      @folder-navigate="emit('folderNavigate', $event)"
    />
  </div>
</template>

<style scoped>
.spotlight-input {
  position: relative;
  z-index: 3;
  width: 100%;
  min-height: 70px;
}
</style>

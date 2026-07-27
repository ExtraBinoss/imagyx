<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  Check,
  Copy,
  ExternalLink,
  FolderOpen,
  FolderPlus,
  LoaderCircle,
  Search,
} from '@lucide/vue'
import type { ImageAsset } from '../../types'
import ThumbnailImage from '../ThumbnailImage.vue'
import Button from '../ui/Button/Button.vue'
import KbdChip from '../ui/KbdChip/KbdChip.vue'
import SpotlightIndexProgress from './SpotlightIndexProgress.vue'
import type { SpotlightIndexJob } from './types'
import { useTranslate } from '../../i18n'

const props = defineProps<{
  results: ImageAsset[]
  selectedIndex: number
  searching: boolean
  hasSearchQuery: boolean
  error: string | null
  copiedImageId: string | null
  copyingImageId: string | null
  revealingImageId: string | null
  openingImageId: string | null
  showAddAction: boolean
  hasFolders: boolean
  libraryReady: boolean
  showBackgroundHint: boolean
  jobs: SpotlightIndexJob[]
  fileManagerName: string
}>()

const emit = defineEmits<{
  select: [index: number]
  open: [image: ImageAsset]
  copy: [image: ImageAsset]
  reveal: [image: ImageAsset]
  addFolder: []
}>()

const { t } = useTranslate()
const RESULT_ROW_HEIGHT = 72
const RESULT_OVERSCAN = 4
const viewport = ref<HTMLElement | null>(null)
const resultList = ref<HTMLElement | null>(null)
const canScrollDown = ref(false)
const scrollTop = ref(0)
const viewportHeight = ref(0)
const resultsOffset = ref(0)
let scrollFrame: number | undefined
let resizeObserver: ResizeObserver | null = null

const visibleResults = computed(() => {
  const firstVisible = Math.floor(
    Math.max(0, scrollTop.value - resultsOffset.value) / RESULT_ROW_HEIGHT,
  )
  const visibleCount = Math.ceil(viewportHeight.value / RESULT_ROW_HEIGHT)
  const start = Math.max(0, firstVisible - RESULT_OVERSCAN)
  const end = Math.min(props.results.length, firstVisible + visibleCount + RESULT_OVERSCAN)
  return props.results.slice(start, end).map((image, offset) => ({
    image,
    index: start + offset,
  }))
})

watch(
  () => [
    props.results.length,
    props.jobs.length,
    props.showAddAction,
    props.hasFolders,
    props.libraryReady,
    props.showBackgroundHint,
    props.searching,
    props.hasSearchQuery,
  ],
  () => { void nextTick(scheduleScrollState) },
)

function updateScrollState() {
  const element = viewport.value
  if (!element) {
    canScrollDown.value = false
    return
  }
  scrollTop.value = element.scrollTop
  viewportHeight.value = element.clientHeight
  resultsOffset.value = resultList.value?.offsetTop ?? 0
  canScrollDown.value = element.scrollHeight > element.clientHeight + 2
    && element.scrollTop + element.clientHeight < element.scrollHeight - 2
}

function scheduleScrollState() {
  if (scrollFrame) return
  scrollFrame = window.requestAnimationFrame(() => {
    scrollFrame = undefined
    updateScrollState()
  })
}

function scrollToIndex(index: number) {
  void nextTick(() => {
    const element = viewport.value
    const list = resultList.value
    if (!element || !list) return
    const rowTop = list.offsetTop + index * RESULT_ROW_HEIGHT
    const rowBottom = rowTop + RESULT_ROW_HEIGHT
    if (rowTop < element.scrollTop) element.scrollTop = rowTop
    else if (rowBottom > element.scrollTop + element.clientHeight) {
      element.scrollTop = rowBottom - element.clientHeight
    }
    scheduleScrollState()
  })
}

onMounted(() => {
  resizeObserver = new ResizeObserver(scheduleScrollState)
  if (viewport.value) resizeObserver.observe(viewport.value)
  scheduleScrollState()
})

onBeforeUnmount(() => {
  if (scrollFrame) window.cancelAnimationFrame(scrollFrame)
  resizeObserver?.disconnect()
})

defineExpose({ scrollToIndex })
</script>

<template>
  <div class="spotlight-results-shell">
    <div ref="viewport" class="spotlight-results" role="listbox" @scroll.passive="scheduleScrollState">
      <div v-if="!libraryReady" class="spotlight-library-loading" aria-live="polite">
        <LoaderCircle class="spin" :size="22" />
        <strong>{{ t('spotlight.loading_library') }}</strong>
      </div>

      <section v-else-if="!hasFolders" class="spotlight-library-empty" aria-labelledby="spotlight-library-empty-title">
        <span class="spotlight-library-empty__icon"><FolderPlus :size="24" /></span>
        <strong id="spotlight-library-empty-title">{{ t('spotlight.no_folder_title') }}</strong>
        <p>{{ t('spotlight.no_folder_desc') }}</p>
        <Button
          class="spotlight-library-empty__button"
          variant="primary"
          size="lg"
          @click="emit('addFolder')"
        >
          <template #leading><FolderPlus :size="17" /></template>
          {{ t('spotlight.no_folder_action') }}
        </Button>
        <small>{{ t('spotlight.no_folder_privacy') }}</small>
      </section>

      <template v-else>
        <Button
          v-if="showAddAction"
          class="spotlight-add-folder"
          variant="secondary"
          size="lg"
          block
          @click="emit('addFolder')"
        >
          <template #leading><FolderPlus :size="18" /></template>
          <span class="spotlight-add-folder__copy">
            <strong>{{ t('spotlight.add_folder_title') }}</strong>
            <small>{{ t('spotlight.add_folder_desc') }}</small>
          </span>
        </Button>

        <aside v-if="showBackgroundHint" class="spotlight-background-hint" aria-live="polite">
          <span class="spotlight-background-hint__icon"><LoaderCircle class="spin" :size="16" /></span>
          <span>
            <strong>{{ t('spotlight.indexing_results_title') }}</strong>
            <small>{{ t('spotlight.indexing_results_desc') }}</small>
          </span>
        </aside>

        <SpotlightIndexProgress v-for="job in jobs" :key="job.folderId" :job="job" />

        <div
          v-if="results.length"
          ref="resultList"
          class="spotlight-result-list"
          :style="{ height: `${results.length * RESULT_ROW_HEIGHT}px` }"
        >
          <div
            class="spotlight-result-list__items"
            :style="{ transform: `translateY(${(visibleResults[0]?.index ?? 0) * RESULT_ROW_HEIGHT}px)` }"
          >
        <div
          v-for="{ image, index } in visibleResults"
          :key="image.id"
          class="spotlight-result"
          :class="{ 'spotlight-result--selected': index === selectedIndex }"
          :data-result-index="index"
          :aria-posinset="index + 1"
          :aria-setsize="results.length"
          :aria-selected="index === selectedIndex"
          role="option"
          tabindex="-1"
          @mouseenter="emit('select', index)"
          @focus="emit('select', index)"
          @click="emit('select', index)"
          @dblclick="emit('open', image)"
        >
          <span class="spotlight-thumb">
            <ThumbnailImage class="spotlight-thumbnail-image" :image="image" />
          </span>
          <span class="spotlight-copy">
            <strong>{{ image.name }}</strong>
            <small>{{ image.width }} × {{ image.height }} · {{ Math.max(1, Math.round(image.sizeBytes / 1024)) }} KB</small>
          </span>
          <span v-if="image.semanticScore != null" class="spotlight-score">{{ Math.round(image.semanticScore * 100) }}%</span>
          <span class="spotlight-actions">
            <Button
              class="spotlight-action-button"
              :class="{ 'spotlight-action-button--success': copiedImageId === image.id }"
              :variant="copiedImageId === image.id ? 'primary' : 'secondary'"
              size="sm"
              :loading="copyingImageId === image.id"
              :aria-label="t('copy_image')"
              @click.stop="emit('copy', image)"
            >
              <template #leading>
                <Check v-if="copiedImageId === image.id" :size="14" />
                <Copy v-else :size="14" />
              </template>
              {{ copiedImageId === image.id ? t('copied') : t('copy') }}
              <template #trailing><KbdChip shortcut="Ctrl+C" size="sm" /></template>
            </Button>
            <Button
              class="spotlight-action-button"
              variant="secondary"
              size="sm"
              :loading="revealingImageId === image.id"
              :aria-label="t('open_in_file_manager', { name: fileManagerName })"
              @click.stop="emit('reveal', image)"
            >
              <template #leading><FolderOpen :size="14" /></template>
              {{ t('open_in_file_manager_short', { name: fileManagerName }) }}
              <template #trailing><KbdChip shortcut="Ctrl+E" size="sm" /></template>
            </Button>
            <Button
              class="spotlight-action-button"
              variant="primary"
              size="sm"
              :loading="openingImageId === image.id"
              :aria-label="t('open_in_imagyx')"
              @click.stop="emit('open', image)"
            >
              <template #leading><ExternalLink :size="14" /></template>
              Imagyx
              <template #trailing><KbdChip shortcut="Ctrl+I" size="sm" /></template>
            </Button>
          </span>
        </div>
          </div>
        </div>

        <div
          v-if="searching && hasSearchQuery && results.length === 0"
          class="spotlight-loading-list"
          :aria-label="t('spotlight.searching')"
        >
          <span v-for="item in 5" :key="item" :style="{ animationDelay: `${item * 45}ms` }" />
        </div>

        <div
          v-else-if="hasSearchQuery && !searching && !results.length && !error && !showAddAction && !showBackgroundHint && jobs.length === 0"
          class="spotlight-empty"
        >
          <Search :size="24" />
          <strong>{{ t('spotlight.no_results_title') }}</strong>
          <span>{{ t('spotlight.no_results_desc') }}</span>
        </div>
      </template>

      <div v-if="error" class="spotlight-error">{{ error }}</div>
    </div>
    <div v-if="canScrollDown" class="spotlight-scroll-shadow" aria-hidden="true" />
  </div>
</template>

<style scoped>
.spotlight-results-shell { position: relative; height: 100%; min-height: 0; overflow: hidden; }
.spotlight-results {
  height: 100%;
  overflow-y: auto;
  padding: 8px 9px 20px;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--border-strong) 78%, transparent) transparent;
}
.spotlight-library-loading {
  display: grid;
  place-items: center;
  align-content: center;
  gap: 12px;
  min-height: 300px;
  color: var(--text-muted);
  font-size: 12px;
}
.spotlight-library-empty {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 370px;
  padding: 34px;
  text-align: center;
}
.spotlight-library-empty__icon {
  display: grid;
  place-items: center;
  width: 54px;
  height: 54px;
  margin-bottom: 15px;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, var(--border));
  border-radius: 16px;
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  color: var(--primary-text);
  box-shadow: inset 0 1px rgb(255 255 255 / 0.1), 0 14px 28px -24px rgb(15 23 42 / 0.5);
}
.spotlight-library-empty strong { color: var(--text); font-size: 16px; letter-spacing: -0.2px; }
.spotlight-library-empty p { max-width: 330px; margin: 8px 0 18px; color: var(--text-muted); font-size: 11px; line-height: 1.55; }
.spotlight-library-empty small { margin-top: 13px; color: var(--text-subtle); font-size: 9px; }
.spotlight-library-empty__button { min-width: 178px; }
.spotlight-add-folder { justify-content: flex-start; margin: 4px 4px 8px; min-height: 64px; text-align: left; }
.spotlight-add-folder__copy { display: block; min-width: 0; text-align: left; }
.spotlight-add-folder__copy strong,
.spotlight-add-folder__copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-add-folder__copy strong { font-size: 11px; color: var(--text); }
.spotlight-add-folder__copy small { margin-top: 3px; color: var(--text-secondary); font-size: 10px; font-weight: 500; }
.spotlight-background-hint {
  display: grid;
  grid-template-columns: 30px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  margin: 4px 4px 8px;
  padding: 9px 11px;
  border: 1px solid color-mix(in srgb, var(--primary) 18%, var(--border));
  border-radius: 12px;
  background: color-mix(in srgb, var(--primary-soft) 30%, var(--surface));
}
.spotlight-background-hint__icon {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: color-mix(in srgb, var(--primary-soft) 68%, var(--surface));
  color: var(--primary-text);
}
.spotlight-background-hint strong,
.spotlight-background-hint small { display: block; }
.spotlight-background-hint strong { color: var(--text); font-size: 10px; }
.spotlight-background-hint small { margin-top: 3px; color: var(--text-muted); font-size: 9px; }
.spotlight-result-list { position: relative; animation: result-list-reveal 180ms ease both; }
.spotlight-result-list__items { will-change: transform; }
.spotlight-result {
  position: relative;
  display: grid;
  grid-template-columns: 54px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 72px;
  padding: 8px 11px;
  overflow: hidden;
  border: 1px solid transparent;
  border-radius: 14px;
  background: transparent;
  color: inherit;
  cursor: default;
  transition: background-color 150ms ease, border-color 150ms ease, transform 180ms cubic-bezier(0.16, 1, 0.3, 1), box-shadow 180ms ease;
}
.spotlight-result:hover,
.spotlight-result--selected {
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 70%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.05);
  transform: translateX(2px) scale(0.998);
  will-change: transform;
}
.spotlight-thumb {
  width: 54px;
  height: 54px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-hover);
  box-shadow: 0 6px 16px rgb(2 6 23 / 0.12);
  transition: transform 220ms cubic-bezier(0.16, 1, 0.3, 1);
}
.spotlight-result:hover .spotlight-thumb,
.spotlight-result--selected .spotlight-thumb { transform: scale(1.035) rotate(-0.35deg); }
.spotlight-thumbnail-image { width: 100%; height: 100%; object-fit: cover; }
.spotlight-copy { min-width: 0; }
.spotlight-copy strong,
.spotlight-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-copy strong { color: var(--text); font-size: 13px; letter-spacing: -0.12px; }
.spotlight-copy small { margin-top: 5px; color: var(--text-muted); font-size: 10px; }
.spotlight-score {
  padding: 4px 7px;
  border-radius: var(--radius-full);
  background: var(--primary-soft);
  color: var(--primary-text);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  transition: opacity 120ms ease, transform 160ms ease;
}
.spotlight-result:hover .spotlight-score,
.spotlight-result--selected .spotlight-score { opacity: 0; transform: translateX(7px); pointer-events: none; }
.spotlight-actions {
  position: absolute;
  right: 8px;
  top: 50%;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 5px;
  padding-left: 28px;
  opacity: 0;
  transform: translateY(-50%);
  pointer-events: none;
  background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--primary-soft) 85%, var(--surface)) 28%);
  border-radius: var(--radius-md);
  transition: opacity 80ms ease;
}
.spotlight-result:hover .spotlight-actions,
.spotlight-result--selected .spotlight-actions,
.spotlight-result:focus-within .spotlight-actions { opacity: 1; pointer-events: auto; will-change: opacity; }
.spotlight-action-button { min-height: 31px; padding-inline: 9px; border-radius: 9px; font-size: 10px; }
.spotlight-action-button--success { animation: action-success 280ms cubic-bezier(0.16, 1, 0.3, 1) both; }
.spotlight-loading-list { display: grid; gap: 8px; padding: 4px; }
.spotlight-loading-list span {
  height: 70px;
  border-radius: 14px;
  background: linear-gradient(100deg, var(--surface-hover) 25%, color-mix(in srgb, var(--primary-soft) 44%, var(--surface)) 44%, var(--surface-hover) 63%);
  background-size: 260% 100%;
  animation: skeleton-shimmer 1.35s linear infinite;
}
.spotlight-empty { display: grid; place-items: center; align-content: center; min-height: 300px; padding: 32px; color: var(--text-muted); text-align: center; }
.spotlight-empty svg { margin-bottom: 13px; color: var(--primary-text); }
.spotlight-empty strong { color: var(--text); font-size: 14px; }
.spotlight-empty span { margin-top: 7px; font-size: 11px; }
.spotlight-error { margin: 10px; padding: 11px 12px; border: 1px solid var(--danger-border); border-radius: 11px; background: var(--danger-surface); color: var(--danger-text); font-size: 11px; }
.spotlight-scroll-shadow {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 4;
  height: 54px;
  pointer-events: none;
  background: linear-gradient(180deg, transparent, color-mix(in srgb, var(--surface-elevated) 96%, transparent) 88%);
  box-shadow: inset 0 -13px 17px -17px rgb(2 6 23 / 0.32);
}
.spin { animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(1turn); } }
@keyframes result-list-reveal { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: none; } }
@keyframes action-success { 0% { transform: scale(0.94); } 55% { transform: scale(1.04); } 100% { transform: none; } }
@keyframes skeleton-shimmer { to { background-position: -160% 0; } }
@media (prefers-reduced-motion: reduce) {
  .spotlight-action-button--success,
  .spin { animation-duration: 0.01ms; }
}
</style>

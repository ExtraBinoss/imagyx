<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { Check, Copy, ExternalLink, FolderOpen, FolderPlus, Search } from '@lucide/vue'
import type { ImageAsset } from '../../types'
import ThumbnailImage from '../ThumbnailImage.vue'
import Button from '../ui/Button/Button.vue'
import CopyButton from '../ui/Button/CopyButton.vue'
import SpotlightIndexProgress from './SpotlightIndexProgress.vue'
import type { SpotlightIndexJob } from './types'

const props = defineProps<{
  results: ImageAsset[]
  selectedIndex: number
  searching: boolean
  error: string | null
  copiedImageId: string | null
  showAddAction: boolean
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

const viewport = ref<HTMLElement | null>(null)
const canScrollDown = ref(false)

watch(() => [props.results.length, props.jobs.length, props.showAddAction], () => {
  void nextTick(updateScrollShadow)
})

function updateScrollShadow() {
  const element = viewport.value
  if (!element) {
    canScrollDown.value = false
    return
  }
  canScrollDown.value = element.scrollHeight > element.clientHeight + 2
    && element.scrollTop + element.clientHeight < element.scrollHeight - 2
}

function scrollToIndex(index: number) {
  void nextTick(() => {
    viewport.value?.querySelector<HTMLElement>(`[data-result-index="${index}"]`)
      ?.scrollIntoView({ block: 'nearest' })
    updateScrollShadow()
  })
}

defineExpose({ scrollToIndex })
</script>

<template>
  <div class="spotlight-results-shell">
    <div ref="viewport" class="spotlight-results" role="listbox" @scroll.passive="updateScrollShadow">
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
          <strong>Ajouter un dossier d’images</strong>
          <small>Choisir un dossier et lancer l’indexation en arrière-plan</small>
        </span>
      </Button>

      <SpotlightIndexProgress v-for="job in jobs" :key="job.folderId" :job="job" />

      <div
        v-for="(image, index) in results"
        :key="image.id"
        class="spotlight-result"
        :class="{ 'spotlight-result--selected': index === selectedIndex }"
        :data-result-index="index"
        :aria-selected="index === selectedIndex"
        role="option"
        tabindex="-1"
        :style="{ animationDelay: `${Math.min(index, 10) * 18}ms` }"
        @mouseenter="emit('select', index)"
        @focus="emit('select', index)"
        @click="emit('select', index)"
        @dblclick="emit('open', image)"
      >
        <span class="spotlight-thumb"><ThumbnailImage :image="image" /></span>
        <span class="spotlight-copy">
          <strong>{{ image.name }}</strong>
          <small>{{ image.width }} × {{ image.height }} · {{ Math.max(1, Math.round(image.sizeBytes / 1024)) }} Ko</small>
        </span>
        <span v-if="image.semanticScore != null" class="spotlight-score">{{ Math.round(image.semanticScore * 100) }}%</span>
        <span class="spotlight-actions">
          <CopyButton
            :copied-text="copiedImageId === image.id ? 'Copied' : 'Copied'"
            idle-text="Copy"
            variant="secondary"
            size="sm"
            class="spotlight-action-button"
            @copy="emit('copy', image)"
          >
            <template #trailing><kbd class="shortcut-kbd">Ctrl+C</kbd></template>
          </CopyButton>
          <Button class="spotlight-action-button" variant="secondary" size="sm" :aria-label="`Open in ${fileManagerName}`" @click.stop="emit('reveal', image)">
            <template #leading><FolderOpen :size="14" /></template>
            {{ fileManagerName }}
            <template #trailing><kbd class="shortcut-kbd">Ctrl+E</kbd></template>
          </Button>
          <Button class="spotlight-action-button" variant="primary" size="sm" aria-label="Open in Imagyx" @click.stop="emit('open', image)">
            <template #leading><ExternalLink :size="14" /></template>
            Imagyx
            <template #trailing><kbd class="shortcut-kbd">Ctrl+I</kbd></template>
          </Button>
        </span>
      </div>

      <div v-if="searching && results.length === 0" class="spotlight-loading-list" aria-label="Recherche en cours">
        <span v-for="item in 5" :key="item" :style="{ animationDelay: `${item * 45}ms` }" />
      </div>

      <div v-else-if="!searching && !results.length && !error && !showAddAction" class="spotlight-empty">
        <Search :size="24" />
        <strong>Aucun résultat convaincant</strong>
        <span>Essaie une description plus courte ou un mot plus visuel.</span>
      </div>

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
.spotlight-add-folder { justify-content: flex-start; margin: 4px 4px 8px; min-height: 64px; text-align: left; }
.spotlight-add-folder__copy { display: block; min-width: 0; text-align: left; }
.spotlight-add-folder__copy strong,
.spotlight-add-folder__copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-add-folder__copy strong { font-size: 11px; color: var(--text); }
.spotlight-add-folder__copy small { margin-top: 3px; color: var(--text-secondary); font-size: 10px; font-weight: 500; }
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
  opacity: 0;
  animation: result-rise 235ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  transition: background-color 150ms ease, border-color 150ms ease, transform 180ms cubic-bezier(0.16, 1, 0.3, 1), box-shadow 180ms ease;
}
.spotlight-result:hover,
.spotlight-result--selected {
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 70%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.05);
  transform: translateX(2px) scale(0.998);
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
.spotlight-thumb :deep(img) { width: 100%; height: 100%; object-fit: cover; }
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
  right: 10px;
  top: 50%;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 5px;
  padding-left: 42px;
  opacity: 0;
  transform: translate(10px, -50%) scale(0.97);
  pointer-events: none;
  background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--primary-soft) 93%, var(--surface)) 32%);
  transition: opacity 150ms ease, transform 190ms cubic-bezier(0.16, 1, 0.3, 1);
}
.spotlight-result:hover .spotlight-actions,
.spotlight-result--selected .spotlight-actions,
.spotlight-result:focus-within .spotlight-actions { opacity: 1; transform: translate(0, -50%) scale(1); pointer-events: auto; }
.spotlight-action-button { min-height: 31px; padding-inline: 9px; border-radius: 9px; font-size: 10px; }
.shortcut-kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 15px;
  height: 15px;
  padding: 0 4px;
  margin-left: 3px;
  border-radius: 4px;
  font-family: inherit;
  font-size: 9px;
  font-weight: 700;
  line-height: 1;
  background: color-mix(in srgb, var(--surface) 80%, black 20%);
  color: var(--text-muted);
  border: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
}
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
@keyframes result-rise { from { opacity: 0; transform: translateY(7px) scale(0.992); } to { opacity: 1; transform: none; } }
@keyframes skeleton-shimmer { to { background-position: -160% 0; } }
</style>

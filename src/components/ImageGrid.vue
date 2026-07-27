<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ExternalLink, FileImage, SearchX } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes, perfLog } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Skeleton from './ui/Skeleton/Skeleton.vue'
import ThumbnailImage from './ThumbnailImage.vue'
import Button from './ui/Button/Button.vue'
import CopyButton from './ui/Button/CopyButton.vue'

const props = defineProps<{ images: ImageAsset[]; loading: boolean; hasFolders: boolean; viewKey: string }>()
const emit = defineEmits<{ explain: [imageId: string]; preview: [image: ImageAsset]; loadMore: [] }>()
const GAP = 16
const MIN_CARD_WIDTH = 180
const META_HEIGHT = 66
const IDLE_OVERSCAN_ROWS = 4
const AHEAD_OVERSCAN_ROWS = 7
const BEHIND_OVERSCAN_ROWS = 2
const viewport = ref<HTMLElement | null>(null)
const viewportWidth = ref(0)
const viewportHeight = ref(0)
const windowStartRow = ref(0)
const windowEndRow = ref(0)
const viewportFirstRow = ref(0)
const viewportLastRow = ref(0)
const scrollDirection = ref<-1 | 0 | 1>(0)
const activeImageId = ref<string | null>(null)
const selectedImageId = ref<string | null>(null)
const isScrolling = ref(false)
const copyBtnRefs = new Map<string, InstanceType<typeof CopyButton>>()
let resizeObserver: ResizeObserver | null = null
let scrollFrame = 0
let scrollTimeout: number | undefined
let latestScrollTop = 0
let lastVirtualScrollTop = 0
let lastLoadMore = 0

const availableWidth = computed(() => Math.max(0, viewportWidth.value - 48))
const columns = computed(() => Math.max(1, Math.floor((availableWidth.value + GAP) / (MIN_CARD_WIDTH + GAP))))
const cardWidth = computed(() => availableWidth.value <= 0 ? MIN_CARD_WIDTH : (availableWidth.value - GAP * (columns.value - 1)) / columns.value)
const rowStride = computed(() => cardWidth.value / 1.08 + META_HEIGHT + GAP)
const totalRows = computed(() => Math.ceil(props.images.length / columns.value))
const startIndex = computed(() => windowStartRow.value * columns.value)
const endIndex = computed(() => Math.min(props.images.length, windowEndRow.value * columns.value))
const visibleEntries = computed(() => {
  const firstVisible = viewportFirstRow.value
  const lastVisible = viewportLastRow.value
  const direction = scrollDirection.value
  return props.images.slice(startIndex.value, endIndex.value).map((image, offset) => {
    const index = startIndex.value + offset
    const row = Math.floor(index / columns.value)
    let priority = Math.abs(row - firstVisible)
    if (row < firstVisible) {
      priority = (firstVisible - row) * 8 + (direction >= 0 ? 64 : 16)
    } else if (row >= lastVisible) {
      priority = (row - lastVisible + 1) * 8 + (direction <= 0 ? 64 : 16)
    }
    return { image, index, priority }
  })
})
const spacerHeight = computed(() => Math.max(0, totalRows.value * rowStride.value - GAP))
const windowOffset = computed(() => windowStartRow.value * rowStride.value)

function registerCopyBtn(id: string, el: unknown) {
  if (el) copyBtnRefs.set(id, el as InstanceType<typeof CopyButton>)
  else copyBtnRefs.delete(id)
}

function activate(image: ImageAsset) {
  if (isScrolling.value) return
  activeImageId.value = image.id
  emit('explain', image.id)
}

function selectImage(image: ImageAsset) {
  activeImageId.value = image.id
  selectedImageId.value = image.id
  emit('explain', image.id)
  scrollToImage(image.id)
}

function scrollToImage(id: string) {
  const index = props.images.findIndex((img) => img.id === id)
  if (index < 0) return
  const row = Math.floor(index / columns.value)
  const targetTop = row * rowStride.value
  if (!viewport.value) return
  if (targetTop < viewport.value.scrollTop) {
    viewport.value.scrollTop = targetTop
  } else if (targetTop + rowStride.value > viewport.value.scrollTop + viewportHeight.value) {
    viewport.value.scrollTop = targetTop + rowStride.value - viewportHeight.value
  }
}

async function copySelectedImage(image: ImageAsset) {
  copyBtnRefs.get(image.id)?.triggerCopied()
  try {
    await imagyxApi.copyImage(image.path)
  } catch {
    await imagyxApi.copyImageToClipboard(image.path)
  }
}

async function openFileInExplorer(image: ImageAsset, event: MouseEvent) {
  event.stopPropagation()
  await imagyxApi.openInFileManager(image.path, false)
}

function moveSelection(deltaIndex: number) {
  if (!props.images.length) return
  const currentIndex = props.images.findIndex((img) => img.id === (selectedImageId.value ?? activeImageId.value))
  const newIndex = currentIndex < 0
    ? 0
    : Math.max(0, Math.min(props.images.length - 1, currentIndex + deltaIndex))
  const targetImage = props.images[newIndex]
  if (targetImage) selectImage(targetImage)
}

function handleGlobalKeydown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null
  if (target?.matches('input, textarea, select, [contenteditable="true"]')) return

  if (event.key === 'ArrowRight') { event.preventDefault(); moveSelection(1); return }
  if (event.key === 'ArrowLeft') { event.preventDefault(); moveSelection(-1); return }
  if (event.key === 'ArrowDown') { event.preventDefault(); moveSelection(columns.value); return }
  if (event.key === 'ArrowUp') { event.preventDefault(); moveSelection(-columns.value); return }

  if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === 'c') {
    const imageId = selectedImageId.value ?? activeImageId.value
    if (!imageId) return
    const image = props.images.find((item) => item.id === imageId)
    if (!image) return
    event.preventDefault()
    event.stopPropagation()
    void copySelectedImage(image)
    return
  }

  if (event.code !== 'Space' && event.key !== ' ') return
  event.preventDefault()
  event.stopPropagation()

  const imageId = selectedImageId.value ?? activeImageId.value
  if (!imageId) return
  const image = props.images.find((item) => item.id === imageId)
  if (image) emit('preview', image)
}

function updateVirtualWindow(scrollTop: number, force = false) {
  const stride = Math.max(1, rowStride.value)
  const nextDirection = scrollTop > lastVirtualScrollTop + 1
    ? 1
    : scrollTop < lastVirtualScrollTop - 1
      ? -1
      : scrollDirection.value
  const firstVisible = Math.max(0, Math.floor(scrollTop / stride))
  const lastVisible = Math.min(totalRows.value, Math.ceil((scrollTop + viewportHeight.value) / stride))
  const before = nextDirection < 0 ? AHEAD_OVERSCAN_ROWS : nextDirection > 0 ? BEHIND_OVERSCAN_ROWS : IDLE_OVERSCAN_ROWS
  const after = nextDirection > 0 ? AHEAD_OVERSCAN_ROWS : nextDirection < 0 ? BEHIND_OVERSCAN_ROWS : IDLE_OVERSCAN_ROWS
  const nextStart = Math.max(0, firstVisible - before)
  const nextEnd = Math.min(totalRows.value, lastVisible + after)

  if (force || scrollDirection.value !== nextDirection) scrollDirection.value = nextDirection
  if (force || viewportFirstRow.value !== firstVisible) viewportFirstRow.value = firstVisible
  if (force || viewportLastRow.value !== lastVisible) viewportLastRow.value = lastVisible
  if (force || windowStartRow.value !== nextStart) windowStartRow.value = nextStart
  if (force || windowEndRow.value !== nextEnd) windowEndRow.value = nextEnd
  lastVirtualScrollTop = scrollTop
}

function measure() {
  if (!viewport.value) return
  const nextWidth = viewport.value.clientWidth
  const nextHeight = viewport.value.clientHeight
  if (viewportWidth.value !== nextWidth) viewportWidth.value = nextWidth
  if (viewportHeight.value !== nextHeight) viewportHeight.value = nextHeight
  latestScrollTop = viewport.value.scrollTop
  updateVirtualWindow(latestScrollTop, true)
}

function handleScroll() {
  if (!isScrolling.value) isScrolling.value = true
  if (scrollTimeout) window.clearTimeout(scrollTimeout)
  scrollTimeout = window.setTimeout(() => {
    isScrolling.value = false
    scrollDirection.value = 0
    updateVirtualWindow(latestScrollTop, true)
  }, 120)

  if (scrollFrame) return
  scrollFrame = requestAnimationFrame(() => {
    scrollFrame = 0
    const element = viewport.value
    if (!element) return

    latestScrollTop = element.scrollTop
    updateVirtualWindow(latestScrollTop)
    const distanceToBottom = spacerHeight.value - (latestScrollTop + viewportHeight.value)
    if (distanceToBottom < 600) {
      const now = performance.now()
      if (now - lastLoadMore > 300) {
        lastLoadMore = now
        perfLog('ImageGrid', 'LoadMore triggered', 0, { displayed: props.images.length })
        emit('loadMore')
      }
    }
  })
}

onMounted(() => {
  measure()
  resizeObserver = new ResizeObserver(measure)
  if (viewport.value) resizeObserver.observe(viewport.value)
  window.addEventListener('keydown', handleGlobalKeydown)
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  if (scrollFrame) cancelAnimationFrame(scrollFrame)
  if (scrollTimeout) window.clearTimeout(scrollTimeout)
  window.removeEventListener('keydown', handleGlobalKeydown)
})

watch(() => props.viewKey, async () => {
  await nextTick()
  if (viewport.value) viewport.value.scrollTop = 0
  latestScrollTop = 0
  lastVirtualScrollTop = 0
  scrollDirection.value = 0
  updateVirtualWindow(0, true)
  activeImageId.value = null
  selectedImageId.value = null
})

watch([columns, rowStride, totalRows], () => {
  updateVirtualWindow(latestScrollTop, true)
})

watch(() => props.images.length, () => {
  const maximum = Math.max(0, spacerHeight.value - viewportHeight.value)
  if (latestScrollTop > maximum && viewport.value) {
    latestScrollTop = maximum
    viewport.value.scrollTop = maximum
    updateVirtualWindow(maximum, true)
  }
  if (selectedImageId.value && !props.images.some((image) => image.id === selectedImageId.value)) {
    selectedImageId.value = null
  }
})
</script>

<template>
  <section ref="viewport" class="image-area" @scroll.passive="handleScroll">
    <div v-if="loading && images.length === 0" class="loading-grid" aria-label="Chargement">
      <Skeleton v-for="item in 18" :key="item" class="skeleton-card" radius="lg" />
    </div>

    <div v-else-if="images.length > 0" class="virtual-grid-spacer" :style="{ height: `${spacerHeight}px` }" role="list" :aria-label="`${images.length} images`">
      <div
        class="virtual-grid-window"
        :class="{ 'is-scrolling': isScrolling }"
        :style="{ transform: `translate3d(0, ${windowOffset}px, 0)`, gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }"
      >
        <article
          v-for="entry in visibleEntries"
          :key="entry.image.id"
          class="image-card"
          :class="{ 'image-card--selected': selectedImageId === entry.image.id }"
          role="listitem"
          tabindex="0"
          :aria-selected="selectedImageId === entry.image.id"
          :aria-posinset="entry.index + 1"
          :aria-setsize="images.length"
          @click="selectImage(entry.image)"
          @mouseenter="activate(entry.image)"
          @focusin="activate(entry.image)"
        >
          <div class="image-frame">
            <ThumbnailImage :image="entry.image" :priority="entry.priority" />
            <Badge v-if="entry.image.semanticScore" class="score-badge" variant="primary">
              {{ Math.round(entry.image.semanticScore * 100) }}%
            </Badge>

            <div class="card-hover-actions">
              <CopyButton
                :ref="(el: unknown) => registerCopyBtn(entry.image.id, el)"
                icon-only
                size="icon"
                variant="secondary"
                class="card-action-btn"
                title="Copy image"
                @copy="copySelectedImage(entry.image)"
              />
              <Button
                variant="secondary"
                size="icon"
                class="card-action-btn"
                title="Open file"
                @click="openFileInExplorer(entry.image, $event)"
              >
                <ExternalLink :size="13" />
              </Button>
            </div>

            <div class="semantic-overlay">
              <div
                v-if="entry.image.semanticMatches?.length"
                class="semantic-marquee"
                :class="{ 'semantic-marquee--animated': entry.image.semanticMatches.length > 2 }"
              >
                <div class="semantic-marquee__track">
                  <span
                    v-for="(match, matchIndex) in (entry.image.semanticMatches.length > 2 ? [...entry.image.semanticMatches, ...entry.image.semanticMatches] : entry.image.semanticMatches)"
                    :key="`${entry.image.id}:${matchIndex}:${match.source}:${match.label}`"
                    class="semantic-chip"
                    :class="`semantic-chip--${match.source}`"
                  >
                    {{ match.label }} · {{ Math.round(match.score * 100) }}%
                  </span>
                </div>
              </div>
              <span v-else class="semantic-overlay__loading">Analyse des tags…</span>
            </div>
          </div>

          <div class="image-meta">
            <strong>{{ entry.image.name }}</strong>
            <span>{{ entry.image.width }} × {{ entry.image.height }} · {{ formatBytes(entry.image.sizeBytes) }}</span>
          </div>
        </article>
      </div>
    </div>

    <div v-else class="empty-state">
      <component :is="hasFolders ? SearchX : FileImage" :size="34" :stroke-width="1.5" />
      <strong>{{ hasFolders ? 'Aucune image trouvée' : 'Ajoute ton premier dossier' }}</strong>
      <p>{{ hasFolders ? 'Essaie un nom de fichier ou une description visuelle différente.' : 'Imagyx l’indexera localement et préparera la recherche sémantique automatiquement.' }}</p>
    </div>
  </section>
</template>

<style scoped>
.virtual-grid-window {
  contain: layout style;
  transform-origin: top left;
  will-change: auto;
}
.virtual-grid-window.is-scrolling {
  pointer-events: none;
  will-change: transform;
}
.image-card {
  contain: layout paint style;
  min-width: 0;
  padding: 3px;
  border: 2px solid transparent;
  border-radius: calc(var(--radius-lg) + 6px);
  outline: none;
  background: transparent;
  cursor: default;
  transition:
    border-color var(--transition-fast),
    background-color var(--transition-fast),
    box-shadow var(--transition-fast);
}
.virtual-grid-window.is-scrolling .image-card {
  transition: none;
}
.virtual-grid-window.is-scrolling :deep(.thumbnail-loader > img) {
  transform: none !important;
  transition: none !important;
}
.virtual-grid-window.is-scrolling :deep(.thumbnail-placeholder::after) {
  animation: none !important;
}
.image-card:hover { background: color-mix(in srgb, var(--surface-hover) 58%, transparent); }
.image-card--selected {
  border-color: var(--primary);
  background: var(--primary-soft);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--primary) 18%, transparent), 0 10px 28px rgb(15 23 42 / 0.08);
}
.image-card:focus-visible { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus-ring-soft); }

.card-hover-actions {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 5px;
  opacity: 0;
  transform: translateY(-4px);
  pointer-events: none;
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}

.image-card:hover .card-hover-actions,
.image-card--selected .card-hover-actions,
.image-card:focus-within .card-hover-actions {
  opacity: 1;
  transform: translateY(0);
  pointer-events: auto;
}

.card-action-btn {
  width: 28px !important;
  height: 28px !important;
  border-radius: var(--radius-sm) !important;
  backdrop-filter: blur(8px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25) !important;
}
.virtual-grid-window.is-scrolling .card-action-btn {
  backdrop-filter: none;
  box-shadow: none !important;
}

.semantic-overlay {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 3;
  min-height: 40px;
  display: flex;
  align-items: center;
  overflow: hidden;
  padding: 7px 0;
  background: linear-gradient(180deg, transparent, rgb(8 10 14 / 0.9));
  opacity: 0;
  transform: translateY(8px);
  pointer-events: none;
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}
.image-card:hover .semantic-overlay,
.image-card:focus-within .semantic-overlay { opacity: 1; transform: translateY(0); }
.semantic-overlay__loading { padding: 0 var(--space-3); color: rgb(255 255 255 / 0.75); font-size: 10px; }
.semantic-marquee {
  width: 100%;
  overflow: hidden;
  pointer-events: auto;
}
.semantic-marquee--animated {
  mask-image: linear-gradient(90deg, transparent, #000 6%, #000 94%, transparent);
}
.semantic-marquee__track {
  display: flex;
  align-items: center;
  width: max-content;
  gap: var(--space-2);
  padding-inline: var(--space-3);
}
.image-card:hover .semantic-marquee--animated .semantic-marquee__track,
.image-card:focus-within .semantic-marquee--animated .semantic-marquee__track {
  animation: semantic-marquee 10s linear infinite;
}
.semantic-marquee:hover .semantic-marquee__track {
  animation-play-state: paused;
}
.semantic-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
  padding: 4px 9px;
  border-radius: var(--radius-full);
  font-size: 10px;
  font-weight: 600;
}

.semantic-chip--filename {
  background: rgba(18, 20, 26, 0.82);
  border: 1px solid rgba(255, 255, 255, 0.18);
  color: #f1f5f9;
}

.semantic-chip--semantic {
  background: color-mix(in srgb, var(--primary) 85%, #0284c7);
  border: 1px solid var(--primary);
  color: #ffffff;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--primary) 40%, transparent);
}
@keyframes semantic-marquee { from { transform: translateX(0); } to { transform: translateX(-50%); } }
@media (prefers-reduced-motion: reduce) {
  .semantic-marquee__track { animation: none !important; }
}
</style>

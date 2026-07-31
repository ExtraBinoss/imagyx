<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ExternalLink, FileImage, FolderPlus, SearchX, Sparkles } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { visualSearch } from '../services/visual-search'
import { formatBytes, perfLog } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Skeleton from './ui/Skeleton/Skeleton.vue'
import ThumbnailImage from './ThumbnailImage.vue'
import Button from './ui/Button/Button.vue'
import CopyButton from './ui/Button/CopyButton.vue'
import { useTranslate } from '../i18n'

const { t } = useTranslate()
const props = defineProps<{ images: ImageAsset[]; loading: boolean; hasFolders: boolean; viewKey: string }>()
const emit = defineEmits<{
  explain: [imageId: string]
  preview: [image: ImageAsset]
  loadMore: []
  addFolder: []
}>()
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
const contextMenu = ref<{ image: ImageAsset; x: number; y: number } | null>(null)
const findingSimilar = ref(false)
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
  await imagyxApi.copyImage(image.path)
}

async function openFileInExplorer(image: ImageAsset, event: MouseEvent) {
  event.stopPropagation()
  await imagyxApi.openInFileManager(image.path, false)
}

function openContextMenu(image: ImageAsset, event: MouseEvent) {
  selectImage(image)
  const width = 292
  const height = 92
  contextMenu.value = {
    image,
    x: Math.max(10, Math.min(window.innerWidth - width - 10, event.clientX)),
    y: Math.max(10, Math.min(window.innerHeight - height - 10, event.clientY)),
  }
}

function closeContextMenu() {
  if (!findingSimilar.value) contextMenu.value = null
}

async function findSimilarFromMenu() {
  const image = contextMenu.value?.image
  if (!image || findingSimilar.value) return
  findingSimilar.value = true
  try {
    await visualSearch.findSimilar(image)
    contextMenu.value = null
  } finally {
    findingSimilar.value = false
  }
}

function handleDocumentPointerDown(event: PointerEvent) {
  const target = event.target
  if (target instanceof Element && target.closest('.image-context-menu')) return
  closeContextMenu()
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
  if (contextMenu.value && event.key === 'Escape') {
    event.preventDefault()
    closeContextMenu()
    return
  }
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
  closeContextMenu()
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
  document.addEventListener('pointerdown', handleDocumentPointerDown, { capture: true })
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  if (scrollFrame) cancelAnimationFrame(scrollFrame)
  if (scrollTimeout) window.clearTimeout(scrollTimeout)
  window.removeEventListener('keydown', handleGlobalKeydown)
  document.removeEventListener('pointerdown', handleDocumentPointerDown, { capture: true })
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
  contextMenu.value = null
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
    <div v-if="loading && images.length === 0" class="loading-grid" :aria-label="t('search.loading')">
      <Skeleton v-for="item in 18" :key="item" class="skeleton-card" radius="lg" />
    </div>

    <div v-else-if="images.length > 0" class="virtual-grid-spacer" :style="{ height: `${spacerHeight}px` }" role="list" :aria-label="t('search.result_count', { count: images.length })">
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
          @contextmenu.prevent="openContextMenu(entry.image, $event)"
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
                :title="t('copy_image')"
                @copy="copySelectedImage(entry.image)"
              />
              <Button
                variant="secondary"
                size="icon"
                class="card-action-btn"
                :title="t('open_file')"
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
              <span v-else class="semantic-overlay__loading">{{ t('search.analyzing_tags') }}</span>
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
      <div v-if="!hasFolders" class="empty-state__card">
        <span class="empty-state__icon"><FolderPlus :size="26" /></span>
        <strong>{{ t('spotlight.no_folder_title') }}</strong>
        <p>{{ t('onboarding.step_indexing_desc') }}</p>
        <Button variant="primary" size="lg" class="empty-state__action" @click="emit('addFolder')">
          <template #leading><FolderPlus :size="18" /></template>
          {{ t('spotlight.no_folder_action') }}
        </Button>
        <small class="empty-state__privacy">{{ t('spotlight.no_folder_privacy') }}</small>
      </div>
      <template v-else>
        <SearchX :size="36" :stroke-width="1.5" />
        <strong>{{ t('search.no_images_title') }}</strong>
        <p>{{ t('search.no_images_desc') }}</p>
      </template>
    </div>
  </section>

  <Teleport to="body">
    <Transition name="context-pop">
      <div
        v-if="contextMenu"
        class="image-context-menu"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        role="menu"
        :aria-label="t('search.visual.find_similar')"
      >
        <button type="button" role="menuitem" :disabled="findingSimilar" @click="findSimilarFromMenu">
          <span class="image-context-menu__icon">
            <Sparkles v-if="!findingSimilar" :size="17" />
            <span v-else class="image-context-menu__spinner" />
          </span>
          <span class="image-context-menu__copy">
            <strong>{{ t('search.visual.find_similar') }}</strong>
            <small>{{ t('search.visual.find_similar_desc') }}</small>
          </span>
        </button>
      </div>
    </Transition>
  </Teleport>
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
  transition: border-color var(--transition-fast), background-color var(--transition-fast), box-shadow var(--transition-fast);
}
.virtual-grid-window.is-scrolling .image-card { transition: none; }
.virtual-grid-window.is-scrolling :deep(.thumbnail-loader > img) { transform: none !important; transition: none !important; }
.virtual-grid-window.is-scrolling :deep(.thumbnail-placeholder::after) { animation: none !important; }
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
.image-card:focus-within .card-hover-actions { opacity: 1; transform: translateY(0); pointer-events: auto; }
.card-action-btn {
  width: 28px !important;
  height: 28px !important;
  border-radius: var(--radius-sm) !important;
  backdrop-filter: blur(8px);
  box-shadow: 0 4px 12px rgb(0 0 0 / 0.25) !important;
}
.virtual-grid-window.is-scrolling .card-action-btn { backdrop-filter: none; box-shadow: none !important; }
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
.semantic-marquee { width: 100%; overflow: hidden; pointer-events: auto; }
.semantic-marquee--animated { mask-image: linear-gradient(90deg, transparent, #000 6%, #000 94%, transparent); }
.semantic-marquee__track { display: flex; align-items: center; width: max-content; gap: var(--space-2); padding-inline: var(--space-3); }
.image-card:hover .semantic-marquee--animated .semantic-marquee__track,
.image-card:focus-within .semantic-marquee--animated .semantic-marquee__track { animation: semantic-marquee 10s linear infinite; }
.semantic-marquee:hover .semantic-marquee__track { animation-play-state: paused; }
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
.semantic-chip--filename { background: rgb(18 20 26 / 0.82); border: 1px solid rgb(255 255 255 / 0.18); color: #f1f5f9; }
.semantic-chip--semantic {
  background: color-mix(in srgb, var(--primary) 85%, #0284c7);
  border: 1px solid var(--primary);
  color: #fff;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--primary) 40%, transparent);
}
.image-context-menu {
  position: fixed;
  z-index: 200;
  width: 292px;
  padding: 6px;
  border: 1px solid color-mix(in srgb, var(--border-strong) 82%, transparent);
  border-radius: 15px;
  background: color-mix(in srgb, var(--surface-elevated) 94%, transparent);
  box-shadow: inset 0 1px rgb(255 255 255 / 0.08), 0 24px 54px -24px rgb(2 6 23 / 0.72);
  backdrop-filter: blur(28px) saturate(1.18);
  transform-origin: top left;
}
.image-context-menu button {
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 54px;
  padding: 7px 9px;
  border: 1px solid color-mix(in srgb, var(--primary) 20%, var(--border));
  border-radius: 11px;
  background: linear-gradient(120deg, color-mix(in srgb, var(--primary-soft) 74%, var(--surface)), transparent);
  color: var(--text);
  text-align: left;
  cursor: default;
  transition: transform 160ms cubic-bezier(0.16, 1, 0.3, 1), background-color 120ms ease;
}
.image-context-menu button:hover,
.image-context-menu button:focus-visible { outline: none; transform: translate3d(2px, 0, 0); background-color: var(--primary-soft); }
.image-context-menu button:disabled { opacity: 0.66; }
.image-context-menu__icon {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--primary-soft) 56%, var(--surface));
  color: var(--primary-text);
}
.image-context-menu__copy { min-width: 0; }
.image-context-menu__copy strong,
.image-context-menu__copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.image-context-menu__copy strong { font-size: 12px; }
.image-context-menu__copy small { margin-top: 3px; color: var(--text-muted); font-size: 9px; }
.image-context-menu__spinner {
  width: 16px;
  height: 16px;
  border: 2px solid color-mix(in srgb, var(--primary) 22%, transparent);
  border-top-color: var(--primary);
  border-radius: 50%;
  animation: context-spin 0.7s linear infinite;
}
.context-pop-enter-active,
.context-pop-leave-active { transition: opacity 120ms ease, transform 170ms cubic-bezier(0.16, 1, 0.3, 1), filter 120ms ease; }
.context-pop-enter-from,
.context-pop-leave-to { opacity: 0; transform: translate3d(0, -4px, 0) scale(0.96); filter: blur(3px); }
@keyframes semantic-marquee { from { transform: translateX(0); } to { transform: translateX(-50%); } }
@keyframes context-spin { to { transform: rotate(1turn); } }
.empty-state {
  display: grid;
  place-items: center;
  align-content: center;
  height: 100%;
  padding: 32px;
  text-align: center;
}
.empty-state__card {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 400px;
  padding: 36px 28px;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 24px;
  background: color-mix(in srgb, var(--surface) 92%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.08),
    0 24px 50px -30px rgb(15 23 42 / 0.45);
}
.empty-state__icon {
  display: grid;
  place-items: center;
  width: 58px;
  height: 58px;
  margin-bottom: 18px;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, var(--border));
  border-radius: 18px;
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  color: var(--primary-text);
  box-shadow: inset 0 1px rgb(255 255 255 / 0.1), 0 14px 28px -24px rgb(15 23 42 / 0.5);
}
.empty-state__card strong {
  color: var(--text);
  font-size: 17px;
  letter-spacing: -0.2px;
}
.empty-state__card p {
  max-width: 320px;
  margin: 10px 0 20px;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.55;
}
.empty-state__action {
  min-width: 190px;
}
.empty-state__privacy {
  margin-top: 15px;
  color: var(--text-subtle);
  font-size: 10px;
}
@media (prefers-reduced-motion: reduce) {
  .semantic-marquee__track,
  .image-context-menu__spinner { animation: none !important; }
  .context-pop-enter-active,
  .context-pop-leave-active { transition-duration: 0.01ms; }
}
</style>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { FileImage, SearchX } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { formatBytes } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Skeleton from './ui/Skeleton/Skeleton.vue'
import ThumbnailImage from './ThumbnailImage.vue'

const props = defineProps<{ images: ImageAsset[]; loading: boolean; hasFolders: boolean; viewKey: string }>()
const emit = defineEmits<{ explain: [imageId: string]; preview: [image: ImageAsset] }>()
const GAP = 16
const MIN_CARD_WIDTH = 180
const META_HEIGHT = 66
const OVERSCAN_ROWS = 3
const viewport = ref<HTMLElement | null>(null)
const viewportWidth = ref(0)
const viewportHeight = ref(0)
const scrollTop = ref(0)
const activeImageId = ref<string | null>(null)
const selectedImageId = ref<string | null>(null)
const isScrolling = ref<boolean>(false)
let resizeObserver: ResizeObserver | null = null
let scrollFrame = 0
let scrollTimeout: number | undefined

const columns = computed(() => Math.max(1, Math.floor((viewportWidth.value + GAP) / (MIN_CARD_WIDTH + GAP))))
const cardWidth = computed(() => viewportWidth.value <= 0 ? MIN_CARD_WIDTH : (viewportWidth.value - GAP * (columns.value - 1)) / columns.value)
const rowStride = computed(() => cardWidth.value / 1.08 + META_HEIGHT + GAP)
const totalRows = computed(() => Math.ceil(props.images.length / columns.value))
const startRow = computed(() => Math.max(0, Math.floor(scrollTop.value / rowStride.value) - OVERSCAN_ROWS))
const endRow = computed(() => Math.min(totalRows.value, Math.ceil((scrollTop.value + viewportHeight.value) / rowStride.value) + OVERSCAN_ROWS))
const startIndex = computed(() => startRow.value * columns.value)
const endIndex = computed(() => Math.min(props.images.length, endRow.value * columns.value))
const visibleEntries = computed(() => props.images.slice(startIndex.value, endIndex.value).map((image, offset) => ({ image, index: startIndex.value + offset })))
const spacerHeight = computed(() => Math.max(0, totalRows.value * rowStride.value - GAP))
const windowOffset = computed(() => startRow.value * rowStride.value)

function activate(image: ImageAsset) {
  if (isScrolling.value) return
  activeImageId.value = image.id
  emit('explain', image.id)
}

function selectImage(image: ImageAsset) {
  activeImageId.value = image.id
  selectedImageId.value = image.id
  emit('explain', image.id)
}

function handleGlobalKeydown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null
  if (target?.matches('input, textarea, select, [contenteditable="true"]')) return
  if (event.code !== 'Space' && event.key !== ' ') return

  // Prevent browser window scrolling on Space
  event.preventDefault()
  event.stopPropagation()

  const imageId = selectedImageId.value ?? activeImageId.value
  if (!imageId) return
  const image = props.images.find((item) => item.id === imageId)
  if (!image) return
  emit('preview', image)
}

function measure() {
  if (!viewport.value) return
  viewportWidth.value = viewport.value.clientWidth
  viewportHeight.value = viewport.value.clientHeight
  scrollTop.value = viewport.value.scrollTop
}

function handleScroll() {
  isScrolling.value = true
  if (scrollTimeout) window.clearTimeout(scrollTimeout)
  scrollTimeout = window.setTimeout(() => {
    isScrolling.value = false
  }, 120)

  if (scrollFrame) return
  scrollFrame = requestAnimationFrame(() => {
    scrollFrame = 0
    if (viewport.value) scrollTop.value = viewport.value.scrollTop
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
  scrollTop.value = 0
  activeImageId.value = null
  selectedImageId.value = null
})

watch(() => props.images.length, () => {
  const maximum = Math.max(0, spacerHeight.value - viewportHeight.value)
  if (scrollTop.value > maximum && viewport.value) {
    viewport.value.scrollTop = maximum
    scrollTop.value = maximum
  }
  if (selectedImageId.value && !props.images.some((image) => image.id === selectedImageId.value)) selectedImageId.value = null
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
        :style="{ transform: `translateY(${windowOffset}px)`, gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }"
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
            <ThumbnailImage :image="entry.image" />
            <Badge v-if="entry.image.semanticScore" class="score-badge" variant="primary">
              {{ Math.round(entry.image.semanticScore * 100) }}%
            </Badge>

            <div class="semantic-overlay">
              <div v-if="entry.image.semanticMatches?.length" class="semantic-marquee">
                <div class="semantic-marquee__track">
                  <span
                    v-for="(match, matchIndex) in [...entry.image.semanticMatches, ...entry.image.semanticMatches]"
                    :key="`${entry.image.id}:${matchIndex}:${match.source}:${match.label}`"
                    class="semantic-chip"
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
  will-change: transform;
}
.virtual-grid-window.is-scrolling {
  pointer-events: none;
}
.image-card {
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
    box-shadow var(--transition-fast),
    transform var(--transition-fast);
}
.image-card:hover { background: color-mix(in srgb, var(--surface-hover) 58%, transparent); }
.image-card--selected {
  border-color: var(--primary);
  background: var(--primary-soft);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--primary) 18%, transparent), 0 10px 28px rgb(15 23 42 / 0.08);
}
.image-card:focus-visible { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus-ring-soft); }

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
.semantic-marquee { width: 100%; overflow: hidden; mask-image: linear-gradient(90deg, transparent, #000 6%, #000 94%, transparent); }
.semantic-marquee__track {
  display: flex;
  width: max-content;
  gap: var(--space-2);
  padding-inline: var(--space-3);
  animation: semantic-marquee 8s linear infinite;
}
.semantic-chip {
  flex: 0 0 auto;
  padding: 4px 8px;
  border: 1px solid rgb(255 255 255 / 0.2);
  border-radius: var(--radius-full);
  background: rgb(18 20 26 / 0.72);
  color: #fff;
  font-size: 10px;
  font-weight: 600;
  backdrop-filter: blur(8px);
}
@keyframes semantic-marquee { from { transform: translateX(0); } to { transform: translateX(-50%); } }
@media (prefers-reduced-motion: reduce) { .semantic-marquee__track { animation: none; } }
</style>

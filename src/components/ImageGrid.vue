<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from 'vue'
import { FileImage, SearchX } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { formatBytes } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Skeleton from './ui/Skeleton/Skeleton.vue'
import ThumbnailImage from './ThumbnailImage.vue'

const props = defineProps<{
  images: ImageAsset[]
  loading: boolean
  hasFolders: boolean
  viewKey: string
}>()

const GAP = 16
const MIN_CARD_WIDTH = 180
const META_HEIGHT = 48
const OVERSCAN_ROWS = 3

const viewport = ref<HTMLElement | null>(null)
const viewportWidth = ref(0)
const viewportHeight = ref(0)
const scrollTop = ref(0)
let resizeObserver: ResizeObserver | null = null
let scrollFrame = 0

const columns = computed(() =>
  Math.max(1, Math.floor((viewportWidth.value + GAP) / (MIN_CARD_WIDTH + GAP))),
)
const cardWidth = computed(() => {
  if (viewportWidth.value <= 0) return MIN_CARD_WIDTH
  return (viewportWidth.value - GAP * (columns.value - 1)) / columns.value
})
const rowStride = computed(() => cardWidth.value / 1.08 + META_HEIGHT + GAP)
const totalRows = computed(() => Math.ceil(props.images.length / columns.value))
const startRow = computed(() =>
  Math.max(0, Math.floor(scrollTop.value / rowStride.value) - OVERSCAN_ROWS),
)
const endRow = computed(() =>
  Math.min(
    totalRows.value,
    Math.ceil((scrollTop.value + viewportHeight.value) / rowStride.value) + OVERSCAN_ROWS,
  ),
)
const startIndex = computed(() => startRow.value * columns.value)
const endIndex = computed(() => Math.min(props.images.length, endRow.value * columns.value))
const visibleEntries = computed(() =>
  props.images.slice(startIndex.value, endIndex.value).map((image, offset) => ({
    image,
    index: startIndex.value + offset,
  })),
)
const spacerHeight = computed(() => Math.max(0, totalRows.value * rowStride.value - GAP))
const windowOffset = computed(() => startRow.value * rowStride.value)

function measure(): void {
  const element = viewport.value
  if (!element) return
  viewportWidth.value = element.clientWidth
  viewportHeight.value = element.clientHeight
  scrollTop.value = element.scrollTop
}

function handleScroll(): void {
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
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  if (scrollFrame) cancelAnimationFrame(scrollFrame)
})

watch(
  () => props.viewKey,
  async () => {
    await nextTick()
    if (viewport.value) viewport.value.scrollTop = 0
    scrollTop.value = 0
  },
)

watch(
  () => props.images.length,
  () => {
    const maximum = Math.max(0, spacerHeight.value - viewportHeight.value)
    if (scrollTop.value > maximum && viewport.value) {
      viewport.value.scrollTop = maximum
      scrollTop.value = maximum
    }
  },
)
</script>

<template>
  <section ref="viewport" class="image-area" @scroll.passive="handleScroll">
    <div v-if="loading && images.length === 0" class="loading-grid" aria-label="Chargement">
      <Skeleton v-for="item in 18" :key="item" class="skeleton-card" radius="lg" />
    </div>

    <div
      v-else-if="images.length > 0"
      class="virtual-grid-spacer"
      :style="{ height: `${spacerHeight}px` }"
      role="list"
      :aria-label="`${images.length} images`"
    >
      <div
        class="virtual-grid-window"
        :style="{
          transform: `translateY(${windowOffset}px)`,
          gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
        }"
      >
        <article
          v-for="entry in visibleEntries"
          :key="entry.image.id"
          class="image-card"
          :title="entry.image.path"
          role="listitem"
          :aria-posinset="entry.index + 1"
          :aria-setsize="images.length"
        >
          <div class="image-frame">
            <ThumbnailImage :image="entry.image" />
            <Badge v-if="entry.image.semanticScore" class="score-badge" variant="primary">
              {{ Math.round(entry.image.semanticScore * 100) }}%
            </Badge>
          </div>
          <div class="image-meta">
            <strong>{{ entry.image.name }}</strong>
            <span>
              {{ entry.image.width }} × {{ entry.image.height }} ·
              {{ formatBytes(entry.image.sizeBytes) }}
            </span>
          </div>
        </article>
      </div>
    </div>

    <div v-else class="empty-state">
      <component :is="hasFolders ? SearchX : FileImage" :size="34" :stroke-width="1.5" />
      <strong>{{ hasFolders ? 'Aucune image trouvée' : 'Ajoute ton premier dossier' }}</strong>
      <p>
        {{
          hasFolders
            ? 'Essaie un nom de fichier ou une description visuelle différente.'
            : 'Imagyx l’indexera localement et préparera la recherche sémantique automatiquement.'
        }}
      </p>
    </div>
  </section>
</template>

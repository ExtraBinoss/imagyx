<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Image as ImageIcon } from '@lucide/vue'
import { imagyxApi } from '../api/tauri'
import type { ImageAsset } from '../types'
import { requestThumbnail } from '../services/thumbnails'

const props = defineProps<{
  image: ImageAsset
}>()

const host = ref<HTMLElement | null>(null)
const source = ref<string | null>(cachedSource())
const failed = ref(false)
const visible = ref(false)
let requestVersion = 0
let cachedSourceFailed = false
let observer: IntersectionObserver | null = null

function cachedSource(): string | null {
  return props.image.thumbnailPath
    ? imagyxApi.fileUrl(props.image.thumbnailPath)
    : null
}

async function loadThumbnail() {
  if (!visible.value || source.value) return
  const version = ++requestVersion
  failed.value = false
  try {
    const url = await requestThumbnail(props.image)
    if (version === requestVersion) source.value = url
  } catch {
    if (version === requestVersion) failed.value = true
  }
}

function handleImageError() {
  if (!props.image.thumbnailPath || cachedSourceFailed) {
    failed.value = true
    return
  }
  cachedSourceFailed = true
  source.value = null
  void loadThumbnail()
}

watch(
  () => [props.image.id, props.image.modifiedAt, props.image.thumbnailPath] as const,
  () => {
    requestVersion += 1
    cachedSourceFailed = false
    source.value = cachedSource()
    failed.value = false
    void loadThumbnail()
  },
)

onMounted(() => {
  const element = host.value
  if (!element || typeof IntersectionObserver === 'undefined') {
    visible.value = true
    void loadThumbnail()
    return
  }
  observer = new IntersectionObserver(
    (entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return
      visible.value = true
      observer?.disconnect()
      observer = null
      void loadThumbnail()
    },
    { rootMargin: '320px' },
  )
  observer.observe(element)
})

onBeforeUnmount(() => {
  requestVersion += 1
  observer?.disconnect()
})
</script>

<template>
  <span ref="host" class="thumbnail-loader">
    <img
      v-if="source"
      :src="source"
      :alt="image.name"
      loading="lazy"
      decoding="async"
      draggable="false"
      @error="handleImageError"
    />
    <span
      v-else
      class="thumbnail-placeholder"
      :class="{ 'thumbnail-placeholder--error': failed }"
    >
      <ImageIcon v-if="failed" :size="22" :stroke-width="1.5" />
    </span>
  </span>
</template>

<style scoped>
.thumbnail-loader,
.thumbnail-loader > img,
.thumbnail-loader > .thumbnail-placeholder {
  display: block;
  width: 100%;
  height: 100%;
}
.thumbnail-loader > img {
  object-fit: cover;
}
</style>

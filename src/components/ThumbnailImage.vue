<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Image as ImageIcon } from '@lucide/vue'
import type { ImageAsset } from '../types'
import {
  forgetThumbnail,
  peekThumbnail,
  requestThumbnail,
} from '../services/thumbnails'

const props = withDefaults(defineProps<{
  image: ImageAsset
  priority?: number
}>(), {
  priority: 0,
})

const source = ref<string | null>(peekThumbnail(props.image))
const failed = ref(false)
let requestVersion = 0
let retriedAfterImageError = false
let controller: AbortController | null = null

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
}

async function loadThumbnail() {
  if (source.value) return
  controller?.abort()
  controller = new AbortController()
  const version = ++requestVersion
  failed.value = false
  try {
    const url = await requestThumbnail(props.image, {
      priority: props.priority,
      signal: controller.signal,
    })
    if (version === requestVersion) source.value = url
  } catch (error) {
    if (version === requestVersion && !isAbortError(error)) failed.value = true
  }
}

function resetThumbnail() {
  controller?.abort()
  requestVersion += 1
  retriedAfterImageError = false
  failed.value = false
  source.value = peekThumbnail(props.image)
  if (!source.value) void loadThumbnail()
}

function handleImageError() {
  if (retriedAfterImageError) {
    failed.value = true
    return
  }
  retriedAfterImageError = true
  forgetThumbnail(props.image)
  source.value = null
  void loadThumbnail()
}

watch(
  () => [props.image.id, props.image.modifiedAt] as const,
  resetThumbnail,
)

onMounted(() => {
  if (!source.value) void loadThumbnail()
})

onBeforeUnmount(() => {
  requestVersion += 1
  controller?.abort()
})
</script>

<template>
  <span class="thumbnail-loader">
    <img
      v-if="source"
      :src="source"
      :alt="image.name"
      loading="eager"
      decoding="async"
      :fetchpriority="priority <= 1 ? 'high' : 'low'"
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
.thumbnail-loader {
  contain: paint;
  overflow: hidden;
}
.thumbnail-loader > img {
  object-fit: cover;
}
</style>

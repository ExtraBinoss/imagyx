<script setup lang="ts">
import { ref, watch } from 'vue'
import { Image as ImageIcon } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { requestThumbnail } from '../services/thumbnails'

const props = defineProps<{
  image: ImageAsset
}>()

const source = ref<string | null>(null)
const failed = ref(false)
let requestVersion = 0

watch(
  () => [props.image.id, props.image.modifiedAt] as const,
  async () => {
    const version = ++requestVersion
    source.value = null
    failed.value = false
    try {
      const url = await requestThumbnail(props.image)
      if (version === requestVersion) source.value = url
    } catch {
      if (version === requestVersion) failed.value = true
    }
  },
  { immediate: true },
)
</script>

<template>
  <img
    v-if="source"
    :src="source"
    :alt="image.name"
    loading="lazy"
    decoding="async"
    draggable="false"
  />
  <div v-else class="thumbnail-placeholder" :class="{ 'thumbnail-placeholder--error': failed }">
    <ImageIcon :size="22" :stroke-width="1.5" />
  </div>
</template>

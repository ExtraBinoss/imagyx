<script setup lang="ts">
import { FileImage, SearchX } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Skeleton from './ui/Skeleton/Skeleton.vue'

defineProps<{
  images: ImageAsset[]
  loading: boolean
  hasFolders: boolean
}>()
</script>

<template>
  <section class="image-area">
    <div v-if="loading && images.length === 0" class="loading-grid" aria-label="Chargement">
      <Skeleton v-for="item in 18" :key="item" class="skeleton-card" radius="lg" />
    </div>

    <div v-else-if="images.length > 0" class="image-grid">
      <article v-for="image in images" :key="image.id" class="image-card" :title="image.path">
        <div class="image-frame">
          <img :src="imagyxApi.thumbnailUrl(image.thumbnailPath)" :alt="image.name" loading="lazy" />
          <Badge v-if="image.semanticScore" class="score-badge" variant="primary">
            {{ Math.round(image.semanticScore * 100) }}%
          </Badge>
        </div>
        <div class="image-meta">
          <strong>{{ image.name }}</strong>
          <span>{{ image.width }} × {{ image.height }} · {{ formatBytes(image.sizeBytes) }}</span>
        </div>
      </article>
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

<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { X } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Button from './ui/Button/Button.vue'

const props = defineProps<{ image: ImageAsset }>()
const emit = defineEmits<{ close: [] }>()

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' || event.code === 'Space') {
    event.preventDefault()
    emit('close')
  }
}

onMounted(() => window.addEventListener('keydown', handleKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <Teleport to="body">
    <div class="preview-backdrop" role="presentation" @pointerdown.self="emit('close')">
      <section class="preview-dialog" role="dialog" aria-modal="true" :aria-label="`Aperçu de ${image.name}`">
        <header class="preview-header">
          <div>
            <strong>{{ image.name }}</strong>
            <span>{{ image.width }} × {{ image.height }} · {{ formatBytes(image.sizeBytes) }}</span>
          </div>
          <Button variant="secondary" size="icon" aria-label="Fermer l’aperçu" @click="emit('close')">
            <X :size="18" />
          </Button>
        </header>
        <div class="preview-stage">
          <img :src="imagyxApi.fileUrl(image.path)" :alt="image.name" />
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.preview-backdrop {
  position: fixed;
  inset: 0;
  z-index: 120;
  display: grid;
  place-items: center;
  padding: 28px;
  background: rgb(5 7 11 / 0.78);
  backdrop-filter: blur(10px);
}

.preview-dialog {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  width: min(94vw, 1280px);
  height: min(90vh, 900px);
  overflow: hidden;
  border: 1px solid rgb(255 255 255 / 0.14);
  border-radius: var(--radius-xl);
  background: var(--surface-elevated);
  box-shadow: 0 28px 80px rgb(0 0 0 / 0.42);
}

.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}

.preview-header div,
.preview-header strong,
.preview-header span {
  min-width: 0;
}

.preview-header strong,
.preview-header span {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-header strong { font-size: var(--text-sm); }
.preview-header span { margin-top: 3px; color: var(--text-muted); font-size: var(--text-xs); }

.preview-stage {
  display: grid;
  place-items: center;
  min-height: 0;
  padding: var(--space-4);
  background: #0b0d12;
}

.preview-stage img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}
</style>

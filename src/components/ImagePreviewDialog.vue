<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { X } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Button from './ui/Button/Button.vue'

const props = defineProps<{ image: ImageAsset | null }>()
const emit = defineEmits<{ close: [] }>()

function handleKeydown(event: KeyboardEvent) {
  if (!props.image) return
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
    <Transition name="preview-morph" appear>
      <div v-if="image" class="preview-backdrop" role="presentation" @pointerdown.self="emit('close')">
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
    </Transition>
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
  backdrop-filter: blur(12px) saturate(0.9);
}

.preview-dialog {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  width: min(94vw, 1280px);
  height: min(90vh, 900px);
  overflow: hidden;
  border: 1px solid rgb(255 255 255 / 0.14);
  border-radius: calc(var(--radius-xl) + 4px);
  background: var(--surface-elevated);
  box-shadow: 0 32px 100px rgb(0 0 0 / 0.48), 0 3px 14px rgb(0 0 0 / 0.28);
  transform-origin: center 58%;
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
.preview-header span { min-width: 0; }
.preview-header strong,
.preview-header span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.preview-header strong { font-size: var(--text-sm); }
.preview-header span { margin-top: 3px; color: var(--text-muted); font-size: var(--text-xs); }

.preview-stage {
  display: grid;
  place-items: center;
  min-height: 0;
  padding: var(--space-4);
  background: radial-gradient(circle at 50% 42%, #171a22, #090b10 68%);
}
.preview-stage img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  filter: drop-shadow(0 18px 34px rgb(0 0 0 / 0.34));
}

.preview-morph-enter-active,
.preview-morph-leave-active {
  transition: opacity 220ms ease, backdrop-filter 260ms ease;
}
.preview-morph-enter-active .preview-dialog,
.preview-morph-leave-active .preview-dialog {
  transition:
    opacity 230ms ease,
    transform 320ms cubic-bezier(0.16, 1, 0.3, 1),
    filter 260ms ease;
}
.preview-morph-enter-active .preview-stage img,
.preview-morph-leave-active .preview-stage img {
  transition: opacity 260ms ease 45ms, transform 340ms cubic-bezier(0.16, 1, 0.3, 1) 25ms;
}
.preview-morph-enter-from,
.preview-morph-leave-to { opacity: 0; backdrop-filter: blur(0); }
.preview-morph-enter-from .preview-dialog {
  opacity: 0;
  transform: translateY(24px) scale(0.88);
  filter: blur(10px);
}
.preview-morph-leave-to .preview-dialog {
  opacity: 0;
  transform: translateY(12px) scale(0.94);
  filter: blur(6px);
}
.preview-morph-enter-from .preview-stage img,
.preview-morph-leave-to .preview-stage img { opacity: 0; transform: scale(0.96); }

@media (prefers-reduced-motion: reduce) {
  .preview-morph-enter-active,
  .preview-morph-leave-active,
  .preview-morph-enter-active .preview-dialog,
  .preview-morph-leave-active .preview-dialog,
  .preview-morph-enter-active .preview-stage img,
  .preview-morph-leave-active .preview-stage img { transition-duration: 0.01ms; }
}
</style>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Copy, ExternalLink, Info, Maximize2, Minimize2, Sparkles, X } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Button from './ui/Button/Button.vue'
import Badge from './ui/Badge/Badge.vue'

const props = defineProps<{ image: ImageAsset | null }>()
const emit = defineEmits<{ close: [] }>()

const copied = ref(false)
const showInfo = ref(false)
const fitMode = ref<'contain' | 'cover'>('contain')

function handleKeydown(event: KeyboardEvent) {
  if (!props.image) return
  if (event.key === 'Escape' || event.code === 'Space' || event.key === ' ') {
    event.preventDefault()
    event.stopPropagation()
    emit('close')
  }
}

async function copyPath() {
  if (!props.image) return
  await imagyxApi.copyImageToClipboard(props.image.path)
  copied.value = true
  setTimeout(() => { copied.value = false }, 2000)
}

async function openInFolder() {
  if (!props.image) return
  await imagyxApi.openInFileManager(props.image.path)
}

onMounted(() => window.addEventListener('keydown', handleKeydown, true))
onBeforeUnmount(() => window.removeEventListener('keydown', handleKeydown, true))
</script>

<template>
  <Teleport to="body">
    <Transition name="preview-fade" appear>
      <div v-if="image" class="preview-overlay" role="presentation" @click.self="emit('close')">
        <div class="preview-modal" role="dialog" aria-modal="true" :aria-label="`Aperçu de ${image.name}`">
          
          <!-- Control Bar -->
          <header class="preview-bar">
            <div class="file-info">
              <span class="file-name">{{ image.name }}</span>
              <span class="file-meta">{{ image.width }} × {{ image.height }} px · {{ formatBytes(image.sizeBytes) }}</span>
            </div>

            <div class="bar-actions">
              <Button
                variant="ghost"
                size="sm"
                :title="fitMode === 'contain' ? 'Ajuster à l\'écran' : 'Taille réelle'"
                @click="fitMode = fitMode === 'contain' ? 'cover' : 'contain'"
              >
                <Maximize2 v-if="fitMode === 'contain'" :size="15" />
                <Minimize2 v-else :size="15" />
              </Button>

              <Button
                variant="ghost"
                size="sm"
                title="Copier le presse-papier"
                @click="copyPath"
              >
                <Copy :size="15" />
                <span>{{ copied ? 'Copié !' : 'Copier' }}</span>
              </Button>

              <Button
                variant="ghost"
                size="sm"
                title="Ouvrir dans l'explorateur"
                @click="openInFolder"
              >
                <ExternalLink :size="15" />
                <span>Révéler</span>
              </Button>

              <Button
                variant="ghost"
                size="sm"
                :class="{ active: showInfo }"
                title="Détails & Tags IA"
                @click="showInfo = !showInfo"
              >
                <Info :size="15" />
              </Button>

              <div class="divider" />

              <Button variant="ghost" size="icon" aria-label="Fermer" class="close-btn" @click="emit('close')">
                <X :size="18" />
              </Button>
            </div>
          </header>

          <!-- Main Viewport -->
          <div class="preview-content" :class="{ 'with-sidebar': showInfo }">
            <div class="stage-wrapper">
              <img
                :src="imagyxApi.fileUrl(image.path)"
                :alt="image.name"
                class="stage-image"
                :style="{ objectFit: fitMode }"
              />
            </div>

            <!-- Details Drawer -->
            <aside v-if="showInfo" class="info-drawer">
              <h3>Détails de l'image</h3>
              
              <div class="info-group">
                <label>Nom</label>
                <p>{{ image.name }}</p>
              </div>

              <div class="info-group">
                <label>Dimensions</label>
                <p>{{ image.width }} × {{ image.height }} pixels</p>
              </div>

              <div class="info-group">
                <label>Taille</label>
                <p>{{ formatBytes(image.sizeBytes) }}</p>
              </div>

              <div class="info-group">
                <label>Chemin d'accès</label>
                <p class="path-text">{{ image.path }}</p>
              </div>

              <div v-if="image.semanticMatches?.length" class="info-group">
                <label class="label-ia"><Sparkles :size="13" /> Détections IA</label>
                <div class="tags-cloud">
                  <Badge
                    v-for="(match, idx) in image.semanticMatches"
                    :key="idx"
                    variant="primary"
                  >
                    {{ match.label }} ({{ Math.round(match.score * 100) }}%)
                  </Badge>
                </div>
              </div>
            </aside>
          </div>

        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.preview-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  background: rgba(10, 12, 16, 0.82);
  backdrop-filter: blur(20px) saturate(1.2);
}

.preview-modal {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 1200px;
  height: 100%;
  max-height: 840px;
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.45);
  overflow: hidden;
  animation: modalPop 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

.preview-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 52px;
  padding: 0 var(--space-4);
  background: var(--surface-elevated);
  border-bottom: 1px solid var(--border);
}

.file-info {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.file-name {
  font-weight: 600;
  font-size: var(--text-sm);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-meta {
  font-size: var(--text-xs);
  color: var(--text-muted);
  white-space: nowrap;
}

.bar-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.divider {
  width: 1px;
  height: 20px;
  background: var(--border);
  margin: 0 var(--space-1);
}

.close-btn {
  border-radius: var(--radius-full);
}

.preview-content {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: #090a0d;
}

.stage-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-6);
  min-width: 0;
  overflow: hidden;
}

.stage-image {
  max-width: 100%;
  max-height: 100%;
  border-radius: var(--radius-md);
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6);
  transition: transform 0.2s ease;
}

.info-drawer {
  width: 300px;
  padding: var(--space-5);
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
}

.info-drawer h3 {
  margin: 0;
  font-size: var(--text-md);
  font-weight: 600;
}

.info-group label {
  display: block;
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin-bottom: var(--space-1);
  font-weight: 500;
}

.label-ia {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--primary) !important;
}

.info-group p {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text);
  word-break: break-word;
}

.path-text {
  font-family: monospace;
  font-size: var(--text-xs) !important;
  color: var(--text-muted) !important;
  background: var(--surface-hover);
  padding: 6px;
  border-radius: var(--radius-sm);
}

.tags-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
  margin-top: 4px;
}

@keyframes modalPop {
  from {
    opacity: 0;
    transform: scale(0.96) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.preview-fade-enter-active,
.preview-fade-leave-active {
  transition: opacity 0.2s ease;
}

.preview-fade-enter-from,
.preview-fade-leave-to {
  opacity: 0;
}
</style>

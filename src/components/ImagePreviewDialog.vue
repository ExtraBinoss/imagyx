<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Check, Copy, FolderOpen, Info, Maximize2, Minimize2, Sparkles, X } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { usePlatformStore } from '../stores/platform'
import { formatBytes } from '../utils'
import Button from './ui/Button/Button.vue'
import Badge from './ui/Badge/Badge.vue'

const props = defineProps<{ image: ImageAsset | null }>()
const emit = defineEmits<{ close: [] }>()

const platform = usePlatformStore()
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

async function copyImage() {
  if (!props.image) return
  try {
    await imagyxApi.copyImage(props.image.path)
    copied.value = true
    setTimeout(() => { copied.value = false }, 1800)
  } catch {
    /* fallback copy path */
    await imagyxApi.copyImageToClipboard(props.image.path)
    copied.value = true
    setTimeout(() => { copied.value = false }, 1800)
  }
}

async function revealInFolder() {
  if (!props.image) return
  await imagyxApi.openInFileManager(props.image.path, true)
}

onMounted(() => window.addEventListener('keydown', handleKeydown, true))
onBeforeUnmount(() => window.removeEventListener('keydown', handleKeydown, true))
</script>

<template>
  <Teleport to="body">
    <Transition name="preview-fade" appear>
      <div v-if="image" class="preview-overlay" role="presentation" @click.self="emit('close')">
        <div class="preview-modal" role="dialog" aria-modal="true" :aria-label="`Aperçu de ${image.name}`">
          
          <!-- Header Bar -->
          <header class="preview-bar">
            <div class="file-info">
              <span class="file-name">{{ image.name }}</span>
              <span class="file-meta">{{ image.width }} × {{ image.height }} px · {{ formatBytes(image.sizeBytes) }}</span>
            </div>

            <div class="bar-actions">
              <Button
                class="spotlight-action-button"
                variant="secondary"
                size="sm"
                aria-label="Copier l’image"
                @click="copyImage"
              >
                <template #leading>
                  <Check v-if="copied" :size="14" />
                  <Copy v-else :size="14" />
                </template>
                {{ copied ? 'Copiée' : 'Copier' }}
              </Button>

              <Button
                class="spotlight-action-button"
                variant="secondary"
                size="sm"
                :aria-label="`Ouvrir dans ${platform.fileManagerName}`"
                @click="revealInFolder"
              >
                <template #leading><FolderOpen :size="14" /></template>
                {{ platform.fileManagerName }}
              </Button>

              <Button
                variant="ghost"
                size="icon"
                :class="{ 'btn-info--active': showInfo }"
                class="btn-info"
                title="Détails & Tags IA"
                @click="showInfo = !showInfo"
              >
                <Info :size="15" />
              </Button>

              <div class="divider" />

              <Button variant="ghost" size="icon" aria-label="Fermer" class="close-btn" @click="emit('close')">
                <X :size="16" />
              </Button>
            </div>
          </header>

          <!-- Main Content -->
          <div class="preview-content">
            <div class="stage-wrapper" @click="fitMode = fitMode === 'contain' ? 'cover' : 'contain'">
              <img
                :src="imagyxApi.fileUrl(image.path)"
                :alt="image.name"
                class="stage-image"
                :style="{ objectFit: fitMode }"
              />
            </div>

            <!-- Animated Details Drawer -->
            <Transition name="drawer-slide">
              <aside v-if="showInfo" class="info-drawer">
                <div class="info-drawer__header">
                  <h3>Informations</h3>
                  <Badge variant="primary" class="ai-badge">
                    <Sparkles :size="12" /> IA Local
                  </Badge>
                </div>
                
                <div class="info-drawer__body">
                  <div class="info-item">
                    <span class="info-item__label">Nom de fichier</span>
                    <span class="info-item__value">{{ image.name }}</span>
                  </div>

                  <div class="info-item">
                    <span class="info-item__label">Dimensions</span>
                    <span class="info-item__value">{{ image.width }} × {{ image.height }} px</span>
                  </div>

                  <div class="info-item">
                    <span class="info-item__label">Taille sur le disque</span>
                    <span class="info-item__value">{{ formatBytes(image.sizeBytes) }}</span>
                  </div>

                  <div class="info-item">
                    <span class="info-item__label">Emplacement</span>
                    <span class="info-item__path">{{ image.path }}</span>
                  </div>

                  <div v-if="image.semanticMatches?.length" class="info-item">
                    <span class="info-item__label">Concepts détectés</span>
                    <div class="tags-cloud">
                      <Badge
                        v-for="(match, idx) in image.semanticMatches"
                        :key="idx"
                        variant="secondary"
                        class="semantic-tag"
                      >
                        {{ match.label }} <small>{{ Math.round(match.score * 100) }}%</small>
                      </Badge>
                    </div>
                  </div>
                </div>
              </aside>
            </Transition>
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
  padding: 24px;
  background: color-mix(in srgb, var(--overlay) 75%, transparent);
  backdrop-filter: blur(28px) saturate(1.18);
}

.preview-modal {
  display: flex;
  flex-direction: column;
  width: 90vw;
  max-width: 960px;
  height: 82vh;
  max-height: 680px;
  background: var(--surface-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-popover);
  overflow: hidden;
  animation: appleModalPop 280ms cubic-bezier(0.16, 1, 0.3, 1) both;
}

.preview-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 52px;
  padding: 0 16px;
  background: color-mix(in srgb, var(--surface) 88%, transparent);
  border-bottom: 1px solid var(--border);
  backdrop-filter: blur(16px);
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
  font-variant-numeric: tabular-nums;
}

.bar-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.divider {
  width: 1px;
  height: 18px;
  background: var(--border);
  margin: 0 2px;
}

.btn-info {
  transition: transform var(--transition-fast), color var(--transition-fast), background-color var(--transition-fast);
}

.btn-info--active {
  color: var(--primary-text) !important;
  background: var(--primary-soft) !important;
  transform: rotate(18deg) scale(1.08);
}

.preview-content {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: color-mix(in srgb, var(--background) 96%, black);
}

.stage-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  min-width: 0;
  overflow: hidden;
  cursor: zoom-in;
}

.stage-image {
  max-width: 100%;
  max-height: 100%;
  border-radius: var(--radius-md);
  box-shadow: 0 14px 40px -8px rgba(0, 0, 0, 0.42);
  transition: transform 220ms ease;
}

.info-drawer {
  width: 280px;
  height: 100%;
  padding: 16px;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
  scrollbar-width: thin;
}

.info-drawer__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}

.info-drawer__header h3 {
  margin: 0;
  font-size: var(--text-sm);
  font-weight: 650;
  color: var(--text);
}

.ai-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
}

.info-drawer__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.info-item__label {
  font-size: 10px;
  font-weight: 500;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.info-item__value {
  font-size: var(--text-xs);
  color: var(--text);
  font-weight: 550;
}

.info-item__path {
  font-family: monospace;
  font-size: 10px;
  color: var(--text-muted);
  background: var(--surface-hover);
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  word-break: break-all;
  border: 1px solid var(--border);
}

.tags-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 4px;
}

.semantic-tag {
  font-size: 11px;
  padding: 3px 8px;
}

.semantic-tag small {
  opacity: 0.7;
  margin-left: 2px;
}

/* Animations */
@keyframes appleModalPop {
  0% {
    opacity: 0;
    transform: scale(0.96) translateY(8px);
    filter: blur(4px);
  }
  100% {
    opacity: 1;
    transform: scale(1) translateY(0);
    filter: blur(0);
  }
}

.drawer-slide-enter-active,
.drawer-slide-leave-active {
  transition: transform 240ms cubic-bezier(0.16, 1, 0.3, 1), opacity 180ms ease, margin-right 240ms cubic-bezier(0.16, 1, 0.3, 1);
}

.drawer-slide-enter-from,
.drawer-slide-leave-to {
  transform: translateX(100%);
  opacity: 0;
  margin-right: -280px;
}

.preview-fade-enter-active,
.preview-fade-leave-active {
  transition: opacity 200ms ease;
}

.preview-fade-enter-from,
.preview-fade-leave-to {
  opacity: 0;
}
</style>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Check, Copy, ExternalLink, FolderOpen, Image as ImageIcon, LoaderCircle, Search, X } from '@lucide/vue'
import type { ImageAsset } from './types'
import { imagyxApi } from './api/tauri'
import { semanticRuntime } from './services/semantic'
import { usePlatformStore } from './stores/platform'
import { debounce, formatBytes } from './utils'
import ThumbnailImage from './components/ThumbnailImage.vue'

const platform = usePlatformStore()
const query = ref('')
const results = ref<ImageAsset[]>([])
const selectedIndex = ref(0)
const searching = ref(false)
const error = ref<string | null>(null)
const copiedImageId = ref<string | null>(null)
const opening = ref(false)
const searchInput = ref<HTMLInputElement | null>(null)
let searchSequence = 0
let copyTimer: number | undefined
let unlistenOpened: UnlistenFn | null = null
let unlistenFocus: UnlistenFn | null = null

const selectedImage = computed(() => results.value[selectedIndex.value] ?? null)
const resultLabel = computed(() => {
  if (!query.value.trim()) return 'Tape une description, une couleur ou un nom de fichier'
  if (searching.value) return 'Recherche locale en cours…'
  return `${results.value.length} résultat${results.value.length > 1 ? 's' : ''}`
})

const searchLater = debounce(() => { void runSearch() }, 120)
watch(query, () => {
  selectedIndex.value = 0
  searchLater()
})

async function runSearch() {
  const sequence = ++searchSequence
  const text = query.value.trim()
  error.value = null
  if (!text) {
    results.value = []
    searching.value = false
    return
  }
  searching.value = true
  try {
    const embedded = await semanticRuntime.embedQuery(text)
    if (sequence !== searchSequence) return
    results.value = await imagyxApi.search({
      query: text,
      queryVector: embedded?.queryVector,
      limit: 60,
    })
  } catch (reason) {
    if (sequence === searchSequence) error.value = String(reason)
  } finally {
    if (sequence === searchSequence) searching.value = false
  }
}

async function copyImage(image: ImageAsset) {
  await imagyxApi.copyImage(image.path)
  copiedImageId.value = image.id
  if (copyTimer) window.clearTimeout(copyTimer)
  copyTimer = window.setTimeout(() => { copiedImageId.value = null }, 1800)
}

async function revealImage(image: ImageAsset) {
  await imagyxApi.openInFileManager(image.path, true)
}

async function openImage(image: ImageAsset) {
  await imagyxApi.openInImagyx(image.id)
}

function moveSelection(delta: number) {
  if (!results.value.length) return
  selectedIndex.value = (selectedIndex.value + delta + results.value.length) % results.value.length
  void nextTick(() => document.querySelector<HTMLElement>(`[data-result-index="${selectedIndex.value}"]`)?.scrollIntoView({ block: 'nearest' }))
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    void imagyxApi.hideSpotlight()
    return
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    moveSelection(1)
    return
  }
  if (event.key === 'ArrowUp') {
    event.preventDefault()
    moveSelection(-1)
    return
  }
  if (event.key === 'Enter' && selectedImage.value) {
    event.preventDefault()
    void openImage(selectedImage.value)
  }
}

function animateOpen() {
  opening.value = false
  query.value = ''
  results.value = []
  selectedIndex.value = 0
  error.value = null
  void nextTick(() => {
    opening.value = true
    searchInput.value?.focus()
  })
}

onMounted(async () => {
  void platform.initialize()
  window.addEventListener('keydown', handleKeydown)
  unlistenOpened = await listen('spotlight-opened', animateOpen)
  unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload }) => {
    if (!payload) void imagyxApi.hideSpotlight()
  })
  animateOpen()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
  unlistenOpened?.()
  unlistenFocus?.()
  if (copyTimer) window.clearTimeout(copyTimer)
})
</script>

<template>
  <main class="spotlight-root">
    <section class="spotlight-panel" :class="{ 'spotlight-panel--opening': opening }" aria-label="Recherche rapide Imagyx">
      <header class="spotlight-header">
        <div class="spotlight-search">
          <Search :size="22" :stroke-width="1.9" />
          <input
            ref="searchInput"
            v-model="query"
            type="search"
            autocomplete="off"
            spellcheck="false"
            placeholder="Chercher dans tes images…"
            aria-label="Recherche rapide"
          />
          <LoaderCircle v-if="searching" class="spin" :size="18" />
          <kbd v-else>Ctrl · Num 9</kbd>
        </div>
        <button class="spotlight-close" type="button" aria-label="Fermer" @click="imagyxApi.hideSpotlight()">
          <X :size="17" />
        </button>
      </header>

      <div class="spotlight-summary">
        <span>{{ resultLabel }}</span>
        <span v-if="query.trim()">Entrée pour ouvrir · ↑↓ pour naviguer</span>
      </div>

      <div class="spotlight-results" role="listbox" :aria-label="resultLabel">
        <div
          v-for="(image, index) in results"
          :key="image.id"
          class="spotlight-result"
          :class="{ 'spotlight-result--selected': index === selectedIndex }"
          :data-result-index="index"
          :aria-selected="index === selectedIndex"
          role="option"
          tabindex="-1"
          :style="{ animationDelay: `${Math.min(index, 10) * 22}ms` }"
          @mouseenter="selectedIndex = index"
          @focus="selectedIndex = index"
          @click="selectedIndex = index"
          @dblclick="openImage(image)"
        >
          <span class="spotlight-thumb"><ThumbnailImage :image="image" /></span>
          <span class="spotlight-copy">
            <strong>{{ image.name }}</strong>
            <small>{{ image.width }} × {{ image.height }} · {{ formatBytes(image.sizeBytes) }}</small>
          </span>
          <span v-if="image.semanticScore != null" class="spotlight-score">{{ Math.round(image.semanticScore * 100) }}%</span>
          <span class="spotlight-actions">
            <button type="button" aria-label="Copier l’image" @click.stop="copyImage(image)">
              <Check v-if="copiedImageId === image.id" :size="15" />
              <Copy v-else :size="15" />
              <span>{{ copiedImageId === image.id ? 'Copiée' : 'Copier' }}</span>
            </button>
            <button type="button" :aria-label="platform.openFolderLabel" @click.stop="revealImage(image)">
              <FolderOpen :size="15" />
              <span>{{ platform.fileManagerName }}</span>
            </button>
            <button type="button" aria-label="Ouvrir dans Imagyx" @click.stop="openImage(image)">
              <ExternalLink :size="15" />
              <span>Imagyx</span>
            </button>
          </span>
        </div>

        <div v-if="!query.trim()" class="spotlight-empty">
          <span class="spotlight-empty__icon"><ImageIcon :size="28" /></span>
          <strong>Ta photothèque, sans quitter ce que tu fais</strong>
          <p>Décris une scène, une personne, un objet ou tape simplement un nom de fichier.</p>
        </div>
        <div v-else-if="!searching && !results.length && !error" class="spotlight-empty">
          <span class="spotlight-empty__icon"><Search :size="28" /></span>
          <strong>Aucun résultat convaincant</strong>
          <p>Essaie une description plus courte ou un mot visuel plus précis.</p>
        </div>
        <div v-if="error" class="spotlight-error">{{ error }}</div>
      </div>

      <footer class="spotlight-footer">
        <span><i></i> Recherche locale · MobileCLIP-S0</span>
        <span>Échap pour fermer</span>
      </footer>
    </section>
  </main>
</template>

<style scoped>
.spotlight-root {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}

.spotlight-panel {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
  width: 100%;
  height: 100%;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
  border-radius: 24px;
  background: color-mix(in srgb, var(--surface-elevated) 94%, transparent);
  box-shadow: 0 32px 90px rgb(2 6 23 / 0.34), 0 8px 28px rgb(2 6 23 / 0.16);
  backdrop-filter: blur(26px) saturate(1.18);
  transform-origin: 50% 18%;
}
.spotlight-panel--opening { animation: spotlight-pop 330ms cubic-bezier(0.16, 1, 0.3, 1) both; }

.spotlight-header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 18px 18px 10px;
}
.spotlight-search {
  display: flex;
  align-items: center;
  gap: 13px;
  flex: 1;
  min-width: 0;
  min-height: 56px;
  padding: 0 16px;
  border: 1px solid var(--border);
  border-radius: 17px;
  background: color-mix(in srgb, var(--surface) 90%, transparent);
  color: var(--text-muted);
  box-shadow: inset 0 1px rgb(255 255 255 / 0.06);
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast), transform var(--transition-fast);
}
.spotlight-search:focus-within {
  border-color: var(--primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 16%, transparent), inset 0 1px rgb(255 255 255 / 0.08);
  transform: translateY(-1px);
}
.spotlight-search input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 19px;
  font-weight: 560;
  letter-spacing: -0.3px;
}
.spotlight-search input::placeholder { color: var(--text-subtle); }
.spotlight-search kbd {
  flex: 0 0 auto;
  padding: 5px 8px;
  border: 1px solid var(--border);
  border-bottom-color: var(--border-strong);
  border-radius: 7px;
  background: var(--surface-hover);
  color: var(--text-muted);
  font-family: inherit;
  font-size: 10px;
  box-shadow: 0 2px 0 var(--border);
}
.spotlight-close {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  padding: 0;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}
.spotlight-close:hover { background: var(--surface-hover); color: var(--text); }

.spotlight-summary,
.spotlight-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  color: var(--text-muted);
  font-size: 11px;
}
.spotlight-summary { padding: 0 22px 10px; }
.spotlight-footer { min-height: 38px; padding: 0 20px; border-top: 1px solid var(--border); }
.spotlight-footer span { display: flex; align-items: center; gap: 7px; }
.spotlight-footer i { width: 7px; height: 7px; border-radius: 50%; background: var(--success-text); box-shadow: 0 0 0 4px color-mix(in srgb, var(--success-text) 12%, transparent); }

.spotlight-results {
  min-height: 0;
  overflow-y: auto;
  padding: 2px 10px 12px;
  scrollbar-gutter: stable;
}
.spotlight-result {
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 68px;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: 14px;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: default;
  opacity: 0;
  animation: result-rise 260ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  transition: background-color var(--transition-fast), border-color var(--transition-fast), transform var(--transition-fast);
}
.spotlight-result--selected {
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  transform: translateX(2px);
}
.spotlight-thumb {
  width: 52px;
  height: 52px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-hover);
}
.spotlight-thumb :deep(img) { width: 100%; height: 100%; object-fit: cover; }
.spotlight-copy { min-width: 0; }
.spotlight-copy strong,
.spotlight-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-copy strong { color: var(--text); font-size: 13px; }
.spotlight-copy small { margin-top: 5px; color: var(--text-muted); font-size: 10px; }
.spotlight-score {
  padding: 4px 7px;
  border-radius: var(--radius-full);
  background: var(--primary-soft);
  color: var(--primary-text);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.spotlight-actions { display: flex; align-items: center; gap: 4px; opacity: 0; transform: translateX(6px); transition: opacity var(--transition-fast), transform var(--transition-fast); }
.spotlight-result--selected .spotlight-actions,
.spotlight-result:focus-within .spotlight-actions { opacity: 1; transform: translateX(0); }
.spotlight-actions button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-height: 31px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: color-mix(in srgb, var(--surface) 86%, transparent);
  color: var(--text-secondary);
  font: inherit;
  font-size: 10px;
  cursor: pointer;
}
.spotlight-actions button:hover { border-color: var(--primary-border); background: var(--primary-soft); color: var(--primary-text); }

.spotlight-empty {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 320px;
  padding: 32px;
  color: var(--text-muted);
  text-align: center;
}
.spotlight-empty__icon { display: grid; place-items: center; width: 58px; height: 58px; margin-bottom: 15px; border: 1px solid var(--border); border-radius: 18px; background: var(--surface-hover); color: var(--primary-text); }
.spotlight-empty strong { color: var(--text); font-size: 14px; }
.spotlight-empty p { max-width: 380px; margin: 8px 0 0; font-size: 11px; line-height: 1.55; }
.spotlight-error { margin: 12px; padding: 12px; border: 1px solid var(--danger-border); border-radius: 12px; background: var(--danger-surface); color: var(--danger-text); font-size: 11px; }
.spin { animation: spin 0.9s linear infinite; }

@keyframes spotlight-pop {
  0% { opacity: 0; transform: translateY(-18px) scale(0.9); filter: blur(10px); }
  62% { opacity: 1; transform: translateY(2px) scale(1.008); filter: blur(0); }
  100% { opacity: 1; transform: none; filter: blur(0); }
}
@keyframes result-rise {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes spin { to { transform: rotate(360deg); } }

@media (max-width: 720px) {
  .spotlight-actions button span { display: none; }
  .spotlight-result { grid-template-columns: 48px minmax(0, 1fr) auto auto; }
}
@media (prefers-reduced-motion: reduce) {
  .spotlight-panel--opening,
  .spotlight-result { animation-duration: 0.01ms; }
}
</style>

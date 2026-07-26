<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Check, Copy, ExternalLink, FolderOpen, LoaderCircle, Search } from '@lucide/vue'
import type { ImageAsset } from './types'
import { imagyxApi } from './api/tauri'
import { semanticRuntime } from './services/semantic'
import { usePlatformStore } from './stores/platform'
import { debounce, formatBytes } from './utils'
import ThumbnailImage from './components/ThumbnailImage.vue'
import Button from './components/ui/Button/Button.vue'
import MovingBorder from './components/ui/MovingBorder/MovingBorder.vue'

const TAG_CACHE_KEY = 'imagyx.spotlight-top-tags.v1'
const FALLBACK_TAGS = [
  'femme', 'portrait', 'paysage', 'chien', 'chat',
  'voiture', 'plage', 'ville', 'nuit', 'coucher de soleil',
]
const initialTags = readCachedTags()

const platform = usePlatformStore()
const currentWindow = getCurrentWindow()
const query = ref('')
const results = ref<ImageAsset[]>([])
const selectedIndex = ref(0)
const searching = ref(false)
const error = ref<string | null>(null)
const copiedImageId = ref<string | null>(null)
const visible = ref(false)
const inputFocused = ref(false)
const resultsOpen = ref(false)
const shellMerged = ref(false)
const searchInput = ref<HTMLInputElement | null>(null)
const resultsViewport = ref<HTMLElement | null>(null)
const canScrollDown = ref(false)
const topTags = ref(initialTags)
const typedTag = ref(initialTags[0] ?? FALLBACK_TAGS[0] ?? 'image')
const resultCache = new Map<string, ImageAsset[]>()

let searchSequence = 0
let morphSequence = 0
let copyTimer: number | undefined
let tagWarmupTimer: number | undefined
let typewriterTimer: number | undefined
let collapseTimer: number | undefined
let expanded = false
let expansionPromise: Promise<void> | null = null
let tagIndex = 0
let characterIndex = typedTag.value.length
let deleting = false
let unlistenWillOpen: UnlistenFn | null = null
let unlistenOpened: UnlistenFn | null = null
let unlistenWillHide: UnlistenFn | null = null
let unlistenFocus: UnlistenFn | null = null

const selectedImage = computed(() => results.value[selectedIndex.value] ?? null)
const hasQuery = computed(() => Boolean(query.value.trim()))
const resultLabel = computed(() => {
  if (searching.value && results.value.length === 0) return 'Recherche…'
  return `${results.value.length} résultat${results.value.length === 1 ? '' : 's'}`
})
const placeholder = computed(() => typedTag.value ? `${capitalize(typedTag.value)}…` : '\u00a0')

const searchLater = debounce(() => { void runSearch() }, 65)

watch(query, (value) => {
  selectedIndex.value = 0
  error.value = null
  const text = value.trim()
  const request = ++morphSequence

  if (!text) {
    searchSequence += 1
    results.value = []
    searching.value = false
    void closeResults(request)
    return
  }

  searchLater()
  void openResults(request)
})

watch(() => results.value.length, () => {
  void nextTick(updateScrollShadow)
})

async function openResults(request: number) {
  if (collapseTimer) {
    window.clearTimeout(collapseTimer)
    collapseTimer = undefined
  }

  try {
    await ensureExpanded()
  } catch (reason) {
    if (request === morphSequence) error.value = String(reason)
    return
  }

  if (request !== morphSequence || !query.value.trim()) return
  await nextPaint(2)
  if (request !== morphSequence || !query.value.trim()) return

  shellMerged.value = true
  await nextPaint()
  if (request !== morphSequence || !query.value.trim()) return

  resultsOpen.value = true
  await nextTick()
  await nextPaint()
  updateScrollShadow()
}

async function closeResults(request: number) {
  resultsOpen.value = false
  canScrollDown.value = false
  await nextPaint()
  if (request !== morphSequence || query.value.trim()) return

  shellMerged.value = false
  if (collapseTimer) window.clearTimeout(collapseTimer)
  collapseTimer = window.setTimeout(() => {
    if (request !== morphSequence || query.value.trim()) return
    void setCompact()
  }, 280)
}

async function ensureExpanded() {
  if (expanded) return
  if (!expansionPromise) {
    expansionPromise = imagyxApi.setSpotlightExpanded(true)
      .then(() => { expanded = true })
      .finally(() => { expansionPromise = null })
  }
  await expansionPromise
}

async function setCompact() {
  if (!expanded) return
  try {
    await imagyxApi.setSpotlightExpanded(false)
    expanded = false
  } catch { /* le prochain lancement réinitialise aussi la taille côté Rust */ }
}

async function runSearch() {
  const sequence = ++searchSequence
  const text = query.value.trim()
  const cacheKey = text.toLocaleLowerCase('fr')
  if (!text) return

  const cached = resultCache.get(cacheKey)
  if (cached) {
    results.value = cached
    searching.value = false
    return
  }

  searching.value = true
  const lexicalPromise = imagyxApi.search({ query: text, limit: 60 })
  const embeddingPromise = text.length >= 2 ? semanticRuntime.embedQuery(text) : Promise.resolve(undefined)

  void lexicalPromise.then((lexicalResults) => {
    if (sequence !== searchSequence || query.value.trim() !== text) return
    if (lexicalResults.length > 0 || results.value.length === 0) results.value = lexicalResults
  }).catch(() => undefined)

  try {
    const embedded = await embeddingPromise
    if (sequence !== searchSequence || query.value.trim() !== text) return
    if (!embedded?.queryVector) {
      const lexicalResults = await lexicalPromise
      if (sequence === searchSequence) results.value = lexicalResults
      return
    }

    const semanticResults = await imagyxApi.search({
      query: text,
      queryVector: embedded.queryVector,
      limit: 60,
    })
    if (sequence !== searchSequence || query.value.trim() !== text) return
    results.value = semanticResults
    rememberResults(cacheKey, semanticResults)
  } catch (reason) {
    if (sequence === searchSequence) {
      error.value = String(reason)
      try {
        results.value = await lexicalPromise
      } catch { /* le message d’erreur principal suffit */ }
    }
  } finally {
    if (sequence === searchSequence) searching.value = false
  }
}

function rememberResults(key: string, images: ImageAsset[]) {
  resultCache.set(key, images)
  if (resultCache.size <= 24) return
  const oldest = resultCache.keys().next().value as string | undefined
  if (oldest) resultCache.delete(oldest)
}

async function copyImage(image: ImageAsset) {
  try {
    await imagyxApi.copyImage(image.path)
    copiedImageId.value = image.id
    if (copyTimer) window.clearTimeout(copyTimer)
    copyTimer = window.setTimeout(() => { copiedImageId.value = null }, 1800)
  } catch (reason) {
    error.value = String(reason)
  }
}

async function revealImage(image: ImageAsset) {
  try { await imagyxApi.openInFileManager(image.path, true) }
  catch (reason) { error.value = String(reason) }
}

async function openImage(image: ImageAsset) {
  try { await imagyxApi.openInImagyx(image.id) }
  catch (reason) { error.value = String(reason) }
}

function moveSelection(delta: number) {
  if (!results.value.length) return
  selectedIndex.value = (selectedIndex.value + delta + results.value.length) % results.value.length
  void nextTick(() => {
    document.querySelector<HTMLElement>(`[data-result-index="${selectedIndex.value}"]`)
      ?.scrollIntoView({ block: 'nearest' })
    updateScrollShadow()
  })
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

function updateScrollShadow() {
  const element = resultsViewport.value
  if (!element) {
    canScrollDown.value = false
    return
  }
  canScrollDown.value = element.scrollHeight > element.clientHeight + 2
    && element.scrollTop + element.clientHeight < element.scrollHeight - 2
}

function prepareOpen() {
  visible.value = false
  query.value = ''
  results.value = []
  selectedIndex.value = 0
  error.value = null
  resultsOpen.value = false
  shellMerged.value = false
  expanded = false
  expansionPromise = null
  canScrollDown.value = false
  morphSequence += 1
}

function animateOpen() {
  prepareOpen()
  void nextPaint(2).then(() => {
    visible.value = true
    searchInput.value?.focus()
  })
}

function prepareHide() {
  visible.value = false
  inputFocused.value = false
  query.value = ''
  results.value = []
  resultsOpen.value = false
  shellMerged.value = false
  expanded = false
  expansionPromise = null
  canScrollDown.value = false
  morphSequence += 1
}

function readCachedTags(): string[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(TAG_CACHE_KEY) ?? '[]') as unknown
    if (Array.isArray(parsed)) {
      const tags = parsed.filter((tag): tag is string => typeof tag === 'string' && tag.trim().length > 0).slice(0, 10)
      if (tags.length) return tags
    }
  } catch { /* cache facultatif */ }
  return [...FALLBACK_TAGS]
}

async function warmSearchRuntime() {
  try { await semanticRuntime.prewarmText() }
  catch { /* la recherche lexicale reste immédiatement disponible */ }
}

async function warmTagSuggestions() {
  try {
    await semanticRuntime.prewarmText()
    const concepts = await semanticRuntime.genericImageConcepts()
    const ranked = await imagyxApi.topImageTags(concepts, 10)
    if (ranked.length) {
      topTags.value = ranked
      localStorage.setItem(TAG_CACHE_KEY, JSON.stringify(ranked))
      tagIndex %= ranked.length
    }
  } catch { /* le fallback garde le launcher instantané */ }
}

function runTypewriter() {
  if (query.value.trim()) {
    typewriterTimer = window.setTimeout(runTypewriter, 220)
    return
  }

  const tags = topTags.value.length ? topTags.value : FALLBACK_TAGS
  const target = tags[tagIndex % tags.length] ?? 'image'
  if (!deleting) {
    characterIndex = Math.min(target.length, characterIndex + 1)
    typedTag.value = target.slice(0, characterIndex)
    if (characterIndex >= target.length) {
      deleting = true
      typewriterTimer = window.setTimeout(runTypewriter, 1250)
      return
    }
    typewriterTimer = window.setTimeout(runTypewriter, 58)
    return
  }

  characterIndex = Math.max(0, characterIndex - 1)
  typedTag.value = target.slice(0, characterIndex)
  if (characterIndex === 0) {
    deleting = false
    tagIndex = (tagIndex + 1) % tags.length
    typewriterTimer = window.setTimeout(runTypewriter, 150)
    return
  }
  typewriterTimer = window.setTimeout(runTypewriter, 32)
}

function capitalize(value: string) {
  return value ? value.charAt(0).toLocaleUpperCase('fr') + value.slice(1) : value
}

function nextPaint(frames = 1): Promise<void> {
  return new Promise((resolve) => {
    let remaining = Math.max(1, frames)
    const tick = () => {
      remaining -= 1
      if (remaining <= 0) resolve()
      else window.requestAnimationFrame(tick)
    }
    window.requestAnimationFrame(tick)
  })
}

onMounted(async () => {
  void platform.initialize()
  void warmSearchRuntime()
  window.addEventListener('keydown', handleKeydown)
  unlistenWillOpen = await listen('spotlight-will-open', prepareOpen)
  unlistenOpened = await listen('spotlight-opened', animateOpen)
  unlistenWillHide = await listen('spotlight-will-hide', prepareHide)
  unlistenFocus = await currentWindow.onFocusChanged(({ payload }) => {
    if (!payload) void imagyxApi.hideSpotlight()
  })
  tagWarmupTimer = window.setTimeout(() => { void warmTagSuggestions() }, 1100)
  typewriterTimer = window.setTimeout(runTypewriter, 900)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
  unlistenWillOpen?.()
  unlistenOpened?.()
  unlistenWillHide?.()
  unlistenFocus?.()
  if (copyTimer) window.clearTimeout(copyTimer)
  if (tagWarmupTimer) window.clearTimeout(tagWarmupTimer)
  if (typewriterTimer) window.clearTimeout(typewriterTimer)
  if (collapseTimer) window.clearTimeout(collapseTimer)
})
</script>

<template>
  <main class="spotlight-root">
    <section class="spotlight-stage" :class="{ 'spotlight-stage--visible': visible }" aria-label="Recherche rapide Imagyx">
      <MovingBorder
        class="spotlight-moving-border"
        border-radius="22px"
        :duration="inputFocused || searching ? 3200 : 4700"
        :active="visible"
      >
        <div class="spotlight-surface" :class="{ 'spotlight-surface--expanded': shellMerged }">
          <div class="spotlight-search">
            <Search :size="22" :stroke-width="1.9" />
            <input
              ref="searchInput"
              v-model="query"
              type="search"
              autocomplete="off"
              spellcheck="false"
              :placeholder="placeholder"
              aria-label="Recherche rapide"
              @focus="inputFocused = true"
              @blur="inputFocused = false"
            />
            <span v-if="hasQuery && !searching" class="spotlight-result-count">{{ resultLabel }}</span>
            <LoaderCircle v-if="searching" class="spin" :size="18" />
            <kbd v-else-if="!hasQuery">Ctrl · Num 9</kbd>
          </div>

          <Transition name="results-morph">
            <section v-if="resultsOpen" class="spotlight-results-panel" aria-live="polite">
              <div class="spotlight-results-shell">
                <div
                  ref="resultsViewport"
                  class="spotlight-results"
                  role="listbox"
                  :aria-label="resultLabel"
                  @scroll.passive="updateScrollShadow"
                >
                  <div
                    v-for="(image, index) in results"
                    :key="image.id"
                    class="spotlight-result"
                    :class="{ 'spotlight-result--selected': index === selectedIndex }"
                    :data-result-index="index"
                    :aria-selected="index === selectedIndex"
                    role="option"
                    tabindex="-1"
                    :style="{ animationDelay: `${Math.min(index, 10) * 18}ms` }"
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
                      <Button class="spotlight-action-button" variant="secondary" size="sm" aria-label="Copier l’image" @click.stop="copyImage(image)">
                        <template #leading>
                          <Check v-if="copiedImageId === image.id" :size="14" />
                          <Copy v-else :size="14" />
                        </template>
                        {{ copiedImageId === image.id ? 'Copiée' : 'Copier' }}
                      </Button>
                      <Button class="spotlight-action-button" variant="secondary" size="sm" :aria-label="platform.openFolderLabel" @click.stop="revealImage(image)">
                        <template #leading><FolderOpen :size="14" /></template>
                        {{ platform.fileManagerName }}
                      </Button>
                      <Button class="spotlight-action-button" variant="primary" size="sm" aria-label="Ouvrir dans Imagyx" @click.stop="openImage(image)">
                        <template #leading><ExternalLink :size="14" /></template>
                        Imagyx
                      </Button>
                    </span>
                  </div>

                  <div v-if="searching && results.length === 0" class="spotlight-loading-list" aria-label="Recherche en cours">
                    <span v-for="item in 5" :key="item" :style="{ animationDelay: `${item * 45}ms` }" />
                  </div>

                  <div v-else-if="!searching && !results.length && !error" class="spotlight-empty">
                    <Search :size="24" />
                    <strong>Aucun résultat convaincant</strong>
                    <span>Essaie une description plus courte ou un mot plus visuel.</span>
                  </div>

                  <div v-if="error" class="spotlight-error">{{ error }}</div>
                </div>
                <div v-if="canScrollDown" class="spotlight-scroll-shadow" aria-hidden="true" />
              </div>
            </section>
          </Transition>
        </div>
      </MovingBorder>
    </section>
  </main>
</template>

<style scoped>
.spotlight-root {
  width: 100%;
  height: 100%;
  overflow: hidden;
  padding: 14px 14px 24px;
  background: transparent;
}

.spotlight-stage {
  width: 100%;
  opacity: 0;
  transform: translateY(-7px) scale(0.97);
  filter: blur(6px);
  pointer-events: none;
  transform-origin: 50% 18px;
}

.spotlight-stage--visible {
  opacity: 1;
  transform: none;
  filter: none;
  pointer-events: auto;
  animation: spotlight-pop 280ms cubic-bezier(0.16, 1, 0.3, 1) both;
}

.spotlight-moving-border {
  width: 100%;
  filter: drop-shadow(0 10px 22px rgb(15 23 42 / 0.09));
  transition: filter 240ms ease;
}

.spotlight-moving-border:has(.spotlight-surface--expanded) {
  filter: drop-shadow(0 18px 36px rgb(15 23 42 / 0.13));
}

:global(:root[data-theme='dark']) .spotlight-moving-border {
  filter: drop-shadow(0 11px 24px rgb(0 0 0 / 0.2));
}

:global(:root[data-theme='dark']) .spotlight-moving-border:has(.spotlight-surface--expanded) {
  filter: drop-shadow(0 20px 40px rgb(0 0 0 / 0.28));
}

.spotlight-surface {
  overflow: hidden;
  border-radius: 21px;
  background: color-mix(in srgb, var(--surface-elevated) 95%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.08),
    inset 0 -1px rgb(15 23 42 / 0.045);
  backdrop-filter: blur(26px) saturate(1.16);
  transition:
    background-color 180ms ease,
    box-shadow 220ms ease;
}

.spotlight-surface:focus-within {
  background: color-mix(in srgb, var(--surface-elevated) 98%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.1),
    inset 0 -1px rgb(15 23 42 / 0.04);
}

.spotlight-search {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 13px;
  min-height: 70px;
  padding: 0 18px;
  color: var(--text-muted);
}

.spotlight-search::after {
  content: '';
  position: absolute;
  right: 16px;
  bottom: 0;
  left: 16px;
  height: 1px;
  background: color-mix(in srgb, var(--border) 76%, transparent);
  opacity: 0;
  transform: scaleX(0.92);
  transition:
    opacity 160ms ease,
    transform 240ms cubic-bezier(0.16, 1, 0.3, 1);
}

.spotlight-surface--expanded .spotlight-search::after {
  opacity: 1;
  transform: scaleX(1);
}

.spotlight-search input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 20px;
  font-weight: 570;
  letter-spacing: -0.38px;
  caret-color: var(--primary);
}

.spotlight-search input::-webkit-search-cancel-button { display: none; }

.spotlight-search input::placeholder {
  color: var(--text-subtle);
  opacity: 1;
  transition: color 180ms ease;
}

.spotlight-search:focus-within input::placeholder {
  color: color-mix(in srgb, var(--text-muted) 86%, var(--primary));
}

.spotlight-search kbd,
.spotlight-result-count {
  flex: 0 0 auto;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  color: var(--text-muted);
  font-family: inherit;
  font-size: 10px;
}

.spotlight-search kbd {
  padding: 5px 8px;
  border-bottom-color: var(--border-strong);
  border-radius: 7px;
  box-shadow: 0 2px 0 color-mix(in srgb, var(--border) 80%, transparent);
}

.spotlight-result-count {
  padding: 5px 9px;
  border-radius: var(--radius-full);
  font-variant-numeric: tabular-nums;
}

.spotlight-results-panel {
  max-height: 458px;
  overflow: hidden;
  transform-origin: 50% 0;
}

.spotlight-results-shell {
  position: relative;
  height: 458px;
  overflow: hidden;
}

.spotlight-results {
  height: 100%;
  overflow-y: auto;
  padding: 8px 9px 20px;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--border-strong) 78%, transparent) transparent;
}

.spotlight-result {
  position: relative;
  display: grid;
  grid-template-columns: 54px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 72px;
  padding: 8px 11px;
  overflow: hidden;
  border: 1px solid transparent;
  border-radius: 14px;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: default;
  opacity: 0;
  animation: result-rise 230ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  transition:
    background-color 145ms ease,
    border-color 145ms ease,
    transform 180ms cubic-bezier(0.16, 1, 0.3, 1),
    box-shadow 180ms ease;
}

.spotlight-result:hover,
.spotlight-result--selected {
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 68%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.045);
  transform: translateX(2px) scale(0.998);
}

.spotlight-thumb {
  width: 54px;
  height: 54px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-hover);
  box-shadow: 0 5px 14px rgb(2 6 23 / 0.1);
  transition: transform 220ms cubic-bezier(0.16, 1, 0.3, 1);
}

.spotlight-result:hover .spotlight-thumb,
.spotlight-result--selected .spotlight-thumb {
  transform: scale(1.035) rotate(-0.3deg);
}

.spotlight-thumb :deep(img) { width: 100%; height: 100%; object-fit: cover; }
.spotlight-copy { min-width: 0; }
.spotlight-copy strong,
.spotlight-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-copy strong { color: var(--text); font-size: 13px; letter-spacing: -0.12px; }
.spotlight-copy small { margin-top: 5px; color: var(--text-muted); font-size: 10px; }

.spotlight-score {
  padding: 4px 7px;
  border-radius: var(--radius-full);
  background: var(--primary-soft);
  color: var(--primary-text);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  transition: opacity 120ms ease, transform 160ms ease;
}

.spotlight-result:hover .spotlight-score,
.spotlight-result--selected .spotlight-score {
  opacity: 0;
  transform: translateX(7px);
}

.spotlight-actions {
  position: absolute;
  right: 10px;
  top: 50%;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 5px;
  padding-left: 42px;
  opacity: 0;
  transform: translate(10px, -50%) scale(0.97);
  pointer-events: none;
  background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--primary-soft) 92%, var(--surface)) 32%);
  transition: opacity 145ms ease, transform 185ms cubic-bezier(0.16, 1, 0.3, 1);
}

.spotlight-result:hover .spotlight-actions,
.spotlight-result--selected .spotlight-actions,
.spotlight-result:focus-within .spotlight-actions {
  opacity: 1;
  transform: translate(0, -50%) scale(1);
  pointer-events: auto;
}

.spotlight-actions :deep(.ui-button) {
  min-height: 31px;
  padding-inline: 9px;
  border-radius: 9px;
  font-size: 10px;
  box-shadow: 0 5px 15px rgb(2 6 23 / 0.1);
  transition: transform 145ms ease, background-color 145ms ease, border-color 145ms ease, color 145ms ease;
}

.spotlight-actions :deep(.ui-button:hover) { transform: translateY(-1px); }

.spotlight-loading-list {
  display: grid;
  gap: 8px;
  padding: 4px;
}

.spotlight-loading-list span {
  height: 70px;
  border-radius: 14px;
  background: linear-gradient(100deg, var(--surface-hover) 25%, color-mix(in srgb, var(--primary-soft) 44%, var(--surface)) 44%, var(--surface-hover) 63%);
  background-size: 260% 100%;
  opacity: 0;
  animation: skeleton-in 220ms ease forwards, skeleton-shimmer 1.35s linear infinite;
}

.spotlight-empty {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 300px;
  padding: 32px;
  color: var(--text-muted);
  text-align: center;
}

.spotlight-empty svg { margin-bottom: 13px; color: var(--primary-text); }
.spotlight-empty strong { color: var(--text); font-size: 14px; }
.spotlight-empty span { margin-top: 7px; font-size: 11px; }
.spotlight-error { margin: 10px; padding: 11px 12px; border: 1px solid var(--danger-border); border-radius: 11px; background: var(--danger-surface); color: var(--danger-text); font-size: 11px; }

.spotlight-scroll-shadow {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 4;
  height: 58px;
  pointer-events: none;
  background: linear-gradient(180deg, transparent, color-mix(in srgb, var(--surface-elevated) 97%, transparent) 88%);
  box-shadow: inset 0 -14px 18px -18px rgb(2 6 23 / 0.32);
}

.results-morph-enter-active,
.results-morph-leave-active {
  transition:
    max-height 260ms cubic-bezier(0.16, 1, 0.3, 1),
    opacity 170ms ease,
    transform 250ms cubic-bezier(0.16, 1, 0.3, 1),
    clip-path 250ms cubic-bezier(0.16, 1, 0.3, 1),
    filter 170ms ease;
}

.results-morph-enter-from,
.results-morph-leave-to {
  max-height: 0;
  opacity: 0;
  transform: translateY(-7px) scaleY(0.965);
  clip-path: inset(0 2.5% 100% 2.5% round 0 0 20px 20px);
  filter: blur(4px);
}

.results-morph-enter-to,
.results-morph-leave-from {
  max-height: 458px;
  opacity: 1;
  transform: none;
  clip-path: inset(0 round 0 0 20px 20px);
  filter: none;
}

.spin { animation: spin 0.85s linear infinite; }

@keyframes spotlight-pop {
  0% { opacity: 0; transform: translateY(-10px) scale(0.95); filter: blur(7px); }
  70% { opacity: 1; transform: translateY(1px) scale(1.004); filter: blur(0); }
  100% { opacity: 1; transform: none; filter: none; }
}

@keyframes result-rise {
  from { opacity: 0; transform: translateY(6px) scale(0.993); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

@keyframes skeleton-in { to { opacity: 1; } }
@keyframes skeleton-shimmer { to { background-position: -160% 0; } }
@keyframes spin { to { transform: rotate(1turn); } }

@media (prefers-reduced-motion: reduce) {
  .spotlight-stage--visible,
  .spotlight-result,
  .spotlight-loading-list span { animation-duration: 0.01ms; }
  .results-morph-enter-active,
  .results-morph-leave-active { transition-duration: 0.01ms; }
}
</style>

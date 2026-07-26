<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import type { ImageAsset, IndexProgress, RuntimeStats } from '../../types'
import { imagyxApi } from '../../api/tauri'
import { semanticRuntime } from '../../services/semantic'
import { usePlatformStore } from '../../stores/platform'
import { useShortcutStore } from '../../stores/shortcut'
import { useThemeStore } from '../../stores/theme'
import { debounce } from '../../utils'
import { capitalize, useTagTypewriter } from '../../useTagTypewriter'
import MovingBorder from '../ui/MovingBorder/MovingBorder.vue'
import SpotlightInput from './SpotlightInput.vue'
import SpotlightResults from './SpotlightResults.vue'
import SpotlightSettings from './SpotlightSettings.vue'
import type { SpotlightIndexJob, SpotlightView } from './types'

const platform = usePlatformStore()
const shortcut = useShortcutStore()
const theme = useThemeStore()
const currentWindow = getCurrentWindow()
const { typedTag } = useTagTypewriter()
const view = ref<SpotlightView>('search')
const searchQuery = ref('')
const settingsQuery = ref('')
const results = ref<ImageAsset[]>([])
const selectedIndex = ref(0)
const searching = ref(false)
const error = ref<string | null>(null)
const copiedImageId = ref<string | null>(null)
const visible = ref(false)
const resultsOpen = ref(false)
const shellMerged = ref(false)
const dialogOpen = ref(false)
const jobs = ref<SpotlightIndexJob[]>([])
const inputView = ref<InstanceType<typeof SpotlightInput> | null>(null)
const resultsView = ref<InstanceType<typeof SpotlightResults> | null>(null)
const resultCache = new Map<string, ImageAsset[]>()

let searchSequence = 0
let morphSequence = 0
let expanded = false
let expansionPromise: Promise<void> | null = null
let copyTimer: number | undefined
let typewriterTimer: number | undefined
let collapseTimer: number | undefined
let jobTimer: number | undefined
let tagIndex = 0
let characterIndex = typedTag.value.length
let deleting = false
let unlistenWillOpen: UnlistenFn | null = null
let unlistenOpened: UnlistenFn | null = null
let unlistenWillHide: UnlistenFn | null = null
let unlistenFocus: UnlistenFn | null = null
let unlistenIndex: UnlistenFn | null = null
let unlistenRuntime: UnlistenFn | null = null

const activeQuery = computed({
  get: () => view.value === 'settings' ? settingsQuery.value : searchQuery.value,
  set: (value: string) => {
    if (view.value === 'settings') settingsQuery.value = value
    else searchQuery.value = value
  },
})
const selectedImage = computed(() => results.value[selectedIndex.value] ?? null)
const resultLabel = computed(() => searching.value && results.value.length === 0
  ? 'Recherche…'
  : `${results.value.length} résultat${results.value.length === 1 ? '' : 's'}`)
const placeholder = computed(() => view.value === 'settings'
  ? 'Search settings…'
  : typedTag.value ? `All images: ${capitalize(typedTag.value)}…` : 'All images: name or description…')
const showAddAction = computed(() => {
  const query = searchQuery.value.trim().toLocaleLowerCase('fr')
  if (!query) return false
  return query.startsWith('add') || query.startsWith('ajout') || query.startsWith('folder') || query.startsWith('dossier')
})

const searchLater = debounce(() => { void runSearch() }, 65)

watch(searchQuery, (value) => {
  selectedIndex.value = 0
  error.value = null
  const request = ++morphSequence
  if (!value.trim()) {
    searchSequence += 1
    results.value = []
    searching.value = false
    if (view.value === 'search') void closePanel(request)
    return
  }
  searchLater()
  void openPanel(request)
})

async function openPanel(request = ++morphSequence) {
  if (collapseTimer) window.clearTimeout(collapseTimer)
  try { await ensureExpanded() }
  catch (reason) { if (request === morphSequence) error.value = String(reason); return }
  if (request !== morphSequence) return
  shellMerged.value = true
  await nextPaint()
  if (request !== morphSequence) return
  resultsOpen.value = true
}

async function closePanel(request = ++morphSequence) {
  resultsOpen.value = false
  await nextPaint()
  if (request !== morphSequence || view.value === 'settings' || searchQuery.value.trim()) return
  shellMerged.value = false
  if (collapseTimer) window.clearTimeout(collapseTimer)
  collapseTimer = window.setTimeout(() => {
    if (request !== morphSequence || view.value === 'settings' || searchQuery.value.trim()) return
    void setCompact()
  }, 220)
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
  try { await imagyxApi.setSpotlightExpanded(false); expanded = false }
  catch { /* le prochain lancement recalcule la fenêtre */ }
}

async function openSettings() {
  view.value = 'settings'
  settingsQuery.value = ''
  const request = ++morphSequence
  await openPanel(request)
  await nextTick()
  inputView.value?.focus()
}

async function backToSearch() {
  view.value = 'search'
  settingsQuery.value = ''
  await nextTick()
  inputView.value?.focus()
  if (!searchQuery.value.trim()) await closePanel(++morphSequence)
}

async function runSearch() {
  const sequence = ++searchSequence
  const text = searchQuery.value.trim()
  const cacheKey = text.toLocaleLowerCase('fr')
  if (!text) return
  const cached = resultCache.get(cacheKey)
  if (cached) { results.value = cached; searching.value = false; return }

  searching.value = true
  const lexicalPromise = imagyxApi.search({ query: text, limit: 60 })
  const embeddingPromise = text.length >= 2 ? semanticRuntime.embedQuery(text) : Promise.resolve(undefined)
  void lexicalPromise.then((images) => {
    if (sequence === searchSequence && searchQuery.value.trim() === text && (images.length || !results.value.length)) results.value = images
  }).catch(() => undefined)

  try {
    const embedded = await embeddingPromise
    if (sequence !== searchSequence || searchQuery.value.trim() !== text) return
    const images = embedded?.queryVector
      ? await imagyxApi.search({ query: text, queryVector: embedded.queryVector, limit: 60 })
      : await lexicalPromise
    if (sequence !== searchSequence || searchQuery.value.trim() !== text) return
    results.value = images
    rememberResults(cacheKey, images)
  } catch (reason) {
    if (sequence === searchSequence) {
      error.value = String(reason)
      try { results.value = await lexicalPromise } catch { /* erreur principale conservée */ }
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

async function addFolder() {
  dialogOpen.value = true
  try {
    const selected = await open({ directory: true, multiple: false, title: 'Choisir un dossier à indexer' })
    if (typeof selected !== 'string') return
    const folder = await imagyxApi.addFolder(selected)
    upsertJob({ folderId: folder.id, folderName: folder.name, current: 0, total: 0, stage: 'discovering', message: 'Analyse du dossier…' })
    void imagyxApi.indexFolder(folder.id).catch((reason) => {
      upsertJob({ folderId: folder.id, folderName: folder.name, current: 0, total: 0, stage: 'error', message: String(reason) })
    })
  } catch (reason) {
    error.value = String(reason)
  } finally {
    dialogOpen.value = false
    void currentWindow.setFocus()
  }
}

function handleIndexProgress(progress: IndexProgress) {
  const stage = progress.stage === 'complete' ? 'complete'
    : progress.stage === 'queued' ? 'queued'
      : progress.stage === 'error' ? 'error'
        : progress.stage === 'metadata' ? 'metadata' : 'discovering'
  upsertJob({ folderId: progress.folderId, folderName: progress.folderName, current: progress.current, total: progress.total, stage, message: progress.message })
  if (stage === 'complete') scheduleJobCleanup(progress.folderId)
}

function handleRuntimeStats(stats: RuntimeStats) {
  const job = [...jobs.value].reverse().find((item) => item.stage === 'queued' || item.stage === 'embedding')
  if (!job) return
  if (['indexing', 'decoding', 'inference', 'saving'].includes(stats.stage)) {
    upsertJob({ ...job, stage: 'embedding', current: stats.current, total: stats.total, message: `Analyse IA · ${stats.current} sur ${stats.total}` })
  } else if (stats.stage === 'ready' && job.stage === 'embedding') {
    upsertJob({ ...job, stage: 'complete', current: job.total, message: 'Indexation terminée' })
    scheduleJobCleanup(job.folderId)
  }
}

function upsertJob(job: SpotlightIndexJob) {
  const index = jobs.value.findIndex((item) => item.folderId === job.folderId)
  if (index < 0) jobs.value = [job, ...jobs.value]
  else jobs.value[index] = job
}

function scheduleJobCleanup(folderId: string) {
  if (jobTimer) window.clearTimeout(jobTimer)
  jobTimer = window.setTimeout(() => { jobs.value = jobs.value.filter((job) => job.folderId !== folderId) }, 4200)
}

async function copyImage(image: ImageAsset) {
  try {
    await imagyxApi.copyImage(image.path)
    copiedImageId.value = image.id
    if (copyTimer) window.clearTimeout(copyTimer)
    copyTimer = window.setTimeout(() => { copiedImageId.value = null }, 1800)
  } catch (reason) { error.value = String(reason) }
}
async function revealImage(image: ImageAsset) {
  try { await imagyxApi.openInFileManager(image.path, true) } catch (reason) { error.value = String(reason) }
}
async function openImage(image: ImageAsset) {
  try { await imagyxApi.openInImagyx(image.id) } catch (reason) { error.value = String(reason) }
}

function moveSelection(delta: number) {
  if (!results.value.length) return
  selectedIndex.value = (selectedIndex.value + delta + results.value.length) % results.value.length
  resultsView.value?.scrollToIndex(selectedIndex.value)
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    if (view.value === 'settings') void backToSearch()
    else void imagyxApi.hideSpotlight()
    return
  }
  if (view.value !== 'search') return
  if (event.key === 'ArrowDown') { event.preventDefault(); moveSelection(1); return }
  if (event.key === 'ArrowUp') { event.preventDefault(); moveSelection(-1); return }
  if (event.key === 'Enter' && selectedImage.value) { event.preventDefault(); void openImage(selectedImage.value) }
}

function prepareOpen() {
  visible.value = false
  view.value = 'search'
  searchQuery.value = ''
  settingsQuery.value = ''
  results.value = []
  selectedIndex.value = 0
  error.value = null
  resultsOpen.value = false
  shellMerged.value = false
  expanded = false
  expansionPromise = null
}
function animateOpen() {
  prepareOpen()
  void nextPaint(2).then(() => { visible.value = true; inputView.value?.focus() })
}
function prepareHide() {
  visible.value = false
  searchQuery.value = ''
  settingsQuery.value = ''
  results.value = []
  resultsOpen.value = false
  shellMerged.value = false
  expanded = false
  expansionPromise = null
}

function nextPaint(count = 1): Promise<void> {
  return new Promise((resolve) => {
    const step = (remaining: number) => window.requestAnimationFrame(() => remaining <= 1 ? resolve() : step(remaining - 1))
    step(count)
  })
}

onMounted(async () => {
  theme.initialize()
  void platform.initialize()
  void shortcut.initialize()
  window.addEventListener('keydown', handleKeydown)
  unlistenWillOpen = await listen('spotlight-will-open', prepareOpen)
  unlistenOpened = await listen('spotlight-opened', animateOpen)
  unlistenWillHide = await listen('spotlight-will-hide', prepareHide)
  unlistenIndex = await listen<IndexProgress>('index-progress', (event) => handleIndexProgress(event.payload))
  unlistenRuntime = await listen<RuntimeStats>('runtime-stats', (event) => handleRuntimeStats(event.payload))
  unlistenFocus = await currentWindow.onFocusChanged(({ payload }) => { if (!payload && !dialogOpen.value) void imagyxApi.hideSpotlight() })
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
  unlistenWillOpen?.(); unlistenOpened?.(); unlistenWillHide?.(); unlistenFocus?.(); unlistenIndex?.(); unlistenRuntime?.()
  if (copyTimer) window.clearTimeout(copyTimer)
  if (collapseTimer) window.clearTimeout(collapseTimer)
  if (jobTimer) window.clearTimeout(jobTimer)
})
</script>

<template>
  <main class="spotlight-root">
    <section class="spotlight-stage" :class="{ 'spotlight-stage--visible': visible }" aria-label="Recherche rapide Imagyx">
      <MovingBorder class="spotlight-border" border-radius="22px" :duration="searching ? 2600 : 4400" :active="visible">
        <div class="spotlight-surface" :class="{ 'spotlight-surface--expanded': shellMerged }">
          <SpotlightInput
            ref="inputView"
            v-model="activeQuery"
            :view="view"
            :placeholder="placeholder"
            :searching="searching"
            :result-label="resultLabel"
            @settings="openSettings"
            @back="backToSearch"
          />

          <Transition name="panel-morph">
            <div v-if="resultsOpen" class="spotlight-panel">
              <Transition name="view-swap" mode="out-in">
                <SpotlightSettings
                  v-if="view === 'settings'"
                  key="settings"
                  variant="spotlight"
                  :query="settingsQuery"
                  :shortcut="shortcut.spotlight"
                  :shortcut-updating="shortcut.updating"
                  :shortcut-error="shortcut.error"
                  :theme-mode="theme.mode"
                  @shortcut-change="shortcut.setSpotlight"
                  @theme-change="theme.setMode"
                />
                <SpotlightResults
                  v-else
                  ref="resultsView"
                  key="results"
                  :results="results"
                  :selected-index="selectedIndex"
                  :searching="searching"
                  :error="error"
                  :copied-image-id="copiedImageId"
                  :show-add-action="showAddAction"
                  :jobs="jobs"
                  :file-manager-name="platform.fileManagerName"
                  @select="selectedIndex = $event"
                  @open="openImage"
                  @copy="copyImage"
                  @reveal="revealImage"
                  @add-folder="addFolder"
                />
              </Transition>
            </div>
          </Transition>
        </div>
      </MovingBorder>
    </section>
  </main>
</template>

<style scoped>
.spotlight-root { width: 100%; height: 100%; overflow: hidden; padding: 16px 14px 24px; background: transparent; }
.spotlight-stage { width: 100%; opacity: 0; transform: translateY(-8px) scale(0.965); filter: blur(7px); pointer-events: none; transform-origin: 50% 18px; }
.spotlight-stage--visible { opacity: 1; transform: none; filter: none; pointer-events: auto; animation: spotlight-pop 300ms cubic-bezier(0.16, 1, 0.3, 1) both; }
.spotlight-border { width: 100%; }
.spotlight-surface {
  width: 100%;
  overflow: hidden;
  border-radius: 21px;
  background: color-mix(in srgb, var(--surface-elevated) 95%, transparent);
  box-shadow: inset 0 1px rgb(255 255 255 / 0.1), 0 8px 24px -20px rgb(15 23 42 / 0.32);
  backdrop-filter: blur(28px) saturate(1.18);
  transition: box-shadow 240ms ease, background-color 180ms ease;
}
.spotlight-surface--expanded { box-shadow: inset 0 1px rgb(255 255 255 / 0.1), 0 18px 38px -28px rgb(15 23 42 / 0.42); }
.spotlight-panel { height: 472px; min-height: 0; overflow: hidden; border-top: 1px solid color-mix(in srgb, var(--border) 76%, transparent); }
.panel-morph-enter-active,
.panel-morph-leave-active { transition: max-height 260ms cubic-bezier(0.16, 1, 0.3, 1), opacity 180ms ease, clip-path 260ms cubic-bezier(0.16, 1, 0.3, 1); overflow: hidden; }
.panel-morph-enter-from,
.panel-morph-leave-to { max-height: 0; opacity: 0; clip-path: inset(0 0 100% 0 round 0 0 21px 21px); }
.panel-morph-enter-to,
.panel-morph-leave-from { max-height: 472px; opacity: 1; clip-path: inset(0 round 0 0 21px 21px); }
.view-swap-enter-active,
.view-swap-leave-active { transition: opacity 120ms ease, transform 170ms cubic-bezier(0.16, 1, 0.3, 1), filter 120ms ease; }
.view-swap-enter-from { opacity: 0; transform: translateX(9px); filter: blur(3px); }
.view-swap-leave-to { opacity: 0; transform: translateX(-7px); filter: blur(3px); }
:global(:root[data-theme='dark']) .spotlight-surface { box-shadow: inset 0 1px rgb(255 255 255 / 0.055), 0 9px 26px -20px rgb(0 0 0 / 0.58); }
:global(:root[data-theme='dark']) .spotlight-surface--expanded { box-shadow: inset 0 1px rgb(255 255 255 / 0.055), 0 20px 42px -28px rgb(0 0 0 / 0.72); }
@keyframes spotlight-pop { 0% { opacity: 0; transform: translateY(-12px) scale(0.94); filter: blur(8px); } 68% { opacity: 1; transform: translateY(1px) scale(1.006); filter: blur(0); } 100% { opacity: 1; transform: none; filter: none; } }
@media (prefers-reduced-motion: reduce) { .spotlight-stage--visible { animation-duration: 0.01ms; } .panel-morph-enter-active, .panel-morph-leave-active, .view-swap-enter-active, .view-swap-leave-active { transition-duration: 0.01ms; } }
</style>

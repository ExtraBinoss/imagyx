<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import type { FolderIndexCoverage, FollowedFolder, ImageAsset, IndexProgress, ModelDownloadProgress, RuntimeStats } from '../../types'
import { imagyxApi } from '../../api/tauri'
import { semanticRuntime } from '../../services/semantic'
import { usePlatformStore } from '../../stores/platform'
import { useShortcutStore } from '../../stores/shortcut'
import { useThemeStore } from '../../stores/theme'
import { debounce, perfLog } from '../../utils'
import { folderQuerySuggestions, formatFolderQuery, parseFolderQuery } from '../../utils/folder-query'
import { capitalize, useTagTypewriter } from '../../useTagTypewriter'
import MovingBorder from '../ui/MovingBorder/MovingBorder.vue'
import SpotlightInput from './SpotlightInput.vue'
import SpotlightResults from './SpotlightResults.vue'
import ImagePreviewDialog from '../ImagePreviewDialog.vue'
import SpotlightSettings from './SpotlightSettings.vue'
import type { SpotlightIndexJob, SpotlightView } from './types'
import { useSpotlightResultActions } from './useSpotlightResultActions'
import { useTranslate } from '../../i18n'
import { spotlightSearchTiming } from '../../config/spotlight-search'

const CACHE_TTL_MS = 2_000
const SEARCH_DEBOUNCE_MS = spotlightSearchTiming.lexicalDebounceMs
const SEMANTIC_DEBOUNCE_MS = spotlightSearchTiming.semanticDebounceMs

interface CachedResults {
  images: ImageAsset[]
  storedAt: number
}

const platform = usePlatformStore()
const shortcut = useShortcutStore()
const theme = useThemeStore()
const currentWindow = getCurrentWindow()
const { typedTag } = useTagTypewriter()
const { t } = useTranslate()
const view = ref<SpotlightView>('search')
const searchQuery = ref('')
const settingsQuery = ref('')
const results = ref<ImageAsset[]>([])
const folders = ref<FollowedFolder[]>([])
const indexCoverage = ref<FolderIndexCoverage[]>([])
const libraryReady = ref(false)
const selectedIndex = ref(0)
const folderSuggestionIndex = ref(0)
const searching = ref(false)
const semanticSearching = ref(false)
const error = ref<string | null>(null)
const visible = ref(false)
const resultsOpen = ref(false)
const shellMerged = ref(false)
const dialogOpen = ref(false)
const previewImage = ref<ImageAsset | null>(null)
const resultsScrolling = ref(false)
const jobs = ref<SpotlightIndexJob[]>([])
const modelProgress = ref<ModelDownloadProgress | null>(null)
const runtimeStats = ref<RuntimeStats | null>(null)
const inputView = ref<InstanceType<typeof SpotlightInput> | null>(null)
const resultsView = ref<InstanceType<typeof SpotlightResults> | null>(null)
const resultCache = new Map<string, CachedResults>()
const {
  copiedImageId,
  copyingImageId,
  revealingImageId,
  openingImageId,
  copyImage,
  revealImage,
  openImage,
  resetActionFeedback,
} = useSpotlightResultActions((reason) => { error.value = String(reason) })

let searchSequence = 0
let hybridFinalSequence = 0
let searchDiagnosticSequence = 0
let pendingSearchDiagnosticId: string | undefined
let pendingSearchUntilLibraryReady: string | null = null
let lastSearchInputAt = 0
let morphSequence = 0
let expanded = false
let expansionPromise: Promise<void> | null = null
let collapseTimer: number | undefined
let jobTimer: number | undefined
const paintFrames = new Set<number>()
let unlistenWillOpen: UnlistenFn | null = null
let unlistenOpened: UnlistenFn | null = null
let unlistenWillHide: UnlistenFn | null = null
let unlistenFocus: UnlistenFn | null = null
let unlistenIndex: UnlistenFn | null = null
let unlistenRuntime: UnlistenFn | null = null
let unlistenLibrary: UnlistenFn | null = null
let unlistenModel: UnlistenFn | null = null
let unlistenVectors: UnlistenFn | null = null
let previewEnterPressed = false
let spotlightOpenStartedAt = 0

const activeQuery = computed({
  get: () => view.value === 'settings' ? settingsQuery.value : searchQuery.value,
  set: (value: string) => {
    if (view.value === 'settings') settingsQuery.value = value
    else searchQuery.value = value
  },
})
const hasFolders = computed(() => folders.value.length > 0)
const hasActiveJobs = computed(() => jobs.value.some((job) => !['complete', 'error'].includes(job.stage)))
const parsedFolderQuery = computed(() => parseFolderQuery(searchQuery.value, folders.value))
const folderSuggestions = computed(() => folderQuerySuggestions(searchQuery.value, folders.value))
const hasSearchQuery = computed(() => Boolean(
  parsedFolderQuery.value.folder || parsedFolderQuery.value.query.length,
))
const incompleteCoverage = computed(() => indexCoverage.value.filter((coverage) => {
  const job = jobs.value.find((item) => item.folderId === coverage.folderId)
  return coverage.embeddedCount < coverage.imageCount
    && !['discovering', 'metadata', 'queued', 'embedding'].includes(job?.stage ?? '')
}))
const selectedImage = computed(() => results.value[selectedIndex.value] ?? null)
const resultLabel = computed(() => {
  if (!libraryReady.value) return t('spotlight.placeholder.loading')
  if (!hasFolders.value) return t('spotlight.setup_required')
  if (searching.value && results.value.length === 0) return t('spotlight.searching')
  if (semanticSearching.value && results.value.length === 0) return t('spotlight.semantic_searching')
  return t('spotlight.result_count', { count: results.value.length })
})
const placeholder = computed(() => {
  if (view.value === 'settings') return t('spotlight.placeholder.settings')
  if (!libraryReady.value) return t('spotlight.placeholder.loading')
  if (!hasFolders.value) return t('spotlight.placeholder.no_folder')
  return typedTag.value
    ? t('all_images_prefix') + `${capitalize(typedTag.value)}…`
    : t('spotlight.placeholder.search')
})
const showAddAction = computed(() => {
  if (!libraryReady.value || !hasFolders.value) return true
  const query = searchQuery.value.trim().toLocaleLowerCase('en')
  if (!query) return false
  return query.startsWith('add') || query.startsWith('folder') || query.startsWith('directory')
})

const searchLater = debounce(() => { void runSearch() }, SEARCH_DEBOUNCE_MS)

function waitForDelay(delayMs: number): Promise<void> {
  if (delayMs <= 0) return Promise.resolve()
  return new Promise((resolve) => window.setTimeout(resolve, delayMs))
}

function nextSearchDiagnosticId(): string | undefined {
  if (!import.meta.env.DEV) return undefined
  searchDiagnosticSequence += 1
  return `spotlight-${Date.now().toString(36)}-${searchDiagnosticSequence}`
}

function deferResultSetLog(callback: () => void): void {
  if (!import.meta.env.DEV) return
  window.requestAnimationFrame(() => window.setTimeout(callback, 0))
}

function logResultSet(
  diagnosticId: string | undefined,
  stage: string,
  images: ImageAsset[],
  durationMs: number,
  details: Record<string, unknown> = {},
): void {
  if (!import.meta.env.DEV || !diagnosticId) return
  deferResultSetLog(() => {
    console.groupCollapsed(
      `[Imagyx][SpotlightSearch][${diagnosticId}] ${stage} · ${images.length} résultat(s) · ${durationMs.toFixed(1)} ms`,
    )
    console.log('Timing', { diagnosticId, stage, durationMs: Number(durationMs.toFixed(2)), ...details })
    console.table(images.map((image, index) => ({
      rank: index + 1,
      id: image.id,
      name: image.name,
      format: image.extension.toLocaleUpperCase(),
      semanticScore: image.semanticScore == null
        ? null
        : Number(image.semanticScore.toFixed(4)),
      relevanceScore: image.relevanceScore == null
        ? null
        : Number(image.relevanceScore.toFixed(4)),
    })))
    console.groupEnd()
  })
}

watch(searchQuery, (value) => {
  folderSuggestionIndex.value = 0
  selectedIndex.value = 0
  error.value = null
  searchSequence += 1
  semanticSearching.value = false
  const request = ++morphSequence
  if (!hasFolders.value) {
    pendingSearchDiagnosticId = undefined
    pendingSearchUntilLibraryReady = value.trim() ? value : null
    results.value = []
    searching.value = false
    semanticSearching.value = false
    if (view.value === 'search') void openPanel(request)
    return
  }
  if (!value.trim()) {
    pendingSearchDiagnosticId = undefined
    pendingSearchUntilLibraryReady = null
    searching.value = true
    semanticSearching.value = false
    void runSearch()
    if (view.value === 'search') void openPanel(request)
    return
  }

  lastSearchInputAt = performance.now()
  pendingSearchDiagnosticId = nextSearchDiagnosticId()
  searching.value = true
  if (import.meta.env.DEV && pendingSearchDiagnosticId) {
    const parsed = parseFolderQuery(value, folders.value)
    console.info(`[Imagyx][SpotlightSearch][${pendingSearchDiagnosticId}] scheduled`, {
      rawQuery: value,
      query: parsed.query,
      folderId: parsed.folder?.id ?? null,
      lexicalDebounceMs: SEARCH_DEBOUNCE_MS,
      semanticDebounceMs: SEMANTIC_DEBOUNCE_MS,
      cacheTtlMs: CACHE_TTL_MS,
    })
  }
  searchLater()
  void openPanel(request)
})

function selectFolderSuggestion(folder: FollowedFolder) {
  searchQuery.value = formatFolderQuery(folder)
  folderSuggestionIndex.value = 0
  // Folder browsing has no query to debounce: show its indexed images as soon
  // as the selection is committed, just like the main Imagyx search.
  void nextTick(() => { void runSearch() })
}

function navigateFolderSuggestions(delta: number) {
  const count = folderSuggestions.value.length
  if (!count) return
  folderSuggestionIndex.value = (folderSuggestionIndex.value + delta + count) % count
}

watch(hasActiveJobs, (active) => {
  if (view.value !== 'search') return
  if (active) void openPanel(++morphSequence)
  else if (!searchQuery.value.trim() && hasFolders.value) void closePanel(++morphSequence)
})

async function syncFolders(openWhenEmpty = false) {
  const startedAt = import.meta.env.DEV ? performance.now() : 0
  try {
    const [folderList, coverage] = await Promise.all([
      imagyxApi.folders(),
      imagyxApi.indexCoverage(),
    ])
    folders.value = folderList
    indexCoverage.value = coverage
    libraryReady.value = true
    if (hasFolders.value && pendingSearchUntilLibraryReady === searchQuery.value) {
      pendingSearchUntilLibraryReady = null
      void runSearch()
    }
    if (openWhenEmpty && (!hasFolders.value || hasActiveJobs.value)) {
      await openPanel(++morphSequence)
    }
    if (import.meta.env.DEV) {
      perfLog('Spotlight', 'folder state IPC', performance.now() - startedAt, {
        folders: folderList.length,
        openWhenEmpty,
      })
    }
  } catch (reason) {
    libraryReady.value = true
    error.value = String(reason)
    if (openWhenEmpty) await openPanel(++morphSequence)
    if (import.meta.env.DEV) perfLog('Spotlight', 'folder state IPC failed', performance.now() - startedAt, { openWhenEmpty })
  }
}

async function openPanel(request = ++morphSequence) {
  if (collapseTimer) window.clearTimeout(collapseTimer)
  shellMerged.value = true
  resultsOpen.value = true
  try { await ensureExpanded() }
  catch (reason) { if (request === morphSequence) error.value = String(reason) }
}

async function closePanel(request = ++morphSequence) {
  if (!hasFolders.value || hasActiveJobs.value) return
  resultsOpen.value = false
  await nextPaint()
  if (request !== morphSequence || view.value === 'settings' || searchQuery.value.trim()) return
  shellMerged.value = false
  if (collapseTimer) window.clearTimeout(collapseTimer)
  collapseTimer = window.setTimeout(() => {
    if (request !== morphSequence || view.value === 'settings' || searchQuery.value.trim() || !hasFolders.value || hasActiveJobs.value) return
    void setCompact()
  }, 220)
}

async function ensureExpanded() {
  if (expanded) return
  if (!expansionPromise) {
    const startedAt = import.meta.env.DEV ? performance.now() : 0
    expansionPromise = imagyxApi.setSpotlightExpanded(true)
      .then(() => {
        expanded = true
        if (import.meta.env.DEV) perfLog('Spotlight', 'native window expand', performance.now() - startedAt)
      })
      .finally(() => { expansionPromise = null })
  }
  await expansionPromise
}

async function setCompact() {
  if (!expanded) return
  try { await imagyxApi.setSpotlightExpanded(false); expanded = false }
  catch { /* the next launch recalculates the window */ }
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
  if (!searchQuery.value.trim() && hasFolders.value && !hasActiveJobs.value) await closePanel(++morphSequence)
  else await openPanel(++morphSequence)
}
async function runSearch() {
  const sequence = ++searchSequence
  const diagnosticId = pendingSearchDiagnosticId ?? nextSearchDiagnosticId()
  pendingSearchDiagnosticId = undefined
  const runStartedAt = performance.now()
  const startedAt = import.meta.env.DEV ? runStartedAt : 0
  const inputStartedAt = lastSearchInputAt
  const inputToRunMs = inputStartedAt > 0 ? runStartedAt - inputStartedAt : 0
  const parsed = parsedFolderQuery.value
  const text = parsed.query
  const folderId = parsed.folder?.id
  const cacheKey = `${folderId ?? 'all'}:${text.toLocaleLowerCase('en')}`
  const trigger = inputToRunMs + 2 < SEARCH_DEBOUNCE_MS ? 'immediate' : 'debounced'

  if (import.meta.env.DEV) {
    perfLog('Spotlight', 'input to search start', inputToRunMs, {
      diagnosticId,
      trigger,
      query: text,
      folderId: folderId ?? null,
      configuredDebounceMs: SEARCH_DEBOUNCE_MS,
      configuredSemanticDebounceMs: SEMANTIC_DEBOUNCE_MS,
    })
    console.info(`[Imagyx][SpotlightSearch][${diagnosticId ?? '-'}] run`, {
      sequence,
      trigger,
      rawQuery: searchQuery.value,
      query: text,
      folderId: folderId ?? null,
      inputToRunMs: Number(inputToRunMs.toFixed(2)),
      configuredDebounceMs: SEARCH_DEBOUNCE_MS,
      configuredSemanticDebounceMs: SEMANTIC_DEBOUNCE_MS,
    })
  }

  if (!hasFolders.value) {
    results.value = []
    searching.value = false
    return
  }

  const cached = resultCache.get(cacheKey)
  const cacheAgeMs = cached ? performance.now() - cached.storedAt : null
  if (cached && cacheAgeMs != null && cacheAgeMs <= CACHE_TTL_MS) {
    results.value = cached.images
    searching.value = false
    semanticSearching.value = false
    if (import.meta.env.DEV) {
      const cacheDurationMs = performance.now() - startedAt
      perfLog('Spotlight', 'result cache hit', cacheDurationMs, {
        diagnosticId,
        query: text,
        folderId: folderId ?? null,
        cacheAgeMs,
        results: cached.images.length,
      })
      logResultSet(diagnosticId, 'cache hit', cached.images, inputToRunMs + cacheDurationMs, {
        query: text,
        folderId: folderId ?? null,
        cacheAgeMs: Number(cacheAgeMs.toFixed(2)),
        inputToRunMs: Number(inputToRunMs.toFixed(2)),
      })
    }
    return
  }
  if (cached) {
    resultCache.delete(cacheKey)
    if (import.meta.env.DEV) {
      console.info(`[Imagyx][SpotlightSearch][${diagnosticId ?? '-'}] cache expired`, {
        query: text,
        folderId: folderId ?? null,
        cacheAgeMs,
        cacheTtlMs: CACHE_TTL_MS,
      })
    }
  }

  searching.value = true
  const lexicalStartedAt = import.meta.env.DEV ? performance.now() : 0
  const lexicalPromise = imagyxApi.search({
    query: text,
    folderId,
    limit: 60,
    diagnosticId: import.meta.env.DEV && diagnosticId
      ? `${diagnosticId}-${text ? 'lexical' : 'browse'}`
      : undefined,
  })
  if (!text) {
    try {
      const images = await lexicalPromise
      if (!matchesActiveSearch(sequence, text, folderId, diagnosticId)) return
      results.value = images
      rememberResults(cacheKey, images)
      if (import.meta.env.DEV) {
        const browseMs = performance.now() - lexicalStartedAt
        perfLog('Spotlight', 'folder browse IPC', browseMs, {
          diagnosticId,
          query: text,
          folderId: folderId ?? null,
          results: images.length,
        })
        logResultSet(diagnosticId, 'folder browse complete', images, inputToRunMs + browseMs, {
          query: text,
          folderId: folderId ?? null,
          inputToRunMs: Number(inputToRunMs.toFixed(2)),
          rustIpcMs: Number(browseMs.toFixed(2)),
        })
      }
    } catch (reason) {
      if (sequence === searchSequence) error.value = String(reason)
    } finally {
      if (sequence === searchSequence) searching.value = false
    }
    return
  }

  void lexicalPromise.then((images) => {
    const lexicalMs = import.meta.env.DEV ? performance.now() - lexicalStartedAt : 0
    if (import.meta.env.DEV) {
      perfLog('Spotlight', 'lexical IPC', lexicalMs, {
        diagnosticId,
        query: text,
        folderId: folderId ?? null,
        results: images.length,
      })
    }
    if (!matchesActiveSearch(sequence, text, folderId, diagnosticId) || hybridFinalSequence === sequence) return
    results.value = images
    searching.value = false
    void nextPaint().then(() => {
      if (!import.meta.env.DEV) return
      const inputToPaintMs = inputStartedAt > 0
        ? performance.now() - inputStartedAt
        : performance.now() - startedAt
      perfLog('Spotlight', 'input to lexical paint', inputToPaintMs, {
        diagnosticId,
        query: text,
        folderId: folderId ?? null,
        configuredDebounceMs: SEARCH_DEBOUNCE_MS,
        lexicalIpcMs: lexicalMs,
        results: images.length,
      })
      logResultSet(diagnosticId, 'lexical first paint', images, inputToPaintMs, {
        query: text,
        folderId: folderId ?? null,
        configuredDebounceMs: SEARCH_DEBOUNCE_MS,
        lexicalIpcMs: Number(lexicalMs.toFixed(2)),
      })
    })
  }).catch((reason) => {
    if (matchesActiveSearch(sequence, text, folderId, diagnosticId)) searching.value = false
    if (import.meta.env.DEV) {
      console.error(`[Imagyx][SpotlightSearch][${diagnosticId ?? '-'}] lexical request failed`, {
        query: text,
        folderId: folderId ?? null,
        reason,
      })
    }
  })

  try {
    const shouldEmbed = text.length >= 2
    if (!shouldEmbed) return
    semanticSearching.value = true
    let semanticWaitMs = 0
    if (shouldEmbed) {
      const semanticWaitStartedAt = performance.now()
      const elapsedSinceInputMs = inputStartedAt > 0
        ? semanticWaitStartedAt - inputStartedAt
        : inputToRunMs
      const remainingSemanticDelayMs = Math.max(0, SEMANTIC_DEBOUNCE_MS - elapsedSinceInputMs)
      if (remainingSemanticDelayMs > 0) await waitForDelay(remainingSemanticDelayMs)
      semanticWaitMs = performance.now() - semanticWaitStartedAt
      if (import.meta.env.DEV) {
        perfLog('Spotlight', 'semantic debounce wait', semanticWaitMs, {
          diagnosticId,
          query: text,
          folderId: folderId ?? null,
          configuredSemanticDebounceMs: SEMANTIC_DEBOUNCE_MS,
          elapsedSinceInputMs,
          remainingSemanticDelayMs,
        })
      }
    }
    if (!matchesActiveSearch(sequence, text, folderId, diagnosticId)) return

    const embeddingStartedAt = import.meta.env.DEV ? performance.now() : 0
    const embedded = shouldEmbed ? await semanticRuntime.embedQuery(text) : undefined
    const embeddingMs = import.meta.env.DEV ? performance.now() - embeddingStartedAt : 0
    if (import.meta.env.DEV) {
      perfLog('Spotlight', 'semantic embedding', embeddingMs, {
        diagnosticId,
        query: text,
        folderId: folderId ?? null,
        vectorReady: Boolean(embedded?.queryVector),
        vectorDimensions: embedded?.queryVector.length ?? 0,
      })
    }
    if (!matchesActiveSearch(sequence, text, folderId, diagnosticId)) return
    const hybridStartedAt = import.meta.env.DEV ? performance.now() : 0
    const images = embedded?.queryVector
      ? await imagyxApi.search({
          query: text,
          folderId,
          queryVector: embedded.queryVector,
          limit: 60,
          diagnosticId: import.meta.env.DEV && diagnosticId
            ? `${diagnosticId}-hybrid`
            : undefined,
        })
      : await lexicalPromise
    const hybridMs = import.meta.env.DEV ? performance.now() - hybridStartedAt : 0
    if (!matchesActiveSearch(sequence, text, folderId, diagnosticId)) return
    hybridFinalSequence = sequence
    results.value = images
    searching.value = false
    rememberResults(cacheKey, images)
    if (import.meta.env.DEV) {
      const totalMs = performance.now() - startedAt
      perfLog('Spotlight', 'semantic search total', totalMs, {
        diagnosticId,
        query: text,
        folderId: folderId ?? null,
        inputToRunMs,
        semanticWaitMs,
        embeddingMs,
        hybridRustIpcMs: hybridMs,
        results: images.length,
      })
      logResultSet(diagnosticId, 'hybrid final', images, inputToRunMs + totalMs, {
        query: text,
        folderId: folderId ?? null,
        configuredDebounceMs: SEARCH_DEBOUNCE_MS,
        configuredSemanticDebounceMs: SEMANTIC_DEBOUNCE_MS,
        inputToRunMs: Number(inputToRunMs.toFixed(2)),
        semanticWaitMs: Number(semanticWaitMs.toFixed(2)),
        embeddingMs: Number(embeddingMs.toFixed(2)),
        hybridRustIpcMs: Number(hybridMs.toFixed(2)),
        runSearchMs: Number(totalMs.toFixed(2)),
      })
    }
  } catch (reason) {
    if (sequence === searchSequence) {
      error.value = String(reason)
      if (hybridFinalSequence !== sequence) {
        try { results.value = await lexicalPromise } catch { /* the primary error remains visible */ }
      }
    }
    if (import.meta.env.DEV) {
      console.error(`[Imagyx][SpotlightSearch][${diagnosticId ?? '-'}] semantic search failed`, {
        query: text,
        folderId: folderId ?? null,
        elapsedMs: Number((performance.now() - startedAt).toFixed(2)),
        reason,
      })
    }
  } finally {
    if (sequence === searchSequence) semanticSearching.value = false
  }
}

function matchesActiveSearch(
  sequence: number,
  query: string,
  folderId?: string,
  diagnosticId?: string,
): boolean {
  const active = parseFolderQuery(searchQuery.value, folders.value)
  const matches = sequence === searchSequence
    && active.query === query
    && active.folder?.id === folderId
  if (import.meta.env.DEV && !matches) {
    console.info(`[Imagyx][SpotlightSearch][${diagnosticId ?? '-'}] stale result ignored`, {
      requestSequence: sequence,
      activeSequence: searchSequence,
      requestQuery: query,
      activeQuery: active.query,
      requestFolderId: folderId ?? null,
      activeFolderId: active.folder?.id ?? null,
    })
  }
  return matches
}

function rememberResults(key: string, images: ImageAsset[]) {
  resultCache.set(key, { images, storedAt: performance.now() })
  if (resultCache.size <= 24) return
  const oldest = resultCache.keys().next().value as string | undefined
  if (oldest) resultCache.delete(oldest)
}

async function addFolder() {
  dialogOpen.value = true
  try {
    const selected = await open({ directory: true, multiple: false, title: t('spotlight.choose_folder_title') })
    if (typeof selected !== 'string') return
    const folder = await imagyxApi.addFolder(selected)
    folders.value = [folder, ...folders.value.filter((item) => item.id !== folder.id)]
    upsertJob({
      folderId: folder.id,
      folderName: folder.name,
      current: 0,
      total: 0,
      stage: 'discovering',
      message: t('spotlight.index_message.scanning'),
    })
    await openPanel(++morphSequence)
    void imagyxApi.indexFolder(folder.id, false).catch((reason) => {
      upsertJob({ folderId: folder.id, folderName: folder.name, current: 0, total: 0, stage: 'error', message: String(reason) })
    })
  } catch (reason) {
    error.value = String(reason)
  } finally {
    dialogOpen.value = false
    void currentWindow.setFocus()
  }
}

async function resumeIncompleteFolders(folderIds: string[]) {
  resultCache.clear()
  for (const folderId of folderIds) {
    const folder = folders.value.find((item) => item.id === folderId)
    if (!folder) continue
    upsertJob({
      folderId: folder.id,
      folderName: folder.name,

      current: 0,
      total: folder.imageCount,
      stage: 'queued',
      message: t('spotlight.index_message.queued', { total: folder.imageCount }),
    })
    try {
      await imagyxApi.indexFolder(folderId, false)
    } catch (reason) {
      upsertJob({
        folderId: folder.id,
        folderName: folder.name,
        current: 0,
        total: folder.imageCount,
        stage: 'error',
        message: String(reason),
      })
    }
  }
  void syncFolders()
}

function indexProgressMessage(progress: IndexProgress, stage: SpotlightIndexJob['stage']): string {
  if (stage === 'complete') return t('spotlight.index_complete')
  if (stage === 'error') return progress.message
  if (stage === 'queued') return t('spotlight.index_message.queued', { total: progress.total })
  if (stage === 'embedding') return t('spotlight.index_message.embedding', { current: progress.current, total: progress.total })
  if (stage === 'metadata') return t('spotlight.index_message.metadata', { current: progress.current, total: progress.total })
  return t('spotlight.index_message.scanning')
}

function handleIndexProgress(progress: IndexProgress) {
  const stage: SpotlightIndexJob['stage'] = progress.stage === 'complete' ? 'complete'
    : progress.stage === 'queued' ? 'queued'
      : progress.stage === 'error' ? 'error'
        : progress.stage === 'embedding' ? 'embedding'
          : progress.stage === 'metadata' ? 'metadata' : 'discovering'
  upsertJob({
    folderId: progress.folderId,
    folderName: progress.folderName,
    current: progress.current,
    total: progress.total,
    stage,
    message: indexProgressMessage(progress, stage),
  })
  if (stage === 'complete') {
    resultCache.clear()
    void syncFolders()
    scheduleJobCleanup(progress.folderId)
  }
}

function handleRuntimeStats(stats: RuntimeStats) {
  runtimeStats.value = stats
  const job = [...jobs.value].reverse().find((item) => item.stage === 'queued' || item.stage === 'embedding')
  if (!job) return
  if (['indexing', 'decoding', 'inference', 'saving'].includes(stats.stage)) {
    upsertJob({ ...job, stage: 'embedding', current: stats.current, total: stats.total, message: t('spotlight.index_message.embedding', { current: stats.current, total: stats.total }) })
  } else if (stats.stage === 'paused') {
    upsertJob({ ...job, stage: 'queued', current: stats.current, total: stats.total, message: t('spotlight.index_message.paused', { current: stats.current, total: stats.total }) })
  } else if (stats.stage === 'ready' && job.stage === 'embedding') {
    upsertJob({ ...job, stage: 'complete', current: job.total, message: t('spotlight.index_complete') })
    resultCache.clear()
    void syncFolders()
    scheduleJobCleanup(job.folderId)
  }
}

function handleModelProgress(progress: ModelDownloadProgress) {
  const previousStage = modelProgress.value?.stage
  modelProgress.value = progress
  if (import.meta.env.DEV && previousStage !== progress.stage) {
    console.info('[Imagyx][SpotlightSearch] model progress', {
      stage: progress.stage,
      file: progress.fileName ?? null,
      currentBytes: progress.currentBytes,
      totalBytes: progress.totalBytes,
      message: progress.message,
    })
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

function moveSelection(delta: number) {
  if (!results.value.length) return
  selectedIndex.value = (selectedIndex.value + delta + results.value.length) % results.value.length
  resultsView.value?.scrollToIndex(selectedIndex.value)
}

function isTextEditingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return target.matches('input, textarea, select, [contenteditable="true"]')
    || Boolean(target.closest('[contenteditable="true"]'))
}

function isShortcutRecordingTarget(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('[data-shortcut-recorder]'))
}

function consumePreviewEnter(event: KeyboardEvent): boolean {
  if (event.key !== 'Enter') return false
  if (previewEnterPressed || event.repeat) {
    event.preventDefault()
    event.stopImmediatePropagation()
    return true
  }
  previewEnterPressed = true
  return false
}

function handleKeydown(event: KeyboardEvent) {
  // ShortcutView owns the complete key chord, including Escape. It must not
  // compete with Spotlight navigation or the global close handler.
  if (isShortcutRecordingTarget(event.target)) return

  const editingText = isTextEditingTarget(event.target)
  if (previewImage.value) {
    if (event.key === 'Enter') {
      if (consumePreviewEnter(event)) return
      event.preventDefault()
      event.stopImmediatePropagation()
      previewImage.value = null
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopImmediatePropagation()
      previewImage.value = null
    }
    return
  }

  if (event.key === 'Escape') {
    event.preventDefault()
    if (view.value === 'settings') void backToSearch()
    else void imagyxApi.hideSpotlight()
    return
  }

  // Text always belongs to the field. Result navigation remains available
  // from it, but folder-query suggestions keep their own arrow/Enter handling.
  if (editingText) {
    if (view.value !== 'search' || folderSuggestions.value.length) return
    if (event.key === 'ArrowDown') { event.preventDefault(); moveSelection(1); return }
    if (event.key === 'ArrowUp') { event.preventDefault(); moveSelection(-1); return }
    if (event.key === 'Enter' && selectedImage.value) {
      if (consumePreviewEnter(event)) return
      event.preventDefault()
      previewImage.value = selectedImage.value
    }
    return
  }

  if (view.value === 'search' && (event.ctrlKey || event.metaKey) && selectedImage.value) {
    const key = event.key.toLocaleLowerCase()
    if (key === 'c' || event.code === 'KeyC') {
      event.preventDefault(); event.stopPropagation(); void copyImage(selectedImage.value); return
    }
    if (key === 'e' || event.code === 'KeyE') {
      event.preventDefault(); event.stopPropagation(); void revealImage(selectedImage.value); return
    }
    if (key === 'i' || event.code === 'KeyI') {
      event.preventDefault(); event.stopPropagation(); void openImage(selectedImage.value); return
    }
  }

  if (view.value !== 'search') return
  if (folderSuggestions.value.length && ['ArrowDown', 'ArrowUp', 'Enter', 'Tab'].includes(event.key)) return
  if (event.key === 'ArrowDown') { event.preventDefault(); moveSelection(1); return }
  if (event.key === 'ArrowUp') { event.preventDefault(); moveSelection(-1); return }
  if (event.key === 'Enter' && selectedImage.value) {
    if (consumePreviewEnter(event)) return
    event.preventDefault()
    previewImage.value = selectedImage.value
  }
}

function handleKeyup(event: KeyboardEvent) {
  if (event.key === 'Enter') previewEnterPressed = false
}

function prepareOpen() {
  spotlightOpenStartedAt = performance.now()
  resultsScrolling.value = false
  previewEnterPressed = false
  visible.value = false
  view.value = 'search'
  searchQuery.value = ''
  settingsQuery.value = ''
  results.value = []
  selectedIndex.value = 0
  searching.value = false
  semanticSearching.value = false
  error.value = null
  resultsOpen.value = false
  shellMerged.value = false
  expanded = false
  expansionPromise = null
  pendingSearchDiagnosticId = undefined
  pendingSearchUntilLibraryReady = null
  lastSearchInputAt = 0
  resetActionFeedback()
  void syncFolders(true).finally(() => {
    if (!import.meta.env.DEV) return
    perfLog('Spotlight', 'open folder state ready', performance.now() - spotlightOpenStartedAt, {
      hasFolders: hasFolders.value,
    })
  })
}

function animateOpen() {
  void nextPaint(2).then(() => {
    visible.value = true
    inputView.value?.focus()
    if (import.meta.env.DEV) {
      perfLog('Spotlight', 'open first paint', performance.now() - spotlightOpenStartedAt, {
        resultsOpen: resultsOpen.value,
        hasFolders: hasFolders.value,
      })
    }
  })
}

function prepareHide() {
  resultsScrolling.value = false
  previewEnterPressed = false
  previewImage.value = null
  visible.value = false
  searchQuery.value = ''
  settingsQuery.value = ''
  results.value = []
  searching.value = false
  semanticSearching.value = false
  resultsOpen.value = false
  shellMerged.value = false
  expanded = false
  expansionPromise = null
  pendingSearchDiagnosticId = undefined
  lastSearchInputAt = 0
  resetActionFeedback()
}

function nextPaint(count = 1): Promise<void> {
  return new Promise((resolve) => {
    const step = (remaining: number) => {
      const frame = window.requestAnimationFrame(() => {
        paintFrames.delete(frame)
        if (remaining <= 1) resolve()
        else step(remaining - 1)
      })
      paintFrames.add(frame)
    }
    step(count)
  })
}

onMounted(async () => {
  theme.initialize()
  void platform.initialize()
  void shortcut.initialize()
  void syncFolders()
  void imagyxApi.appInfo().then((info) => {
    handleModelProgress(info.modelProgress)
    handleRuntimeStats(info.runtimeStats)
  }).catch(() => undefined)
  if (import.meta.env.DEV) {
    console.info('[Imagyx][SpotlightSearch] development diagnostics enabled', {
      lexicalDebounceMs: SEARCH_DEBOUNCE_MS,
      semanticDebounceMs: SEMANTIC_DEBOUNCE_MS,
      cacheTtlMs: CACHE_TTL_MS,
      targetFirstResultMs: 100,
      targetFinalResultMs: 200,
      note: 'Le lexical part rapidement; le sémantique attend une courte période de calme et vérifie la requête active avant l’embedding.',
    })
  }
  window.addEventListener('keydown', handleKeydown, { capture: true })
  window.addEventListener('keyup', handleKeyup, { capture: true })
  const unlisteners = await Promise.all([
    listen('spotlight-will-open', prepareOpen),
    listen('spotlight-opened', animateOpen),
    listen('spotlight-will-hide', prepareHide),
    listen<IndexProgress>('index-progress', (event) => handleIndexProgress(event.payload)),
    listen<RuntimeStats>('runtime-stats', (event) => handleRuntimeStats(event.payload)),
    listen<ModelDownloadProgress>('model-download-progress', (event) => handleModelProgress(event.payload)),
    listen('library-updated', () => { resultCache.clear(); void syncFolders() }),
    listen('vectors-ready', () => {
      resultCache.clear()
      if (searchQuery.value.trim()) void runSearch()
    }),
  ]);
  [
    unlistenWillOpen,
    unlistenOpened,
    unlistenWillHide,
    unlistenIndex,
    unlistenRuntime,
    unlistenModel,
    unlistenLibrary,
    unlistenVectors,
  ] = unlisteners
  unlistenFocus = await currentWindow.onFocusChanged(({ payload }) => {
    if (!payload && !dialogOpen.value) void imagyxApi.hideSpotlight()
  })
  void imagyxApi.spotlightFrontendReady()
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown, { capture: true })
  window.removeEventListener('keyup', handleKeyup, { capture: true })
  unlistenWillOpen?.()
  unlistenOpened?.()
  unlistenWillHide?.()
  unlistenFocus?.()
  unlistenIndex?.()
  unlistenRuntime?.()
  unlistenModel?.()
  unlistenLibrary?.()
  unlistenVectors?.()
  if (collapseTimer) window.clearTimeout(collapseTimer)
  if (jobTimer) window.clearTimeout(jobTimer)
  for (const frame of paintFrames) window.cancelAnimationFrame(frame)
  paintFrames.clear()
})
</script>

<template>
  <main class="spotlight-root" :class="{ 'spotlight-root--scrolling': resultsScrolling }">
    <section
      class="spotlight-stage"
      :class="{ 'spotlight-stage--visible': visible }"
      :aria-label="t('spotlight.aria')"
    >
      <MovingBorder
        class="spotlight-border"
        border-radius="22px"
        size="md"
        :duration="searching || hasActiveJobs ? 2600 : 4400"
        :active="visible"
      >
        <div
          class="spotlight-surface"
          :class="{ 'spotlight-surface--expanded': shellMerged }"
        >
          <SpotlightInput
            ref="inputView"
            v-model="activeQuery"
            :view="view"
            :placeholder="placeholder"
            :searching="searching || semanticSearching"
            :result-label="resultLabel"
            :folder-suggestions="folderSuggestions"
            :folder-suggestion-index="folderSuggestionIndex"
            @settings="openSettings"
            @back="backToSearch"
            @folder-select="selectFolderSuggestion"
            @folder-navigate="navigateFolderSuggestions"
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
                  @scroll-state="resultsScrolling = $event"
                />
                <SpotlightResults
                  v-else
                  ref="resultsView"
                  key="results"
                  :results="results"
                  :selected-index="selectedIndex"
                  :searching="searching"
                  :semantic-searching="semanticSearching"
                  :has-search-query="hasSearchQuery"
                  :error="error"
                  :copied-image-id="copiedImageId"
                  :copying-image-id="copyingImageId"
                  :revealing-image-id="revealingImageId"
                  :opening-image-id="openingImageId"
                  :show-add-action="showAddAction"
                  :has-folders="hasFolders"
                  :library-ready="libraryReady"
                  :show-background-hint="hasActiveJobs"
                  :incomplete-coverage="incompleteCoverage"
                  :jobs="jobs"
                  :file-manager-name="platform.fileManagerName"
                  :performance-mode="resultsScrolling"
                  :model-progress="modelProgress"
                  :runtime-stats="runtimeStats"
                  @select="selectedIndex = $event"
                  @scroll-state="resultsScrolling = $event"
                  @open="openImage"
                  @copy="copyImage"
                  @reveal="revealImage"
                  @add-folder="addFolder"
                  @resume="resumeIncompleteFolders"
                />
              </Transition>
            </div>
          </Transition>
        </div>
      </MovingBorder>
    </section>
    <ImagePreviewDialog
      :image="previewImage"
      :keyboard-shortcuts="false"
      @close="previewImage = null"
    />
  </main>
</template>

<style scoped>
.spotlight-root {
  --spotlight-panel-height: 462px;
  width: 100%;
  height: 100%;
  overflow: hidden;
  padding: 16px 14px 24px;
  background: transparent;
}
.spotlight-stage {
  width: 100%;
  opacity: 0;
  transform: translateY(-8px) scale(0.965);
  filter: blur(7px);
  pointer-events: none;
  transform-origin: 50% 18px;
}
.spotlight-stage--visible {
  opacity: 1;
  transform: none;
  filter: none;
  pointer-events: auto;
  animation: spotlight-pop 300ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
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
.spotlight-surface--expanded {
  box-shadow: inset 0 1px rgb(255 255 255 / 0.1), 0 18px 38px -28px rgb(15 23 42 / 0.42);
}
.spotlight-root--scrolling .spotlight-surface {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
.spotlight-root--scrolling .spotlight-surface { transition: none; }
.spotlight-panel {
  height: var(--spotlight-panel-height);
  min-height: 0;
  overflow: hidden;
  border-top: 1px solid color-mix(in srgb, var(--border) 76%, transparent);
}
.panel-morph-enter-active,
.panel-morph-leave-active {
  transition: max-height 260ms cubic-bezier(0.16, 1, 0.3, 1), opacity 180ms ease, clip-path 260ms cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
}
.panel-morph-enter-from,
.panel-morph-leave-to {
  max-height: 0;
  opacity: 0;
  clip-path: inset(0 0 100% 0 round 0 0 21px 21px);
}
.panel-morph-enter-to,
.panel-morph-leave-from {
  max-height: var(--spotlight-panel-height);
  opacity: 1;
  clip-path: inset(0 round 0 0 21px 21px);
}
.view-swap-enter-active,
.view-swap-leave-active {
  transition: opacity 120ms ease, transform 170ms cubic-bezier(0.16, 1, 0.3, 1), filter 120ms ease;
}
.view-swap-enter-from { opacity: 0; transform: translateX(9px); filter: blur(3px); }
.view-swap-leave-to { opacity: 0; transform: translateX(-7px); filter: blur(3px); }
:global(:root[data-theme="dark"]) .spotlight-surface {
  box-shadow: inset 0 1px rgb(255 255 255 / 0.055), 0 9px 26px -20px rgb(0 0 0 / 0.58);
}
:global(:root[data-theme="dark"]) .spotlight-surface--expanded {
  box-shadow: inset 0 1px rgb(255 255 255 / 0.055), 0 20px 42px -28px rgb(0 0 0 / 0.72);
}
@keyframes spotlight-pop {
  0% { opacity: 0; transform: translateY(-12px) scale(0.94); filter: blur(8px); }
  68% { opacity: 1; transform: translateY(1px) scale(1.006); filter: blur(0); }
  100% { opacity: 1; transform: none; filter: none; }
}
@media (prefers-reduced-motion: reduce) {
  .spotlight-stage--visible { animation-duration: 0.01ms; }
  .panel-morph-enter-active,
  .panel-morph-leave-active,
  .view-swap-enter-active,
  .view-swap-leave-active { transition-duration: 0.01ms; }
}
</style>

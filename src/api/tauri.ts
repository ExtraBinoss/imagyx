import { reactive } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type {
  AppInfo,
  FolderIndexCoverage,
  FollowedFolder,
  ImageAsset,
  ImageConversionFormat,
  ImageConversionResult,
  ImageEmbedding,
  ImageExplanation,
  ModelDownloadProgress,
  QueryConcept,
  RuntimeStats,
  SearchPage,
  SearchRequest,
} from '../types'
import { perfLog } from '../utils'
import { nativeDialogIsOpen } from '../services/native-dialog-state'
import { spotlightSearchPagination } from '../services/spotlight-search-pagination'
import { visualSearchSession } from '../services/visual-search-session'

type SearchMode = 'browse' | 'lexical' | 'hybrid' | 'visual'

let searchDiagnosticSequence = 0

function searchMode(request: SearchRequest): SearchMode {
  if (request.mode === 'visual') return 'visual'
  if (!request.query.trim()) return 'browse'
  return request.queryVector?.length ? 'hybrid' : 'lexical'
}

function nextSearchDiagnosticId(mode: SearchMode): string | undefined {
  if (!import.meta.env.DEV) return undefined
  searchDiagnosticSequence += 1
  const surface = typeof document === 'undefined'
    ? 'unknown'
    : document.documentElement.dataset.window ?? 'unknown'
  return `${surface}-${mode}-${Date.now().toString(36)}-${searchDiagnosticSequence}`
}

function deferSearchResultLog(callback: () => void): void {
  if (!import.meta.env.DEV) return
  window.requestAnimationFrame(() => {
    window.setTimeout(callback, 0)
  })
}

function logSearchResults(
  diagnosticId: string,
  mode: SearchMode,
  request: SearchRequest,
  page: SearchPage,
  durationMs: number,
): void {
  if (!import.meta.env.DEV) return
  console.groupCollapsed(
    `[Imagyx][Search][${diagnosticId}] ${mode} · ${page.items.length}/${page.total} résultat(s) · ${durationMs.toFixed(1)} ms`,
  )
  console.log('Request', {
    diagnosticId,
    mode,
    query: request.query,
    folderId: request.folderId ?? null,
    limit: request.limit ?? 2_000,
    offset: request.offset ?? 0,
    queryVectorDimensions: request.queryVector?.length ?? 0,
    excludeImageId: request.excludeImageId ?? null,
    returned: page.items.length,
    total: page.total,
  })
  console.table(page.items.map((image, index) => ({
    rank: (request.offset ?? 0) + index + 1,
    id: image.id,
    name: image.name,
    format: image.extension.toLocaleUpperCase(),
    semanticScore: image.semanticScore == null
      ? null
      : Number(image.semanticScore.toFixed(4)),
    folderId: image.folderId,
  })))
  console.groupEnd()
}

async function invokeSearchPage(
  request: SearchRequest,
  trackAsInitialSearch: boolean,
): Promise<SearchPage> {
  const sessionRequest = visualSearchSession.requestForActiveSession(request)
  const mode = searchMode(sessionRequest)
  const diagnosticId = import.meta.env.DEV
    ? sessionRequest.diagnosticId ?? nextSearchDiagnosticId(mode)
    : undefined
  const normalizedRequest: SearchRequest = {
    ...sessionRequest,
    limit: sessionRequest.limit ?? 2_000,
    offset: sessionRequest.offset ?? 0,
    diagnosticId,
  }
  // Spotlight keeps the synthetic visual token as its cache/session identity,
  // while carrying the resolved vector and native mode for deeper pages.
  const paginationRequest: SearchRequest = {
    ...request,
    limit: normalizedRequest.limit,
    offset: normalizedRequest.offset,
    diagnosticId,
    queryVector: normalizedRequest.queryVector,
    mode: normalizedRequest.mode,
    excludeImageId: normalizedRequest.excludeImageId,
  }
  const paginationContext = trackAsInitialSearch
    ? spotlightSearchPagination.beginInitialSearch(paginationRequest)
    : null
  const startedAt = import.meta.env.DEV ? performance.now() : 0

  if (import.meta.env.DEV && diagnosticId) {
    console.info(`[Imagyx][Search][${diagnosticId}] start`, {
      mode,
      query: normalizedRequest.query,
      sessionQuery: request.query,
      folderId: normalizedRequest.folderId ?? null,
      limit: normalizedRequest.limit,
      offset: normalizedRequest.offset,
      queryVectorDimensions: normalizedRequest.queryVector?.length ?? 0,
      excludeImageId: normalizedRequest.excludeImageId ?? null,
    })
  }

  try {
    const page = await invoke<SearchPage>('search_image_page', {
      request: {
        query: normalizedRequest.query,
        folderId: normalizedRequest.folderId ?? null,
        limit: normalizedRequest.limit,
        offset: normalizedRequest.offset,
        queryVector: normalizedRequest.queryVector ?? null,
        mode: normalizedRequest.mode ?? null,
        excludeImageId: normalizedRequest.excludeImageId ?? null,
      },
      diagnosticId: import.meta.env.DEV ? diagnosticId ?? null : null,
    })
    const items = paginationContext
      ? reactive(page.items) as ImageAsset[]
      : page.items
    const normalizedPage = { items, total: page.total }

    if (paginationContext) {
      spotlightSearchPagination.completeInitialSearch(
        paginationContext,
        paginationRequest,
        normalizedPage,
        items,
      )
    }

    if (import.meta.env.DEV) {
      const durationMs = performance.now() - startedAt
      perfLog('SearchIPC', `${mode} Rust round trip`, durationMs, {
        diagnosticId,
        query: normalizedRequest.query,
        sessionQuery: request.query,
        offset: normalizedRequest.offset,
        results: items.length,
        total: normalizedPage.total,
        queryVectorDimensions: normalizedRequest.queryVector?.length ?? 0,
      })
      if (diagnosticId) {
        deferSearchResultLog(() => logSearchResults(
          diagnosticId,
          mode,
          normalizedRequest,
          normalizedPage,
          durationMs,
        ))
      }
    }
    return normalizedPage
  } catch (error) {
    if (import.meta.env.DEV) {
      const durationMs = performance.now() - startedAt
      console.error(`[Imagyx][Search][${diagnosticId ?? 'unknown'}] failed after ${durationMs.toFixed(1)} ms`, {
        mode,
        query: normalizedRequest.query,
        sessionQuery: request.query,
        offset: normalizedRequest.offset,
        error,
      })
    }
    throw error
  }
}

async function searchImages(request: SearchRequest): Promise<ImageAsset[]> {
  return (await invokeSearchPage(request, true)).items
}

async function searchImagePage(request: SearchRequest): Promise<SearchPage> {
  return invokeSearchPage(request, false)
}

export const imagyxApi = {
  appInfo: () => invoke<AppInfo>('get_app_info'),
  platform: () => invoke<string>('get_platform'),
  runtimeStats: () => invoke<RuntimeStats>('get_runtime_stats'),
  updateRuntimeStats: (stats: RuntimeStats) => invoke<void>('update_runtime_stats', { stats }),
  updateModelProgress: (progress: ModelDownloadProgress) =>
    invoke<void>('update_model_progress', { progress }),
  setTrayPaused: (paused: boolean) => invoke<void>('set_tray_paused', { paused }),
  spotlightShortcut: () => invoke<string>('get_spotlight_shortcut'),
  setSpotlightShortcut: (shortcut: string) =>
    invoke<string>('set_spotlight_shortcut', { shortcut }),
  launchOnStartup: () => invoke<boolean>('get_launch_on_startup'),
  setLaunchOnStartup: (enabled: boolean) =>
    invoke<boolean>('set_launch_on_startup', { enabled }),
  semanticProviderReady: () => invoke<boolean>('semantic_provider_ready'),
  setSemanticProviderReady: (ready: boolean) => invoke<void>('set_semantic_provider_ready', { ready }),
  prepareLocalModel: (modelKey: string) => invoke<string>('prepare_local_model', { modelKey }),
  resetEmbeddings: () => invoke<void>('reset_embeddings'),
  folders: () => invoke<FollowedFolder[]>('list_folders'),
  indexCoverage: () => invoke<FolderIndexCoverage[]>('get_index_coverage'),
  addFolder: (path: string) => invoke<FollowedFolder>('add_folder', { path }),
  removeFolder: (folderId: string) => invoke<void>('remove_folder', { folderId }),
  indexFolder: (folderId: string, force = true) =>
    invoke<void>('index_folder', { folderId, force }),
  pendingImages: (folderId?: string) =>
    invoke<ImageAsset[]>('pending_images', { folderId: folderId ?? null }),
  prepareAiImages: (imageIds: string[], batchId: string) =>
    invoke<ArrayBuffer>('prepare_ai_images', { imageIds, batchId }),
  prepareVisualQueryImage: (path: string) =>
    invoke<ArrayBuffer>('prepare_visual_query_image', { path }),
  imageEmbedding: (imageId: string) =>
    invoke<number[]>('get_image_embedding', { imageId }),
  saveEmbeddings: (embeddings: ImageEmbedding[]) =>
    invoke<void>('save_embeddings', { embeddings }),
  explainResults: (imageIds: string[], concepts: QueryConcept[]) =>
    invoke<ImageExplanation[]>('explain_results', { imageIds, concepts }),
  topImageTags: (concepts: QueryConcept[], limit = 10) =>
    invoke<string[]>('top_image_tags', { concepts, limit }),
  thumbnail: (image: Pick<ImageAsset, 'id' | 'path' | 'modifiedAt'>) =>
    invoke<string>('get_thumbnail', {
      imageId: image.id,
      path: image.path,
      modifiedAt: image.modifiedAt,
    }),
  search: searchImages,
  searchPage: searchImagePage,
  openInFileManager: (path: string, reveal = false) =>
    invoke<void>('open_in_file_manager', { path, reveal }),
  copyImage: (path: string) => invoke<void>('copy_image_to_clipboard', { path }),
  convertImage: (imageId: string, targetFormat: ImageConversionFormat) =>
    invoke<ImageConversionResult>('convert_image', { imageId, targetFormat }),
  openInImagyx: (imageId: string) => invoke<void>('open_in_imagyx', { imageId }),
  openOnboarding: () => invoke<void>('open_onboarding'),
  setSpotlightExpanded: (expanded: boolean) =>
    invoke<void>('set_spotlight_expanded', { expanded }),
  spotlightFrontendReady: () => invoke<void>('spotlight_frontend_ready'),
  hideSpotlight: () => (
    nativeDialogIsOpen()
      ? Promise.resolve()
      : invoke<void>('hide_spotlight')
  ),
  fileUrl: (path: string) => convertFileSrc(path),
}

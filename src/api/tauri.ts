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
import { spotlightSearchPagination } from '../services/spotlight-search-pagination'

type SearchMode = 'browse' | 'lexical' | 'hybrid'

let searchDiagnosticSequence = 0

function searchMode(request: SearchRequest): SearchMode {
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
  const mode = searchMode(request)
  const diagnosticId = import.meta.env.DEV
    ? request.diagnosticId ?? nextSearchDiagnosticId(mode)
    : undefined
  const normalizedRequest: SearchRequest = {
    ...request,
    limit: request.limit ?? 2_000,
    offset: request.offset ?? 0,
    diagnosticId,
  }
  const paginationContext = trackAsInitialSearch
    ? spotlightSearchPagination.beginInitialSearch(normalizedRequest)
    : null
  const startedAt = import.meta.env.DEV ? performance.now() : 0

  if (import.meta.env.DEV && diagnosticId) {
    console.info(`[Imagyx][Search][${diagnosticId}] start`, {
      mode,
      query: normalizedRequest.query,
      folderId: normalizedRequest.folderId ?? null,
      limit: normalizedRequest.limit,
      offset: normalizedRequest.offset,
      queryVectorDimensions: normalizedRequest.queryVector?.length ?? 0,
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
        normalizedRequest,
        normalizedPage,
        items,
      )
    }

    if (import.meta.env.DEV) {
      const durationMs = performance.now() - startedAt
      perfLog('SearchIPC', `${mode} Rust round trip`, durationMs, {
        diagnosticId,
        query: normalizedRequest.query,
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
  hideSpotlight: () => invoke<void>('hide_spotlight'),
  fileUrl: (path: string) => convertFileSrc(path),
}

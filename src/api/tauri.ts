import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type {
  AppInfo,
  FollowedFolder,
  ImageAsset,
  ImageEmbedding,
  ImageExplanation,
  ModelDownloadProgress,
  QueryConcept,
  RuntimeStats,
  SearchRequest,
} from '../types'

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
  search: (request: SearchRequest) =>
    invoke<ImageAsset[]>('search_images', {
      request: {
        query: request.query,
        folderId: request.folderId ?? null,
        limit: request.limit ?? 2_000,
        offset: request.offset ?? 0,
        queryVector: request.queryVector ?? null,
      },
    }),
  openInFileManager: (path: string, reveal = false) =>
    invoke<void>('open_in_file_manager', { path, reveal }),
  copyImage: (path: string) => invoke<void>('copy_image_to_clipboard', { path }),
  openInImagyx: (imageId: string) => invoke<void>('open_in_imagyx', { imageId }),
  openOnboarding: () => invoke<void>('open_onboarding'),
  setSpotlightExpanded: (expanded: boolean) =>
    invoke<void>('set_spotlight_expanded', { expanded }),
  hideSpotlight: () => invoke<void>('hide_spotlight'),
  fileUrl: (path: string) => convertFileSrc(path),
}

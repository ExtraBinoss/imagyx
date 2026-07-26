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
  runtimeStats: () => invoke<RuntimeStats>('get_runtime_stats'),
  updateRuntimeStats: (stats: RuntimeStats) => invoke<void>('update_runtime_stats', { stats }),
  updateModelProgress: (progress: ModelDownloadProgress) =>
    invoke<void>('update_model_progress', { progress }),
  prepareLocalModel: () => invoke<string>('prepare_local_model'),
  folders: () => invoke<FollowedFolder[]>('list_folders'),
  addFolder: (path: string) => invoke<FollowedFolder>('add_folder', { path }),
  removeFolder: (folderId: string) => invoke<void>('remove_folder', { folderId }),
  indexFolder: (folderId: string) => invoke<void>('index_folder', { folderId }),
  pendingImages: (folderId?: string) =>
    invoke<ImageAsset[]>('pending_images', { folderId: folderId ?? null }),
  prepareAiImages: (imageIds: string[]) => invoke<string[]>('prepare_ai_images', { imageIds }),
  saveEmbeddings: (embeddings: ImageEmbedding[]) =>
    invoke<void>('save_embeddings', { embeddings }),
  explainResults: (imageIds: string[], concepts: QueryConcept[]) =>
    invoke<ImageExplanation[]>('explain_results', { imageIds, concepts }),
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
        queryVector: request.queryVector ?? null,
      },
    }),
  fileUrl: (path: string) => convertFileSrc(path),
}

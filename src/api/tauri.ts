import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type {
  AppInfo,
  FollowedFolder,
  ImageAsset,
  ImageEmbedding,
  ModelDownloadProgress,
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
  saveEmbeddings: (embeddings: ImageEmbedding[]) =>
    invoke<void>('save_embeddings', { embeddings }),
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

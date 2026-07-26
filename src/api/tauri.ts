import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { AppInfo, FollowedFolder, ImageAsset, SearchRequest } from '../types'

export const imagyxApi = {
  appInfo: () => invoke<AppInfo>('get_app_info'),
  folders: () => invoke<FollowedFolder[]>('list_folders'),
  addFolder: (path: string) => invoke<FollowedFolder>('add_folder', { path }),
  removeFolder: (folderId: string) => invoke<void>('remove_folder', { folderId }),
  indexFolder: (folderId: string) => invoke<void>('index_folder', { folderId }),
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
      },
    }),
  fileUrl: (path: string) => convertFileSrc(path),
}

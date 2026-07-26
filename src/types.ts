export interface FollowedFolder {
  id: string
  name: string
  path: string
  imageCount: number
  createdAt: number
}

export interface ImageAsset {
  id: string
  folderId: string
  path: string
  name: string
  extension: string
  width: number
  height: number
  sizeBytes: number
  modifiedAt: number
  thumbnailPath: string
  semanticScore?: number
}

export interface AppInfo {
  rootDir: string
  modelsDir: string
  databasePath: string
  thumbnailsDir: string
  aiBackend: string
  aiReady: boolean
}

export interface IndexProgress {
  folderId: string
  folderName: string
  current: number
  total: number
  stage: 'discovering' | 'thumbnails' | 'embedding' | 'saving' | 'complete' | 'error'
  message: string
}

export interface SearchRequest {
  query: string
  folderId?: string
  limit?: number
}

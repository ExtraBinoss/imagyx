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

export type ModelDownloadStage = 'idle' | 'checking' | 'downloading' | 'loading' | 'ready' | 'error'

export interface ModelDownloadProgress {
  stage: ModelDownloadStage
  fileName?: string
  currentBytes: number
  totalBytes: number
  currentFile: number
  totalFiles: number
  message: string
}

export interface AppInfo {
  rootDir: string
  modelsDir: string
  databasePath: string
  thumbnailsDir: string
  aiBackend: string
  aiReady: boolean
  modelProgress: ModelDownloadProgress
}

export interface IndexProgress {
  folderId: string
  folderName: string
  current: number
  total: number
  batchCurrent?: number
  batchTotal?: number
  stage: 'discovering' | 'metadata' | 'embedding' | 'saving' | 'queued' | 'complete' | 'error'
  message: string
}

export interface SearchRequest {
  query: string
  folderId?: string
  limit?: number
}

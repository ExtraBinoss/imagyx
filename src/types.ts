export interface FollowedFolder {
  id: string
  name: string
  path: string
  imageCount: number
  createdAt: number
}

export interface FolderIndexCoverage {
  folderId: string
  imageCount: number
  embeddedCount: number
}

export interface SemanticMatch {
  label: string
  score: number
  source: 'semantic' | 'filename'
}

export interface ImageExplanation {
  imageId: string
  matches: SemanticMatch[]
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
  semanticMatches?: SemanticMatch[]
}

export interface ImageEmbedding {
  imageId: string
  vector: number[]
}

export interface QueryConcept {
  label: string
  vector: number[]
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

export interface ModelStatus {
  ready: boolean
  backend: string
  accelerationActive: boolean
  accelerationLabel: string
  fallbackReason?: string
}

export interface RuntimeStats {
  modelName: string
  stage: string
  backendRequested: string
  backendEffective: string
  accelerationActive: boolean
  accelerationLabel: string
  batchSize: number
  current: number
  total: number
  batchCurrent: number
  batchTotal: number
  imagesPerSecond: number
  averageMsPerImage: number
  decodeMs: number
  inferenceMs: number
  saveMs: number
  elapsedMs: number
  systemCpuPercent: number
  processCpuPercent: number
  memoryUsedBytes: number
  memoryTotalBytes: number
  processMemoryBytes: number
  modelCacheBytes: number
  thumbnailCacheItems: number
  fallbackReason?: string
  updatedAt: number
}

export interface AppInfo {
  rootDir: string
  modelsDir: string
  databasePath: string
  thumbnailsDir: string
  aiBackend: string
  aiReady: boolean
  modelProgress: ModelDownloadProgress
  runtimeStats: RuntimeStats
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
  offset?: number
  queryVector?: number[]
}

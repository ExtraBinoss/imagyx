import { defineStore } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppInfo,
  FollowedFolder,
  ImageAsset,
  IndexProgress,
  ModelDownloadProgress,
  ModelStatus,
  RuntimeStats,
} from '../types'
import { imagyxApi } from '../api/tauri'
import { semanticRuntime } from '../services/semantic'
import { formatBytes } from '../utils'
import { useToastStore } from './toasts'

interface LibraryState {
  folders: FollowedFolder[]
  images: ImageAsset[]
  selectedFolderId: string | null
  query: string
  loading: boolean
  initialized: boolean
  refreshScheduled: boolean
  appInfo: AppInfo | null
  progress: IndexProgress | null
  modelProgress: ModelDownloadProgress | null
  runtimeStats: RuntimeStats | null
  error: string | null
  listeners: UnlistenFn[]
}

export const useLibraryStore = defineStore('library', {
  state: (): LibraryState => ({
    folders: [],
    images: [],
    selectedFolderId: null,
    query: '',
    loading: false,
    initialized: false,
    refreshScheduled: false,
    appInfo: null,
    progress: null,
    modelProgress: null,
    runtimeStats: null,
    error: null,
    listeners: [],
  }),

  getters: {
    selectedFolder(state): FollowedFolder | undefined {
      return state.folders.find((folder) => folder.id === state.selectedFolderId)
    },
    totalImages(state): number {
      return state.folders.reduce((sum, folder) => sum + folder.imageCount, 0)
    },
  },

  actions: {
    async initialize() {
      if (this.initialized) return
      this.loading = true
      try {
        semanticRuntime.setCallbacks({
          progress: (progress) => this.handleModelProgress(progress),
          stats: (stats) => {
            this.runtimeStats = stats
            if (this.appInfo) {
              this.appInfo.runtimeStats = stats
              this.appInfo.aiBackend = stats.backendEffective
              this.appInfo.aiReady = stats.stage === 'ready' || stats.stage === 'indexing'
            }
          },
        })
        await this.bindEvents()
        const [appInfo, folders] = await Promise.all([imagyxApi.appInfo(), imagyxApi.folders()])
        this.appInfo = appInfo
        this.runtimeStats = appInfo.runtimeStats
        this.folders = folders
        this.handleModelProgress(appInfo.modelProgress)
        await this.refreshImages()
        this.initialized = true
        void semanticRuntime
          .prepare()
          .then(() => semanticRuntime.indexPending())
          .then(() => this.scheduleRefresh())
          .catch((error) => this.reportError(error))
      } catch (error) {
        this.reportError(error)
      } finally {
        this.loading = false
      }
    },

    async bindEvents() {
      if (this.listeners.length > 0) return
      const progressUnlisten = await listen<IndexProgress>('index-progress', (event) => {
        this.progress = event.payload
        if (event.payload.stage === 'complete') this.scheduleRefresh()
      })
      const updatedUnlisten = await listen('library-updated', () => this.scheduleRefresh())
      const semanticUnlisten = await listen<string>('semantic-index-requested', (event) => {
        void semanticRuntime
          .indexPending(event.payload)
          .then(() => this.scheduleRefresh())
          .catch((error) => this.reportError(error))
      })
      const modelUnlisten = await listen<ModelStatus>('model-status', (event) => {
        if (this.appInfo) {
          this.appInfo.aiReady = event.payload.ready
          this.appInfo.aiBackend = event.payload.backend
        }
      })
      const modelProgressUnlisten = await listen<ModelDownloadProgress>(
        'model-download-progress',
        (event) => this.handleModelProgress(event.payload),
      )
      const runtimeUnlisten = await listen<RuntimeStats>('runtime-stats', (event) => {
        this.runtimeStats = event.payload
        if (this.appInfo) this.appInfo.runtimeStats = event.payload
      })
      this.listeners.push(
        progressUnlisten,
        updatedUnlisten,
        semanticUnlisten,
        modelUnlisten,
        modelProgressUnlisten,
        runtimeUnlisten,
      )
    },

    scheduleRefresh() {
      if (this.refreshScheduled) return
      this.refreshScheduled = true
      window.setTimeout(() => {
        this.refreshScheduled = false
        void Promise.all([this.refreshFolders(), this.refreshImages()])
      }, 120)
    },

    handleModelProgress(progress: ModelDownloadProgress) {
      this.modelProgress = progress
      if (this.appInfo) this.appInfo.modelProgress = progress
      const toasts = useToastStore()
      if (progress.stage === 'idle') return
      if (progress.stage === 'ready') {
        toasts.upsert({
          id: 'model-download',
          title: 'IA locale prête',
          description: progress.message,
          kind: 'success',
          duration: 3200,
        })
        return
      }
      if (progress.stage === 'error') {
        toasts.upsert({
          id: 'model-download',
          title: 'Chargement du modèle impossible',
          description: progress.message,
          kind: 'error',
          duration: 9000,
        })
        return
      }
      const hasByteProgress = progress.totalBytes > 0
      const percent = hasByteProgress
        ? Math.min(100, (progress.currentBytes / progress.totalBytes) * 100)
        : undefined
      const byteLabel = hasByteProgress
        ? `${formatBytes(progress.currentBytes)} sur ${formatBytes(progress.totalBytes)}`
        : undefined
      const fileLabel = progress.totalFiles > 0
        ? `Fichier ${progress.currentFile} sur ${progress.totalFiles}`
        : undefined
      toasts.upsert({
        id: 'model-download',
        title: progress.stage === 'downloading' ? 'Téléchargement de MobileCLIP-S0' : 'Préparation de WebGPU',
        description: progress.message,
        kind: 'info',
        progress: percent,
        progressLabel: [byteLabel, fileLabel].filter(Boolean).join(' · ') || progress.fileName,
        persistent: true,
      })
    },

    reportError(error: unknown) {
      this.error = String(error)
      useToastStore().upsert({
        id: 'library-error',
        title: 'Une opération a échoué',
        description: this.error,
        kind: 'error',
        duration: 7000,
      })
    },

    async refreshFolders() {
      this.folders = await imagyxApi.folders()
      if (this.selectedFolderId && !this.folders.some((folder) => folder.id === this.selectedFolderId)) {
        this.selectedFolderId = null
      }
    },

    async refreshImages() {
      this.loading = true
      this.error = null
      try {
        const queryVector = this.query ? await semanticRuntime.embedText(this.query) : undefined
        this.images = await imagyxApi.search({
          query: this.query,
          queryVector,
          folderId: this.selectedFolderId ?? undefined,
          limit: 20_000,
        })
      } catch (error) {
        this.reportError(error)
      } finally {
        this.loading = false
      }
    },

    async addFolder(path: string) {
      try {
        const folder = await imagyxApi.addFolder(path)
        await this.refreshFolders()
        this.selectedFolderId = folder.id
        void imagyxApi.indexFolder(folder.id).catch((error) => this.reportError(error))
      } catch (error) {
        this.reportError(error)
      }
    },

    async removeFolder(folderId: string) {
      try {
        await imagyxApi.removeFolder(folderId)
        await this.refreshFolders()
        await this.refreshImages()
      } catch (error) {
        this.reportError(error)
      }
    },

    reindexFolder(folderId: string) {
      void imagyxApi.indexFolder(folderId).catch((error) => this.reportError(error))
    },

    selectFolder(folderId: string | null) {
      this.selectedFolderId = folderId
      void this.refreshImages()
    },

    setQuery(query: string) {
      this.query = query
    },
  },
})

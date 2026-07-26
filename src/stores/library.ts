import { defineStore } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppInfo,
  FollowedFolder,
  ImageAsset,
  IndexProgress,
  ModelDownloadProgress,
} from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import { useToastStore } from './toasts'

interface LibraryState {
  folders: FollowedFolder[]
  images: ImageAsset[]
  selectedFolderId: string | null
  query: string
  loading: boolean
  initialized: boolean
  appInfo: AppInfo | null
  progress: IndexProgress | null
  modelProgress: ModelDownloadProgress | null
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
    appInfo: null,
    progress: null,
    modelProgress: null,
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
        await this.bindEvents()
        const [appInfo, folders] = await Promise.all([imagyxApi.appInfo(), imagyxApi.folders()])
        this.appInfo = appInfo
        this.folders = folders
        this.handleModelProgress(appInfo.modelProgress)
        await this.refreshImages()
        this.initialized = true
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
        if (event.payload.stage === 'complete') {
          void this.refreshFolders()
          void this.refreshImages()
        }
      })
      const updatedUnlisten = await listen('library-updated', () => {
        void this.refreshFolders()
        void this.refreshImages()
      })
      const modelUnlisten = await listen<{ ready: boolean; backend: string }>('model-status', (event) => {
        if (this.appInfo) {
          this.appInfo.aiReady = event.payload.ready
          this.appInfo.aiBackend = event.payload.backend
        }
      })
      const modelProgressUnlisten = await listen<ModelDownloadProgress>(
        'model-download-progress',
        (event) => this.handleModelProgress(event.payload),
      )
      this.listeners.push(
        progressUnlisten,
        updatedUnlisten,
        modelUnlisten,
        modelProgressUnlisten,
      )
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
          title: 'Téléchargement du modèle impossible',
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
        title:
          progress.stage === 'downloading'
            ? 'Téléchargement de l’IA locale'
            : 'Préparation de l’IA locale',
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
        this.images = await imagyxApi.search({
          query: this.query,
          folderId: this.selectedFolderId ?? undefined,
          limit: 300,
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
        await imagyxApi.indexFolder(folder.id)
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

    async reindexFolder(folderId: string) {
      try {
        await imagyxApi.indexFolder(folderId)
      } catch (error) {
        this.reportError(error)
      }
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

import { defineStore } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AppInfo, FollowedFolder, ImageAsset, IndexProgress } from '../types'
import { imagyxApi } from '../api/tauri'

interface LibraryState {
  folders: FollowedFolder[]
  images: ImageAsset[]
  selectedFolderId: string | null
  query: string
  loading: boolean
  initialized: boolean
  appInfo: AppInfo | null
  progress: IndexProgress | null
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
        const [appInfo, folders] = await Promise.all([imagyxApi.appInfo(), imagyxApi.folders()])
        this.appInfo = appInfo
        this.folders = folders
        await this.bindEvents()
        await this.refreshImages()
        this.initialized = true
      } catch (error) {
        this.error = String(error)
      } finally {
        this.loading = false
      }
    },

    async bindEvents() {
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
      this.listeners.push(progressUnlisten, updatedUnlisten, modelUnlisten)
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
        this.error = String(error)
      } finally {
        this.loading = false
      }
    },

    async addFolder(path: string) {
      const folder = await imagyxApi.addFolder(path)
      await this.refreshFolders()
      this.selectedFolderId = folder.id
      await imagyxApi.indexFolder(folder.id)
    },

    async removeFolder(folderId: string) {
      await imagyxApi.removeFolder(folderId)
      await this.refreshFolders()
      await this.refreshImages()
    },

    async reindexFolder(folderId: string) {
      await imagyxApi.indexFolder(folderId)
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

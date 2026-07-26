import { defineStore } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AppInfo, FollowedFolder, ImageAsset, IndexProgress, ModelDownloadProgress, ModelStatus, RuntimeStats, SemanticMatch } from '../types'
import { imagyxApi } from '../api/tauri'
import { semanticRuntime, type SemanticModelKey } from '../services/semantic'
import { formatBytes } from '../utils'
import { useToastStore } from './toasts'

interface LibraryState {
  folders: FollowedFolder[]
  images: ImageAsset[]
  selectedFolderId: string | null
  selectedModel: SemanticModelKey
  query: string
  loading: boolean
  semanticSearching: boolean
  searchSequence: number
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
    folders: [], images: [], selectedFolderId: null, selectedModel: semanticRuntime.modelKey,
    query: '', loading: false, semanticSearching: false, searchSequence: 0, initialized: false,
    refreshScheduled: false, appInfo: null, progress: null, modelProgress: null,
    runtimeStats: null, error: null, listeners: [],
  }),
  getters: {
    selectedFolder: (state) => state.folders.find((folder) => folder.id === state.selectedFolderId),
    totalImages: (state) => state.folders.reduce((sum, folder) => sum + folder.imageCount, 0),
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
              this.appInfo.aiReady = stats.backendEffective !== 'En attente' && stats.stage !== 'error'
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
        void semanticRuntime.prepare().then(() => semanticRuntime.indexPending()).then(() => this.scheduleRefresh()).catch((error) => this.reportError(error))
      } catch (error) { this.reportError(error) } finally { this.loading = false }
    },
    async bindEvents() {
      if (this.listeners.length) return
      const progress = await listen<IndexProgress>('index-progress', (event) => { this.progress = event.payload; if (event.payload.stage === 'complete') this.scheduleRefresh() })
      const updated = await listen('library-updated', () => this.scheduleRefresh())
      const semantic = await listen<string>('semantic-index-requested', (event) => { void semanticRuntime.indexPending(event.payload).then(() => this.scheduleRefresh()).catch((error) => this.reportError(error)) })
      const model = await listen<ModelStatus>('model-status', (event) => { if (this.appInfo) { this.appInfo.aiReady = event.payload.ready; this.appInfo.aiBackend = event.payload.backend } })
      const download = await listen<ModelDownloadProgress>('model-download-progress', (event) => this.handleModelProgress(event.payload))
      const runtime = await listen<RuntimeStats>('runtime-stats', (event) => { this.runtimeStats = event.payload; if (this.appInfo) this.appInfo.runtimeStats = event.payload })
      this.listeners.push(progress, updated, semantic, model, download, runtime)
    },
    scheduleRefresh() {
      if (this.refreshScheduled) return
      this.refreshScheduled = true
      window.setTimeout(() => { this.refreshScheduled = false; void Promise.all([this.refreshFolders(), this.refreshImages()]) }, 120)
    },
    handleModelProgress(progress: ModelDownloadProgress) {
      this.modelProgress = progress
      if (this.appInfo) this.appInfo.modelProgress = progress
      const toasts = useToastStore()
      if (progress.stage === 'idle') return
      if (progress.stage === 'ready') {
        toasts.upsert({ id: 'model-download', title: 'IA locale prête', description: progress.message, kind: 'success', duration: 3200 })
        return
      }
      if (progress.stage === 'error') {
        toasts.upsert({ id: 'model-download', title: 'Chargement du modèle impossible', description: progress.message, kind: 'error', duration: 9000 })
        return
      }
      const hasBytes = progress.totalBytes > 0
      toasts.upsert({
        id: 'model-download', title: progress.stage === 'downloading' ? 'Téléchargement du modèle' : 'Préparation de WebGPU',
        description: progress.message, kind: 'info',
        progress: hasBytes ? Math.min(100, progress.currentBytes / progress.totalBytes * 100) : undefined,
        progressLabel: [hasBytes ? `${formatBytes(progress.currentBytes)} sur ${formatBytes(progress.totalBytes)}` : undefined,
          progress.totalFiles ? `Fichier ${progress.currentFile} sur ${progress.totalFiles}` : undefined].filter(Boolean).join(' · ') || progress.fileName,
        persistent: true,
      })
    },
    reportError(error: unknown) {
      this.error = String(error)
      useToastStore().upsert({ id: 'library-error', title: 'Une opération a échoué', description: this.error, kind: 'error', duration: 7000 })
    },
    async refreshFolders() {
      this.folders = await imagyxApi.folders()
      if (this.selectedFolderId && !this.folders.some((folder) => folder.id === this.selectedFolderId)) this.selectedFolderId = null
    },
    async refreshImages() {
      const sequence = ++this.searchSequence
      const query = this.query.trim()
      this.error = null
      this.semanticSearching = Boolean(query)
      if (!this.images.length) this.loading = true
      try {
        const embedded = query ? await semanticRuntime.embedQuery(query) : undefined
        if (sequence !== this.searchSequence) return
        const images = await imagyxApi.search({ query, queryVector: embedded?.queryVector, folderId: this.selectedFolderId ?? undefined, limit: 20_000 })
        if (sequence !== this.searchSequence) return
        if (embedded?.concepts.length && images.length) {
          const explanations = await imagyxApi.explainResults(images.slice(0, 500).map((image) => image.id), embedded.concepts)
          if (sequence !== this.searchSequence) return
          const byImage = new Map(explanations.map((item) => [item.imageId, item.matches]))
          const words = new Set(embedded.concepts.map((concept) => concept.label))
          this.images = images.map((image) => {
            const haystack = `${image.name} ${image.path}`.toLocaleLowerCase('fr')
            const filename: SemanticMatch[] = [...words].filter((word) => haystack.includes(word)).map((label) => ({ label, score: 1, source: 'filename' as const }))
            return { ...image, semanticMatches: [...filename, ...(byImage.get(image.id) ?? [])].slice(0, 3) }
          })
        } else this.images = images
      } catch (error) { if (sequence === this.searchSequence) this.reportError(error) }
      finally { if (sequence === this.searchSequence) { this.loading = false; this.semanticSearching = false } }
    },
    async selectModel(modelKey: SemanticModelKey) {
      if (modelKey === this.selectedModel) return
      this.selectedModel = modelKey
      this.semanticSearching = true
      try {
        await semanticRuntime.selectModel(modelKey)
        await this.refreshImages()
      } catch (error) { this.reportError(error) }
      finally { this.semanticSearching = false }
    },
    async addFolder(path: string) {
      try { const folder = await imagyxApi.addFolder(path); await this.refreshFolders(); this.selectedFolderId = folder.id; void imagyxApi.indexFolder(folder.id).catch((error) => this.reportError(error)) }
      catch (error) { this.reportError(error) }
    },
    async removeFolder(folderId: string) {
      try { await imagyxApi.removeFolder(folderId); await this.refreshFolders(); await this.refreshImages() }
      catch (error) { this.reportError(error) }
    },
    reindexFolder(folderId: string) { void imagyxApi.indexFolder(folderId).then(() => semanticRuntime.indexPending(folderId)).then(() => this.scheduleRefresh()).catch((error) => this.reportError(error)) },
    selectFolder(folderId: string | null) { this.selectedFolderId = folderId; void this.refreshImages() },
    setQuery(query: string) { this.query = query },
  },
})

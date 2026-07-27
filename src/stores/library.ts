import { markRaw } from "vue";
import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  FollowedFolder,
  ImageAsset,
  IndexProgress,
  ModelDownloadProgress,
  ModelStatus,
  QueryConcept,
  RuntimeStats,
  SemanticMatch,
} from "../types";
import { imagyxApi } from "../api/tauri";
import { semanticRuntime } from "../services/semantic";
import { formatBytes, perfLog } from "../utils";
import { storeT } from "../i18n";
import { useToastStore } from "./toasts";

const explainingImages = new Set<string>();
const BROWSE_PAGE_SIZE = 48;
const SEARCH_RESULT_LIMIT = 60;
let progressClearTimer: number | undefined;

function imageList(images: ImageAsset[]): ImageAsset[] {
  return markRaw(images);
}

function deduplicateMatches(
  matches: SemanticMatch[],
  limit = 8,
): SemanticMatch[] {
  const byLabel = new Map<string, SemanticMatch>();
  for (const match of matches) {
    const key = match.label.trim().toLocaleLowerCase("fr");
    const existing = byLabel.get(key);
    if (!existing) {
      byLabel.set(key, match);
    } else if (match.source === "semantic" && existing.source !== "semantic") {
      byLabel.set(key, match);
    } else if (
      match.score > existing.score &&
      match.source === existing.source
    ) {
      byLabel.set(key, match);
    }
  }
  return [...byLabel.values()]
    .sort((left, right) => right.score - left.score)
    .slice(0, limit);
}

interface LibraryState {
  folders: FollowedFolder[];
  images: ImageAsset[];
  selectedFolderId: string | null;
  query: string;
  activeConcepts: QueryConcept[];
  loading: boolean;
  loadingMore: boolean;
  hasMoreImages: boolean;
  semanticSearching: boolean;
  searchSequence: number;
  initialized: boolean;
  refreshScheduled: boolean;
  appInfo: AppInfo | null;
  progress: IndexProgress | null;
  modelProgress: ModelDownloadProgress | null;
  runtimeStats: RuntimeStats | null;
  lastIndexedAt: string | null;
  error: string | null;
  listeners: UnlistenFn[];
}

export const useLibraryStore = defineStore("library", {
  state: (): LibraryState => ({
    folders: [],
    images: [],
    selectedFolderId: null,
    query: "",
    activeConcepts: [],
    loading: false,
    loadingMore: false,
    hasMoreImages: false,
    semanticSearching: false,
    searchSequence: 0,
    initialized: false,
    refreshScheduled: false,
    appInfo: null,
    progress: null,
    modelProgress: null,
    runtimeStats: null,
    lastIndexedAt: localStorage.getItem("imagyx.last-indexed-at"),
    error: null,
    listeners: [],
  }),
  getters: {
    selectedFolder: (state) =>
      state.folders.find((folder) => folder.id === state.selectedFolderId),
    totalImages: (state) =>
      state.folders.reduce((sum, folder) => sum + folder.imageCount, 0),
  },
  actions: {
    markIndexed() {
      const now = new Date().toISOString();
      this.lastIndexedAt = now;
      try {
        localStorage.setItem("imagyx.last-indexed-at", now);
      } catch {
        /* ignore */
      }
    },

    async initialize() {
      if (this.initialized) return;
      const started = performance.now();
      this.loading = true;
      try {
        localStorage.removeItem("imagyx.semantic-model");
        semanticRuntime.setCallbacks({
          progress: (progress) => this.handleModelProgress(progress),
          stats: (stats) => this.handleRuntimeStats(stats),
        });

        const eventsStarted = performance.now();
        const eventsPromise = this.bindEvents().then(() =>
          perfLog(
            "LibraryStore",
            "bindEvents",
            performance.now() - eventsStarted,
          ),
        );
        const firstPagePromise = this.refreshImages();
        const appInfoStarted = performance.now();
        const appInfoPromise = imagyxApi.appInfo().then((value) => {
          perfLog(
            "LibraryStore",
            "get_app_info IPC",
            performance.now() - appInfoStarted,
          );
          return value;
        });
        const foldersStarted = performance.now();
        const foldersPromise = imagyxApi.folders().then((value) => {
          perfLog(
            "LibraryStore",
            "list_folders IPC",
            performance.now() - foldersStarted,
            { folders: value.length },
          );
          return value;
        });

        const [appInfo, folders] = await Promise.all([
          appInfoPromise,
          foldersPromise,
        ]);
        this.appInfo = appInfo;
        this.runtimeStats = appInfo.runtimeStats;
        this.folders = folders;
        this.handleModelProgress(appInfo.modelProgress);
        await Promise.all([firstPagePromise, eventsPromise]);
        this.initialized = true;
        this.scheduleSemanticWarmup();
        perfLog(
          "LibraryStore",
          "initialize total",
          performance.now() - started,
          { firstPageImages: this.images.length, folders: this.folders.length },
        );
      } catch (error) {
        this.reportError(error);
      } finally {
        this.loading = false;
      }
    },

    scheduleSemanticWarmup() {
      const idleWindow = window as Window & {
        requestIdleCallback?: (
          callback: IdleRequestCallback,
          options?: IdleRequestOptions,
        ) => number;
      };
      const warmText = () => {
        const started = performance.now();
        void semanticRuntime
          .prewarmText()
          .then(() => {
            perfLog(
              "SemanticIA",
              "text runtime prewarm",
              performance.now() - started,
            );
          })
          .catch((error) => this.reportError(error));
      };

      window.requestAnimationFrame(() =>
        window.requestAnimationFrame(() => {
          if (idleWindow.requestIdleCallback) {
            idleWindow.requestIdleCallback(warmText, { timeout: 650 });
          } else {
            window.setTimeout(warmText, 350);
          }
        }),
      );
    },

    async bindEvents() {
      if (this.listeners.length) return;
      const listeners = await Promise.all([
        listen<IndexProgress>("index-progress", (event) =>
          this.handleIndexProgress(event.payload),
        ),
        listen("library-updated", () => this.scheduleRefresh()),
        listen<string>("semantic-index-requested", (event) => {
          void semanticRuntime
            .indexPending(event.payload)
            .then(() => this.scheduleRefresh())
            .catch((error) => this.reportError(error));
        }),
        listen<ModelStatus>("model-status", (event) => {
          if (this.appInfo) {
            this.appInfo.aiReady = event.payload.ready;
            this.appInfo.aiBackend = event.payload.backend;
          }
        }),
        listen<ModelDownloadProgress>("model-download-progress", (event) =>
          this.handleModelProgress(event.payload),
        ),
        listen<RuntimeStats>("runtime-stats", (event) =>
          this.handleRuntimeStats(event.payload),
        ),
      ]);
      this.listeners.push(...listeners);
    },

    handleIndexProgress(progress: IndexProgress) {
      this.progress = progress;
      if (progress.current > 0) this.markIndexed();
      const toasts = useToastStore();
      if (progress.stage === "complete") {
        this.markIndexed();
        this.scheduleRefresh();
        toasts.upsert({
          id: `reindex-${progress.folderId}`,
          title: storeT('indexing.toast.complete_title', undefined, { folder: progress.folderName }),
          description: progress.message,
          kind: "success",
          duration: 2600,
        });
        this.scheduleProgressClear(progress.folderId);
      } else if (progress.stage === "error") {
        useToastStore().upsert({
          id: `reindex-${progress.folderId}`,
          title: storeT('indexing.toast.error_title', undefined, { folder: progress.folderName }),
          description: progress.message,
          kind: "error",
          duration: 8000,
        });
      }
    },

    handleRuntimeStats(stats: RuntimeStats) {
      this.runtimeStats = stats;
      if (stats.current > 0) this.markIndexed();
      if (this.appInfo) {
        this.appInfo.runtimeStats = stats;
        this.appInfo.aiBackend = stats.backendEffective;
        this.appInfo.aiReady =
          stats.stage !== "error" && stats.stage !== "loading-text";
      }
      if (stats.stage === "ready" && this.progress?.stage === "queued") {
        const progress = this.progress;
        this.progress = {
          ...progress,
          stage: "complete",
          current: progress.total,
          message: storeT('indexing.title.completed'),
        };
        useToastStore().upsert({
          id: `reindex-${progress.folderId}`,
          title: storeT('indexing.toast.complete_title', undefined, { folder: progress.folderName }),
          description: storeT('indexing.toast.complete_desc'),
          kind: "success",
          duration: 2600,
        });
        this.scheduleProgressClear(progress.folderId);
      }
    },

    scheduleProgressClear(folderId: string) {
      if (progressClearTimer) window.clearTimeout(progressClearTimer);
      progressClearTimer = window.setTimeout(() => {
        if (this.progress?.folderId === folderId) this.progress = null;
      }, 2400);
    },

    scheduleRefresh() {
      if (this.refreshScheduled) return;
      this.refreshScheduled = true;
      window.setTimeout(() => {
        this.refreshScheduled = false;
        void Promise.all([this.refreshFolders(), this.refreshImages()]);
      }, 160);
    },

    handleModelProgress(progress: ModelDownloadProgress) {
      this.modelProgress = progress;
      if (this.appInfo) this.appInfo.modelProgress = progress;
      const toasts = useToastStore();
      if (progress.stage === "idle") return;
      if (progress.stage === "ready") {
        toasts.upsert({
          id: "model-download",
          title: storeT('indexing.toast.model_ready'),
          description: progress.message,
          kind: "success",
          duration: 3200,
        });
        return;
      }
      if (progress.stage === "error") {
        useToastStore().upsert({
          id: "model-download",
          title: storeT('indexing.toast.model_error'),
          description: progress.message,
          kind: "error",
          duration: 9000,
        });
        return;
      }
      const hasBytes = progress.totalBytes > 0;
      toasts.upsert({
        id: "model-download",
        title:
          progress.stage === "downloading"
            ? storeT('indexing.toast.model_downloading')
            : storeT('indexing.toast.model_preparing'),
        description: progress.message,
        kind: "info",
        progress: hasBytes
          ? Math.min(100, (progress.currentBytes / progress.totalBytes) * 100)
          : undefined,
        progressLabel:
          [
            hasBytes
              ? storeT('indexing.message.downloading', undefined, { current: formatBytes(progress.currentBytes), total: formatBytes(progress.totalBytes) })
              : undefined,
            progress.totalFiles
              ? storeT('indexing.toast.file_progress', undefined, { current: progress.currentFile, total: progress.totalFiles })
              : undefined,
          ]
            .filter(Boolean)
            .join(" · ") || progress.fileName,
        persistent: true,
      });
    },

    reportError(error: unknown) {
      this.error = String(error);
      useToastStore().upsert({
        id: "library-error",
        title: storeT('indexing.toast.operation_failed'),
        description: this.error,
        kind: "error",
        duration: 7000,
      });
    },

    async refreshFolders() {
      const started = performance.now();
      this.folders = await imagyxApi.folders();
      perfLog(
        "LibraryStore",
        "refreshFolders",
        performance.now() - started,
        { folders: this.folders.length },
      );
      if (
        this.selectedFolderId &&
        !this.folders.some((folder) => folder.id === this.selectedFolderId)
      ) {
        this.selectedFolderId = null;
      }
    },

    async refreshImages() {
      const start = performance.now();
      const sequence = ++this.searchSequence;
      const query = this.query.trim();
      const folderId = this.selectedFolderId ?? undefined;
      this.error = null;
      this.activeConcepts = [];
      this.semanticSearching = Boolean(query);
      this.loadingMore = false;
      this.hasMoreImages = false;
      if (!this.images.length) this.loading = true;
      try {
        if (!query) {
          const results = await imagyxApi.search({
            query,
            folderId,
            limit: BROWSE_PAGE_SIZE,
            offset: 0,
          });
          if (sequence !== this.searchSequence) return;
          this.images = imageList(results);
          this.hasMoreImages = results.length === BROWSE_PAGE_SIZE;
          perfLog(
            "LibraryStore",
            "refreshImages first browse page",
            performance.now() - start,
            { received: results.length, hasMore: this.hasMoreImages },
          );
          return;
        }

        const lexicalStarted = performance.now();
        const lexicalPromise = imagyxApi
          .search({ query, folderId, limit: SEARCH_RESULT_LIMIT })
          .then((results) => {
            if (
              sequence === this.searchSequence &&
              (results.length > 0 || this.images.length === 0)
            ) {
              this.images = imageList(results);
              this.loading = false;
              perfLog(
                "LibraryStore",
                "refreshImages fast lexical",
                performance.now() - lexicalStarted,
                { results: results.length },
              );
            }
            return results;
          });
        const embeddingStarted = performance.now();
        const embeddingPromise = semanticRuntime.embedQuery(query).then((result) => {
          perfLog(
            "SemanticIA",
            "embedQuery",
            performance.now() - embeddingStarted,
            { queryLength: query.length, vectorReady: Boolean(result?.queryVector) },
          );
          return result;
        });
        const [lexicalResult, embeddingResult] = await Promise.allSettled([
          lexicalPromise,
          embeddingPromise,
        ]);
        if (lexicalResult.status === "rejected") throw lexicalResult.reason;
        if (sequence !== this.searchSequence) return;
        if (embeddingResult.status === "rejected") throw embeddingResult.reason;

        const embedded = embeddingResult.value;
        this.activeConcepts = embedded?.concepts ?? [];
        if (!embedded?.queryVector) return;
        const hybridStarted = performance.now();
        const hybrid = await imagyxApi.search({
          query,
          queryVector: embedded.queryVector,
          folderId,
          limit: SEARCH_RESULT_LIMIT,
        });
        if (sequence !== this.searchSequence) return;
        this.images = imageList(hybrid);
        perfLog(
          "LibraryStore",
          "refreshImages hybrid IPC",
          performance.now() - hybridStarted,
          { results: hybrid.length },
        );
        perfLog(
          "LibraryStore",
          "refreshImages semantic total",
          performance.now() - start,
          { results: hybrid.length },
        );
      } catch (error) {
        if (sequence === this.searchSequence) this.reportError(error);
      } finally {
        if (sequence === this.searchSequence) {
          this.loading = false;
          this.semanticSearching = false;
        }
      }
    },

    async loadMoreImages() {
      if (
        this.query.trim() ||
        !this.hasMoreImages ||
        this.loadingMore
      ) {
        return;
      }
      const sequence = this.searchSequence;
      const offset = this.images.length;
      const folderId = this.selectedFolderId ?? undefined;
      const started = performance.now();
      this.loadingMore = true;
      try {
        const page = await imagyxApi.search({
          query: "",
          folderId,
          limit: BROWSE_PAGE_SIZE,
          offset,
        });
        if (sequence !== this.searchSequence) return;
        const existing = new Set(this.images.map((image) => image.id));
        const appended = page.filter((image) => !existing.has(image.id));
        this.images = imageList([...this.images, ...appended]);
        this.hasMoreImages = page.length === BROWSE_PAGE_SIZE;
        perfLog(
          "LibraryStore",
          "loadMoreImages backend page",
          performance.now() - started,
          {
            offset,
            received: page.length,
            appended: appended.length,
            totalLoaded: this.images.length,
          },
        );
      } catch (error) {
        if (sequence === this.searchSequence) this.reportError(error);
      } finally {
        if (sequence === this.searchSequence) this.loadingMore = false;
      }
    },

    async explainImage(imageId: string) {
      const image = this.images.find((item) => item.id === imageId);
      if (
        !image ||
        image.semanticMatches?.length ||
        explainingImages.has(imageId)
      ) {
        return;
      }
      explainingImages.add(imageId);
      try {
        const concepts = this.activeConcepts.length
          ? this.activeConcepts
          : await semanticRuntime.genericImageConcepts();
        const explanation = (
          await imagyxApi.explainResults([imageId], concepts)
        )[0];
        if (!explanation) return;
        const haystack = `${image.name} ${image.path}`.toLocaleLowerCase("fr");
        const filename: SemanticMatch[] = this.activeConcepts
          .filter((concept) =>
            haystack.includes(concept.label.toLocaleLowerCase("fr")),
          )
          .map((concept) => ({
            label: concept.label,
            score: 1,
            source: "filename" as const,
          }));
        const semantic = explanation.matches.filter(
          (match) => match.score >= 0.5,
        );
        this.images = imageList(
          this.images.map((item) =>
            item.id === imageId
              ? {
                  ...item,
                  semanticMatches: deduplicateMatches([
                    ...filename,
                    ...semantic,
                  ]),
                }
              : item,
          ),
        );
      } catch (error) {
        this.reportError(error);
      } finally {
        explainingImages.delete(imageId);
      }
    },

    pauseIndexing() {
      semanticRuntime.pauseIndexing();
    },

    async resumeIndexing() {
      try {
        await semanticRuntime.resumeIndexing();
        this.scheduleRefresh();
      } catch (error) {
        this.reportError(error);
      }
    },

    async addFolder(path: string) {
      try {
        const folder = await imagyxApi.addFolder(path);
        await this.refreshFolders();
        this.selectedFolderId = folder.id;
        this.progress = {
          folderId: folder.id,
          folderName: folder.name,
          current: 0,
          total: 0,
          stage: "discovering",
          message: storeT('indexing.message.analyzing'),
        };
        void imagyxApi
          .indexFolder(folder.id)
          .catch((error) => this.reportError(error));
      } catch (error) {
        this.reportError(error);
      }
    },

    async removeFolder(folderId: string) {
      try {
        await imagyxApi.removeFolder(folderId);
        await this.refreshFolders();
        await this.refreshImages();
      } catch (error) {
        this.reportError(error);
      }
    },

    async reindexFolder(folderId: string) {
      const folder = this.folders.find((item) => item.id === folderId);
      if (!folder) return;
      this.progress = {
        folderId,
        folderName: folder.name,
        current: 0,
        total: folder.imageCount,
        stage: "discovering",
        message: storeT('indexing.message.reindexing', undefined, { folder: folder!.name }),
      };
      useToastStore().upsert({
        id: `reindex-${folderId}`,
        title: storeT('indexing.toast.reindex_title', undefined, { folder: folder.name }),
        description: storeT('indexing.toast.reindex_desc'),
        kind: "info",
        persistent: true,
      });
      try {
        await imagyxApi.indexFolder(folderId);
        await semanticRuntime.indexPending(folderId);
        this.scheduleRefresh();
      } catch (error) {
        this.progress = {
          ...this.progress,
          folderId,
          folderName: folder.name,
          current: 0,
          total: folder.imageCount,
          stage: "error",
          message: String(error),
        };
        this.reportError(error);
      }
    },

    selectFolder(folderId: string | null) {
      this.selectedFolderId = folderId;
      void this.refreshImages();
    },

    setQuery(query: string) {
      this.query = query;
    },
  },
});

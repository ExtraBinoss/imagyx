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
import { useToastStore } from "./toasts";

const explainingImages = new Set<string>();
const INITIAL_DISPLAY_LIMIT = 30;
const PAGE_INCREMENT = 30;

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
    .sort((a, b) => b.score - a.score)
    .slice(0, limit);
}

interface LibraryState {
  folders: FollowedFolder[];
  allSearchResults: ImageAsset[];
  images: ImageAsset[];
  displayLimit: number;
  selectedFolderId: string | null;
  query: string;
  activeConcepts: QueryConcept[];
  loading: boolean;
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
    allSearchResults: [],
    images: [],
    displayLimit: INITIAL_DISPLAY_LIMIT,
    selectedFolderId: null,
    query: "",
    activeConcepts: [],
    loading: false,
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
      this.loading = true;
      try {
        localStorage.removeItem("imagyx.semantic-model");
        semanticRuntime.setCallbacks({
          progress: (progress) => this.handleModelProgress(progress),
          stats: (stats) => {
            this.runtimeStats = stats;
            if (stats.current > 0) this.markIndexed();
            if (this.appInfo) {
              this.appInfo.runtimeStats = stats;
              this.appInfo.aiBackend = stats.backendEffective;
              this.appInfo.aiReady =
                stats.backendEffective !== "En attente" &&
                stats.stage !== "error";
            }
          },
        });
        await this.bindEvents();
        const [appInfo, folders] = await Promise.all([
          imagyxApi.appInfo(),
          imagyxApi.folders(),
        ]);
        this.appInfo = appInfo;
        this.runtimeStats = appInfo.runtimeStats;
        this.folders = folders;
        this.handleModelProgress(appInfo.modelProgress);
        await this.refreshImages();
        this.initialized = true;
        // Defer semantic runtime preparation to keep startup UI buttery smooth
        if (typeof window !== "undefined" && "requestIdleCallback" in window) {
          window.requestIdleCallback(() => {
            void semanticRuntime
              .prepare()
              .then(() => semanticRuntime.indexPending())
              .then(() => this.scheduleRefresh())
              .catch((error) => this.reportError(error));
          });
        } else {
          setTimeout(() => {
            void semanticRuntime
              .prepare()
              .then(() => semanticRuntime.indexPending())
              .then(() => this.scheduleRefresh())
              .catch((error) => this.reportError(error));
          }, 400);
        }
      } catch (error) {
        this.reportError(error);
      } finally {
        this.loading = false;
      }
    },
    async bindEvents() {
      if (this.listeners.length) return;
      const progress = await listen<IndexProgress>(
        "index-progress",
        (event) => {
          this.progress = event.payload;
          if (event.payload.current > 0) this.markIndexed();
          if (event.payload.stage === "complete") {
            this.markIndexed();
            this.scheduleRefresh();
          }
        },
      );
      const updated = await listen("library-updated", () =>
        this.scheduleRefresh(),
      );
      const semantic = await listen<string>(
        "semantic-index-requested",
        (event) => {
          void semanticRuntime
            .indexPending(event.payload)
            .then(() => this.scheduleRefresh())
            .catch((error) => this.reportError(error));
        },
      );
      const model = await listen<ModelStatus>("model-status", (event) => {
        if (this.appInfo) {
          this.appInfo.aiReady = event.payload.ready;
          this.appInfo.aiBackend = event.payload.backend;
        }
      });
      const download = await listen<ModelDownloadProgress>(
        "model-download-progress",
        (event) => this.handleModelProgress(event.payload),
      );
      const runtime = await listen<RuntimeStats>("runtime-stats", (event) => {
        this.runtimeStats = event.payload;
        if (event.payload.current > 0) this.markIndexed();
        if (this.appInfo) this.appInfo.runtimeStats = event.payload;
      });
      this.listeners.push(
        progress,
        updated,
        semantic,
        model,
        download,
        runtime,
      );
    },
    scheduleRefresh() {
      if (this.refreshScheduled) return;
      this.refreshScheduled = true;
      window.setTimeout(() => {
        this.refreshScheduled = false;
        void Promise.all([this.refreshFolders(), this.refreshImages()]);
      }, 120);
    },
    handleModelProgress(progress: ModelDownloadProgress) {
      this.modelProgress = progress;
      if (this.appInfo) this.appInfo.modelProgress = progress;
      const toasts = useToastStore();
      if (progress.stage === "idle") return;
      if (progress.stage === "ready") {
        toasts.upsert({
          id: "model-download",
          title: "IA locale prête",
          description: progress.message,
          kind: "success",
          duration: 3200,
        });
        return;
      }
      if (progress.stage === "error") {
        toasts.upsert({
          id: "model-download",
          title: "Chargement du modèle impossible",
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
            ? "Téléchargement du modèle"
            : "Préparation de WebGPU",
        description: progress.message,
        kind: "info",
        progress: hasBytes
          ? Math.min(100, (progress.currentBytes / progress.totalBytes) * 100)
          : undefined,
        progressLabel:
          [
            hasBytes
              ? `${formatBytes(progress.currentBytes)} sur ${formatBytes(progress.totalBytes)}`
              : undefined,
            progress.totalFiles
              ? `Fichier ${progress.currentFile} sur ${progress.totalFiles}`
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
        title: "Une opération a échoué",
        description: this.error,
        kind: "error",
        duration: 7000,
      });
    },
    async refreshFolders() {
      this.folders = await imagyxApi.folders();
      if (
        this.selectedFolderId &&
        !this.folders.some((folder) => folder.id === this.selectedFolderId)
      )
        this.selectedFolderId = null;
    },
    async refreshImages() {
      const start = performance.now();
      const sequence = ++this.searchSequence;
      const query = this.query.trim();
      const folderId = this.selectedFolderId ?? undefined;
      this.error = null;
      this.activeConcepts = [];
      this.semanticSearching = Boolean(query);
      this.displayLimit = INITIAL_DISPLAY_LIMIT;
      if (!this.images.length) this.loading = true;
      try {
        if (!query) {
          const results = await imagyxApi.search({
            query,
            folderId,
            limit: 20_000,
          });
          if (sequence !== this.searchSequence) return;
          this.allSearchResults = results;
          this.images = results.slice(0, this.displayLimit);
          perfLog("LibraryStore", "refreshImages (browse all)", performance.now() - start, {
            totalResults: results.length,
            displayed: this.images.length,
          });
          return;
        }

        const lexicalPromise = imagyxApi
          .search({ query, folderId, limit: 20_000 })
          .then((results) => {
            if (sequence === this.searchSequence) {
              this.allSearchResults = results;
              this.images = results.slice(0, this.displayLimit);
              this.loading = false;
              perfLog("LibraryStore", "refreshImages (fast lexical)", performance.now() - start, {
                totalResults: results.length,
                displayed: this.images.length,
              });
            }
            return results;
          });
        const [lexicalResult, embeddingResult] = await Promise.allSettled([
          lexicalPromise,
          semanticRuntime.embedQuery(query),
        ]);
        if (lexicalResult.status === "rejected") throw lexicalResult.reason;
        if (sequence !== this.searchSequence) return;
        if (embeddingResult.status === "rejected") throw embeddingResult.reason;

        const embedded = embeddingResult.value;
        this.activeConcepts = embedded?.concepts ?? [];
        if (!embedded?.queryVector) return;
        const hybrid = await imagyxApi.search({
          query,
          queryVector: embedded.queryVector,
          folderId,
          limit: 20_000,
        });
        if (sequence !== this.searchSequence) return;
        this.allSearchResults = hybrid;
        this.images = hybrid.slice(0, this.displayLimit);
        perfLog("LibraryStore", "refreshImages (hybrid semantic finish)", performance.now() - start, {
          totalResults: hybrid.length,
          displayed: this.images.length,
        });
      } catch (error) {
        if (sequence === this.searchSequence) this.reportError(error);
      } finally {
        if (sequence === this.searchSequence) {
          this.loading = false;
          this.semanticSearching = false;
        }
      }
    },
    loadMoreImages() {
      if (this.displayLimit >= this.allSearchResults.length) return;
      this.displayLimit += PAGE_INCREMENT;
      this.images = this.allSearchResults.slice(0, this.displayLimit);
      perfLog("LibraryStore", "loadMoreImages", 0, {
        newDisplayLimit: this.displayLimit,
        total: this.allSearchResults.length,
      });
    },
    async explainImage(imageId: string) {
      const image = this.images.find((item) => item.id === imageId);
      if (
        !image ||
        image.semanticMatches?.length ||
        explainingImages.has(imageId)
      )
        return;
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
        const index = this.images.findIndex((item) => item.id === imageId);
        if (index >= 0)
          this.images[index] = {
            ...this.images[index],
            semanticMatches: deduplicateMatches([...filename, ...semantic]),
          };
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
    reindexFolder(folderId: string) {
      void imagyxApi
        .indexFolder(folderId)
        .then(() => semanticRuntime.indexPending(folderId))
        .then(() => this.scheduleRefresh())
        .catch((error) => this.reportError(error));
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

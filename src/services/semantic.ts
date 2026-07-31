import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { imagyxApi } from '../api/tauri'
import type { ImageAsset, ModelDownloadProgress, QueryConcept, RuntimeStats } from '../types'
import { perfLog } from '../utils'
import { requestSemanticEmbedding } from './semantic-channel'
import {
  buildQueryPromptPlan,
  combinePromptVectors,
  weightedPromptVectors,
} from './query-prompts'
import { configureLocalOnnxWasm } from './onnx-wasm-assets'
import {
  VisionWorkerClient,
  type VisionDevice,
  type VisionInferenceProfile,
} from './vision-worker-client'

const MODEL_ID = 'Xenova/mobileclip_s0'
const MODEL_NAME = 'MobileCLIP-S0'
const INDEX_PAUSED_KEY = 'imagyx.index-paused'
const INITIAL_WEBGPU_BATCH_SIZE = 8
const MIN_WEBGPU_BATCH_SIZE = 8
const INITIAL_WASM_BATCH_SIZE = 4
const MIN_WASM_BATCH_SIZE = 2
const MAX_BATCH_SIZE = 16
const MAX_SEARCH_PRIORITY_WAIT_MS = 200
const AI_IMAGE_EDGE = 224
const AI_IMAGE_CHANNELS = 3
const AI_IMAGE_BYTES = AI_IMAGE_EDGE * AI_IMAGE_EDGE * AI_IMAGE_CHANNELS
const QUERY_CACHE_CAPACITY = 24
const currentWindow = getCurrentWindow()
const DEFAULT_IMAGE_LABELS = [
  'personne', 'femme', 'homme', 'enfant', 'groupe de personnes', 'visage',
  'animal', 'chien', 'chat', 'oiseau', 'voiture', 'vélo', 'bâtiment', 'maison',
  'ville', 'rue', 'intérieur', 'extérieur', 'nature', 'forêt', 'montagne', 'plage',
  'mer', 'ciel', 'nuit', 'coucher de soleil', 'nourriture', 'fleur', 'texte',
  'capture d’écran', 'dessin', 'illustration', 'portrait', 'paysage', 'objet rouge',
  'objet bleu', 'objet vert', 'objet jaune', 'image sombre', 'image lumineuse',
]

type Device = VisionDevice
type TransformersModule = typeof import('@huggingface/transformers')
type RuntimeCallbacks = {
  progress: (progress: ModelDownloadProgress) => void
  stats: (stats: RuntimeStats) => void
}

type PreparedBatch = {
  batch: ImageAsset[]
  pixels: ArrayBuffer
  prepareMs: number
}

type UiResponsivenessProfile = {
  samples: number
  delayedFrames: number
  maxEventLoopDelayMs: number
  averageEventLoopDelayMs: number
}

type RustAiImagePrepProfile = {
  batchId: string
  count: number
  outputBytes: number
  cacheHits: number
  cacheMisses: number
  databaseLookupMs: number
  rayonWallMs: number
  packMs: number
  commandWallMs: number
  cumulativeLockWaitMs: number
  cumulativeCacheLookupMs: number
  cumulativeThumbnailReadDecodeMs: number
  cumulativeSourceReadDecodeMs: number
  cumulativeThumbnailResizeMs: number
  cumulativeThumbnailEncodeWriteMs: number
  cumulativeAiResizeMs: number
  cumulativeImageTotalMs: number
  maxImageTotalMs: number
}

let transformersLoading: Promise<TransformersModule> | null = null
let rustProfileListener: Promise<void> | null = null

function loadTransformers(): Promise<TransformersModule> {
  if (!transformersLoading) {
    const started = performance.now()
    transformersLoading = import('@huggingface/transformers').then((module) => {
      perfLog('SemanticIA', 'Transformers.js module load', performance.now() - started)
      return module
    })
  }
  return transformersLoading
}

function roundMs(value: number): number {
  return Math.round(value * 100) / 100
}

function logRustAiPrepProfile(profile: RustAiImagePrepProfile) {
  console.groupCollapsed(
    `[Imagyx][Rust AI Prep][${profile.batchId}] ${profile.count} img · ${roundMs(profile.commandWallMs)} ms · cache ${profile.cacheHits}/${profile.count}`,
  )
  console.table({
    database_lookup_ms: roundMs(profile.databaseLookupMs),
    rayon_parallel_wall_ms: roundMs(profile.rayonWallMs),
    rgb_pack_ms: roundMs(profile.packMs),
    command_end_to_end_ms: roundMs(profile.commandWallMs),
    cumulative_lock_wait_ms: roundMs(profile.cumulativeLockWaitMs),
    cumulative_cache_lookup_ms: roundMs(profile.cumulativeCacheLookupMs),
    cumulative_cached_jpeg_read_decode_ms: roundMs(profile.cumulativeThumbnailReadDecodeMs),
    cumulative_original_read_decode_ms: roundMs(profile.cumulativeSourceReadDecodeMs),
    cumulative_thumbnail_resize_ms: roundMs(profile.cumulativeThumbnailResizeMs),
    cumulative_thumbnail_encode_write_ms: roundMs(profile.cumulativeThumbnailEncodeWriteMs),
    cumulative_ai_224_resize_ms: roundMs(profile.cumulativeAiResizeMs),
    cumulative_all_images_ms: roundMs(profile.cumulativeImageTotalMs),
    slowest_image_ms: roundMs(profile.maxImageTotalMs),
  })
  console.log('Rust batch metadata', {
    batchId: profile.batchId,
    count: profile.count,
    outputBytes: profile.outputBytes,
    cacheHits: profile.cacheHits,
    cacheMisses: profile.cacheMisses,
    note: 'Les valeurs cumulative_* sont additionnées entre les threads Rayon et peuvent dépasser rayon_parallel_wall_ms. Le temps mur à comparer au frontend est command_end_to_end_ms.',
  })
  console.groupEnd()
}

function ensureRustProfileListener(): Promise<void> {
  if (!rustProfileListener) {
    rustProfileListener = listen<RustAiImagePrepProfile>('ai-image-prep-profile', (event) => {
      logRustAiPrepProfile(event.payload)
    }).then(() => undefined)
  }
  return rustProfileListener
}

function startUiResponsivenessProbe(intervalMs = 16): () => UiResponsivenessProfile {
  let previous = performance.now()
  let samples = 0
  let delayedFrames = 0
  let totalDelayMs = 0
  let maxEventLoopDelayMs = 0
  const handle = window.setInterval(() => {
    const now = performance.now()
    const delayMs = Math.max(0, now - previous - intervalMs)
    previous = now
    samples += 1
    totalDelayMs += delayMs
    maxEventLoopDelayMs = Math.max(maxEventLoopDelayMs, delayMs)
    if (delayMs > 16.7) delayedFrames += 1
  }, intervalMs)

  return () => {
    window.clearInterval(handle)
    return {
      samples,
      delayedFrames,
      maxEventLoopDelayMs,
      averageEventLoopDelayMs: samples > 0 ? totalDelayMs / samples : 0,
    }
  }
}

async function profileUiWhile<T>(task: Promise<T>): Promise<{ value: T; ui: UiResponsivenessProfile }> {
  const stopProbe = startUiResponsivenessProbe()
  try {
    const value = await task
    return { value, ui: stopProbe() }
  } catch (error) {
    stopProbe()
    throw error
  }
}

function logIndexBatchProfile(
  profile: VisionInferenceProfile,
  rustPrepareMs: number,
  sqliteMs: number,
  batchTotalMs: number,
  ui: UiResponsivenessProfile,
) {
  const workerStageSumMs = profile.queueWaitMs
    + profile.rawImageWrapMs
    + profile.preprocessMs
    + profile.modelAwaitMs
    + profile.normalizeMs
    + profile.readbackToListMs
    + profile.flattenMs
  const unaccountedWorkerMs = Math.max(0, profile.workerTotalMs - workerStageSumMs)
  const accountedMainMs = rustPrepareMs + profile.clientTotalMs + sqliteMs
  const unaccountedMainMs = Math.max(0, batchTotalMs - accountedMainMs)
  const perImageMs = batchTotalMs / Math.max(1, profile.count)

  console.groupCollapsed(
    `[Imagyx][AI Profile][${profile.batchId}] ${profile.count} img · ${roundMs(batchTotalMs)} ms · ${roundMs(perImageMs)} ms/img`,
  )
  console.table({
    rust_thumbnail_rgb_ipc_ms: roundMs(rustPrepareMs),
    transfer_main_to_worker_ms: roundMs(profile.transferToWorkerMs),
    worker_queue_wait_ms: roundMs(profile.queueWaitMs),
    raw_image_views_ms: roundMs(profile.rawImageWrapMs),
    transformers_preprocess_ms: roundMs(profile.preprocessMs),
    model_await_wall_ms: roundMs(profile.modelAwaitMs),
    tensor_normalize_ms: roundMs(profile.normalizeMs),
    gpu_readback_to_list_ms: roundMs(profile.readbackToListMs),
    flatten_float32_worker_ms: roundMs(profile.flattenMs),
    worker_unaccounted_ms: roundMs(unaccountedWorkerMs),
    worker_total_ms: roundMs(profile.workerTotalMs),
    return_worker_to_main_ms: roundMs(profile.returnToMainMs),
    worker_round_trip_ms: roundMs(profile.roundTripMs),
    vectors_deserialize_main_ms: roundMs(profile.deserializeMs),
    sqlite_save_ms: roundMs(sqliteMs),
    main_unaccounted_ms: roundMs(unaccountedMainMs),
    batch_end_to_end_ms: roundMs(batchTotalMs),
    main_event_loop_max_delay_ms: roundMs(ui.maxEventLoopDelayMs),
    main_event_loop_average_delay_ms: roundMs(ui.averageEventLoopDelayMs),
  })
  console.log('Batch metadata', {
    requestId: profile.requestId,
    batchId: profile.batchId,
    count: profile.count,
    inputBytes: profile.inputBytes,
    outputBytes: profile.outputBytes,
    uiProbeSamples: ui.samples,
    uiDelayedFrames: ui.delayedFrames,
    interpretation: {
      modelAwait: 'Temps mur de l’appel Transformers/ONNX. Il peut inclure soumission WebGPU et attente backend.',
      gpuReadbackToList: 'Temps pour matérialiser le tenseur en tableaux JS; c’est un point de synchronisation GPU probable.',
      rustThumbnailRgbIpc: 'Commande Tauri complète: cache/decode/resize Rust + assemblage RGB + transport IPC.',
    },
  })
  console.groupEnd()
}

export interface EmbeddedQuery {
  queryVector: number[]
  concepts: QueryConcept[]
}

class SemanticRuntime {
  private readonly visionWorker = new VisionWorkerClient()
  private textModel: any = null
  private tokenizer: any = null
  private visionDevice: Device = 'webgpu'
  private visionReady = false
  private loading: Promise<void> | null = null
  private textLoading: Promise<void> | null = null
  private textPriming: Promise<void> | null = null
  private textPrimed = false
  private environmentLoading: Promise<string> | null = null
  private localModelPath: string | null = null
  private indexing: Promise<void> | null = null
  private pendingIndexFolders = new Set<string>()
  private pendingIndexAll = false
  private activeSearchEmbeddings = 0
  private searchPriorityWaiters = new Set<() => void>()
  private genericConcepts: QueryConcept[] | null = null
  private callbacks: RuntimeCallbacks | null = null
  private queryCache = new Map<string, EmbeddedQuery>()
  private queryInFlight = new Map<string, Promise<EmbeddedQuery>>()
  private paused = localStorage.getItem(INDEX_PAUSED_KEY) === 'true'
  private stats: RuntimeStats = { ...defaultStats(), stage: this.paused ? 'paused' : 'idle' }

  get isPaused() { return this.paused }

  setCallbacks(callbacks: RuntimeCallbacks) {
    this.callbacks = callbacks
    callbacks.stats(this.stats)
  }

  pauseIndexing() {
    this.paused = true
    localStorage.setItem(INDEX_PAUSED_KEY, 'true')
    this.patchStats({ stage: 'paused', imagesPerSecond: 0 })
  }

  async resumeIndexing(folderId?: string) {
    this.paused = false
    localStorage.removeItem(INDEX_PAUSED_KEY)
    await this.indexPending(folderId)
  }

  async prepare() {
    if (this.visionReady) return
    if (this.loading) return this.loading
    this.loading = this.loadVisionRuntime()
    try { await this.loading } finally { this.loading = null }
  }

  async prewarmText() {
    if (currentWindow.label === 'spotlight') return
    await this.ensureTextReady()
    if (this.textPrimed) return
    if (!this.textPriming) {
      this.textPriming = this.primeTextRuntime()
        .finally(() => { this.textPriming = null })
    }
    await this.textPriming
  }

  async indexPending(folderId?: string) {
    if (this.paused) {
      this.patchStats({ stage: 'paused', imagesPerSecond: 0 })
      return
    }
    if (folderId) this.pendingIndexFolders.add(folderId)
    else this.pendingIndexAll = true

    if (this.indexing) return this.indexing
    this.indexing = this.drainPendingIndexQueue()
    try { await this.indexing } finally { this.indexing = null }
  }

  private async drainPendingIndexQueue() {
    while (!this.paused && (this.pendingIndexAll || this.pendingIndexFolders.size > 0)) {
      const folderId = this.nextPendingFolder()
      await this.runPendingIndex(folderId)
    }
  }

  private nextPendingFolder(): string | undefined {
    if (this.pendingIndexAll) {
      this.pendingIndexAll = false
      this.pendingIndexFolders.clear()
      return undefined
    }
    const folderId = this.pendingIndexFolders.values().next().value
    if (!folderId) return undefined
    this.pendingIndexFolders.delete(folderId)
    return folderId
  }

  private async prepareImageBatch(batch: ImageAsset[], batchId: string): Promise<PreparedBatch> {
    const started = performance.now()
    const pixels = await imagyxApi.prepareAiImages(batch.map((asset) => asset.id), batchId)
    const expectedBytes = batch.length * AI_IMAGE_BYTES
    if (pixels.byteLength !== expectedBytes) {
      throw new Error(`Batch RGB invalide: ${pixels.byteLength} octets au lieu de ${expectedBytes}`)
    }
    return { batch, pixels, prepareMs: performance.now() - started }
  }

  private nextBatchSize(batchSize: number, inferenceMs: number, count: number): number {
    const inferencePerImage = inferenceMs / Math.max(1, count)
    if (this.visionDevice === 'webgpu') {
      if (inferencePerImage < 500 && batchSize < MAX_BATCH_SIZE) {
        return Math.min(MAX_BATCH_SIZE, batchSize * 2)
      }
      if (inferencePerImage > 1_200 && batchSize > MIN_WEBGPU_BATCH_SIZE) {
        return Math.max(MIN_WEBGPU_BATCH_SIZE, Math.floor(batchSize / 2))
      }
      return batchSize
    }
    if (inferencePerImage < 300 && batchSize < MAX_BATCH_SIZE) {
      return Math.min(MAX_BATCH_SIZE, batchSize * 2)
    }
    if (inferencePerImage > 900 && batchSize > MIN_WASM_BATCH_SIZE) {
      return Math.max(MIN_WASM_BATCH_SIZE, Math.floor(batchSize / 2))
    }
    return batchSize
  }

  private async runPendingIndex(folderId?: string) {
    await ensureRustProfileListener()
    const pending = await imagyxApi.pendingImages(folderId)
    if (pending.length === 0) {
      this.patchStats({ stage: 'ready', current: 0, total: 0 })
      return
    }

    await this.prepare()
    const started = performance.now()
    const runId = Date.now().toString(36)
    let processed = 0
    let batchSize = this.visionDevice === 'webgpu' ? INITIAL_WEBGPU_BATCH_SIZE : INITIAL_WASM_BATCH_SIZE
    let batchCurrent = 0
    let decodeMs = 0
    let inferenceMs = 0
    let saveMs = 0
    this.patchStats({ stage: 'indexing', current: 0, total: pending.length, batchCurrent: 0,
      batchTotal: Math.ceil(pending.length / batchSize), batchSize, decodeMs: 0, inferenceMs: 0,
      saveMs: 0, elapsedMs: 0, imagesPerSecond: 0, averageMsPerImage: 0 })

    while (processed < pending.length) {
      if (this.paused) {
        this.patchStats({ stage: 'paused', current: processed, total: pending.length, imagesPerSecond: 0 })
        return
      }

      // Give a currently running Spotlight embedding a short head start, but
      // never let a stream of searches starve the image-indexing queue. Query
      // embeddings can include cold model loading, so this wait must be
      // bounded rather than tied exclusively to their completion.
      await this.waitForSearchPriority()

      const batchStarted = performance.now()
      const batch = pending.slice(processed, processed + batchSize)
      batchCurrent += 1
      const batchId = `${runId}-${batchCurrent}`
      this.patchStats({ stage: 'decoding', batchCurrent, batchSize: batch.length })
      const prepared = await this.prepareImageBatch(batch, batchId)

      this.patchStats({ stage: 'inference' })
      const measured = await profileUiWhile(
        this.visionWorker.infer(prepared.pixels, prepared.batch.length, batchId),
      )
      const result = measured.value
      decodeMs += prepared.prepareMs + result.profile.preprocessMs
      inferenceMs += result.profile.modelAwaitMs

      this.patchStats({ stage: 'saving' })
      const saveStarted = performance.now()
      await imagyxApi.saveEmbeddings(prepared.batch.map((asset, index) => ({
        imageId: asset.id,
        vector: result.vectors[index] ?? [],
      })))
      const currentSaveMs = performance.now() - saveStarted
      saveMs += currentSaveMs
      const batchTotalMs = performance.now() - batchStarted
      logIndexBatchProfile(result.profile, prepared.prepareMs, currentSaveMs, batchTotalMs, measured.ui)

      processed += prepared.batch.length
      batchSize = this.nextBatchSize(batchSize, result.profile.modelAwaitMs, prepared.batch.length)

      const elapsedMs = performance.now() - started
      const imagesPerSecond = processed / Math.max(elapsedMs / 1000, 0.001)
      this.patchStats({ stage: 'indexing', current: processed, total: pending.length, batchCurrent,
        batchTotal: batchCurrent + Math.ceil((pending.length - processed) / batchSize), batchSize,
        decodeMs: Math.round(decodeMs), inferenceMs: Math.round(inferenceMs), saveMs: Math.round(saveMs),
        elapsedMs: Math.round(elapsedMs), imagesPerSecond, averageMsPerImage: elapsedMs / processed })
    }
    this.patchStats({ stage: 'ready', current: pending.length, total: pending.length })
  }

  async embedQuery(query: string): Promise<EmbeddedQuery | undefined> {
    const trimmed = query.trim()
    if (!trimmed) return undefined
    if (currentWindow.label === 'spotlight') {
      return requestSemanticEmbedding(trimmed)
    }
    return this.embedLocalQuery(trimmed)
  }

  // Spotlight requests are already debounced and sequenced by its own window.
  // They must bypass the main window's search wrapper, which cancels stale
  // typing there and would otherwise cancel a newer delegated query as well.
  async embedDelegatedQuery(query: string): Promise<EmbeddedQuery | undefined> {
    const trimmed = query.trim()
    if (!trimmed) return undefined
    return this.embedLocalQuery(trimmed)
  }

  private async embedLocalQuery(query: string): Promise<EmbeddedQuery> {
    const cacheKey = query.toLocaleLowerCase('fr')
    const cached = this.queryCache.get(cacheKey)
    if (cached) {
      this.queryCache.delete(cacheKey)
      this.queryCache.set(cacheKey, cached)
      return cached
    }

    const inFlight = this.queryInFlight.get(cacheKey)
    if (inFlight) return inFlight

    const embedding = this.computeLocalQuery(query, cacheKey)
    this.queryInFlight.set(cacheKey, embedding)
    try {
      return await embedding
    } finally {
      if (this.queryInFlight.get(cacheKey) === embedding) this.queryInFlight.delete(cacheKey)
    }
  }

  private async computeLocalQuery(query: string, cacheKey: string): Promise<EmbeddedQuery> {

    this.activeSearchEmbeddings += 1
    try {
      const embeddingStartedAt = performance.now()
      const warmupStartedAt = performance.now()
      await this.prewarmText()
      const warmupMs = performance.now() - warmupStartedAt
      const plan = buildQueryPromptPlan(query)
      const positivePrompts = plan.positiveGroups.flatMap((group) => group.prompts)
      const texts = [
        ...positivePrompts,
        ...plan.negativePrompts,
      ]
      const tokenizationStartedAt = performance.now()
      const inputs = this.tokenizer(texts, {
        padding: 'max_length',
        truncation: true,
        max_length: 77,
      })
      const tokenizationMs = performance.now() - tokenizationStartedAt
      const inferenceStartedAt = performance.now()
      const output = await this.textModel(inputs)
      const inferenceMs = performance.now() - inferenceStartedAt
      const vectors = tensorRows(output.text_embeds)
      let positiveOffset = 0
      const positiveGroups = plan.positiveGroups.map((group) => {
        const groupVectors = vectors.slice(positiveOffset, positiveOffset + group.prompts.length)
        positiveOffset += group.prompts.length
        return { vectors: groupVectors, weight: group.weight }
      })
      const negativeEnd = positiveOffset + plan.negativePrompts.length
      const queryVector = combinePromptVectors(
        weightedPromptVectors(positiveGroups),
        vectors.slice(positiveOffset, negativeEnd),
        plan.negativeWeight,
      )
      if (!queryVector.length) throw new Error('Embedding de recherche vide')

      // Concepts are intentionally not part of the hot search path. They are
      // generated lazily by genericImageConcepts() only when the user asks for
      // an explanation of a concrete result.
      const embedded: EmbeddedQuery = { queryVector, concepts: [] }
      this.queryCache.set(cacheKey, embedded)
      while (this.queryCache.size > QUERY_CACHE_CAPACITY) {
        const oldest = this.queryCache.keys().next().value as string | undefined
        if (!oldest) break
        this.queryCache.delete(oldest)
      }
      perfLog('SemanticIA', 'text query embedding', performance.now() - embeddingStartedAt, {
        query,
        prompts: texts.length,
        conceptsDeferred: plan.conceptLabels.length,
        subjectLabels: plan.subjectLabels,
        detectedColors: plan.detectedColors,
        warmupMs,
        tokenizationMs,
        inferenceMs,
      })
      return embedded
    } finally {
      this.activeSearchEmbeddings -= 1
      if (this.activeSearchEmbeddings === 0) {
        for (const resolve of this.searchPriorityWaiters) resolve()
        this.searchPriorityWaiters.clear()
      }
    }
  }

  private waitForSearchPriority(): Promise<void> {
    if (this.activeSearchEmbeddings === 0) return Promise.resolve()
    return new Promise((resolve) => {
      let settled = false
      let timeout: number | undefined
      const release = () => {
        if (settled) return
        settled = true
        if (timeout !== undefined) window.clearTimeout(timeout)
        this.searchPriorityWaiters.delete(release)
        resolve()
      }

      this.searchPriorityWaiters.add(release)
      timeout = window.setTimeout(release, MAX_SEARCH_PRIORITY_WAIT_MS)
    })
  }

  async genericImageConcepts(): Promise<QueryConcept[]> {
    if (this.genericConcepts) return this.genericConcepts
    await this.prewarmText()
    const prompts = DEFAULT_IMAGE_LABELS.map((label) => `une photo de ${label}`)
    const inputs = this.tokenizer(prompts, { padding: 'max_length', truncation: true, max_length: 77 })
    const output = await this.textModel(inputs)
    const vectors = tensorRows(output.text_embeds)
    this.genericConcepts = DEFAULT_IMAGE_LABELS.flatMap((label, index) => {
      const vector = vectors[index]
      return vector ? [{ label, vector }] : []
    })
    return this.genericConcepts
  }

  private async ensureTextReady() {
    if (this.textModel && this.tokenizer) return
    if (this.textLoading) return this.textLoading
    this.textLoading = (async () => {
      const startedAt = performance.now()
      this.publishProgress({ stage: 'loading', message: `Préparation de ${MODEL_NAME}…`, currentBytes: 0, totalBytes: 0, currentFile: 0, totalFiles: 0 })
      this.patchStats({ stage: 'loading-text' })
      await this.ensureModelEnvironment()
      try {
        await this.loadTextForDevice('webgpu')
        this.patchStats({ stage: this.paused ? 'paused' : 'ready' })
      } catch (error) {
        this.textModel = null
        this.tokenizer = null
        await this.loadTextForDevice('wasm')
        this.patchStats({
          stage: this.paused ? 'paused' : 'ready',
          fallbackReason: `Recherche texte sur WASM: ${String(error)}`,
        })
      }
      this.publishProgress({ stage: 'ready', message: `${MODEL_NAME} prêt hors connexion.`, currentBytes: 0, totalBytes: 0, currentFile: 6, totalFiles: 6 })
      perfLog('SemanticIA', 'text model load', performance.now() - startedAt, {
        device: this.textModel ? 'ready' : 'unavailable',
      })
    })()
    try { await this.textLoading } finally { this.textLoading = null }
  }

  private async primeTextRuntime() {
    const inputs = this.tokenizer(['une photo'], {
      padding: 'max_length',
      truncation: true,
      max_length: 77,
    })
    const output = await this.textModel(inputs)
    tensorRows(output.text_embeds)
    this.textPrimed = true
  }

  private async loadTextForDevice(device: Device) {
    const { AutoTokenizer, CLIPTextModelWithProjection } = await loadTransformers()
    const options = this.modelOptions(device)
    ;[this.tokenizer, this.textModel] = await Promise.all([
      AutoTokenizer.from_pretrained(MODEL_ID, options),
      CLIPTextModelWithProjection.from_pretrained(MODEL_ID, options),
    ])
    this.textPrimed = false
  }

  private async ensureModelEnvironment(): Promise<string> {
    if (this.localModelPath) return this.localModelPath
    if (this.environmentLoading) return this.environmentLoading
    this.environmentLoading = (async () => {
      const [{ env }, modelRoot] = await Promise.all([
        loadTransformers(),
        imagyxApi.prepareLocalModel('mobileclip-s0'),
      ])
      const modelsDir = modelRoot.replace(/[\\/]+Xenova[\\/]mobileclip_s0$/, '')
      const localModelPath = `${imagyxApi.fileUrl(modelsDir).replace(/\/$/, '')}/`
      env.allowLocalModels = true
      env.allowRemoteModels = false
      env.localModelPath = localModelPath
      env.useBrowserCache = false
      configureLocalOnnxWasm(env)
      this.localModelPath = localModelPath
      return localModelPath
    })()
    try {
      return await this.environmentLoading
    } finally {
      this.environmentLoading = null
    }
  }

  private async loadVisionRuntime() {
    const start = performance.now()
    this.publishProgress({ stage: 'checking', message: `Vérification de ${MODEL_NAME}…`, currentBytes: 0, totalBytes: 0, currentFile: 0, totalFiles: 0 })
    this.patchStats({ stage: 'loading', modelName: MODEL_NAME, backendRequested: 'WebGPU' })
    const localModelPath = await this.ensureModelEnvironment()
    const ready = await this.visionWorker.initialize(MODEL_ID, localModelPath)
    this.visionDevice = ready.device
    this.visionReady = true
    this.patchStats({
      stage: this.paused ? 'paused' : 'ready',
      backendEffective: ready.backendEffective,
      accelerationActive: ready.accelerationActive,
      accelerationLabel: ready.accelerationLabel,
      fallbackReason: ready.fallbackReason,
    })
    this.publishProgress({ stage: 'ready', message: `${MODEL_NAME} prêt hors connexion.`, currentBytes: 0, totalBytes: 0, currentFile: 6, totalFiles: 6 })
    perfLog('SemanticIA', 'Vision Worker Model Load', performance.now() - start, {
      device: this.visionDevice,
      worker: true,
      initProfile: ready.initProfile,
    })
  }

  private modelOptions(device: Device) {
    return { device, dtype: 'fp32', local_files_only: true } as const
  }

  private publishProgress(progress: ModelDownloadProgress) {
    if (!this.callbacks) return
    this.callbacks.progress(progress)
    void imagyxApi.updateModelProgress(progress)
  }

  private patchStats(patch: Partial<RuntimeStats>) {
    const stage = patch.stage === 'ready' && this.textLoading
      ? 'text-ready'
      : patch.stage === 'ready' && this.loading
        ? 'vision-ready'
        : patch.stage
    this.stats = {
      ...this.stats,
      ...patch,
      ...(stage ? { stage } : {}),
      updatedAt: Date.now(),
    }
    if (!this.callbacks) return
    this.callbacks.stats(this.stats)
    void imagyxApi.updateRuntimeStats(this.stats)
  }
}

function tensorRows(tensor: any): number[][] {
  return (tensor.normalize().tolist() as number[][]).map((row) => Array.from(row, Number))
}

function defaultStats(): RuntimeStats {
  return { modelName: MODEL_NAME, stage: 'idle', backendRequested: 'WebGPU', backendEffective: 'En attente', accelerationActive: false,
    accelerationLabel: 'Non initialisée', batchSize: INITIAL_WEBGPU_BATCH_SIZE, current: 0, total: 0, batchCurrent: 0, batchTotal: 0,
    imagesPerSecond: 0, averageMsPerImage: 0, decodeMs: 0, inferenceMs: 0, saveMs: 0, elapsedMs: 0, systemCpuPercent: 0,
    processCpuPercent: 0, memoryUsedBytes: 0, memoryTotalBytes: 0, processMemoryBytes: 0, modelCacheBytes: 0, thumbnailCacheItems: 0, updatedAt: Date.now() }
}

export const semanticRuntime = new SemanticRuntime()

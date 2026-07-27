import { getCurrentWindow } from '@tauri-apps/api/window'
import { imagyxApi } from '../api/tauri'
import type { ImageAsset, ModelDownloadProgress, QueryConcept, RuntimeStats } from '../types'
import { perfLog } from '../utils'
import { requestSemanticEmbedding } from './semantic-channel'
import { buildQueryPromptPlan, combinePromptVectors } from './query-prompts'

const MODEL_ID = 'Xenova/mobileclip_s0'
const MODEL_NAME = 'MobileCLIP-S0'
const INDEX_PAUSED_KEY = 'imagyx.index-paused'
const INITIAL_WEBGPU_BATCH_SIZE = 8
const MIN_WEBGPU_BATCH_SIZE = 8
const INITIAL_WASM_BATCH_SIZE = 4
const MIN_WASM_BATCH_SIZE = 2
const MAX_BATCH_SIZE = 16
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

type Device = 'webgpu' | 'wasm'
type TransformersModule = typeof import('@huggingface/transformers')
type RuntimeCallbacks = {
  progress: (progress: ModelDownloadProgress) => void
  stats: (stats: RuntimeStats) => void
}

type PreparedBatch = {
  batch: ImageAsset[]
  inputs: any
  prepareMs: number
}

let transformersLoading: Promise<TransformersModule> | null = null

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

export interface EmbeddedQuery {
  queryVector: number[]
  concepts: QueryConcept[]
}

class SemanticRuntime {
  private visionModel: any = null
  private textModel: any = null
  private processor: any = null
  private tokenizer: any = null
  private device: Device = 'webgpu'
  private loading: Promise<void> | null = null
  private textLoading: Promise<void> | null = null
  private textPriming: Promise<void> | null = null
  private textPrimed = false
  private environmentLoading: Promise<void> | null = null
  private environmentReady = false
  private indexing: Promise<void> | null = null
  private genericConcepts: QueryConcept[] | null = null
  private callbacks: RuntimeCallbacks | null = null
  private queryCache = new Map<string, EmbeddedQuery>()
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
    if (this.visionModel && this.processor) return
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
    if (this.indexing) return this.indexing
    this.indexing = this.runPendingIndex(folderId)
    try { await this.indexing } finally { this.indexing = null }
  }

  private async prepareImageBatch(batch: ImageAsset[], RawImage: any): Promise<PreparedBatch> {
    const started = performance.now()
    const packed = await imagyxApi.prepareAiImages(batch.map((asset) => asset.id))
    const expectedBytes = batch.length * AI_IMAGE_BYTES
    if (packed.byteLength !== expectedBytes) {
      throw new Error(`Batch RGB invalide: ${packed.byteLength} octets au lieu de ${expectedBytes}`)
    }

    const images = batch.map((_, index) => new RawImage(
      new Uint8Array(packed, index * AI_IMAGE_BYTES, AI_IMAGE_BYTES),
      AI_IMAGE_EDGE,
      AI_IMAGE_EDGE,
      AI_IMAGE_CHANNELS,
    ))
    const inputs = await this.processor(images.length === 1 ? images[0] : images)
    return { batch, inputs, prepareMs: performance.now() - started }
  }

  private nextBatchSize(batchSize: number, inferenceMs: number, count: number): number {
    const inferencePerImage = inferenceMs / Math.max(1, count)
    if (this.device === 'webgpu') {
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
    const pending = await imagyxApi.pendingImages(folderId)
    if (pending.length === 0) {
      this.patchStats({ stage: 'ready', current: 0, total: 0 })
      return
    }

    await this.prepare()
    const { RawImage } = await loadTransformers()
    const started = performance.now()
    let processed = 0
    let batchSize = this.device === 'webgpu' ? INITIAL_WEBGPU_BATCH_SIZE : INITIAL_WASM_BATCH_SIZE
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

      const batch = pending.slice(processed, processed + batchSize)
      batchCurrent += 1
      this.patchStats({ stage: 'decoding', batchCurrent, batchSize: batch.length })
      const prepared = await this.prepareImageBatch(batch, RawImage)
      decodeMs += prepared.prepareMs

      this.patchStats({ stage: 'inference' })
      const inferenceStarted = performance.now()
      const output = await this.visionModel(prepared.inputs)
      const currentInferenceMs = performance.now() - inferenceStarted
      const vectors = tensorRows(output.image_embeds)
      inferenceMs += currentInferenceMs

      this.patchStats({ stage: 'saving' })
      const saveStarted = performance.now()
      await imagyxApi.saveEmbeddings(prepared.batch.map((asset, index) => ({
        imageId: asset.id,
        vector: vectors[index] ?? [],
      })))
      saveMs += performance.now() - saveStarted
      processed += prepared.batch.length
      batchSize = this.nextBatchSize(batchSize, currentInferenceMs, prepared.batch.length)

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

  private async embedLocalQuery(query: string): Promise<EmbeddedQuery> {
    const cacheKey = query.toLocaleLowerCase('fr')
    const cached = this.queryCache.get(cacheKey)
    if (cached) {
      this.queryCache.delete(cacheKey)
      this.queryCache.set(cacheKey, cached)
      return cached
    }

    await this.prewarmText()
    const plan = buildQueryPromptPlan(query)
    const texts = [
      ...plan.positivePrompts,
      ...plan.negativePrompts,
      ...plan.conceptPrompts,
    ]
    const inputs = this.tokenizer(texts, {
      padding: 'max_length',
      truncation: true,
      max_length: 77,
    })
    const output = await this.textModel(inputs)
    const vectors = tensorRows(output.text_embeds)
    const positiveEnd = plan.positivePrompts.length
    const negativeEnd = positiveEnd + plan.negativePrompts.length
    const queryVector = combinePromptVectors(
      vectors.slice(0, positiveEnd),
      vectors.slice(positiveEnd, negativeEnd),
      plan.negativeWeight,
    )
    if (!queryVector.length) throw new Error('Embedding de recherche vide')

    const concepts = plan.conceptLabels.flatMap((label, index) => {
      const vector = vectors[negativeEnd + index]
      return vector ? [{ label, vector }] : []
    })
    const embedded = { queryVector, concepts }
    this.queryCache.set(cacheKey, embedded)
    while (this.queryCache.size > QUERY_CACHE_CAPACITY) {
      const oldest = this.queryCache.keys().next().value as string | undefined
      if (!oldest) break
      this.queryCache.delete(oldest)
    }
    return embedded
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
      this.patchStats({ stage: 'loading-text' })
      await this.ensureModelEnvironment()
      try {
        this.device = 'webgpu'
        await this.loadTextForDevice('webgpu')
        this.patchStats({
          stage: this.paused ? 'paused' : 'ready',
          backendEffective: 'Transformers.js · WebGPU',
          accelerationActive: true,
          accelerationLabel: 'GPU WebGPU actif',
          fallbackReason: undefined,
        })
      } catch (error) {
        this.device = 'wasm'
        this.textModel = null
        this.tokenizer = null
        await this.loadTextForDevice('wasm')
        this.patchStats({
          stage: this.paused ? 'paused' : 'ready',
          backendEffective: 'Transformers.js · WASM',
          accelerationActive: false,
          accelerationLabel: 'CPU WASM',
          fallbackReason: `WebGPU indisponible: ${String(error)}`,
        })
      }
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

  private async ensureModelEnvironment() {
    if (this.environmentReady) return
    if (this.environmentLoading) return this.environmentLoading
    this.environmentLoading = (async () => {
      const [{ env }, modelRoot] = await Promise.all([
        loadTransformers(),
        imagyxApi.prepareLocalModel('mobileclip-s0'),
      ])
      const modelsDir = modelRoot.replace(/[\\/]+Xenova[\\/]mobileclip_s0$/, '')
      env.allowLocalModels = true
      env.allowRemoteModels = false
      env.localModelPath = `${imagyxApi.fileUrl(modelsDir).replace(/\/$/, '')}/`
      env.useBrowserCache = false
      this.environmentReady = true
    })()
    try { await this.environmentLoading } finally { this.environmentLoading = null }
  }

  private async loadVisionRuntime() {
    const start = performance.now()
    this.publishProgress({ stage: 'checking', message: `Vérification de ${MODEL_NAME}…`, currentBytes: 0, totalBytes: 0, currentFile: 0, totalFiles: 0 })
    this.patchStats({ stage: 'loading', modelName: MODEL_NAME, backendRequested: 'WebGPU' })
    await this.ensureModelEnvironment()
    try {
      this.device = 'webgpu'
      await this.loadVisionForDevice('webgpu')
      this.patchStats({ stage: this.paused ? 'paused' : 'ready', backendEffective: 'Transformers.js · WebGPU', accelerationActive: true, accelerationLabel: 'GPU WebGPU actif', fallbackReason: undefined })
    } catch (error) {
      this.device = 'wasm'
      this.visionModel = null
      this.processor = null
      await this.loadVisionForDevice('wasm')
      this.patchStats({ stage: this.paused ? 'paused' : 'ready', backendEffective: 'Transformers.js · WASM', accelerationActive: false, accelerationLabel: 'CPU WASM', fallbackReason: `WebGPU indisponible: ${String(error)}` })
    }
    this.publishProgress({ stage: 'ready', message: `${MODEL_NAME} prêt hors connexion.`, currentBytes: 0, totalBytes: 0, currentFile: 6, totalFiles: 6 })
    perfLog('SemanticIA', 'Vision Model Load', performance.now() - start, { device: this.device })
  }

  private async loadVisionForDevice(device: Device) {
    const {
      AutoProcessor,
      CLIPVisionModelWithProjection,
      RawImage,
    } = await loadTransformers()
    const options = this.modelOptions(device)
    ;[this.processor, this.visionModel] = await Promise.all([
      AutoProcessor.from_pretrained(MODEL_ID, options),
      CLIPVisionModelWithProjection.from_pretrained(MODEL_ID, options),
    ])
    const blank = new RawImage(new Uint8ClampedArray(224 * 224 * 4), 224, 224, 4)
    await this.visionModel(await this.processor(blank))
  }

  private modelOptions(device: Device = this.device) {
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

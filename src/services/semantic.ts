import {
  AutoProcessor,
  AutoTokenizer,
  CLIPTextModelWithProjection,
  CLIPVisionModelWithProjection,
  RawImage,
  env,
} from '@huggingface/transformers'
import { imagyxApi } from '../api/tauri'
import type { ModelDownloadProgress, QueryConcept, RuntimeStats } from '../types'

const MODEL_ID = 'Xenova/mobileclip_s0'
const MODEL_NAME = 'MobileCLIP-S0'
const INITIAL_BATCH_SIZE = 4
const MAX_BATCH_SIZE = 16
const DEFAULT_IMAGE_LABELS = [
  'personne', 'femme', 'homme', 'enfant', 'groupe de personnes', 'visage',
  'animal', 'chien', 'chat', 'oiseau', 'voiture', 'vélo', 'bâtiment', 'maison',
  'ville', 'rue', 'intérieur', 'extérieur', 'nature', 'forêt', 'montagne', 'plage',
  'mer', 'ciel', 'nuit', 'coucher de soleil', 'nourriture', 'fleur', 'texte',
  'capture d’écran', 'dessin', 'illustration', 'portrait', 'paysage', 'objet rouge',
  'objet bleu', 'objet vert', 'objet jaune', 'image sombre', 'image lumineuse',
]

type Device = 'webgpu' | 'wasm'
type RuntimeCallbacks = {
  progress: (progress: ModelDownloadProgress) => void
  stats: (stats: RuntimeStats) => void
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
  private genericConcepts: QueryConcept[] | null = null
  private callbacks: RuntimeCallbacks | null = null
  private stats: RuntimeStats = defaultStats()

  setCallbacks(callbacks: RuntimeCallbacks) {
    this.callbacks = callbacks
    callbacks.stats(this.stats)
  }

  async prepare() {
    if (this.visionModel && this.processor) return
    if (this.loading) return this.loading
    this.loading = this.loadVisionRuntime()
    try { await this.loading } finally { this.loading = null }
  }

  async indexPending(folderId?: string) {
    await this.prepare()
    const pending = await imagyxApi.pendingImages(folderId)
    if (pending.length === 0) return
    const started = performance.now()
    let processed = 0
    let batchSize = INITIAL_BATCH_SIZE
    let batchCurrent = 0
    let decodeMs = 0
    let inferenceMs = 0
    let saveMs = 0
    this.patchStats({ stage: 'indexing', current: 0, total: pending.length, batchCurrent: 0,
      batchTotal: Math.ceil(pending.length / batchSize), batchSize, decodeMs: 0, inferenceMs: 0,
      saveMs: 0, elapsedMs: 0, imagesPerSecond: 0, averageMsPerImage: 0 })

    while (processed < pending.length) {
      const batch = pending.slice(processed, processed + batchSize)
      batchCurrent += 1
      this.patchStats({ stage: 'decoding', batchCurrent, batchSize: batch.length })
      const decodeStarted = performance.now()
      const preparedPaths = await imagyxApi.prepareAiImages(batch.map((asset) => asset.id))
      const images = await Promise.all(preparedPaths.map((path) => RawImage.read(imagyxApi.fileUrl(path))))
      const imageInputs = await this.processor(images.length === 1 ? images[0] : images)
      decodeMs += performance.now() - decodeStarted
      this.patchStats({ stage: 'inference' })
      const inferenceStarted = performance.now()
      const output = await this.visionModel(imageInputs)
      const vectors = tensorRows(output.image_embeds)
      inferenceMs += performance.now() - inferenceStarted
      this.patchStats({ stage: 'saving' })
      const saveStarted = performance.now()
      await imagyxApi.saveEmbeddings(batch.map((asset, index) => ({ imageId: asset.id, vector: vectors[index] ?? [] })))
      saveMs += performance.now() - saveStarted
      processed += batch.length
      const elapsedMs = performance.now() - started
      const imagesPerSecond = processed / Math.max(elapsedMs / 1000, 0.001)
      const averageImageMs = (decodeMs + inferenceMs + saveMs) / processed
      if (averageImageMs < 90 && batchSize < MAX_BATCH_SIZE) batchSize = Math.min(MAX_BATCH_SIZE, batchSize * 2)
      if (averageImageMs > 450 && batchSize > 2) batchSize = Math.max(2, Math.floor(batchSize / 2))
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
    await this.ensureTextReady()
    const labels = queryConceptLabels(trimmed)
    const texts = [trimmed, ...labels]
    const inputs = this.tokenizer(texts, { padding: 'max_length', truncation: true, max_length: 77 })
    const output = await this.textModel(inputs)
    const vectors = tensorRows(output.text_embeds)
    const queryVector = vectors[0]
    if (!queryVector) throw new Error('Embedding de recherche vide')
    return { queryVector, concepts: labels.flatMap((label, index) => vectors[index + 1] ? [{ label, vector: vectors[index + 1] }] : []) }
  }

  async genericImageConcepts(): Promise<QueryConcept[]> {
    if (this.genericConcepts) return this.genericConcepts
    await this.ensureTextReady()
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
    await this.prepare()
    if (this.textModel && this.tokenizer) return
    if (this.textLoading) return this.textLoading
    this.textLoading = (async () => {
      this.patchStats({ stage: 'loading-text' })
      const options = this.modelOptions()
      ;[this.tokenizer, this.textModel] = await Promise.all([
        AutoTokenizer.from_pretrained(MODEL_ID, options),
        CLIPTextModelWithProjection.from_pretrained(MODEL_ID, options),
      ])
      this.patchStats({ stage: 'ready' })
    })()
    try { await this.textLoading } finally { this.textLoading = null }
  }

  private async loadVisionRuntime() {
    this.publishProgress({ stage: 'checking', message: `Vérification de ${MODEL_NAME}…`, currentBytes: 0, totalBytes: 0, currentFile: 0, totalFiles: 0 })
    this.patchStats({ stage: 'loading', modelName: MODEL_NAME, backendRequested: 'WebGPU' })
    const modelRoot = await imagyxApi.prepareLocalModel('mobileclip-s0')
    const modelsDir = modelRoot.replace(/[\\/]+Xenova[\\/]mobileclip_s0$/, '')
    env.allowLocalModels = true
    env.allowRemoteModels = false
    env.localModelPath = `${imagyxApi.fileUrl(modelsDir).replace(/\/$/, '')}/`
    env.useBrowserCache = false
    try {
      this.device = 'webgpu'
      await this.loadVisionForDevice('webgpu')
      this.patchStats({ stage: 'ready', backendEffective: 'Transformers.js · WebGPU', accelerationActive: true, accelerationLabel: 'GPU WebGPU actif', fallbackReason: undefined })
    } catch (error) {
      this.device = 'wasm'; this.visionModel = null; this.processor = null
      await this.loadVisionForDevice('wasm')
      this.patchStats({ stage: 'ready', backendEffective: 'Transformers.js · WASM', accelerationActive: false, accelerationLabel: 'CPU WASM', fallbackReason: `WebGPU indisponible: ${String(error)}` })
    }
    this.publishProgress({ stage: 'ready', message: `${MODEL_NAME} prêt hors connexion.`, currentBytes: 0, totalBytes: 0, currentFile: 6, totalFiles: 6 })
  }

  private async loadVisionForDevice(device: Device) {
    const options = this.modelOptions(device)
    ;[this.processor, this.visionModel] = await Promise.all([
      AutoProcessor.from_pretrained(MODEL_ID, options),
      CLIPVisionModelWithProjection.from_pretrained(MODEL_ID, options),
    ])
    const blank = new RawImage(new Uint8ClampedArray(224 * 224 * 4), 224, 224, 4)
    await this.visionModel(await this.processor(blank))
  }

  private modelOptions(device: Device = this.device) { return { device, dtype: 'fp32', local_files_only: true } as const }
  private publishProgress(progress: ModelDownloadProgress) { this.callbacks?.progress(progress); void imagyxApi.updateModelProgress(progress) }
  private patchStats(patch: Partial<RuntimeStats>) { this.stats = { ...this.stats, ...patch, updatedAt: Date.now() }; this.callbacks?.stats(this.stats); void imagyxApi.updateRuntimeStats(this.stats) }
}

function queryConceptLabels(query: string): string[] {
  const seen = new Set<string>()
  return query.toLocaleLowerCase('fr').split(/[^\p{L}\p{N}-]+/u).map((word) => word.trim())
    .filter((word) => word.length >= 2).filter((word) => !seen.has(word) && Boolean(seen.add(word))).slice(0, 6)
}

function tensorRows(tensor: any): number[][] {
  return (tensor.normalize().tolist() as number[][]).map((row) => Array.from(row, Number))
}

function defaultStats(): RuntimeStats {
  return { modelName: MODEL_NAME, stage: 'idle', backendRequested: 'WebGPU', backendEffective: 'En attente', accelerationActive: false,
    accelerationLabel: 'Non initialisée', batchSize: INITIAL_BATCH_SIZE, current: 0, total: 0, batchCurrent: 0, batchTotal: 0,
    imagesPerSecond: 0, averageMsPerImage: 0, decodeMs: 0, inferenceMs: 0, saveMs: 0, elapsedMs: 0, systemCpuPercent: 0,
    processCpuPercent: 0, memoryUsedBytes: 0, memoryTotalBytes: 0, processMemoryBytes: 0, modelCacheBytes: 0, thumbnailCacheItems: 0, updatedAt: Date.now() }
}

export const semanticRuntime = new SemanticRuntime()

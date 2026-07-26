import {
  AutoProcessor,
  AutoTokenizer,
  CLIPTextModelWithProjection,
  CLIPVisionModelWithProjection,
  RawImage,
  env,
} from '@huggingface/transformers'
import { imagyxApi } from '../api/tauri'
import type { ModelDownloadProgress, RuntimeStats } from '../types'

const MODEL_ID = 'Xenova/mobileclip_s0'
const MODEL_NAME = 'MobileCLIP-S0'
const INITIAL_BATCH_SIZE = 4
const MAX_BATCH_SIZE = 16

type Device = 'webgpu' | 'wasm'
type RuntimeCallbacks = {
  progress: (progress: ModelDownloadProgress) => void
  stats: (stats: RuntimeStats) => void
}

class SemanticRuntime {
  private visionModel: any = null
  private textModel: any = null
  private processor: any = null
  private tokenizer: any = null
  private device: Device = 'webgpu'
  private loading: Promise<void> | null = null
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
    try {
      await this.loading
    } finally {
      this.loading = null
    }
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

    this.patchStats({
      stage: 'indexing', current: 0, total: pending.length, batchCurrent: 0,
      batchTotal: Math.ceil(pending.length / batchSize), batchSize,
      decodeMs: 0, inferenceMs: 0, saveMs: 0, elapsedMs: 0,
      imagesPerSecond: 0, averageMsPerImage: 0,
    })

    while (processed < pending.length) {
      const batch = pending.slice(processed, processed + batchSize)
      batchCurrent += 1
      this.patchStats({ stage: 'decoding', batchCurrent, batchSize: batch.length })
      const decodeStarted = performance.now()
      const images = await Promise.all(batch.map((asset) => RawImage.read(imagyxApi.fileUrl(asset.path))))
      const imageInputs = await this.processor(images.length === 1 ? images[0] : images)
      decodeMs += performance.now() - decodeStarted

      this.patchStats({ stage: 'inference' })
      const inferenceStarted = performance.now()
      const output = await this.visionModel(imageInputs)
      const vectors = tensorRows(output.image_embeds)
      inferenceMs += performance.now() - inferenceStarted

      this.patchStats({ stage: 'saving' })
      const saveStarted = performance.now()
      await imagyxApi.saveEmbeddings(
        batch.map((asset, index) => ({ imageId: asset.id, vector: vectors[index] ?? [] })),
      )
      saveMs += performance.now() - saveStarted
      processed += batch.length

      const elapsedMs = performance.now() - started
      const imagesPerSecond = processed / Math.max(elapsedMs / 1000, 0.001)
      const averageBatchMs = (decodeMs + inferenceMs + saveMs) / processed
      if (averageBatchMs < 90 && batchSize < MAX_BATCH_SIZE) batchSize = Math.min(MAX_BATCH_SIZE, batchSize * 2)
      if (averageBatchMs > 450 && batchSize > 2) batchSize = Math.max(2, Math.floor(batchSize / 2))

      this.patchStats({
        stage: 'indexing', current: processed, total: pending.length, batchCurrent,
        batchTotal: batchCurrent + Math.ceil((pending.length - processed) / batchSize),
        batchSize, decodeMs: Math.round(decodeMs), inferenceMs: Math.round(inferenceMs),
        saveMs: Math.round(saveMs), elapsedMs: Math.round(elapsedMs), imagesPerSecond,
        averageMsPerImage: elapsedMs / processed,
      })
    }

    this.patchStats({ stage: 'ready', current: pending.length, total: pending.length })
  }

  async embedText(query: string): Promise<number[] | undefined> {
    if (!query.trim()) return undefined
    await this.prepare()
    if (!this.textModel || !this.tokenizer) {
      this.patchStats({ stage: 'loading-text' })
      const options = this.modelOptions()
      ;[this.tokenizer, this.textModel] = await Promise.all([
        AutoTokenizer.from_pretrained(MODEL_ID, options),
        CLIPTextModelWithProjection.from_pretrained(MODEL_ID, options),
      ])
      this.patchStats({ stage: 'ready' })
    }
    const inputs = this.tokenizer([query], { padding: 'max_length', truncation: true })
    const output = await this.textModel(inputs)
    return tensorRows(output.text_embeds)[0]
  }

  private async loadVisionRuntime() {
    this.publishProgress({
      stage: 'checking', message: 'Vérification du cache MobileCLIP-S0…',
      currentBytes: 0, totalBytes: 0, currentFile: 0, totalFiles: 0,
    })
    this.patchStats({ stage: 'loading', backendRequested: 'WebGPU' })

    const modelRoot = await imagyxApi.prepareLocalModel()
    const modelsDir = modelRoot.replace(/[\\/]+Xenova[\\/]mobileclip_s0$/, '')
    env.allowLocalModels = true
    env.allowRemoteModels = false
    env.localModelPath = `${imagyxApi.fileUrl(modelsDir).replace(/\/$/, '')}/`
    env.useBrowserCache = false

    try {
      this.device = 'webgpu'
      await this.loadVisionForDevice('webgpu')
      this.patchStats({
        stage: 'ready', backendEffective: 'Transformers.js · WebGPU',
        accelerationActive: true, accelerationLabel: 'GPU WebGPU actif', fallbackReason: undefined,
      })
    } catch (error) {
      this.device = 'wasm'
      this.visionModel = null
      this.processor = null
      await this.loadVisionForDevice('wasm')
      this.patchStats({
        stage: 'ready', backendEffective: 'Transformers.js · WASM',
        accelerationActive: false, accelerationLabel: 'CPU WASM',
        fallbackReason: `WebGPU indisponible: ${String(error)}`,
      })
    }

    this.publishProgress({
      stage: 'ready', message: `${MODEL_NAME} prêt hors connexion avec ${this.device === 'webgpu' ? 'WebGPU' : 'WASM'}.`,
      currentBytes: 0, totalBytes: 0, currentFile: 7, totalFiles: 7,
    })
  }

  private async loadVisionForDevice(device: Device) {
    const options = this.modelOptions(device)
    ;[this.processor, this.visionModel] = await Promise.all([
      AutoProcessor.from_pretrained(MODEL_ID, options),
      CLIPVisionModelWithProjection.from_pretrained(MODEL_ID, options),
    ])
    const blank = new RawImage(new Uint8ClampedArray(224 * 224 * 4), 224, 224, 4)
    const input = await this.processor(blank)
    await this.visionModel(input)
  }

  private modelOptions(device: Device = this.device) {
    return { device, dtype: 'fp32', local_files_only: true } as const
  }

  private publishProgress(progress: ModelDownloadProgress) {
    this.callbacks?.progress(progress)
    void imagyxApi.updateModelProgress(progress)
  }

  private patchStats(patch: Partial<RuntimeStats>) {
    this.stats = { ...this.stats, ...patch, updatedAt: Date.now() }
    this.callbacks?.stats(this.stats)
    void imagyxApi.updateRuntimeStats(this.stats)
  }
}

function tensorRows(tensor: any): number[][] {
  const normalized = tensor.normalize()
  const rows = normalized.tolist() as number[][]
  return rows.map((row) => Array.from(row, Number))
}

function defaultStats(): RuntimeStats {
  return {
    modelName: MODEL_NAME, stage: 'idle', backendRequested: 'WebGPU',
    backendEffective: 'En attente', accelerationActive: false,
    accelerationLabel: 'Non initialisée', batchSize: INITIAL_BATCH_SIZE,
    current: 0, total: 0, batchCurrent: 0, batchTotal: 0,
    imagesPerSecond: 0, averageMsPerImage: 0, decodeMs: 0,
    inferenceMs: 0, saveMs: 0, elapsedMs: 0, systemCpuPercent: 0,
    processCpuPercent: 0, memoryUsedBytes: 0, memoryTotalBytes: 0,
    processMemoryBytes: 0, modelCacheBytes: 0, thumbnailCacheItems: 0,
    updatedAt: Date.now(),
  }
}

export const semanticRuntime = new SemanticRuntime()

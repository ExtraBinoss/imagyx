import {
  AutoProcessor,
  CLIPVisionModelWithProjection,
  RawImage,
  env,
} from '@huggingface/transformers'
import { configureLocalOnnxWasm } from '../services/onnx-wasm-assets'

const IMAGE_EDGE = 224
const IMAGE_CHANNELS = 3
const IMAGE_BYTES = IMAGE_EDGE * IMAGE_EDGE * IMAGE_CHANNELS

type Device = 'webgpu' | 'wasm'

type InitProfile = {
  processorLoadMs: number
  modelLoadMs: number
  warmupPreprocessMs: number
  warmupModelMs: number
  totalMs: number
  hardwareConcurrency: number
  webGpuExposed: boolean
}

type InitRequest = {
  type: 'init'
  requestId: number
  modelId: string
  localModelPath: string
}

type InferRequest = {
  type: 'infer'
  requestId: number
  batchId: string
  sentAtEpochMs: number
  pixels: ArrayBuffer
  count: number
  receivedAtEpochMs?: number
  receivedAtPerfMs?: number
}

type WorkerRequest = InitRequest | InferRequest

type ReadyResponse = {
  type: 'ready'
  requestId: number
  device: Device
  backendEffective: string
  accelerationActive: boolean
  accelerationLabel: string
  fallbackReason?: string
  initProfile: InitProfile
}

type WorkerTimingProfile = {
  batchId: string
  inputBytes: number
  outputBytes: number
  workerReceivedAtEpochMs: number
  workerRespondedAtEpochMs: number
  transferToWorkerMs: number
  queueWaitMs: number
  rawImageWrapMs: number
  preprocessMs: number
  modelAwaitMs: number
  normalizeMs: number
  readbackToListMs: number
  flattenMs: number
  workerTotalMs: number
}

type ResultResponse = {
  type: 'result'
  requestId: number
  vectors: ArrayBuffer
  count: number
  dimension: number
  profile: WorkerTimingProfile
}

type ErrorResponse = {
  type: 'error'
  requestId: number
  message: string
}

type WorkerResponse = ReadyResponse | ResultResponse | ErrorResponse

type WorkerScope = {
  onmessage: ((event: MessageEvent<WorkerRequest>) => void) | null
  postMessage: (message: WorkerResponse, transfer?: Transferable[]) => void
}

const scope = globalThis as unknown as WorkerScope
let processor: any = null
let visionModel: any = null
let device: Device = 'webgpu'
let initialization: Promise<Omit<ReadyResponse, 'type' | 'requestId'>> | null = null
let inferenceQueue: Promise<void> = Promise.resolve()

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function post(message: WorkerResponse, transfer: Transferable[] = []) {
  scope.postMessage(message, transfer)
}

function roundMs(value: number): number {
  return Math.round(value * 100) / 100
}

function logWorkerBatch(profile: WorkerTimingProfile, count: number, dimension: number) {
  const perImageMs = profile.workerTotalMs / Math.max(1, count)
  console.groupCollapsed(
    `[Imagyx][VisionWorker][${profile.batchId}] ${count} img · ${roundMs(profile.workerTotalMs)} ms worker · ${roundMs(perImageMs)} ms/img`,
  )
  console.table({
    transfer_main_to_worker_ms: roundMs(profile.transferToWorkerMs),
    worker_queue_wait_ms: roundMs(profile.queueWaitMs),
    raw_image_views_ms: roundMs(profile.rawImageWrapMs),
    transformers_preprocess_ms: roundMs(profile.preprocessMs),
    model_await_wall_ms: roundMs(profile.modelAwaitMs),
    tensor_normalize_ms: roundMs(profile.normalizeMs),
    gpu_readback_to_list_ms: roundMs(profile.readbackToListMs),
    flatten_float32_ms: roundMs(profile.flattenMs),
    worker_total_ms: roundMs(profile.workerTotalMs),
  })
  console.log('Payload', {
    count,
    dimension,
    inputBytes: profile.inputBytes,
    outputBytes: profile.outputBytes,
    device,
    note: 'model_await_wall_ms est un temps mur JS autour du backend. gpu_readback_to_list_ms mesure la synchronisation/readback visible depuis Transformers.js.',
  })
  console.groupEnd()
}

async function resetVisionRuntime() {
  try {
    await visionModel?.dispose?.()
  } catch {
    // A partially initialized backend may not be disposable.
  }
  visionModel = null
  processor = null
}

async function loadForDevice(modelId: string, target: Device): Promise<Omit<InitProfile, 'totalMs' | 'hardwareConcurrency' | 'webGpuExposed'>> {
  const options = { device: target, dtype: 'fp32', local_files_only: true } as const
  let processorLoadMs = 0
  let modelLoadMs = 0

  await Promise.all([
    (async () => {
      const started = performance.now()
      processor = await AutoProcessor.from_pretrained(modelId, options)
      processorLoadMs = performance.now() - started
    })(),
    (async () => {
      const started = performance.now()
      visionModel = await CLIPVisionModelWithProjection.from_pretrained(modelId, options)
      modelLoadMs = performance.now() - started
    })(),
  ])

  const blank = new RawImage(
    new Uint8Array(IMAGE_BYTES),
    IMAGE_EDGE,
    IMAGE_EDGE,
    IMAGE_CHANNELS,
  )
  const warmupPreprocessStarted = performance.now()
  const warmupInputs = await processor(blank)
  const warmupPreprocessMs = performance.now() - warmupPreprocessStarted
  const warmupModelStarted = performance.now()
  await visionModel(warmupInputs)
  const warmupModelMs = performance.now() - warmupModelStarted

  return { processorLoadMs, modelLoadMs, warmupPreprocessMs, warmupModelMs }
}

async function initialize(request: InitRequest) {
  if (!initialization) {
    initialization = (async () => {
      const totalStarted = performance.now()
      env.allowLocalModels = true
      env.allowRemoteModels = false
      env.localModelPath = request.localModelPath
      env.useBrowserCache = false
      configureLocalOnnxWasm(env)

      let fallbackReason: string | undefined
      let loadProfile: Omit<InitProfile, 'totalMs' | 'hardwareConcurrency' | 'webGpuExposed'>
      try {
        device = 'webgpu'
        loadProfile = await loadForDevice(request.modelId, device)
      } catch (webGpuError) {
        fallbackReason = `WebGPU worker indisponible: ${errorMessage(webGpuError)}`
        await resetVisionRuntime()
        device = 'wasm'
        loadProfile = await loadForDevice(request.modelId, device)
      }

      const initProfile: InitProfile = {
        ...loadProfile,
        totalMs: performance.now() - totalStarted,
        hardwareConcurrency: navigator.hardwareConcurrency || 1,
        webGpuExposed: 'gpu' in navigator,
      }
      const ready = {
        device,
        backendEffective: device === 'webgpu'
          ? 'Transformers.js Worker · WebGPU'
          : 'Transformers.js Worker · WASM',
        accelerationActive: device === 'webgpu',
        accelerationLabel: device === 'webgpu' ? 'GPU WebGPU · worker' : 'CPU WASM · worker',
        ...(fallbackReason ? { fallbackReason } : {}),
        initProfile,
      }
      console.info('[Imagyx][VisionWorker] initialization profile', ready)
      return ready
    })().catch((error) => {
      initialization = null
      throw error
    })
  }
  return initialization
}

async function runInference(request: InferRequest): Promise<ResultResponse> {
  if (!initialization || !processor || !visionModel) {
    throw new Error('Le worker vision n’est pas initialisé')
  }
  if (!Number.isInteger(request.count) || request.count <= 0) {
    throw new Error(`Nombre d’images invalide: ${request.count}`)
  }

  const expectedBytes = request.count * IMAGE_BYTES
  if (request.pixels.byteLength !== expectedBytes) {
    throw new Error(
      `Batch RGB invalide: ${request.pixels.byteLength} octets au lieu de ${expectedBytes}`,
    )
  }

  const workerStarted = performance.now()
  const workerReceivedAtEpochMs = request.receivedAtEpochMs ?? Date.now()
  const receivedAtPerfMs = request.receivedAtPerfMs ?? workerStarted
  const transferToWorkerMs = Math.max(0, workerReceivedAtEpochMs - request.sentAtEpochMs)
  const queueWaitMs = Math.max(0, workerStarted - receivedAtPerfMs)

  const rawImageStarted = performance.now()
  const images = Array.from({ length: request.count }, (_, index) => new RawImage(
    new Uint8Array(request.pixels, index * IMAGE_BYTES, IMAGE_BYTES),
    IMAGE_EDGE,
    IMAGE_EDGE,
    IMAGE_CHANNELS,
  ))
  const rawImageWrapMs = performance.now() - rawImageStarted

  const preprocessStarted = performance.now()
  const inputs = await processor(images.length === 1 ? images[0] : images)
  const preprocessMs = performance.now() - preprocessStarted

  const inferenceStarted = performance.now()
  const output = await visionModel(inputs)
  const modelAwaitMs = performance.now() - inferenceStarted

  const normalizeStarted = performance.now()
  const normalized = output.image_embeds.normalize()
  const normalizeMs = performance.now() - normalizeStarted

  const readbackStarted = performance.now()
  const rows = normalized.tolist() as number[][]
  const readbackToListMs = performance.now() - readbackStarted

  if (rows.length !== request.count) {
    throw new Error(`Le modèle a retourné ${rows.length} vecteurs pour ${request.count} images`)
  }
  const dimension = rows[0]?.length ?? 0
  if (dimension <= 0 || rows.some((row) => row.length !== dimension)) {
    throw new Error('Dimensions de vecteurs incohérentes')
  }

  const flattenStarted = performance.now()
  const flat = new Float32Array(request.count * dimension)
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index]
    if (row) flat.set(row, index * dimension)
  }
  const flattenMs = performance.now() - flattenStarted
  const vectors = flat.buffer as ArrayBuffer
  const workerTotalMs = performance.now() - workerStarted
  const profile: WorkerTimingProfile = {
    batchId: request.batchId,
    inputBytes: request.pixels.byteLength,
    outputBytes: vectors.byteLength,
    workerReceivedAtEpochMs,
    workerRespondedAtEpochMs: 0,
    transferToWorkerMs,
    queueWaitMs,
    rawImageWrapMs,
    preprocessMs,
    modelAwaitMs,
    normalizeMs,
    readbackToListMs,
    flattenMs,
    workerTotalMs,
  }
  return {
    type: 'result',
    requestId: request.requestId,
    vectors,
    count: request.count,
    dimension,
    profile,
  }
}

async function handleMessage(request: WorkerRequest) {
  try {
    if (request.type === 'init') {
      const ready = await initialize(request)
      post({ type: 'ready', requestId: request.requestId, ...ready })
      return
    }

    const result = await runInference(request)
    result.profile.workerRespondedAtEpochMs = Date.now()
    logWorkerBatch(result.profile, result.count, result.dimension)
    post(result, [result.vectors])
  } catch (error) {
    post({ type: 'error', requestId: request.requestId, message: errorMessage(error) })
  }
}

scope.onmessage = (event) => {
  const request = event.data
  if (request.type === 'init') {
    void handleMessage(request)
    return
  }

  request.receivedAtEpochMs = Date.now()
  request.receivedAtPerfMs = performance.now()
  inferenceQueue = inferenceQueue
    .then(() => handleMessage(request))
    .catch(() => undefined)
}

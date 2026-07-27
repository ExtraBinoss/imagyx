import {
  AutoProcessor,
  CLIPVisionModelWithProjection,
  RawImage,
  env,
} from '@huggingface/transformers'

const IMAGE_EDGE = 224
const IMAGE_CHANNELS = 3
const IMAGE_BYTES = IMAGE_EDGE * IMAGE_EDGE * IMAGE_CHANNELS

type Device = 'webgpu' | 'wasm'

type InitRequest = {
  type: 'init'
  requestId: number
  modelId: string
  localModelPath: string
}

type InferRequest = {
  type: 'infer'
  requestId: number
  pixels: ArrayBuffer
  count: number
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
}

type ResultResponse = {
  type: 'result'
  requestId: number
  vectors: ArrayBuffer
  count: number
  dimension: number
  preprocessMs: number
  inferenceMs: number
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

async function resetVisionRuntime() {
  try {
    await visionModel?.dispose?.()
  } catch {
    // A partially initialized backend may not be disposable.
  }
  visionModel = null
  processor = null
}

async function loadForDevice(modelId: string, target: Device) {
  const options = { device: target, dtype: 'fp32', local_files_only: true } as const
  ;[processor, visionModel] = await Promise.all([
    AutoProcessor.from_pretrained(modelId, options),
    CLIPVisionModelWithProjection.from_pretrained(modelId, options),
  ])

  const blank = new RawImage(
    new Uint8Array(IMAGE_BYTES),
    IMAGE_EDGE,
    IMAGE_EDGE,
    IMAGE_CHANNELS,
  )
  await visionModel(await processor(blank))
}

async function initialize(request: InitRequest) {
  if (!initialization) {
    initialization = (async () => {
      env.allowLocalModels = true
      env.allowRemoteModels = false
      env.localModelPath = request.localModelPath
      env.useBrowserCache = false

      try {
        device = 'webgpu'
        await loadForDevice(request.modelId, device)
        return {
          device,
          backendEffective: 'Transformers.js Worker · WebGPU',
          accelerationActive: true,
          accelerationLabel: 'GPU WebGPU · worker',
        }
      } catch (webGpuError) {
        await resetVisionRuntime()
        device = 'wasm'
        await loadForDevice(request.modelId, device)
        return {
          device,
          backendEffective: 'Transformers.js Worker · WASM',
          accelerationActive: false,
          accelerationLabel: 'CPU WASM · worker',
          fallbackReason: `WebGPU worker indisponible: ${errorMessage(webGpuError)}`,
        }
      }
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

  const images = Array.from({ length: request.count }, (_, index) => new RawImage(
    new Uint8Array(request.pixels, index * IMAGE_BYTES, IMAGE_BYTES),
    IMAGE_EDGE,
    IMAGE_EDGE,
    IMAGE_CHANNELS,
  ))

  const preprocessStarted = performance.now()
  const inputs = await processor(images.length === 1 ? images[0] : images)
  const preprocessMs = performance.now() - preprocessStarted

  const inferenceStarted = performance.now()
  const output = await visionModel(inputs)
  const rows = output.image_embeds.normalize().tolist() as number[][]
  const inferenceMs = performance.now() - inferenceStarted

  if (rows.length !== request.count) {
    throw new Error(`Le modèle a retourné ${rows.length} vecteurs pour ${request.count} images`)
  }
  const dimension = rows[0]?.length ?? 0
  if (dimension <= 0 || rows.some((row) => row.length !== dimension)) {
    throw new Error('Dimensions de vecteurs incohérentes')
  }

  const flat = new Float32Array(request.count * dimension)
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index]
    if (row) flat.set(row, index * dimension)
  }
  const vectors = flat.buffer as ArrayBuffer
  return {
    type: 'result',
    requestId: request.requestId,
    vectors,
    count: request.count,
    dimension,
    preprocessMs,
    inferenceMs,
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

  inferenceQueue = inferenceQueue
    .then(() => handleMessage(request))
    .catch(() => undefined)
}

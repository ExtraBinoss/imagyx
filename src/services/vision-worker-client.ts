export type VisionDevice = 'webgpu' | 'wasm'

export interface VisionWorkerReady {
  device: VisionDevice
  backendEffective: string
  accelerationActive: boolean
  accelerationLabel: string
  fallbackReason?: string
  initProfile: {
    processorLoadMs: number
    modelLoadMs: number
    warmupPreprocessMs: number
    warmupModelMs: number
    totalMs: number
    hardwareConcurrency: number
    webGpuExposed: boolean
  }
}

export interface VisionInferenceProfile {
  requestId: number
  batchId: string
  count: number
  inputBytes: number
  outputBytes: number
  transferToWorkerMs: number
  queueWaitMs: number
  rawImageWrapMs: number
  preprocessMs: number
  modelAwaitMs: number
  normalizeMs: number
  readbackToListMs: number
  flattenMs: number
  workerTotalMs: number
  returnToMainMs: number
  roundTripMs: number
  deserializeMs: number
  clientTotalMs: number
}

export interface VisionInferenceResult {
  vectors: number[][]
  profile: VisionInferenceProfile
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
}

type WorkerRequest = InitRequest | InferRequest

type ReadyResponse = VisionWorkerReady & {
  type: 'ready'
  requestId: number
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
type SuccessfulResponse = Exclude<WorkerResponse, ErrorResponse>

type TimedResponse<T extends SuccessfulResponse> = {
  response: T
  roundTripMs: number
  receivedAtEpochMs: number
}

type PendingRequest = {
  sentAtPerfMs: number
  resolve: (response: SuccessfulResponse, roundTripMs: number, receivedAtEpochMs: number) => void
  reject: (error: Error) => void
}

// Every surface and feature uses this shared module-level worker. Creating a
// client is cheap and never loads a second MobileCLIP model into memory.
let sharedWorker: Worker | null = null
let sharedInitialization: Promise<VisionWorkerReady> | null = null
const sharedPending = new Map<number, PendingRequest>()
let sharedNextRequestId = 1

export class VisionWorkerClient {
  initialize(modelId: string, localModelPath: string): Promise<VisionWorkerReady> {
    if (!sharedInitialization) {
      sharedInitialization = this.request<ReadyResponse>((requestId) => ({
        type: 'init',
        requestId,
        modelId,
        localModelPath,
      })).then(({ response }) => {
        const { type: _type, requestId: _requestId, ...ready } = response
        console.info('[Imagyx][VisionWorker] runtime ready', ready)
        return ready
      })
    }
    return sharedInitialization
  }

  async infer(pixels: ArrayBuffer, count: number, batchId: string): Promise<VisionInferenceResult> {
    const sentAtEpochMs = Date.now()
    const { response, roundTripMs, receivedAtEpochMs } = await this.request<ResultResponse>(
      (requestId) => ({ type: 'infer', requestId, batchId, sentAtEpochMs, pixels, count }),
      [pixels],
    )
    if (response.count !== count || response.dimension <= 0) {
      throw new Error('Réponse invalide du worker vision')
    }

    const deserializeStarted = performance.now()
    const flat = new Float32Array(response.vectors)
    const expectedLength = response.count * response.dimension
    if (flat.length !== expectedLength) {
      throw new Error(`Vecteurs invalides: ${flat.length} valeurs au lieu de ${expectedLength}`)
    }

    const vectors = Array.from({ length: response.count }, (_, index) => {
      const start = index * response.dimension
      return Array.from(flat.subarray(start, start + response.dimension))
    })
    const deserializeMs = performance.now() - deserializeStarted
    const returnToMainMs = Math.max(0, receivedAtEpochMs - response.profile.workerRespondedAtEpochMs)
    return {
      vectors,
      profile: {
        requestId: response.requestId,
        batchId: response.profile.batchId,
        count: response.count,
        inputBytes: response.profile.inputBytes,
        outputBytes: response.profile.outputBytes,
        transferToWorkerMs: response.profile.transferToWorkerMs,
        queueWaitMs: response.profile.queueWaitMs,
        rawImageWrapMs: response.profile.rawImageWrapMs,
        preprocessMs: response.profile.preprocessMs,
        modelAwaitMs: response.profile.modelAwaitMs,
        normalizeMs: response.profile.normalizeMs,
        readbackToListMs: response.profile.readbackToListMs,
        flattenMs: response.profile.flattenMs,
        workerTotalMs: response.profile.workerTotalMs,
        returnToMainMs,
        roundTripMs,
        deserializeMs,
        clientTotalMs: roundTripMs + deserializeMs,
      },
    }
  }

  private request<T extends SuccessfulResponse>(
    buildRequest: (requestId: number) => WorkerRequest,
    transfer: Transferable[] = [],
  ): Promise<TimedResponse<T>> {
    const worker = this.ensureWorker()
    const requestId = sharedNextRequestId
    sharedNextRequestId += 1

    return new Promise<TimedResponse<T>>((resolve, reject) => {
      sharedPending.set(requestId, {
        sentAtPerfMs: performance.now(),
        resolve: (response, roundTripMs, receivedAtEpochMs) => resolve({
          response: response as T,
          roundTripMs,
          receivedAtEpochMs,
        }),
        reject,
      })
      worker.postMessage(buildRequest(requestId), transfer)
    })
  }

  private ensureWorker(): Worker {
    if (sharedWorker) return sharedWorker

    const worker = new Worker(new URL('../workers/vision-worker.ts', import.meta.url), {
      type: 'module',
      name: 'imagyx-vision',
    })
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const receivedAtEpochMs = Date.now()
      const response = event.data
      const pending = sharedPending.get(response.requestId)
      if (!pending) return
      sharedPending.delete(response.requestId)
      const roundTripMs = performance.now() - pending.sentAtPerfMs
      if (response.type === 'error') {
        pending.reject(new Error(response.message))
      } else {
        pending.resolve(response, roundTripMs, receivedAtEpochMs)
      }
    }
    worker.onerror = (event) => {
      this.failWorker(new Error(event.message || 'Le worker vision a échoué'))
    }
    worker.onmessageerror = () => {
      this.failWorker(new Error('Message invalide reçu du worker vision'))
    }
    sharedWorker = worker
    return worker
  }

  private failWorker(error: Error) {
    for (const pending of sharedPending.values()) pending.reject(error)
    sharedPending.clear()
    sharedWorker?.terminate()
    sharedWorker = null
    sharedInitialization = null
  }
}

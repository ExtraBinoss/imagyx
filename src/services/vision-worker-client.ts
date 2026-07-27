export type VisionDevice = 'webgpu' | 'wasm'

export interface VisionWorkerReady {
  device: VisionDevice
  backendEffective: string
  accelerationActive: boolean
  accelerationLabel: string
  fallbackReason?: string
}

export interface VisionInferenceResult {
  vectors: number[][]
  preprocessMs: number
  inferenceMs: number
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
  pixels: ArrayBuffer
  count: number
}

type WorkerRequest = InitRequest | InferRequest

type ReadyResponse = VisionWorkerReady & {
  type: 'ready'
  requestId: number
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

type SuccessfulResponse = Exclude<WorkerResponse, ErrorResponse>

type PendingRequest = {
  resolve: (response: SuccessfulResponse) => void
  reject: (error: Error) => void
}

export class VisionWorkerClient {
  private worker: Worker | null = null
  private initialization: Promise<VisionWorkerReady> | null = null
  private readonly pending = new Map<number, PendingRequest>()
  private nextRequestId = 1

  initialize(modelId: string, localModelPath: string): Promise<VisionWorkerReady> {
    if (!this.initialization) {
      this.initialization = this.request<ReadyResponse>((requestId) => ({
        type: 'init',
        requestId,
        modelId,
        localModelPath,
      })).then(({ type: _type, requestId: _requestId, ...ready }) => ready)
    }
    return this.initialization
  }

  async infer(pixels: ArrayBuffer, count: number): Promise<VisionInferenceResult> {
    const response = await this.request<ResultResponse>(
      (requestId) => ({ type: 'infer', requestId, pixels, count }),
      [pixels],
    )
    if (response.count !== count || response.dimension <= 0) {
      throw new Error('Réponse invalide du worker vision')
    }

    const flat = new Float32Array(response.vectors)
    const expectedLength = response.count * response.dimension
    if (flat.length !== expectedLength) {
      throw new Error(`Vecteurs invalides: ${flat.length} valeurs au lieu de ${expectedLength}`)
    }

    const vectors = Array.from({ length: response.count }, (_, index) => {
      const start = index * response.dimension
      return Array.from(flat.subarray(start, start + response.dimension))
    })
    return {
      vectors,
      preprocessMs: response.preprocessMs,
      inferenceMs: response.inferenceMs,
    }
  }

  private request<T extends SuccessfulResponse>(
    buildRequest: (requestId: number) => WorkerRequest,
    transfer: Transferable[] = [],
  ): Promise<T> {
    const worker = this.ensureWorker()
    const requestId = this.nextRequestId
    this.nextRequestId += 1

    return new Promise<T>((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (response) => resolve(response as T),
        reject,
      })
      worker.postMessage(buildRequest(requestId), transfer)
    })
  }

  private ensureWorker(): Worker {
    if (this.worker) return this.worker

    const worker = new Worker(new URL('../workers/vision-worker.ts', import.meta.url), {
      type: 'module',
      name: 'imagyx-vision',
    })
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const response = event.data
      const pending = this.pending.get(response.requestId)
      if (!pending) return
      this.pending.delete(response.requestId)
      if (response.type === 'error') {
        pending.reject(new Error(response.message))
      } else {
        pending.resolve(response)
      }
    }
    worker.onerror = (event) => {
      this.failWorker(new Error(event.message || 'Le worker vision a échoué'))
    }
    worker.onmessageerror = () => {
      this.failWorker(new Error('Message invalide reçu du worker vision'))
    }
    this.worker = worker
    return worker
  }

  private failWorker(error: Error) {
    for (const pending of this.pending.values()) pending.reject(error)
    this.pending.clear()
    this.worker?.terminate()
    this.worker = null
    this.initialization = null
  }
}

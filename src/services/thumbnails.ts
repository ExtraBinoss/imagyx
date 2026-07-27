import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { perfSample } from '../utils'

const MAX_CONCURRENT_REQUESTS = 4
const MAX_QUEUED_REQUESTS = 128
const MEMORY_CACHE_CAPACITY = 8_192

export interface ThumbnailRequestOptions {
  priority?: number
  signal?: AbortSignal
}

type ThumbnailAsset = Pick<ImageAsset, 'id' | 'path' | 'modifiedAt'> &
  Partial<Pick<ImageAsset, 'thumbnailPath'>>

interface Subscriber {
  active: boolean
  resolve: (url: string) => void
  reject: (reason: unknown) => void
  cleanup: () => void
}

interface QueueTask {
  key: string
  image: ThumbnailAsset
  priority: number
  queuedAt: number
  state: 'queued' | 'active'
  subscribers: Set<Subscriber>
}

const urls = new Map<string, string>()
const tasks = new Map<string, QueueTask>()
const queue: QueueTask[] = []
let activeRequests = 0

function keyFor(image: Pick<ImageAsset, 'id' | 'modifiedAt'>): string {
  return `${image.id}:${image.modifiedAt}`
}

function abortError(message = 'Thumbnail request cancelled'): DOMException {
  return new DOMException(message, 'AbortError')
}

function remember(key: string, url: string): void {
  urls.delete(key)
  urls.set(key, url)
  while (urls.size > MEMORY_CACHE_CAPACITY) {
    const oldest = urls.keys().next().value as string | undefined
    if (!oldest) break
    urls.delete(oldest)
  }
}

export function peekThumbnail(image: ThumbnailAsset): string | null {
  const key = keyFor(image)
  const cached = urls.get(key)
  if (cached) {
    remember(key, cached)
    return cached
  }
  if (!image.thumbnailPath) return null
  const url = imagyxApi.fileUrl(image.thumbnailPath)
  remember(key, url)
  return url
}

export function forgetThumbnail(image: ThumbnailAsset): void {
  urls.delete(keyFor(image))
  image.thumbnailPath = ''
}

function removeQueuedTask(task: QueueTask): void {
  const index = queue.indexOf(task)
  if (index >= 0) queue.splice(index, 1)
  tasks.delete(task.key)
}

function rejectTask(task: QueueTask, reason: unknown): void {
  removeQueuedTask(task)
  for (const subscriber of task.subscribers) {
    if (!subscriber.active) continue
    subscriber.active = false
    subscriber.cleanup()
    subscriber.reject(reason)
  }
  task.subscribers.clear()
}

function makeRoom(): void {
  while (queue.length >= MAX_QUEUED_REQUESTS) {
    let worstIndex = 0
    for (let index = 1; index < queue.length; index += 1) {
      const current = queue[index]
      const worst = queue[worstIndex]
      if (!current || !worst) continue
      if (current.priority > worst.priority ||
        (current.priority === worst.priority && current.queuedAt < worst.queuedAt)) {
        worstIndex = index
      }
    }
    const dropped = queue[worstIndex]
    if (!dropped) return
    rejectTask(dropped, abortError('Thumbnail request superseded by the current viewport'))
  }
}

function nextTask(): QueueTask | undefined {
  if (queue.length === 0) return undefined
  let bestIndex = 0
  for (let index = 1; index < queue.length; index += 1) {
    const current = queue[index]
    const best = queue[bestIndex]
    if (!current || !best) continue
    if (current.priority < best.priority ||
      (current.priority === best.priority && current.queuedAt < best.queuedAt)) {
      bestIndex = index
    }
  }
  return queue.splice(bestIndex, 1)[0]
}

function settle(task: QueueTask, url: string): void {
  for (const subscriber of task.subscribers) {
    if (!subscriber.active) continue
    subscriber.active = false
    subscriber.cleanup()
    subscriber.resolve(url)
  }
  task.subscribers.clear()
}

function fail(task: QueueTask, error: unknown): void {
  for (const subscriber of task.subscribers) {
    if (!subscriber.active) continue
    subscriber.active = false
    subscriber.cleanup()
    subscriber.reject(error)
  }
  task.subscribers.clear()
}

async function runTask(task: QueueTask): Promise<void> {
  task.state = 'active'
  const startedAt = performance.now()
  perfSample('Thumbnail', 'queue wait', startedAt - task.queuedAt)
  try {
    const mockUrl = (task.image as unknown as { thumbnail_url?: string; preview_url?: string }).thumbnail_url ||
      (task.image as unknown as { thumbnail_url?: string; preview_url?: string }).preview_url
    if (mockUrl) {
      remember(task.key, mockUrl)
      settle(task, mockUrl)
      return
    }

    const path = await imagyxApi.thumbnail(task.image)
    task.image.thumbnailPath = path
    const url = imagyxApi.fileUrl(path)
    remember(task.key, url)
    perfSample('Thumbnail', 'IPC and generation', performance.now() - startedAt)
    settle(task, url)
  } catch (error) {
    perfSample('Thumbnail', 'failed request', performance.now() - startedAt)
    fail(task, error)
  } finally {
    tasks.delete(task.key)
  }
}

function pump(): void {
  while (activeRequests < MAX_CONCURRENT_REQUESTS) {
    const task = nextTask()
    if (!task) return
    if (task.subscribers.size === 0) {
      tasks.delete(task.key)
      continue
    }
    activeRequests += 1
    void runTask(task).finally(() => {
      activeRequests -= 1
      pump()
    })
  }
}

function subscribe(task: QueueTask, signal?: AbortSignal): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    if (signal?.aborted) {
      reject(abortError())
      return
    }

    const subscriber: Subscriber = {
      active: true,
      resolve,
      reject,
      cleanup: () => undefined,
    }
    const abort = () => {
      if (!subscriber.active) return
      subscriber.active = false
      subscriber.cleanup()
      task.subscribers.delete(subscriber)
      reject(abortError())
      if (task.state === 'queued' && task.subscribers.size === 0) {
        removeQueuedTask(task)
      }
    }
    subscriber.cleanup = () => signal?.removeEventListener('abort', abort)
    signal?.addEventListener('abort', abort, { once: true })
    task.subscribers.add(subscriber)
  })
}

export function requestThumbnail(
  image: ThumbnailAsset,
  options: ThumbnailRequestOptions = {},
): Promise<string> {
  const cached = peekThumbnail(image)
  if (cached) return Promise.resolve(cached)
  if (options.signal?.aborted) return Promise.reject(abortError())

  const key = keyFor(image)
  const priority = Number.isFinite(options.priority) ? options.priority ?? 0 : 0
  const existing = tasks.get(key)
  if (existing) {
    existing.priority = Math.min(existing.priority, priority)
    return subscribe(existing, options.signal)
  }

  makeRoom()
  const task: QueueTask = {
    key,
    image,
    priority,
    queuedAt: performance.now(),
    state: 'queued',
    subscribers: new Set(),
  }
  tasks.set(key, task)
  queue.push(task)
  const request = subscribe(task, options.signal)
  pump()
  return request
}
